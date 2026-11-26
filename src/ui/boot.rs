//! Power-on self test shown at start. Any key skips it.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use super::widgets::{big_text, big_text_width, center, seg_bar};
use crate::app::{App, BOOT_TIME};
use crate::sysmon::fmt_bytes;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let m = &app.mon;
    let elapsed = app.boot_start.elapsed().as_secs_f64();
    let progress = elapsed / BOOT_TIME.as_secs_f64();

    let checks: Vec<(String, String)> = vec![
        (
            "MAINFRAME BIOS REV 0.1 ▸ MEMORY TEST".into(),
            fmt_bytes(m.mem_total as f64),
        ),
        (
            "PROBING REACTOR CORES".into(),
            format!("{} FOUND", m.core_count()),
        ),
        (
            "THERMAL SENSOR GRID".into(),
            format!("{} ONLINE", m.temps.len()),
        ),
        (
            "STORAGE VAULT SEALS".into(),
            format!("{} HOLDING", m.disks_rows.len()),
        ),
        ("COMMS ARRAY".into(), "CARRIER OK".into()),
        (
            "LOADING PROCESS MANIFEST".into(),
            format!("{} ENTRIES", m.procs.len()),
        ),
        (
            format!("HANDSHAKE WITH NODE {}", m.host.to_uppercase()),
            "ACCEPTED".into(),
        ),
        ("TERMINAL LOCATION".into(), "LEVEL 142 ▸ I.T.".into()),
        ("OPERATOR CLEARANCE".into(), "GRANTED".into()),
    ];

    // the biggest logo that fits
    let scale = (1..=4)
        .rev()
        .find(|&s| big_text_width("UNDERTOP", s) + 4 <= area.width)
        .unwrap_or(1);
    let logo_w = big_text_width("UNDERTOP", scale);
    let block_h = 3 + 2 + checks.len() as u16 + 3;
    let top = area.y + area.height.saturating_sub(block_h) / 2;
    let lx = area.x + area.width.saturating_sub(logo_w) / 2;
    big_text(
        f.buffer_mut(),
        lx,
        top,
        "UNDERTOP",
        scale,
        t.bright,
        t.warn,
        area,
    );

    let w = 60u16.min(area.width.saturating_sub(4));
    let x = area.x + (area.width - w) / 2;
    let buf = f.buffer_mut();
    buf.set_string(
        x,
        top + 4,
        center("SUBTERRANEAN OPERATIONS TERMINAL ▸ COLD START", w as usize),
        Style::new().fg(t.dim),
    );

    let per_line = 0.85 / checks.len() as f64;
    for (i, (label, result)) in checks.iter().enumerate() {
        let start = i as f64 * per_line;
        if progress < start {
            break;
        }
        let y = top + 6 + i as u16;
        let local = ((progress - start) / per_line).min(1.0);
        // typewriter for the label, then dot leaders, then the result
        let shown = ((label.chars().count() as f64) * (local * 2.0).min(1.0)) as usize;
        let text: String = label.chars().take(shown).collect();
        buf.set_string(x, y, &text, Style::new().fg(t.fg));
        if local >= 0.5 {
            let res = format!(" {result}");
            let dots = (w as usize).saturating_sub(label.chars().count() + res.chars().count());
            buf.set_string(
                x + label.chars().count() as u16,
                y,
                ".".repeat(dots),
                Style::new().fg(t.faint),
            );
            buf.set_string(
                x + w - res.chars().count() as u16,
                y,
                &res,
                Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
            );
        } else if (app.frame / 2).is_multiple_of(2) {
            buf.set_string(
                x + text.chars().count() as u16,
                y,
                "█",
                Style::new().fg(t.fg),
            );
        }
    }

    let by = top + 7 + checks.len() as u16;
    seg_bar(buf, Rect::new(x, by, w.saturating_sub(6), 1), progress, &t);
    buf.set_string(
        x + w - 4,
        by,
        format!("{:>3.0}%", (progress * 100.0).min(100.0)),
        Style::new().fg(t.bright),
    );
    if (app.frame / 5).is_multiple_of(2) {
        buf.set_string(
            x,
            by + 2,
            center("PRESS ANY KEY TO SKIP", w as usize),
            Style::new().fg(t.dim),
        );
    }
}
