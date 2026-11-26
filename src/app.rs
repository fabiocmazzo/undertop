//! Application state, main loop and input handling.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::time::{Duration, Instant};

use color_eyre::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::TableState;

use crate::config::Settings;
use crate::sysmon::{Dossier, Monitor, ProcRow};
use crate::theme::{THEMES, Theme};
use crate::ui;

/// Animation frame period (~12 fps): enough for blinking, waves and the ticker.
const FRAME: Duration = Duration::from_millis(80);
/// How long the boot sequence runs before it gets out of the way by itself.
pub const BOOT_TIME: Duration = Duration::from_millis(3200);
/// The CRT power-down animation after `q`.
pub const SHUTDOWN_TIME: Duration = Duration::from_millis(900);
/// How long a lockdown lasts if nobody acknowledges it.
pub const LOCKDOWN_TIME: Duration = Duration::from_secs(8);
/// A heat-triggered lockdown happens at most this often.
const LOCKDOWN_COOLDOWN: Duration = Duration::from_secs(600);
/// How long each ticker alert stays on screen.
const ALERT_TIME: Duration = Duration::from_secs(6);
/// Index of the LOCKDOWN palette in `THEMES`.
const LOCKDOWN_THEME: usize = 3;

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

    fn cmp(self, desc: bool, a: &ProcRow, b: &ProcRow) -> Ordering {
        let o = match self {
            SortKey::Cpu => a.cpu.total_cmp(&b.cpu),
            SortKey::Mem => a.mem.cmp(&b.mem),
            SortKey::Pid => a.pid.cmp(&b.pid),
            SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortKey::Time => a.run_time.cmp(&b.run_time),
        };
        if desc { o.reverse() } else { o }
    }
}

pub struct KillPrompt {
    pub pid: u32,
    pub name: String,
}

pub struct Lockdown {
    pub start: Instant,
    pub reason: String,
    prev_theme: usize,
    last_bell: Option<Instant>,
}

pub struct App {
    pub mon: Monitor,
    pub settings: Settings,
    pub theme_idx: usize,
    pub sort: SortKey,
    pub sort_desc: bool,
    pub tree: bool,
    pub table: TableState,
    pub visible: Vec<ProcRow>,
    pub filter: String,
    pub filter_mode: bool,
    pub show_help: bool,
    pub kill: Option<KillPrompt>,
    /// PID whose dossier is open, and its data (refreshed with the telemetry).
    pub dossier: Option<(u32, Option<Dossier>)>,
    pub lockdown: Option<Lockdown>,
    pub effects: bool,
    pub paused: bool,
    pub boot_start: Instant,
    pub booting: bool,
    pub shutdown: Option<Instant>,
    pub frame: u64,
    pub interval: Duration,
    pub status: Option<(String, Instant)>,
    /// Alert on the ticker right now, and the ones waiting.
    pub alert: Option<(String, Instant)>,
    alerts: VecDeque<String>,
    pub rng: u64,
    last_heat_lockdown: Option<Instant>,
    selected_pid: Option<u32>,
    last_refresh: Instant,
    bell: bool,
    quit: bool,
}

