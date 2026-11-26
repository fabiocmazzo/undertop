//! Telemetry collection: wraps `sysinfo` and keeps short histories for the charts.

use std::collections::VecDeque;
use std::fs;
use std::time::Instant;

use color_eyre::Result;
use sysinfo::{
    Components, Disks, Networks, Pid, ProcessStatus, ProcessesToUpdate, Signal, System, Users,
};

/// Samples kept per history (one per refresh).
pub const HISTORY: usize = 300;

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
    last_net: Instant,
    refreshes: u64,

    pub cpu_total: History,
    pub cpu_cores: Vec<History>,
    pub avg_freq: u64,
    pub mem: History,
    pub rx: History,
    pub tx: History,
    pub rx_total: u64,
    pub tx_total: u64,

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
        let mut sys = System::new_all()?;
        sys.refresh_cpu_all();
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
            last_net: Instant::now(),
            refreshes: 0,
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
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);

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
        let secs = self.last_net.elapsed().as_secs_f64().max(0.001);
        self.last_net = Instant::now();
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
        self.components.refresh(false);
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
