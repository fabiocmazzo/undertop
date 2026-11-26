//! Telemetry collection: wraps `sysinfo` and keeps short histories for the charts.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::time::{Duration, Instant};

use color_eyre::Result;
use sysinfo::{
    Components, Disks, Networks, Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, Signal,
    System, UpdateKind, Users,
};

use crate::gpu::{GpuRow, Gpus};

/// Samples kept per history (one per refresh).
pub const HISTORY: usize = 300;
/// How long a process must hold (nearly) a full core before the ticker reports it.
const HOG_TIME: Duration = Duration::from_secs(5);
/// "Nearly a full core": a busy single-threaded process reads 98-102 %, so 100 would be flaky.
const HOG_CPU: f32 = 90.0;

#[derive(Debug, Clone)]
pub struct History {
    buf: VecDeque<f64>,
}

impl History {
    pub fn new() -> Self {
        Self {
            buf: VecDeque::with_capacity(HISTORY),
        }
    }

    pub fn push(&mut self, v: f64) {
        if self.buf.len() == HISTORY {
            self.buf.pop_front();
        }
        self.buf.push_back(v);
    }

    pub fn last(&self) -> f64 {
        self.buf.back().copied().unwrap_or(0.0)
    }

    pub fn max(&self) -> f64 {
        self.buf.iter().copied().fold(0.0, f64::max)
    }

    /// The newest `n` samples, oldest first.
    pub fn tail(&self, n: usize) -> impl Iterator<Item = f64> + '_ {
        self.buf
            .iter()
            .copied()
            .skip(self.buf.len().saturating_sub(n))
    }
}

#[derive(Debug, Clone)]
pub struct ProcRow {
    pub pid: u32,
    pub ppid: Option<u32>,
    /// Branch drawing in tree view (`├─ `), empty otherwise.
    pub tree: String,
    pub name: String,
    pub cmd: String,
    pub user: String,
    pub cpu: f32,
    pub mem: u64,
    pub status: &'static str,
    pub run_time: u64,
}

#[derive(Debug, Clone)]
pub struct TempRow {
    pub label: String,
    pub temp: f32,
    pub crit: f32,
}

#[derive(Debug, Clone)]
pub struct DiskRow {
    pub mount: String,
    pub total: u64,
    pub used: u64,
}

#[derive(Debug, Clone)]
pub struct Battery {
    pub capacity: u8,
    pub status: String,
}

pub struct Monitor {
    sys: System,
    networks: Networks,
    disks: Disks,
    components: Components,
    users: Users,
    gpus: Gpus,
    last_refresh: Instant,
    refreshes: u64,
    last_diskstats: Option<(u64, u64)>,
    /// PIDs holding a full core: since when, and whether it was reported.
    hogs: HashMap<u32, (Instant, bool)>,
    /// (pid, kind) already reported, so each alert is said once.
    reported: HashSet<(u32, u8)>,
    /// Seconds covered by the last refresh.
    pub dt: f64,
    /// PID whose disk I/O is also sampled (the open dossier); reading it for every process is costly.
    pub watch: Option<u32>,
    /// Consecutive refreshes that sampled `watch`; its first I/O reading is a total, not a rate.
    watch_rounds: u32,
    watch_last: Option<u32>,
    /// Ticker alerts produced by the last refresh; the app drains them.
    pub alerts: Vec<String>,

    pub cpu_total: History,
    pub cpu_cores: Vec<History>,
    pub avg_freq: u64,
    pub mem: History,
    pub rx: History,
    pub tx: History,
    pub rx_total: u64,
    pub tx_total: u64,
    /// Disk read / write bytes per second (whole devices).
    pub disk_r: History,
    pub disk_w: History,
    pub gpu_rows: Vec<GpuRow>,

    pub mem_total: u64,
    pub mem_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,

    pub procs: Vec<ProcRow>,
    pub temps: Vec<TempRow>,
    pub disks_rows: Vec<DiskRow>,
    pub battery: Option<Battery>,
    pub load: (f64, f64, f64),
    pub uptime: u64,