impl App {
    pub fn new(settings: Settings) -> Result<Self> {
        let mon = Monitor::new()?;
        let mut app = Self {
            mon,
            theme_idx: settings.theme_idx,
            sort: SortKey::Cpu,
            sort_desc: true,
            tree: false,
            table: TableState::default().with_selected(Some(0)),
            visible: Vec::new(),
            filter: String::new(),
            filter_mode: false,
            show_help: false,
            kill: None,
            dossier: None,
            lockdown: None,
            effects: settings.crt,
            paused: false,
            boot_start: Instant::now(),
            booting: settings.boot,
            shutdown: None,
            frame: 0,
            interval: settings.interval,
            status: None,
            alert: None,
            alerts: VecDeque::new(),
            rng: 0x9E37_79B9_7F4A_7C15 ^ std::process::id() as u64,
            last_heat_lockdown: None,
            selected_pid: None,
            last_refresh: Instant::now(),
            bell: false,
            quit: false,
            settings,
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
            if std::mem::take(&mut self.bell) {
                // BEL straight to the terminal, between two frames
                let mut out = std::io::stdout();
                out.write_all(b"\x07")?;
                out.flush()?;
            }
            let deadline = Instant::now() + FRAME;
            while let Some(left) = deadline.checked_duration_since(Instant::now()) {
                if !event::poll(left)? {
                    break;
                }
                if let Event::Key(key) = event::read()?
                    && key.kind == KeyEventKind::Press
                {
                    self.on_key(key);
                }
                if self.quit {
                    return Ok(());
                }
            }
            self.tick();
        }
        Ok(())
    }

