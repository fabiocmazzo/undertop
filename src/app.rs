//! Application state, main loop and input handling.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::time::{Duration, Instant};

use color_eyre::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::layout::{Position, Rect};
use ratatui::widgets::TableState;

use crate::config::Settings;
use crate::shift::ShiftStats;
use crate::sysmon::{Dossier, History, Monitor, ProcRow};
use crate::theme::{SECRET_THEME, THEMES, Theme};
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
/// The shift report signs itself out after this long.
pub const REPORT_TIME: Duration = Duration::from_secs(12);
/// How long each schematic page stays up before the next one.
pub const SCHEMATIC_PAGES: [Duration; 4] = [
    Duration::from_millis(3200),
    Duration::from_secs(9),
    Duration::from_secs(9),
    Duration::from_secs(9),
];
/// The last schematic page: what the facility says about operators who look at schematics.
pub const SAFEGUARD_PAGE: usize = 3;
/// Typed in the manifest, opens the facility schematic.
const SCHEMATIC_CODE: &str = "142";

/// A surprise inspection: a scan line sweeps the screen, then a stamp says how it went.
pub const INSPECTION_SCAN: Duration = Duration::from_millis(3600);
pub const INSPECTION_TIME: Duration = Duration::from_secs(8);
/// Inspections come at least this long apart, plus a random part of `INSPECTION_SPREAD`.
const INSPECTION_MIN: Duration = Duration::from_secs(10 * 60);
const INSPECTION_SPREAD: Duration = Duration::from_secs(40 * 60);
/// An old code, typed in the manifest, unlocks the secret palette.
const KONAMI: [KeyCode; 10] = [
    KeyCode::Up,
    KeyCode::Up,
    KeyCode::Down,
    KeyCode::Down,
    KeyCode::Left,
    KeyCode::Right,
    KeyCode::Left,
    KeyCode::Right,
    KeyCode::Char('b'),
    KeyCode::Char('a'),
];

pub struct Inspection {
    pub start: Instant,
    /// Set when the scan reaches the bottom: (passed, why).
    pub verdict: Option<(bool, String)>,
}

/// The facility schematic easter egg: a short sequence of screens.
pub struct Schematic {
    pub page: usize,
    pub page_start: Instant,
}

