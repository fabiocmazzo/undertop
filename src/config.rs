//! Settings: defaults, then the config file, then command-line flags.
//!
//! The file lives at `$XDG_CONFIG_HOME/undertop/config.toml` (or `~/.config/undertop/config.toml`).
//! `undertop --print-config` prints a commented template with the defaults.

use std::path::{Path, PathBuf};
use std::time::Duration;

use color_eyre::Result;
use color_eyre::eyre::{WrapErr, eyre};
use serde::Deserialize;

use crate::theme::THEMES;

/// The default palette: SILO, teal line-art.
const SILO_THEME: usize = 4;

pub const TEMPLATE: &str = "\
# undertop configuration
# Save as ~/.config/undertop/config.toml. Every key is optional.

# silo, amber, phosphor, cobalt or lockdown
theme = \"silo\"

# stations on the console, in any order (the layout is fixed):
# cpu, memory, thermal, comms, vaults, gpu, procs
panels = [\"cpu\", \"memory\", \"thermal\", \"comms\", \"vaults\", \"gpu\", \"procs\"]

# milliseconds between telemetry samples (250 to 10000)
interval_ms = 1000

# CRT emulation: scanlines, vignette, rolling band, brownouts, glitches
crt = true

# power-on self test at start
boot = true

# CRT power-down animation when leaving
power_down = true

# end-of-shift report when leaving with q (also printed to the terminal afterwards)
shift_report = true

# robotic voice for facility announcements (uses espeak-ng, espeak or spd-say if installed)
voice = true

# surprise inspections, about every 30 minutes on average
inspections = true

# seal the facility (LOCKDOWN palette, alarm) when a sensor reaches 95 °C
# or 95 % of its critical temperature; at most once every 10 minutes
lockdown_on_heat = true
";

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct File {
    theme: Option<String>,
    panels: Option<Vec<String>>,
    interval_ms: Option<u64>,
    crt: Option<bool>,
    boot: Option<bool>,
    power_down: Option<bool>,
    shift_report: Option<bool>,
    lockdown_on_heat: Option<bool>,
    voice: Option<bool>,
    inspections: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub theme_idx: usize,
    pub panels: Panels,
    pub interval: Duration,
    pub crt: bool,
    pub boot: bool,
    pub power_down: bool,
    pub shift_report: bool,
    pub lockdown_on_heat: bool,
    pub voice: bool,
    pub inspections: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme_idx: SILO_THEME,
            panels: Panels::default(),
            interval: Duration::from_millis(1000),
            crt: true,
            boot: true,
            power_down: true,
            shift_report: true,
            lockdown_on_heat: true,
            voice: true,
            inspections: true,
        }
    }
}

/// Which stations the console shows. The header and the footer are always there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panels {
    pub cpu: bool,
    pub memory: bool,
    pub thermal: bool,
    pub comms: bool,
    pub vaults: bool,
    pub gpu: bool,
    pub procs: bool,
}

pub const PANEL_NAMES: [&str; 7] = [
    "cpu", "memory", "thermal", "comms", "vaults", "gpu", "procs",
];

impl Default for Panels {
    fn default() -> Self {
        Self::parse(PANEL_NAMES).unwrap()
    }
}

impl Panels {
    /// From a list of names, e.g. `["cpu", "procs"]`; anything not listed is hidden.
    pub fn parse<S: AsRef<str>>(names: impl IntoIterator<Item = S>) -> Result<Self> {
        let mut p = Self {
            cpu: false,
            memory: false,
            thermal: false,
            comms: false,
            vaults: false,
            gpu: false,
            procs: false,
        };
        for n in names {
            let n = n.as_ref().trim().to_ascii_lowercase();
            let slot = match n.as_str() {
                "cpu" => &mut p.cpu,
                "memory" | "mem" => &mut p.memory,
                "thermal" | "temp" => &mut p.thermal,
                "comms" | "net" => &mut p.comms,
                "vaults" | "disks" => &mut p.vaults,
                "gpu" => &mut p.gpu,
                "procs" | "processes" => &mut p.procs,
                _ => {
                    return Err(eyre!(
                        "unknown panel \"{n}\" (use {})",
                        PANEL_NAMES.join(", ")
                    ));
                }
            };
            *slot = true;
        }
        Ok(p)
    }
}

pub fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("undertop").join("config.toml"))
}

pub fn theme_index(name: &str) -> Option<usize> {
    THEMES
        .iter()
        .position(|t| t.name.eq_ignore_ascii_case(name))
}

impl Settings {
    /// Reads `path`. A missing file at the default path is fine; a missing file the user named is not.
    pub fn load(path: &Path, required: bool) -> Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !required => {
                return Ok(Self::default());
            }
            Err(e) => return Err(e).wrap_err_with(|| format!("cannot read {}", path.display())),
        };
        Self::parse(&text).wrap_err_with(|| format!("in {}", path.display()))
    }

    fn parse(text: &str) -> Result<Self> {
        let f: File = toml::from_str(text)?;
        let mut s = Self::default();
        if let Some(name) = f.theme {
            s.theme_idx = theme_index(&name).ok_or_else(|| eyre!("unknown theme \"{name}\""))?;
        }
        if let Some(names) = f.panels {
            s.panels = Panels::parse(&names)?;
        }
        if let Some(ms) = f.interval_ms {
            if !(250..=10_000).contains(&ms) {
                return Err(eyre!("interval_ms must be between 250 and 10000, not {ms}"));
            }
            s.interval = Duration::from_millis(ms);
        }
        s.crt = f.crt.unwrap_or(s.crt);
        s.boot = f.boot.unwrap_or(s.boot);
        s.power_down = f.power_down.unwrap_or(s.power_down);
        s.shift_report = f.shift_report.unwrap_or(s.shift_report);
        s.lockdown_on_heat = f.lockdown_on_heat.unwrap_or(s.lockdown_on_heat);
        s.voice = f.voice.unwrap_or(s.voice);
        s.inspections = f.inspections.unwrap_or(s.inspections);
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_parses_to_defaults() {
        let s = Settings::parse(TEMPLATE).unwrap();
        let d = Settings::default();
        assert_eq!(s.theme_idx, d.theme_idx);
        assert_eq!(s.interval, d.interval);
        assert_eq!(
            (s.crt, s.boot, s.power_down, s.lockdown_on_heat),
            (true, true, true, true)
        );
    }

    #[test]
    fn rejects_bad_values() {
        assert!(Settings::parse("theme = \"pink\"").is_err());
        assert!(Settings::parse("interval_ms = 5").is_err());
        assert!(Settings::parse("colour = 1").is_err());
        assert_eq!(Settings::parse("theme = \"COBALT\"").unwrap().theme_idx, 2);
        assert!(Settings::parse("panels = [\"cpu\", \"radar\"]").is_err());
    }

    #[test]
    fn panels_list_what_is_shown() {
        let s = Settings::parse("panels = [\"CPU\", \"procs\"]").unwrap();
        assert!(s.panels.cpu && s.panels.procs);
        assert!(!s.panels.memory && !s.panels.gpu);
        assert_eq!(
            Settings::default().panels,
            Panels::parse(PANEL_NAMES).unwrap()
        );
        assert!(!Panels::parse(Vec::<String>::new()).unwrap().cpu);
    }
}