    pub host: String,
    pub kernel: String,
    pub os: String,
    pub cpu_brand: String,
}

impl Monitor {
    pub fn new() -> Result<Self> {
        let mut sys = System::new()?;
        sys.refresh_cpu_all();
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, process_kind());
        let cores = sys.cpus().len();
        let cpu_brand = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "UNKNOWN CORE".into());
        let mut m = Self {
            sys,
            networks: Networks::new_with_refreshed_list()?,
            disks: Disks::new_with_refreshed_list()?,
            components: Components::new_with_refreshed_list()?,
            users: Users::new_with_refreshed_list()?,
            gpus: Gpus::detect(),
            last_refresh: Instant::now(),
            refreshes: 0,
            last_diskstats: None,
            hogs: HashMap::new(),
            reported: HashSet::new(),
            dt: 1.0,
            watch: None,
            watch_rounds: 0,
            watch_last: None,
            alerts: Vec::new(),
            disk_r: History::new(),
            disk_w: History::new(),
            gpu_rows: Vec::new(),
            cpu_total: History::new(),
            cpu_cores: (0..cores).map(|_| History::new()).collect(),
            avg_freq: 0,
            mem: History::new(),
            rx: History::new(),
            tx: History::new(),
            rx_total: 0,
            tx_total: 0,
            mem_total: 0,
            mem_used: 0,
            swap_total: 0,
            swap_used: 0,
            procs: Vec::new(),
            temps: Vec::new(),
            disks_rows: Vec::new(),
            battery: None,
            load: (0.0, 0.0, 0.0),
            uptime: 0,
            host: System::host_name().unwrap_or_else(|_| "UNKNOWN".into()),
            kernel: System::kernel_version().unwrap_or_else(|_| "?".into()),
            os: System::long_os_version().unwrap_or_else(|_| "LINUX".into()),
            cpu_brand,
        };
        m.refresh();
        Ok(m)
    }

    pub fn refresh(&mut self) {
        self.refreshes += 1;
        self.dt = self.last_refresh.elapsed().as_secs_f64().max(0.001);
        self.last_refresh = Instant::now();
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        // Only what the manifest shows. sysinfo's default also lists every thread as a process
        // (3000+ here) and reads every process's I/O: 45 ms per refresh instead of ~7.
        self.sys
            .refresh_processes_specifics(ProcessesToUpdate::All, true, process_kind());
        self.watch_rounds = if self.watch.is_some() && self.watch == self.watch_last {
            self.watch_rounds + 1
        } else {
            u32::from(self.watch.is_some())
        };
        self.watch_last = self.watch;
        if let Some(pid) = self.watch {
            let pid = [Pid::from_u32(pid)];
            self.sys.refresh_processes_specifics(
                ProcessesToUpdate::Some(&pid),
                false,
                ProcessRefreshKind::nothing()
                    .with_disk_usage()
                    .without_tasks(),
            );
        }

        // CPU
        let cpus = self.sys.cpus();
        for (h, c) in self.cpu_cores.iter_mut().zip(cpus) {
            h.push(c.usage() as f64);
        }
        self.cpu_total.push(self.sys.global_cpu_usage() as f64);
        self.avg_freq = if cpus.is_empty() {
            0
        } else {
            cpus.iter().map(|c| c.frequency()).sum::<u64>() / cpus.len() as u64
        };

        // Memory
        self.mem_total = self.sys.total_memory();
        self.mem_used = self.sys.used_memory();
        self.swap_total = self.sys.total_swap();
        self.swap_used = self.sys.used_swap();
        self.mem.push(if self.mem_total > 0 {
            self.mem_used as f64 / self.mem_total as f64 * 100.0
        } else {
            0.0
        });

        // Network: `received()` is the amount since the previous refresh.
        self.networks.refresh(true);
        let secs = self.dt;
        let (mut rx, mut tx) = (0u64, 0u64);
        let (mut rx_t, mut tx_t) = (0u64, 0u64);
        for (name, data) in self.networks.iter() {
            if name.to_string_lossy() == "lo" {
                continue;
            }
            rx += data.received();
            tx += data.transmitted();
            rx_t += data.total_received();
            tx_t += data.total_transmitted();
        }
        self.rx.push(rx as f64 / secs);
        self.tx.push(tx as f64 / secs);
        self.rx_total = rx_t;
        self.tx_total = tx_t;

        // Sensors
        // NVMe sensors cost ~2 ms each to read (the drive is asked); they change slowly.
        let slow_round = self.refreshes % 5 == 1;
        for c in self.components.iter_mut() {
            if slow_round || !c.label().starts_with("nvme") {
                c.refresh();
            }
        }
        let mut temps: Vec<TempRow> = self
            .components
            .iter()
            .filter_map(|c| {
                let temp = c.temperature()?;
                if !temp.is_finite() || temp <= 0.0 {
                    return None;
                }
                Some(TempRow {
                    label: c.label().to_string(),
                    temp,
                    crit: c.critical().filter(|v| *v > 20.0).unwrap_or(100.0),
                })
            })
            .collect();
        // Package and composite sensors first, then the hottest.
        temps.sort_by(|a, b| {
            let pa = is_headline_sensor(&a.label);
            let pb = is_headline_sensor(&b.label);
            pb.cmp(&pa).then(b.temp.total_cmp(&a.temp))
        });
        self.temps = temps;

        // Disks change slowly; refresh every 5 ticks.
        if self.refreshes % 5 == 1 {
            self.disks.refresh(true);
            let mut seen = Vec::new();
            let mut rows = Vec::new();
            for d in self.disks.iter() {
                let fs = d.file_system().to_string_lossy();
                if matches!(
                    fs.as_ref(),
                    "tmpfs" | "devtmpfs" | "squashfs" | "overlay" | "efivarfs"
                ) || d.total_space() == 0
                {
                    continue;
                }
                // btrfs subvolumes show the same device many times; keep the first mount.
                let name = d.name().to_string_lossy().to_string();
                if seen.contains(&name) {
                    continue;
                }
                seen.push(name);
                rows.push(DiskRow {
                    mount: d.mount_point().to_string_lossy().to_string(),
                    total: d.total_space(),
                    used: d.total_space().saturating_sub(d.available_space()),
                });
            }
            rows.sort_by(|a, b| a.mount.cmp(&b.mount));
            self.disks_rows = rows;
            self.battery = read_battery();
        }

        // Processes
        self.procs = self
            .sys
            .processes()
            .values()
            .filter(|p| p.thread_kind().is_none())
            .map(|p| {
                let cmd = p
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ");
                let name = p.name().to_string_lossy().to_string();
                ProcRow {
                    pid: p.pid().as_u32(),
                    ppid: p.parent().map(|pp| pp.as_u32()),
                    tree: String::new(),
                    cmd: if cmd.is_empty() {
                        format!("[{name}]")
                    } else {
                        cmd
                    },
                    name,
                    user: p
                        .user_id()
                        .and_then(|u| self.users.get_user_by_id(u))
                        .map(|u| u.name().to_string())
                        .unwrap_or_else(|| "?".into()),
                    cpu: p.cpu_usage(),
                    mem: p.memory(),
                    status: status_code(p.status()),
                    run_time: p.run_time(),
                }
            })
            .collect();

        let la = System::load_average().unwrap_or(sysinfo::LoadAvg {
            one: 0.0,
            five: 0.0,
            fifteen: 0.0,
        });
        self.load = (la.one, la.five, la.fifteen);
        self.uptime = System::uptime().unwrap_or(0);

        // Disk I/O
        if let Some((r, w)) = read_diskstats() {
            if let Some((pr, pw)) = self.last_diskstats {
                self.disk_r.push(r.saturating_sub(pr) as f64 / secs);
                self.disk_w.push(w.saturating_sub(pw) as f64 / secs);
            }
            self.last_diskstats = Some((r, w));
        }

        self.gpu_rows = self.gpus.read();
        self.watch_for_alerts();
    }

    /// Fills `alerts` with things the ticker should announce: hogs, leaks and zombies.
    fn watch_for_alerts(&mut self) {
        const DRAIN: u8 = 1;
        const ZOMBIE: u8 = 2;
        let now = Instant::now();
        let mut hot = HashSet::new();
        let mut present = HashSet::new();
        for p in &self.procs {
            present.insert(p.pid);
            if p.cpu >= HOG_CPU {
                hot.insert(p.pid);
                let entry = self.hogs.entry(p.pid).or_insert((now, false));
                if !entry.1 && entry.0.elapsed() >= HOG_TIME {
                    entry.1 = true;
                    self.alerts.push(format!(
                        "PID {} ({}) EXCEEDING ALLOCATION ▸ {:.0}% CPU",
                        p.pid,
                        p.name.to_uppercase(),
                        p.cpu
                    ));
                }
            }
            if self.mem_total > 0
                && p.mem > self.mem_total / 4
                && self.reported.insert((p.pid, DRAIN))
            {
                self.alerts.push(format!(
                    "PID {} ({}) IS DRAINING THE RESERVOIR ▸ {}",
                    p.pid,
                    p.name.to_uppercase(),
                    fmt_bytes(p.mem as f64)
                ));
            }
            if p.status == "ZMB" && self.reported.insert((p.pid, ZOMBIE)) {
                self.alerts.push(format!(
                    "PID {} ({}) HAS DIED BUT WILL NOT LEAVE ▸ NOTIFY SANITATION",
                    p.pid,
                    p.name.to_uppercase()
                ));
            }
        }
        self.hogs.retain(|pid, _| hot.contains(pid));
        self.reported.retain(|(pid, _)| present.contains(pid));
    }

    /// Everything known about one process, for the dossier overlay.
    pub fn dossier(&self, pid: u32) -> Option<Dossier> {
        let p = self.sys.process(Pid::from_u32(pid))?;
        let row = self.procs.iter().find(|r| r.pid == pid)?;
        let proc_dir = format!("/proc/{pid}");
        let link = |name: &str| {
            fs::read_link(format!("{proc_dir}/{name}"))
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        };
        let threads = fs::read_to_string(format!("{proc_dir}/status"))
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("Threads:"))
                    .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
            });
        let cgroup = fs::read_to_string(format!("{proc_dir}/cgroup"))
            .ok()
            .and_then(|s| {
                s.lines()
                    .next()
                    .map(|l| l.splitn(3, ':').nth(2).unwrap_or(l).to_string())
            });
        let fds = fs::read_dir(format!("{proc_dir}/fd")).ok().map(|dir| {
            let mut targets: Vec<String> = dir
                .flatten()
                .filter_map(|e| fs::read_link(e.path()).ok())
                .map(|t| t.to_string_lossy().to_string())
                .collect();
            let count = targets.len();
            // files first, then sockets and pipes
            targets.sort_by_key(|t| (!t.starts_with('/'), t.clone()));
            targets.dedup();
            (count, targets)
        });
        let parent = row
            .ppid
            .and_then(|pp| self.procs.iter().find(|r| r.pid == pp));
        let mut children: Vec<(u32, String)> = self
            .procs
            .iter()
            .filter(|r| r.ppid == Some(pid))
            .map(|r| (r.pid, r.name.clone()))
            .collect();
        children.sort();
        // a rate needs two readings of the same process in a row
        let io = (self.watch == Some(pid) && self.watch_rounds >= 2).then(|| p.disk_usage());
        Some(Dossier {
            row: row.clone(),
            parent: parent.map(|r| (r.pid, r.name.clone())),
            started: p.start_time(),
            vmem: p.virtual_memory(),
            threads,
            exe: link("exe"),
            cwd: link("cwd"),
            cgroup,
            fds,
            io: io.map(|u| {
                (
                    u.read_bytes as f64 / self.dt,
                    u.written_bytes as f64 / self.dt,
                )
            }),
            children,
        })
    }

    pub fn core_count(&self) -> usize {
        self.cpu_cores.len()
    }

    /// Sends a signal to a process. Returns `Some(true)` when it was delivered.
    pub fn signal(&self, pid: u32, force: bool) -> Option<bool> {
        let sig = if force { Signal::Kill } else { Signal::Term };
        self.sys.process(Pid::from_u32(pid))?.kill_with(sig)
    }
}