/// Two clicks on the same row within this time open its dossier.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// Where things were drawn in the last frame, so a mouse click can find them.
#[derive(Default, Debug)]
pub struct Hits {
    /// Rows of the manifest table (below its header).
    pub rows: Rect,
    /// Sortable header columns: (x from, x to, key), on line `header_y`.
    pub header_y: u16,
    pub columns: Vec<(u16, u16, SortKey)>,
    /// Footer keycaps: (x from, x to, key they press), on line `footer_y`.
    pub footer_y: u16,
    pub keys: Vec<(u16, u16, KeyCode)>,
    /// The dossier box, when open.
    pub dossier: Option<Rect>,
}

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
    /// CPU % and resident memory of the dossier's process, sampled since it was opened.
    pub dossier_cpu: History,
    pub dossier_mem: History,
    pub schematic: Option<Schematic>,
    pub inspection: Option<Inspection>,
    next_inspection: Instant,
    /// The last keys pressed in the manifest, for `KONAMI`.
    keys: Vec<KeyCode>,
    secret_unlocked: bool,
    /// The last digits typed in the manifest, for `SCHEMATIC_CODE`.
    typed: String,
    /// When the end-of-shift report appeared (after `q`).
    pub report: Option<Instant>,
    pub shift: ShiftStats,
    pub hits: Hits,
    last_click: Option<(Instant, usize)>,
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
            report: None,
            dossier_cpu: History::new(),
            dossier_mem: History::new(),
            schematic: None,
            inspection: None,
            // UNDERTOP_INSPECT=now schedules one right after start (for trying it out)
            next_inspection: Instant::now()
                + if std::env::var("UNDERTOP_INSPECT").as_deref() == Ok("now") {
                    Duration::from_secs(4)
                } else {
                    INSPECTION_MIN
                },
            keys: Vec::new(),
            secret_unlocked: settings.theme_idx == SECRET_THEME,
            typed: String::new(),
            shift: ShiftStats::new(),
            hits: Hits::default(),
            last_click: None,
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
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key(key),
                    Event::Mouse(m) => self.on_mouse(m),
                    _ => {}
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
        if self.shutdown.is_none() && self.report.is_some_and(|t| t.elapsed() >= REPORT_TIME) {
            self.power_down();
        }
        if let Some(s) = &self.schematic
            && s.page_start.elapsed() >= SCHEMATIC_PAGES[s.page]
        {
            self.next_schematic_page();
        }
        if self.booting && self.boot_start.elapsed() >= BOOT_TIME {
            self.booting = false;
        }
        self.tick_inspection();
        if !self.paused && self.last_refresh.elapsed() >= self.interval {
            self.last_refresh = Instant::now();
            self.mon.watch = self.dossier.as_ref().map(|(pid, _)| *pid);
            self.mon.refresh();
            self.rebuild();
            self.shift.alerts += self.mon.alerts.len() as u32;
            self.alerts.extend(self.mon.alerts.drain(..));
            self.shift.record(&self.mon);
            if let Some((pid, _)) = self.dossier {
                self.dossier = Some((pid, self.mon.dossier(pid)));
                self.sample_dossier();
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
            None => {
                self.shift.lockdowns += 1;
                self.theme_idx
            }
        };
        self.theme_idx = LOCKDOWN_THEME;
        self.schematic = None;
        self.inspection = None;
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

    /// `q`: first the shift report (if enabled), then the power-down (if enabled), then out.
    fn leave(&mut self) {
        if self.settings.shift_report && self.report.is_none() {
            self.report = Some(Instant::now());
        } else {
            self.power_down();
        }
    }

    fn unlock_secret(&mut self) {
        let first = !self.secret_unlocked;
        self.secret_unlocked = true;
        self.theme_idx = SECRET_THEME;
        self.bell = true;
        self.flash(if first {
            "OVERRIDE ACCEPTED ▸ SECRET PALETTE SAFEGUARD UNLOCKED FOR THIS SHIFT"
        } else {
            "PALETTE ▸ SAFEGUARD"
        });
        if first && self.settings.voice {
            crate::voice::say("Override accepted.");
        }
    }

    fn power_down(&mut self) {
        if self.settings.power_down {
            self.shutdown = Some(Instant::now());
        } else {
            self.quit = true;
        }
    }

    fn open_dossier(&mut self) {
        if let Some(pid) = self.selected().map(|p| p.pid) {
            self.dossier = Some((pid, self.mon.dossier(pid)));
            self.shift.dossiers += 1;
            self.dossier_cpu = History::new();
            self.dossier_mem = History::new();
            self.sample_dossier();
        }
    }

    fn sample_dossier(&mut self) {
        if let Some((_, Some(d))) = &self.dossier {
            self.dossier_cpu.push(d.row.cpu as f64);
            self.dossier_mem.push(d.row.mem as f64);
        }
    }

    /// Nothing else on screen that an inspection would cover or interrupt.
    fn console_is_clear(&self) -> bool {
        !self.booting
            && self.shutdown.is_none()
            && self.report.is_none()
            && self.schematic.is_none()
            && self.lockdown.is_none()
            && self.kill.is_none()
            && self.dossier.is_none()
            && !self.show_help
            && !self.filter_mode
    }

    fn tick_inspection(&mut self) {
        if let Some(i) = &self.inspection {
            let elapsed = i.start.elapsed();
            if i.verdict.is_none() && elapsed >= INSPECTION_SCAN {
                let verdict = self.inspect();
                if verdict.0 {
                    self.shift.inspections_passed += 1;
                } else {
                    self.shift.inspections_failed += 1;
                    self.bell = true;
                }
                if self.settings.voice {
                    crate::voice::say(if verdict.0 {
                        "Level one four two. Inspection passed."
                    } else {
                        "Level one four two. Inspection failed."
                    });
                }
                if let Some(i) = &mut self.inspection {
                    i.verdict = Some(verdict);
                }
            } else if elapsed >= INSPECTION_TIME {
                self.inspection = None;
            }
            return;
        }
        if Instant::now() < self.next_inspection {
            return;
        }
        if !self.settings.inspections && std::env::var("UNDERTOP_INSPECT").as_deref() != Ok("now") {
            self.schedule_inspection();
            return;
        }
        // an inspector never interrupts; they wait until the operator is done
        if self.console_is_clear() {
            self.start_inspection();
        }
    }

    pub(crate) fn start_inspection(&mut self) {
        self.inspection = Some(Inspection {
            start: Instant::now(),
            verdict: None,
        });
        self.schedule_inspection();
    }

    fn schedule_inspection(&mut self) {
        self.next_inspection =
            Instant::now() + INSPECTION_MIN + INSPECTION_SPREAD.mul_f64(self.rand());
    }

    /// What the inspector finds: zombies, a recent alert or a critical load fail the level.
    fn inspect(&self) -> (bool, String) {
        let zombies = self.mon.procs.iter().filter(|p| p.status == "ZMB").count();
        if zombies > 0 {
            let es = if zombies == 1 { "" } else { "ES" };
            return (
                false,
                format!("{zombies} ZOMBIE PROCESS{es} ON THE MANIFEST"),
            );
        }
        if self.alert.is_some() || !self.alerts.is_empty() {
            return (false, "AN UNRESOLVED ALERT ON THE TICKER".into());
        }
        if self.mon.cpu_total.last() > 90.0 {
            return (false, "REACTOR RUNNING ABOVE 90%".into());
        }
        (true, "NO ZOMBIES ▸ NO ALERTS ▸ REACTOR STABLE".into())
    }

    pub(crate) fn open_schematic(&mut self) {
        self.schematic = Some(Schematic {
            page: 0,
            page_start: Instant::now(),
        });
    }

    fn next_schematic_page(&mut self) {
        if let Some(s) = &mut self.schematic {
            if s.page + 1 < SCHEMATIC_PAGES.len() {
                s.page += 1;
                s.page_start = Instant::now();
                if s.page == SAFEGUARD_PAGE {
                    self.bell = true;
                    if self.settings.voice {
                        crate::voice::say("Attention. Safeguard will be activated.");
                    }
                }
            } else {
                self.schematic = None;
                self.flash("SCHEMATIC SEALED ▸ YOU SAW NOTHING");
            }
        }
    }

    fn on_mouse(&mut self, m: MouseEvent) {
        let at = Position::new(m.column, m.row);
        let click = matches!(m.kind, MouseEventKind::Down(MouseButton::Left));
        let scroll = match m.kind {
            MouseEventKind::ScrollDown => 3,
            MouseEventKind::ScrollUp => -3,
            _ => 0,
        };
        if !click && scroll == 0 {
            return;
        }
        // Overlays: a click dismisses what a key would, except the termination dialog, which
        // only the keyboard may answer.
        if self.shutdown.is_some() || self.kill.is_some() {
            return;
        }
        if self.booting
            || self.report.is_some()
            || self.schematic.is_some()
            || self.inspection.is_some()
            || self.lockdown.is_some()
            || self.show_help
        {
            if click {
                self.on_key(KeyEvent::from(KeyCode::Char(' ')));
            }
            return;
        }
        if self.dossier.is_some() {
            if click && !self.hits.dossier.is_some_and(|r| r.contains(at)) {
                self.dossier = None;
            }
            return;
        }
        if scroll != 0 {
            self.move_by(scroll);
            return;
        }
        let rows = self.hits.rows;
        if rows.contains(at) {
            let idx = self.table.offset() + (at.y - rows.y) as usize;
            if idx < self.visible.len() {
                let double = self
                    .last_click
                    .is_some_and(|(t, i)| i == idx && t.elapsed() <= DOUBLE_CLICK);
                self.select(idx);
                if double {
                    self.last_click = None;
                    self.open_dossier();
                } else {
                    self.last_click = Some((Instant::now(), idx));
                }
            }
        } else if at.y == self.hits.header_y {
            if let Some(&(_, _, key)) = self
                .hits
                .columns
                .iter()
                .find(|(a, b, _)| (*a..*b).contains(&at.x))
            {
                self.set_sort(key);
            }
        } else if at.y == self.hits.footer_y
            && let Some(&(_, _, code)) = self
                .hits
                .keys
                .iter()
                .find(|(a, b, _)| (*a..*b).contains(&at.x))
        {
            self.on_key(KeyEvent::from(code));
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        // Some terminals (kitty's keyboard protocol and others) send Shift+K as 'k' with SHIFT
        // instead of 'K'; every binding here means the character, so make it one.
        let key = match key.code {
            KeyCode::Char(c)
                if c.is_ascii_lowercase() && key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                KeyEvent::new(KeyCode::Char(c.to_ascii_uppercase()), key.modifiers)
            }
            _ => key,
        };
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
        if self.report.is_some() {
            self.power_down();
            return;
        }
        if self.inspection.is_some() {
            // the inspector steps aside
            self.inspection = None;
            return;
        }
        if self.schematic.is_some() {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                self.schematic = None;
            } else {
                self.next_schematic_page();
            }
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
        self.keys.push(key.code);
        if self.keys.len() > KONAMI.len() {
            self.keys.remove(0);
        }
        if self.keys == KONAMI {
            self.keys.clear();
            self.unlock_secret();
            return;
        }
        if let KeyCode::Char(c @ '0'..='9') = key.code {
            self.typed.push(c);
            if self.typed.len() > SCHEMATIC_CODE.len() {
                self.typed.remove(0);
            }
            if self.typed == SCHEMATIC_CODE {
                self.typed.clear();
                self.open_schematic();
            }
            return;
        }
        self.typed.clear();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.leave(),
            KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::F(1) => self.show_help = true,
            KeyCode::Char('!') => {
                self.start_lockdown("MANUAL OVERRIDE ▸ OPERATOR INITIATED".into())
            }
            KeyCode::Enter => self.open_dossier(),
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
                // the secret palette joins the cycle only once unlocked
                let count = if self.secret_unlocked {
                    THEMES.len()
                } else {
                    SECRET_THEME
                };
                self.theme_idx = (self.theme_idx + 1) % count;
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
            Some(true) => {
                self.shift.terminations += 1;
                self.flash(format!("{sig} DISPATCHED ▸ PID {pid} ({name})"))
            }
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

    mod mouse {
        use ratatui::{Terminal, backend::TestBackend};

        use super::super::*;

        fn drawn_app() -> (App, Terminal<TestBackend>) {
            let mut app = App::new(Settings {
                boot: false,
                ..Settings::default()
            })
            .unwrap();
            let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
            term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
            (app, term)
        }

        fn at(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
            MouseEvent {
                kind,
                column,
                row,
                modifiers: KeyModifiers::NONE,
            }
        }

        fn click(app: &mut App, x: u16, y: u16) {
            app.on_mouse(at(MouseEventKind::Down(MouseButton::Left), x, y));
        }

        #[test]
        fn click_selects_and_double_click_opens_dossier() {
            let (mut app, _term) = drawn_app();
            let r = app.hits.rows;
            assert!(r.height > 3 && app.visible.len() > 3);
            click(&mut app, r.x + 10, r.y + 2);
            assert_eq!(app.table.selected(), Some(2));
            assert!(app.dossier.is_none());
            click(&mut app, r.x + 10, r.y + 2);
            assert_eq!(app.dossier.map(|(pid, _)| pid), Some(app.visible[2].pid));
            assert_eq!(app.shift.dossiers, 1);
        }

        #[test]
        fn click_outside_dossier_closes_it() {
            let (mut app, mut term) = drawn_app();
            app.on_key(KeyEvent::from(KeyCode::Enter));
            term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
            let d = app.hits.dossier.unwrap();
            click(&mut app, d.x + 2, d.y + 2);
            assert!(app.dossier.is_some());
            click(&mut app, 0, 0);
            assert!(app.dossier.is_none());
        }

        #[test]
        fn header_click_sorts_and_wheel_scrolls() {
            let (mut app, _term) = drawn_app();
            let &(x, _, key) = app
                .hits
                .columns
                .iter()
                .find(|c| c.2 == SortKey::Pid)
                .unwrap();
            assert_eq!(key, SortKey::Pid);
            let y = app.hits.header_y;
            click(&mut app, x + 1, y);
            assert_eq!(app.sort, SortKey::Pid);
            // sorting follows the selected process; the wheel moves 3 rows from there
            let before = app.table.selected().unwrap();
            app.on_mouse(at(MouseEventKind::ScrollUp, 50, 30));
            assert_eq!(app.table.selected(), Some(before.saturating_sub(3)));
        }

        #[test]
        fn footer_quit_shows_the_report_then_powers_down() {
            let (mut app, _term) = drawn_app();
            let &(x, _, _) = app
                .hits
                .keys
                .iter()
                .find(|k| k.2 == KeyCode::Char('q'))
                .unwrap();
            let y = app.hits.footer_y;
            click(&mut app, x, y);
            assert!(app.report.is_some() && app.shutdown.is_none());
            app.on_key(KeyEvent::from(KeyCode::Char('z')));
            assert!(app.shutdown.is_some());
        }

        #[test]
        fn typing_142_opens_the_schematic_and_keys_walk_it() {
            let (mut app, _term) = drawn_app();
            app.settings.voice = false;
            for c in "9142".chars() {
                app.on_key(KeyEvent::from(KeyCode::Char(c)));
            }
            assert_eq!(app.schematic.as_ref().map(|s| s.page), Some(0));
            for page in 1..SCHEMATIC_PAGES.len() {
                app.on_key(KeyEvent::from(KeyCode::Char(' ')));
                assert_eq!(app.schematic.as_ref().map(|s| s.page), Some(page));
            }
            app.on_key(KeyEvent::from(KeyCode::Char(' ')));
            assert!(app.schematic.is_none());
            // a digit typed with another key between them does not count
            for c in ['1', '4', 'v', '2'] {
                app.on_key(KeyEvent::from(KeyCode::Char(c)));
            }
            assert!(app.schematic.is_none());
        }

        #[test]
        fn the_old_code_unlocks_the_secret_palette() {
            let (mut app, _term) = drawn_app();
            app.settings.voice = false;
            let cycle = |app: &mut App| {
                let mut seen = Vec::new();
                for _ in 0..THEMES.len() {
                    app.on_key(KeyEvent::from(KeyCode::Char('t')));
                    seen.push(app.theme_idx);
                }
                seen
            };
            assert!(!cycle(&mut app).contains(&SECRET_THEME));
            for k in KONAMI {
                app.on_key(KeyEvent::from(k));
            }
            assert_eq!(app.theme_idx, SECRET_THEME);
            assert!(cycle(&mut app).contains(&SECRET_THEME));
        }

        #[test]
        fn an_inspection_scans_stamps_and_steps_aside() {
            let (mut app, _term) = drawn_app();
            app.settings.voice = false;
            app.alert = None;
            app.alerts.clear();
            app.start_inspection();
            assert!(app.next_inspection >= Instant::now() + INSPECTION_MIN);
            app.inspection.as_mut().unwrap().start -= INSPECTION_SCAN;
            app.tick_inspection();
            let verdict = app.inspection.as_ref().and_then(|i| i.verdict.clone());
            assert!(verdict.is_some());
            assert_eq!(
                app.shift.inspections_passed + app.shift.inspections_failed,
                1
            );
            // any key dismisses it, and does nothing else
            let theme = app.theme_idx;
            app.on_key(KeyEvent::from(KeyCode::Char('t')));
            assert!(app.inspection.is_none());
            assert_eq!(app.theme_idx, theme);
            // the inspector waits while a dossier is open
            app.next_inspection = Instant::now();
            app.dossier = Some((1, app.mon.dossier(1)));
            app.tick_inspection();
            assert!(app.inspection.is_none());
        }

        #[test]
        fn shift_k_opens_termination_however_the_terminal_spells_it() {
            for key in [
                KeyEvent::from(KeyCode::Char('K')),
                KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT),
                KeyEvent::new(KeyCode::Char('k'), KeyModifiers::SHIFT),
            ] {
                let (mut app, _term) = drawn_app();
                app.on_key(key);
                assert!(app.kill.is_some(), "{key:?}");
            }
            let (mut app, _term) = drawn_app();
            app.on_key(KeyEvent::from(KeyCode::Down));
            app.on_key(KeyEvent::from(KeyCode::Char('k')));
            assert!(app.kill.is_none());
            assert_eq!(app.table.selected(), Some(0));
        }

        #[test]
        fn the_mouse_never_answers_a_termination() {
            let (mut app, _term) = drawn_app();
            app.on_key(KeyEvent::from(KeyCode::Char('K')));
            assert!(app.kill.is_some());
            click(&mut app, 80, 22);
            assert!(app.kill.is_some());
        }
    }
}
