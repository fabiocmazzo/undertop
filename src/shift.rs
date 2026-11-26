//! The end-of-shift report: what happened while the terminal was open.

use std::time::Instant;

use chrono::{DateTime, Local};

use crate::sysmon::{Monitor, fmt_bytes, fmt_duration};

pub struct ShiftStats {
    start: Instant,
    cpu_peak: Option<(f64, DateTime<Local>)>,
    cpu_sum: f64,
    samples: u64,
    mem_peak: (f64, u64),
    temp_peak: Option<(f32, String)>,
    net_start: Option<(u64, u64)>,
    net: (u64, u64),
    pub alerts: u32,
    pub lockdowns: u32,
    pub terminations: u32,
    pub dossiers: u32,
}

impl ShiftStats {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            cpu_peak: None,
            cpu_sum: 0.0,
            samples: 0,
            mem_peak: (0.0, 0),
            temp_peak: None,
            net_start: None,
            net: (0, 0),
            alerts: 0,
            lockdowns: 0,
            terminations: 0,
            dossiers: 0,
        }
    }

    /// Called after every telemetry refresh.
    pub fn record(&mut self, m: &Monitor) {
        let cpu = m.cpu_total.last();
        self.cpu_sum += cpu;
        self.samples += 1;
        if self.cpu_peak.as_ref().is_none_or(|(p, _)| cpu > *p) {
            self.cpu_peak = Some((cpu, Local::now()));
        }
        let mem = m.mem.last();
        if mem > self.mem_peak.0 {
            self.mem_peak = (mem, m.mem_used);
        }
        if let Some(hot) = m.temps.iter().max_by(|a, b| a.temp.total_cmp(&b.temp))
            && self.temp_peak.as_ref().is_none_or(|(t, _)| hot.temp > *t)
        {
            self.temp_peak = Some((hot.temp, hot.label.to_uppercase()));
        }
        let (rx0, tx0) = *self.net_start.get_or_insert((m.rx_total, m.tx_total));
        self.net = (
            m.rx_total.saturating_sub(rx0),
            m.tx_total.saturating_sub(tx0),
        );
    }

    pub fn verdict(&self) -> &'static str {
        if self.lockdowns > 0 {
            "INCIDENT LOGGED. EXPECT AN INTERVIEW."
        } else if self.terminations > 3 {
            "EXCESSIVE FORCE NOTED IN YOUR RECORD."
        } else if self.alerts > 0 || self.terminations > 0 {
            "SATISFACTORY. REPORT FOR YOUR NEXT SHIFT."
        } else {
            "EXEMPLARY. NOTHING TO REPORT. SUSPICIOUSLY SO."
        }
    }

    /// The report as (label, value) lines.
    pub fn lines(&self, m: &Monitor) -> Vec<(&'static str, String)> {
        let user = std::env::var("USER").unwrap_or_else(|_| "UNKNOWN".into());
        let avg = if self.samples > 0 {
            self.cpu_sum / self.samples as f64
        } else {
            0.0
        };
        let cpu = match &self.cpu_peak {
            Some((p, at)) => format!(
                "PEAK {p:.0}% AT {} ▸ AVERAGE {avg:.0}%",
                at.format("%H:%M:%S")
            ),
            None => "NO READINGS".into(),
        };
        let temp = match &self.temp_peak {
            Some((t, label)) => format!("PEAK {t:.0}°C ({label})"),
            None => "NO SENSORS".into(),
        };
        let plural =
            |n: u32, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        vec![
            (
                "OPERATOR",
                format!("{} @ {}", user.to_uppercase(), m.host.to_uppercase()),
            ),
            ("ON DUTY", fmt_duration(self.start.elapsed().as_secs())),
            ("REACTOR", cpu),
            (
                "RESERVOIR",
                format!(
                    "PEAK {:.0}% ({})",
                    self.mem_peak.0,
                    fmt_bytes(self.mem_peak.1 as f64)
                ),
            ),
            ("THERMAL", temp),
            (
                "COMMS",
                format!(
                    "▼ {} ▲ {} THIS SHIFT",
                    fmt_bytes(self.net.0 as f64),
                    fmt_bytes(self.net.1 as f64)
                ),
            ),
            (
                "INCIDENTS",
                format!(
                    "{} ▸ {} ▸ {}",
                    plural(self.alerts, "ALERT", "ALERTS"),
                    plural(self.lockdowns, "LOCKDOWN", "LOCKDOWNS"),
                    plural(self.terminations, "TERMINATION", "TERMINATIONS")
                ),
            ),
            ("DOSSIERS", format!("{} OPENED", self.dossiers)),
            ("VERDICT", self.verdict().into()),
        ]
    }

    /// Plain text for the scrollback after the terminal is restored.
    pub fn text(&self, m: &Monitor) -> String {
        let mut out = String::from("── END OF SHIFT REPORT ──────────────────────────────\n");
        for (k, v) in self.lines(m) {
            out.push_str(&format!("  {k:<10} {v}\n"));
        }
        out.push_str("─────────────────────────── REMAIN CALM. REMAIN BELOW.\n");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::ShiftStats;

    #[test]
    fn verdict_follows_incidents() {
        let mut s = ShiftStats::new();
        assert!(s.verdict().starts_with("EXEMPLARY"));
        s.alerts = 2;
        assert!(s.verdict().starts_with("SATISFACTORY"));
        s.terminations = 4;
        assert!(s.verdict().starts_with("EXCESSIVE"));
        s.lockdowns = 1;
        assert!(s.verdict().starts_with("INCIDENT"));
    }
}