/// What the dossier shows. `None` fields could not be read: usually a process of another user.
#[derive(Debug, Clone)]
pub struct Dossier {
    pub row: ProcRow,
    pub parent: Option<(u32, String)>,
    /// Unix time.
    pub started: u64,
    pub vmem: u64,
    pub threads: Option<u32>,
    pub exe: Option<String>,
    pub cwd: Option<String>,
    pub cgroup: Option<String>,
    /// (open descriptors, their distinct targets)
    pub fds: Option<(usize, Vec<String>)>,
    /// (read, write) bytes per second; `None` until it has been sampled twice.
    pub io: Option<(f64, f64)>,
    pub children: Vec<(u32, String)>,
}

/// Total bytes (read, written) by whole disks since boot, from `/proc/diskstats`.
fn read_diskstats() -> Option<(u64, u64)> {
    let text = fs::read_to_string("/proc/diskstats").ok()?;
    let (mut r, mut w) = (0u64, 0u64);
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 10 || !is_whole_disk(f[2]) {
            continue;
        }
        // fields: major minor name reads merged SECTORS_READ ms writes merged SECTORS_WRITTEN ...
        r += f[5].parse::<u64>().unwrap_or(0) * 512;
        w += f[9].parse::<u64>().unwrap_or(0) * 512;
    }
    Some((r, w))
}

