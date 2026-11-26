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

pub const TEMPLATE: &str = "\
# undertop configuration
# Save as ~/.config/undertop/config.toml. Every key is optional.

# amber, phosphor, cobalt or lockdown
theme = \"amber\"

# milliseconds between telemetry samples (250 to 10000)
interval_ms = 1000

# CRT emulation: scanlines, vignette, rolling band, brownouts, glitches
crt = true

# power-on self test at start
boot = true

# CRT power-down animation when leaving
power_down = true

# seal the facility (LOCKDOWN palette, alarm) when a sensor reaches 95 °C
# or 95 % of its critical temperature; at most once every 10 minutes
lockdown_on_heat = true
";

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct File {
    theme: Option<String>,
    interval_ms: Option<u64>,
    crt: Option<bool>,
    boot: Option<bool>,
    power_down: Option<bool>,
    lockdown_on_heat: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub theme_idx: usize,
    pub interval: Duration,
    pub crt: bool,
    pub boot: bool,
    pub power_down: bool,
    pub lockdown_on_heat: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme_idx: 0,
            interval: Duration::from_millis(1000),
            crt: true,
            boot: true,
            power_down: true,
            lockdown_on_heat: true,
        }
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
        if let Some(ms) = f.interval_ms {
            if !(250..=10_000).contains(&ms) {
                return Err(eyre!("interval_ms must be between 250 and 10000, not {ms}"));
            }
            s.interval = Duration::from_millis(ms);
        }
        s.crt = f.crt.unwrap_or(s.crt);
        s.boot = f.boot.unwrap_or(s.boot);
        s.power_down = f.power_down.unwrap_or(s.power_down);
        s.lockdown_on_heat = f.lockdown_on_heat.unwrap_or(s.lockdown_on_heat);
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
    }
}
