//! Application state, main loop and input handling.

use std::time::{Duration, Instant};

use color_eyre::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::TableState;

use crate::sysmon::{Monitor, ProcRow};
use crate::theme::{THEMES, Theme};
use crate::ui;

/// Animation frame period (~12 fps): enough for blinking, waves and the ticker.
const FRAME: Duration = Duration::from_millis(80);
/// How long the boot sequence runs before it gets out of the way by itself.
pub const BOOT_TIME: Duration = Duration::from_millis(3200);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortKey {
    Cpu,
    Mem,
    Pid,
    Name,
    Time,
}

impl SortKey {
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Cpu => "CPU",
            SortKey::Mem => "MEM",
            SortKey::Pid => "PID",
            SortKey::Name => "NAME",
            SortKey::Time => "TIME",
        }
    }

    fn next(self) -> Self {
        match self {
            SortKey::Cpu => SortKey::Mem,
            SortKey::Mem => SortKey::Pid,
            SortKey::Pid => SortKey::Name,
            SortKey::Name => SortKey::Time,
            SortKey::Time => SortKey::Cpu,
        }
    }
}

pub struct KillPrompt {
    pub pid: u32,
    pub name: String,
}

pub struct App {
    pub mon: Monitor,
    pub theme_idx: usize,
    pub sort: SortKey,
    pub sort_desc: bool,
    pub table: TableState,
    pub visible: Vec<ProcRow>,
    pub filter: String,
    pub filter_mode: bool,
    pub show_help: bool,
    pub kill: Option<KillPrompt>,
    pub effects: bool,
    pub paused: bool,
    pub boot_start: Instant,
    pub booting: bool,
    pub frame: u64,
    pub interval: Duration,
    pub status: Option<(String, Instant)>,
    pub rng: u64,
    selected_pid: Option<u32>,
    last_refresh: Instant,
    quit: bool,
}

impl App {
    pub fn new(boot: bool) -> Result<Self> {
        let mon = Monitor::new()?;
        let mut app = Self {
            mon,
            theme_idx: 0,
            sort: SortKey::Cpu,
            sort_desc: true,
            table: TableState::default().with_selected(Some(0)),
            visible: Vec::new(),
            filter: String::new(),
            filter_mode: false,
            show_help: false,
            kill: None,
            effects: true,
            paused: false,
            boot_start: Instant::now(),
            booting: boot,
            frame: 0,
            interval: Duration::from_millis(1000),
            status: None,
            rng: 0x9E37_79B9_7F4A_7C15 ^ std::process::id() as u64,
            selected_pid: None,
            last_refresh: Instant::now(),
            quit: false,
        };
        app.rebuild();
        Ok(app)
    }

    pub fn theme(&self) -> Theme {
        THEMES[self.theme_idx]
    }