    /// Everything time-driven between two frames.
    fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        if self.shutdown.is_some_and(|t| t.elapsed() >= SHUTDOWN_TIME) {
            self.quit = true;
            return;
        }
        if self.booting && self.boot_start.elapsed() >= BOOT_TIME {
            self.booting = false;
        }
        if !self.paused && self.last_refresh.elapsed() >= self.interval {
            self.last_refresh = Instant::now();
            self.mon.watch = self.dossier.as_ref().map(|(pid, _)| *pid);
            self.mon.refresh();
            self.rebuild();
            self.alerts.extend(self.mon.alerts.drain(..));
            if let Some((pid, _)) = self.dossier {
                self.dossier = Some((pid, self.mon.dossier(pid)));
            }
            self.check_heat();
        }
        if self
            .status
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(4))
        {
            self.status = None;
        }
        // a status message hides the alert line, so alerts only advance while it is visible
        if self.status.is_none()
            && self
                .alert
                .as_ref()
                .is_none_or(|(_, t)| t.elapsed() > ALERT_TIME)
        {
            self.alert = self.alerts.pop_front().map(|a| (a, Instant::now()));
        }
        if let Some(l) = &mut self.lockdown {
            if l.start.elapsed() >= LOCKDOWN_TIME {
                self.end_lockdown();
            } else if l
                .last_bell
                .is_none_or(|t| t.elapsed() >= Duration::from_secs(2))
            {
                l.last_bell = Some(Instant::now());
                self.bell = true;
            }
        }
    }

    fn check_heat(&mut self) {
        if !self.settings.lockdown_on_heat || self.lockdown.is_some() || self.booting {
            return;
        }
        if self
            .last_heat_lockdown
            .is_some_and(|t| t.elapsed() < LOCKDOWN_COOLDOWN)
        {
            return;
        }
        let hot = self
            .mon
            .temps
            .iter()
            .find(|s| s.temp >= 95.0 || s.temp >= s.crit * 0.95)
            .map(|s| {
                format!(
                    "THERMAL EXCURSION ▸ {} AT {:.0}°C",
                    s.label.to_uppercase(),
                    s.temp
                )
            });
        if let Some(reason) = hot {
            self.last_heat_lockdown = Some(Instant::now());
            self.start_lockdown(reason);
        }
    }

    pub(crate) fn start_lockdown(&mut self, reason: String) {
        let prev_theme = match &self.lockdown {
            Some(l) => l.prev_theme,
            None => self.theme_idx,
        };
        self.theme_idx = LOCKDOWN_THEME;
        self.lockdown = Some(Lockdown {
            start: Instant::now(),
            reason,
            prev_theme,
            last_bell: None,
        });
    }

    fn end_lockdown(&mut self) {
        if let Some(l) = self.lockdown.take() {
            self.theme_idx = l.prev_theme;
            self.flash("LOCKDOWN LIFTED ▸ RESUME YOUR DUTIES");
        }
    }

    fn flash(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), Instant::now()));
    }

    /// Sorts and filters the process list (flat or as a tree), keeping the selection on the same PID.
    pub fn rebuild(&mut self) {
        let needle = self.filter.to_lowercase();
        let matches = |p: &ProcRow| {
            needle.is_empty()
                || p.cmd.to_lowercase().contains(&needle)
                || p.user.to_lowercase().contains(&needle)
                || p.pid.to_string().contains(&needle)
        };
        let (sort, desc) = (self.sort, self.sort_desc);
        self.visible = if self.tree {
            build_tree(&self.mon.procs, &matches, |a, b| sort.cmp(desc, a, b))
        } else {
            let mut rows: Vec<ProcRow> = self
                .mon
                .procs
                .iter()
                .filter(|p| matches(p))
                .cloned()
                .collect();
            rows.sort_by(|a, b| sort.cmp(desc, a, b));
            rows
        };
        let idx = self
            .selected_pid
            .and_then(|pid| self.visible.iter().position(|p| p.pid == pid))
            .unwrap_or_else(|| {
                self.table
                    .selected()
                    .unwrap_or(0)
                    .min(self.visible.len().saturating_sub(1))
            });
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

    fn selected(&self) -> Option<&ProcRow> {
        self.table.selected().and_then(|i| self.visible.get(i))
    }

    fn request_kill(&mut self, pid: u32) {
        let name = self
            .mon
            .procs
            .iter()
            .find(|p| p.pid == pid)
            .map(|p| p.name.clone());
        if let Some(name) = name {
            self.kill = Some(KillPrompt { pid, name });
        }
    }

    fn leave(&mut self) {
        if self.settings.power_down {
            self.shutdown = Some(Instant::now());
        } else {
            self.quit = true;
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        if self.shutdown.is_some() {
            // impatient operators may skip the power-down
            self.quit = true;
            return;
        }
        if self.booting {
            self.booting = false;
            return;
        }
        if self.lockdown.is_some() {
            self.end_lockdown();
            return;
        }
        if let Some(k) = &self.kill {
            let (pid, name) = (k.pid, k.name.clone());
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    self.terminate(pid, &name, false)
                }
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
        if let Some((pid, _)) = self.dossier {
            match key.code {
                KeyCode::Delete | KeyCode::Char('K') | KeyCode::F(9) => self.request_kill(pid),
                KeyCode::Char('!') => {
                    self.start_lockdown("MANUAL OVERRIDE ▸ OPERATOR INITIATED".into())
                }
                _ => self.dossier = None,
            }
            return;
        }
        let page = 10;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.leave(),
            KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::F(1) => self.show_help = true,
            KeyCode::Char('!') => {
                self.start_lockdown("MANUAL OVERRIDE ▸ OPERATOR INITIATED".into())
            }
            KeyCode::Enter => {
                if let Some(pid) = self.selected().map(|p| p.pid) {
                    self.dossier = Some((pid, self.mon.dossier(pid)));
                }
            }
            KeyCode::Char('v') | KeyCode::F(5) => {
                self.tree = !self.tree;
                self.rebuild();
                self.flash(if self.tree {
                    "VIEW ▸ LINEAGE (TREE)"
                } else {
                    "VIEW ▸ MANIFEST (FLAT)"
                });
            }
            KeyCode::Char('t') => {
                self.theme_idx = (self.theme_idx + 1) % THEMES.len();
                self.flash(format!("PALETTE ▸ {}", self.theme().name));
            }
            KeyCode::Char('x') => {
                self.effects = !self.effects;
                self.flash(if self.effects {
                    "CRT EMULATION ▸ ON"
                } else {
                    "CRT EMULATION ▸ OFF"
                });
            }
            KeyCode::Char(' ') => {
                self.paused = !self.paused;
                self.flash(if self.paused {
                    "TELEMETRY ▸ FROZEN"
                } else {
                    "TELEMETRY ▸ LIVE"
                });
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
                self.interval =
                    (self.interval + Duration::from_millis(250)).min(Duration::from_secs(10));
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
                if let Some(pid) = self.selected().map(|p| p.pid) {
                    self.request_kill(pid);
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

/// Orders processes as a tree (parents before children, siblings sorted by `cmp`) and draws the
/// branches into `ProcRow::tree`. With a filter, keeps the matches and their ancestors.
fn build_tree(
    procs: &[ProcRow],
    matches: &dyn Fn(&ProcRow) -> bool,
    cmp: impl Fn(&ProcRow, &ProcRow) -> Ordering,
) -> Vec<ProcRow> {
    let by_pid: HashMap<u32, &ProcRow> = procs.iter().map(|p| (p.pid, p)).collect();
    let mut keep: HashSet<u32> = HashSet::new();
    for p in procs.iter().filter(|p| matches(p)) {
        let mut cur = Some(p);
        while let Some(c) = cur {
            if !keep.insert(c.pid) {
                break;
            }
            cur = c.ppid.and_then(|pp| by_pid.get(&pp).copied());
        }
    }
    let mut children: HashMap<u32, Vec<&ProcRow>> = HashMap::new();
    let mut roots: Vec<&ProcRow> = Vec::new();
    for p in procs.iter().filter(|p| keep.contains(&p.pid)) {
        match p.ppid.filter(|pp| keep.contains(pp) && *pp != p.pid) {
            Some(pp) => children.entry(pp).or_default().push(p),
            None => roots.push(p),
        }
    }
    roots.sort_by(|a, b| cmp(a, b));
    for list in children.values_mut() {
        list.sort_by(|a, b| cmp(a, b));
    }

    let mut out = Vec::with_capacity(keep.len());
    // (row, prefix for its children, branch drawn before it)
    let mut stack: Vec<(&ProcRow, String, String)> = roots
        .iter()
        .rev()
        .map(|r| (*r, String::new(), String::new()))
        .collect();
    while let Some((row, child_prefix, branch)) = stack.pop() {
        let mut r = row.clone();
        r.tree = branch;
        out.push(r);
        if let Some(kids) = children.get(&row.pid) {
            for (i, kid) in kids.iter().enumerate().rev() {
                let last = i == kids.len() - 1;
                let branch = format!("{child_prefix}{}", if last { "└─ " } else { "├─ " });
                let next = format!("{child_prefix}{}", if last { "   " } else { "│  " });
                stack.push((kid, next, branch));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pid: u32, ppid: Option<u32>, name: &str) -> ProcRow {
        ProcRow {
            pid,
            ppid,
            tree: String::new(),
            name: name.into(),
            cmd: name.into(),
            user: "root".into(),
            cpu: 0.0,
            mem: 0,
            status: "SLP",
            run_time: 0,
        }
    }

    #[test]
    fn tree_orders_and_draws_branches() {
        let procs = vec![
            row(1, None, "init"),
            row(10, Some(1), "shell"),
            row(11, Some(10), "vim"),
            row(12, Some(10), "make"),
            row(20, Some(1), "daemon"),
        ];
        let out = build_tree(&procs, &|_| true, |a, b| a.pid.cmp(&b.pid));
        let lines: Vec<String> = out
            .iter()
            .map(|r| format!("{}{}", r.tree, r.name))
            .collect();
        assert_eq!(
            lines,
            ["init", "├─ shell", "│  ├─ vim", "│  └─ make", "└─ daemon"]
        );
    }

    #[test]
    fn tree_filter_keeps_ancestors() {
        let procs = vec![
            row(1, None, "init"),
            row(10, Some(1), "shell"),
            row(11, Some(10), "vim"),
            row(20, Some(1), "daemon"),
        ];
        let out = build_tree(&procs, &|p| p.name == "vim", |a, b| a.pid.cmp(&b.pid));
        let names: Vec<&str> = out.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["init", "shell", "vim"]);
    }
}
