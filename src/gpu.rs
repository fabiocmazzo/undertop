//! GPU telemetry for the AUX REACTOR station.
//!
//! - Intel iGPUs: frequencies from `/sys/class/drm/card*/gt_*_freq_mhz` (there is no utilization
//!   counter in sysfs, so the actual/maximum frequency ratio stands in for load).
//! - NVIDIA dGPUs: `nvidia-smi`, but only while the GPU is already awake. On laptops the dGPU sleeps
//!   (runtime PM) and any query wakes it for ~16 s, which costs battery. So a background thread reads
//!   the PCI `runtime_status` every second (a sysfs read does not wake it) and runs `nvidia-smi` only
//!   when the status is `active`, at most every 20 s: longer than the idle delay, so our own query
//!   never keeps it awake.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Time between `nvidia-smi` queries while the dGPU is awake and power-managed.
const NV_INTERVAL_PM: Duration = Duration::from_secs(20);
/// Time between queries when the GPU has no runtime power management (desktop cards).
const NV_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuState {
    Active,
    /// Suspended by runtime power management; not queried so it stays asleep.
    Dormant,
    /// Not sampled yet.
    Unknown,
}

#[derive(Debug, Clone)]
pub struct GpuRow {
    pub name: String,
    pub state: GpuState,
    /// 0.0..=1.0
    pub load: Option<f64>,
    pub freq_mhz: Option<u32>,
    pub max_mhz: Option<u32>,
    pub temp: Option<f32>,
    /// (used, total) in MiB
    pub vram: Option<(u64, u64)>,
    /// When the numbers above were last read (NVIDIA only).
    pub sampled: Option<Instant>,
}

impl GpuRow {
    fn new(name: String) -> Self {
        Self {
            name,
            state: GpuState::Unknown,
            load: None,
            freq_mhz: None,
            max_mhz: None,
            temp: None,
            vram: None,
            sampled: None,
        }
    }
}

pub struct Gpus {
    intel: Vec<PathBuf>,
    nvidia: Vec<Arc<Mutex<GpuRow>>>,
}

impl Gpus {
    pub fn detect() -> Self {
        let mut intel = Vec::new();
        let mut nvidia = Vec::new();
        let Ok(dir) = fs::read_dir("/sys/class/drm") else {
            return Self { intel, nvidia };
        };
        let mut cards: Vec<PathBuf> = dir
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("card") && !n.contains('-'))
            })
            .collect();
        cards.sort();
        for card in cards {
            let vendor = read_trim(&card.join("device/vendor")).unwrap_or_default();
            match vendor.as_str() {
                "0x8086" if card.join("gt_act_freq_mhz").exists() => intel.push(card),
                "0x10de" if has_nvidia_smi() => {
                    let row = Arc::new(Mutex::new(GpuRow::new(
                        nvidia_model().unwrap_or_else(|| "NVIDIA dGPU".into()),
                    )));
                    spawn_nvidia_sampler(card.join("device/power/runtime_status"), row.clone());
                    nvidia.push(row);
                }
                _ => {}
            }
        }
        Self { intel, nvidia }
    }

    pub fn read(&self) -> Vec<GpuRow> {
        let mut rows = Vec::new();
        for card in &self.intel {
            let num = |f: &str| read_trim(&card.join(f)).and_then(|s| s.parse::<u32>().ok());
            let act = num("gt_act_freq_mhz");
            let max = num("gt_RP0_freq_mhz").or_else(|| num("gt_max_freq_mhz"));
            let mut row = GpuRow::new("INTEL iGPU".into());
            row.state = GpuState::Active;
            row.freq_mhz = act;
            row.max_mhz = max;
            row.load = match (act, max) {
                (Some(a), Some(m)) if m > 0 => Some((a as f64 / m as f64).clamp(0.0, 1.0)),
                _ => None,
            };
            rows.push(row);
        }
        for nv in &self.nvidia {
            if let Ok(row) = nv.lock() {
                rows.push(row.clone());
            }
        }
        rows
    }
}

fn read_trim(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

fn has_nvidia_smi() -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|d| d.join("nvidia-smi").is_file()))
}

/// The model name from the driver's procfs, which does not wake the GPU.
fn nvidia_model() -> Option<String> {
    let dir = fs::read_dir("/proc/driver/nvidia/gpus").ok()?;
    for e in dir.flatten() {
        let info = fs::read_to_string(e.path().join("information")).ok()?;
        if let Some(line) = info.lines().find(|l| l.starts_with("Model:")) {
            return Some(short_name(line.trim_start_matches("Model:").trim()));
        }
    }
    None
}

fn short_name(full: &str) -> String {
    full.replace("NVIDIA ", "")
        .replace("GeForce ", "")
        .replace(" Laptop GPU", "")
        .to_uppercase()
}

fn spawn_nvidia_sampler(status_path: PathBuf, row: Arc<Mutex<GpuRow>>) {
    let managed = status_path.exists();
    let interval = if managed { NV_INTERVAL_PM } else { NV_INTERVAL };
    thread::spawn(move || {
        let mut last: Option<Instant> = None;
        loop {
            let awake = !managed || read_trim(&status_path).as_deref() == Some("active");
            if !awake {
                if let Ok(mut r) = row.lock() {
                    r.state = GpuState::Dormant;
                }
            } else if last.is_none_or(|t| t.elapsed() >= interval) {
                last = Some(Instant::now());
                let sample = query_nvidia();
                if let Ok(mut r) = row.lock() {
                    r.state = GpuState::Active;
                    if let Some(s) = sample {
                        r.name = s.name;
                        r.load = s.load;
                        r.temp = s.temp;
                        r.vram = s.vram;
                        r.freq_mhz = s.freq;
                        r.max_mhz = s.max;
                        r.sampled = Some(Instant::now());
                    }
                }
            } else if let Ok(mut r) = row.lock() {
                r.state = GpuState::Active;
            }
            thread::sleep(Duration::from_secs(1));
        }
    });
}

struct NvSample {
    name: String,
    load: Option<f64>,
    temp: Option<f32>,
    vram: Option<(u64, u64)>,
    freq: Option<u32>,
    max: Option<u32>,
}

fn query_nvidia() -> Option<NvSample> {
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,utilization.gpu,temperature.gpu,memory.used,memory.total,clocks.gr,clocks.max.gr",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    parse_nvidia(text.lines().next()?)
}

fn parse_nvidia(line: &str) -> Option<NvSample> {
    let f: Vec<&str> = line.split(',').map(str::trim).collect();
    if f.len() < 7 {
        return None;
    }
    let num = |s: &str| s.parse::<f64>().ok();
    Some(NvSample {
        name: short_name(f[0]),
        load: num(f[1]).map(|v| (v / 100.0).clamp(0.0, 1.0)),
        temp: num(f[2]).map(|v| v as f32),
        vram: num(f[3]).zip(num(f[4])).map(|(u, t)| (u as u64, t as u64)),
        freq: num(f[5]).map(|v| v as u32),
        max: num(f[6]).map(|v| v as u32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nvidia_smi_line() {
        let s = parse_nvidia("NVIDIA GeForce RTX 4060 Laptop GPU, 37, 45.98, 2, 8188, 2010, 3105")
            .unwrap();
        assert_eq!(s.name, "RTX 4060");
        assert_eq!(s.load, Some(0.37));
        assert_eq!(s.vram, Some((2, 8188)));
        assert_eq!(s.max, Some(3105));
        assert!(parse_nvidia("[N/A]").is_none());
    }
}