    /// xorshift64: enough randomness for flicker and glitches, no dependency needed.
    pub fn rand(&mut self) -> f64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.quit {
            terminal.draw(|f| ui::draw(f, self))?;
            let deadline = Instant::now() + FRAME;
            while let Some(left) = deadline.checked_duration_since(Instant::now()) {
                if !event::poll(left)? {
                    break;
                }
                if let Event::Key(key) = event::read()?
                    && key.kind == KeyEventKind::Press {
                        self.on_key(key);
                    }
                if self.quit {
                    return Ok(());
                }
            }
            self.frame = self.frame.wrapping_add(1);
            if self.booting && self.boot_start.elapsed() >= BOOT_TIME {
                self.booting = false;
            }
            if !self.paused && self.last_refresh.elapsed() >= self.interval {
                self.last_refresh = Instant::now();
                self.mon.refresh();
                self.rebuild();
            }
            if self.status.as_ref().is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(4)) {
                self.status = None;
            }
        }
        Ok(())
    }

    fn flash(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), Instant::now()));
    }

    /// Sorts and filters the process list, keeping the selection on the same PID.
    pub fn rebuild(&mut self) {
        let needle = self.filter.to_lowercase();
        let mut rows: Vec<ProcRow> = self
            .mon
            .procs
            .iter()
            .filter(|p| {
                needle.is_empty()
                    || p.cmd.to_lowercase().contains(&needle)
                    || p.user.to_lowercase().contains(&needle)
                    || p.pid.to_string().contains(&needle)
            })
            .cloned()
            .collect();
        rows.sort_by(|a, b| {
            let o = match self.sort {
                SortKey::Cpu => a.cpu.total_cmp(&b.cpu),
                SortKey::Mem => a.mem.cmp(&b.mem),
                SortKey::Pid => a.pid.cmp(&b.pid),
                SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortKey::Time => a.run_time.cmp(&b.run_time),
            };
            if self.sort_desc { o.reverse() } else { o }
        });
        self.visible = rows;
        let idx = self
            .selected_pid
            .and_then(|pid| self.visible.iter().position(|p| p.pid == pid))
            .unwrap_or_else(|| self.table.selected().unwrap_or(0).min(self.visible.len().saturating_sub(1)));
        self.select(idx);
    }

    fn select(&mut self, idx: usize) {
        if self.visible.is_empty() {
            self.table.select(None);
            self.selected_pid = None;
        } else {
            let idx = idx.min(self.visible.len() - 1);
            self.table.select(Some(idx));
            self.selected_pid = Some(self.visible[idx].pid);
        }
    }

    fn move_by(&mut self, delta: isize) {
        let cur = self.table.selected().unwrap_or(0) as isize;
        self.select((cur + delta).max(0) as usize);
    }

    fn on_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        if self.booting {
            self.booting = false;
            return;
        }
        if let Some(k) = &self.kill {
            let (pid, name) = (k.pid, k.name.clone());
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => self.terminate(pid, &name, false),
                KeyCode::Char('9') => self.terminate(pid, &name, true),
                _ => self.flash("TERMINATION ABORTED"),
            }
            self.kill = None;
            return;
        }
        if self.filter_mode {
            match key.code {
                KeyCode::Esc => {
                    self.filter.clear();
                    self.filter_mode = false;
                }
                KeyCode::Enter => self.filter_mode = false,
                KeyCode::Backspace => {
                    self.filter.pop();
                }
                KeyCode::Char(c) => self.filter.push(c),
                _ => {}
            }
            self.rebuild();
            return;
        }
        if self.show_help {
            self.show_help = false;
            return;
        }
        let page = 10;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::F(1) => self.show_help = true,
            KeyCode::Char('t') => {
                self.theme_idx = (self.theme_idx + 1) % THEMES.len();
                self.flash(format!("PALETTE ▸ {}", self.theme().name));
            }
            KeyCode::Char('x') => {
                self.effects = !self.effects;
                self.flash(if self.effects { "CRT EMULATION ▸ ON" } else { "CRT EMULATION ▸ OFF" });
            }
            KeyCode::Char(' ') => {
                self.paused = !self.paused;
                self.flash(if self.paused { "TELEMETRY ▸ FROZEN" } else { "TELEMETRY ▸ LIVE" });
            }
            KeyCode::Char('s') | KeyCode::Tab => {
                self.sort = self.sort.next();
                self.sort_desc = !matches!(self.sort, SortKey::Pid | SortKey::Name);
                self.rebuild();
            }
            KeyCode::Char('c') => self.set_sort(SortKey::Cpu),
            KeyCode::Char('m') => self.set_sort(SortKey::Mem),
            KeyCode::Char('p') => self.set_sort(SortKey::Pid),
            KeyCode::Char('n') => self.set_sort(SortKey::Name),
            KeyCode::Char('r') => {
                self.sort_desc = !self.sort_desc;
                self.rebuild();
            }
            KeyCode::Char('/') | KeyCode::Char('f') => self.filter_mode = true,
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.interval = (self.interval + Duration::from_millis(250)).min(Duration::from_secs(10));
                self.flash(format!("SAMPLE RATE ▸ {} ms", self.interval.as_millis()));
            }
            KeyCode::Char('-') => {
                self.interval = self
                    .interval
                    .saturating_sub(Duration::from_millis(250))
                    .max(Duration::from_millis(250));
                self.flash(format!("SAMPLE RATE ▸ {} ms", self.interval.as_millis()));
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_by(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_by(-1),
            KeyCode::PageDown => self.move_by(page),
            KeyCode::PageUp => self.move_by(-page),
            KeyCode::Home | KeyCode::Char('g') => self.select(0),
            KeyCode::End | KeyCode::Char('G') => self.select(usize::MAX),
            KeyCode::Delete | KeyCode::Char('K') | KeyCode::F(9) => {
                if let Some(p) = self.table.selected().and_then(|i| self.visible.get(i)) {
                    self.kill = Some(KillPrompt { pid: p.pid, name: p.name.clone() });
                }
            }
            _ => {}
        }
    }

    fn set_sort(&mut self, key: SortKey) {
        if self.sort == key {
            self.sort_desc = !self.sort_desc;
        } else {
            self.sort = key;
            self.sort_desc = !matches!(key, SortKey::Pid | SortKey::Name);
        }
        self.rebuild();
    }

    fn terminate(&mut self, pid: u32, name: &str, force: bool) {
        let sig = if force { "SIGKILL" } else { "SIGTERM" };
        match self.mon.signal(pid, force) {
            Some(true) => self.flash(format!("{sig} DISPATCHED ▸ PID {pid} ({name})")),
            Some(false) => self.flash(format!("ACCESS DENIED ▸ PID {pid} ({name})")),
            None => self.flash(format!("PID {pid} NO LONGER ON THE MANIFEST")),
        }
    }
}