/// `nvme0n1`, `sda`, `vdb`, `mmcblk0`: yes. Partitions, loop, ram, dm and zram devices: no.
fn is_whole_disk(name: &str) -> bool {
    let digits_tail = |prefix: &str| {
        name.strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
    };
    if let Some(rest) = name.strip_prefix("nvme") {
        return !rest.contains('p') && rest.contains('n');
    }
    if name.starts_with("sd") || name.starts_with("vd") || name.starts_with("hd") {
        return name[2..].chars().all(|c| c.is_ascii_lowercase()) && name.len() > 2;
    }
    digits_tail("mmcblk")
}

fn process_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing()
        .with_cpu()
        .with_memory()
        .with_user(UpdateKind::OnlyIfNotSet)
        .with_cmd(UpdateKind::OnlyIfNotSet)
        .without_tasks()
}

fn is_headline_sensor(label: &str) -> bool {
    let l = label.to_lowercase();
    l.contains("package") || l.contains("composite") || l.contains("tctl")
}

fn status_code(s: ProcessStatus) -> &'static str {
    match s {
        ProcessStatus::Run => "RUN",
        ProcessStatus::Sleep => "SLP",
        ProcessStatus::Idle => "IDL",
        ProcessStatus::Zombie => "ZMB",
        ProcessStatus::Stop => "STP",
        ProcessStatus::Tracing => "TRC",
        ProcessStatus::Dead => "DED",
        ProcessStatus::UninterruptibleDiskSleep => "DSK",
        _ => "???",
    }
}

fn read_battery() -> Option<Battery> {
    let dir = fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in dir.flatten() {
        let path = entry.path();
        if !entry.file_name().to_string_lossy().starts_with("BAT") {
            continue;
        }
        let capacity = fs::read_to_string(path.join("capacity"))
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let status = fs::read_to_string(path.join("status"))
            .map(|s| s.trim().to_uppercase())
            .unwrap_or_else(|_| "UNKNOWN".into());
        return Some(Battery { capacity, status });
    }
    None
}

pub fn fmt_bytes(b: f64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut v = b;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{v:.0} {}", UNITS[i])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

pub fn fmt_duration(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if d > 0 {
        format!("{d}D {h:02}:{m:02}:{s:02}")
    } else {
        format!("{h:02}:{m:02}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::is_whole_disk;

    #[test]
    fn whole_disks_only() {
        for yes in ["nvme0n1", "nvme1n1", "sda", "vdb", "mmcblk0"] {
            assert!(is_whole_disk(yes), "{yes}");
        }
        for no in [
            "nvme0n1p2",
            "sda1",
            "loop0",
            "zram0",
            "dm-0",
            "mmcblk0p1",
            "sr0",
        ] {
            assert!(!is_whole_disk(no), "{no}");
        }
    }
}

/// Runs a real busy process and waits for the ticker alert. Slow (~8 s), so not part of the
/// default run: `cargo test -- --ignored hog_alert`.
#[cfg(test)]
mod hog_tests {
    use super::Monitor;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    #[test]
    #[ignore]
    fn hog_alert_fires_for_a_busy_process() {
        let mut child = Command::new("yes").stdout(Stdio::null()).spawn().unwrap();
        let pid = child.id();
        let mut m = Monitor::new().unwrap();
        let start = Instant::now();
        let mut seen = Vec::new();
        while start.elapsed() < Duration::from_secs(9) {
            std::thread::sleep(Duration::from_millis(1000));
            m.refresh();
            if let Some(p) = m.procs.iter().find(|p| p.pid == pid) {
                seen.push(p.cpu);
            }
            if m.alerts.iter().any(|a| a.contains(&format!("PID {pid} "))) {
                child.kill().ok();
                child.wait().ok();
                eprintln!("cpu samples: {seen:?}");
                return;
            }
        }
        child.kill().ok();
        child.wait().ok();
        panic!("no alert for `yes`; cpu samples: {seen:?}");
    }
}

/// Where the time of one refresh goes. `cargo test --release -- --ignored refresh_cost --nocapture`
#[cfg(test)]
mod cost_tests {
    use super::*;

    #[test]
    #[ignore]
    fn refresh_cost() {
        let mut sys = System::new_all().unwrap();
        let mut nets = Networks::new_with_refreshed_list().unwrap();
        let mut comps = Components::new_with_refreshed_list().unwrap();
        let mut disks = Disks::new_with_refreshed_list().unwrap();
        let gpus = Gpus::detect();
        let n = 20;
        let time = |name: &str, f: &mut dyn FnMut()| {
            let t = Instant::now();
            for _ in 0..n {
                f();
            }
            eprintln!(
                "{name:<24} {:>7.2} ms",
                t.elapsed().as_secs_f64() * 1000.0 / n as f64
            );
        };
        time("refresh_cpu_usage", &mut || sys.refresh_cpu_usage());
        time("refresh_cpu_frequency", &mut || sys.refresh_cpu_frequency());
        time("refresh_memory", &mut || sys.refresh_memory());
        time("processes (sysinfo default)", &mut || {
            sys.refresh_processes(ProcessesToUpdate::All, true);
        });
        let mut lean = System::new().unwrap();
        time("processes (undertop)", &mut || {
            lean.refresh_processes_specifics(ProcessesToUpdate::All, true, process_kind());
        });
        for c in comps.iter_mut() {
            let t = Instant::now();
            c.refresh();
            eprintln!(
                "  sensor {:<40} {:>6.2} ms",
                c.label(),
                t.elapsed().as_secs_f64() * 1000.0
            );
        }
        time("networks", &mut || nets.refresh(true));
        time("components (sensors)", &mut || comps.refresh(false));
        time("disks (every 5th)", &mut || disks.refresh(true));
        time("diskstats", &mut || {
            read_diskstats();
        });
        time("gpus", &mut || {
            gpus.read();
        });
        time("battery", &mut || {
            read_battery();
        });
    }
}
