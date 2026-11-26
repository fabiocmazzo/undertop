//! CRT emulation applied over the finished frame: scanlines, a vignette, brownout flicker and
//! the occasional signal glitch. Toggle with `x`.

use ratatui::Frame;

use crate::app::App;
use crate::theme::scale;

pub fn apply(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let (w, h) = (area.width as f64, area.height as f64);

    // Brownout: a strip of a few rows dims for one frame. (A whole-screen dip looked similar but
    // repainted every cell twice, which stalls slower terminals and the vhs recorder.)
    let brownout = if app.rand() < 0.05 {
        Some((app.rand() * h) as u16)
    } else {
        None
    };
    // A glitch band: a row shifted sideways for one frame.
    let glitch_row = if app.rand() < 0.04 {
        Some((app.rand() * h) as u16)
    } else {
        None
    };
    let glitch_shift = 1 + (app.rand() * 3.0) as u16;
    // A slow bright band rolling down the screen, like an old monitor's refresh. It moves one whole
    // row every 3 frames, so most frames repaint nothing for it (terminals and recorders thank us).
    let roll = ((app.frame / 3) as f64) % (h + 8.0) - 4.0;

    let buf = f.buffer_mut();
    for y in area.y..area.bottom() {
        let scan = if y % 2 == 1 { 0.80 } else { 1.0 };
        let band = 1.0 + 0.10 * (1.0 - ((y as f64 - roll).abs() / 4.0).min(1.0));
        for x in area.x..area.right() {
            // vignette: darker toward the corners
            let dx = (x as f64 - w / 2.0) / (w / 2.0);
            let dy = (y as f64 - h / 2.0) / (h / 2.0);
            let vig = 1.0 - 0.22 * (dx * dx + dy * dy).min(1.0);
            let dip = brownout.is_some_and(|b| y >= b && y < b + 3);
            let k = scan * band * vig * if dip { 0.75 } else { 1.0 };
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.fg = scale(cell.fg, k);
                cell.bg = scale(cell.bg, (scan * vig).max(0.6));
            }
        }
        if Some(y) == glitch_row {
            let row: Vec<_> = (area.x..area.right())
                .filter_map(|x| buf.cell((x, y)).cloned())
                .collect();
            for (i, x) in (area.x..area.right()).enumerate() {
                let src =
                    (i + row.len() - glitch_shift as usize % row.len().max(1)) % row.len().max(1);
                if let (Some(cell), Some(s)) = (buf.cell_mut((x, y)), row.get(src)) {
                    *cell = s.clone();
                }
            }
        }
    }
}

/// The old-CRT switch-off after `q`, with `p` going from 0 to 1: the picture squashes into a
/// horizontal line, the line shrinks to a dot, the dot fades.
pub fn power_down(f: &mut Frame, p: f64, t: &crate::theme::Theme) {
    use crate::theme::lerp;
    let area = f.area();
    let buf = f.buffer_mut();
    let snapshot = buf.clone();
    let (w, h) = (area.width as f64, area.height as f64);
    let (cx, cy) = (area.x as f64 + w / 2.0, area.y as f64 + h / 2.0);
    let blank = |cell: &mut ratatui::buffer::Cell| {
        cell.reset();
        cell.set_bg(t.bg);
    };
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if let Some(c) = buf.cell_mut((x, y)) {
                blank(c);
            }
        }
    }
    if p < 0.55 {
        // vertical collapse: rows are sampled from a picture squeezed toward the middle
        let q = p / 0.55;
        let half = ((1.0 - q) * h / 2.0).max(0.5);
        let squeeze = (h / 2.0) / half;
        for y in area.y..area.bottom() {
            let dy = y as f64 + 0.5 - cy;
            if dy.abs() > half {
                continue;
            }
            let src = ((cy + dy * squeeze).floor() as u16).clamp(area.y, area.bottom() - 1);
            for x in area.x..area.right() {
                if let (Some(dst), Some(s)) = (buf.cell_mut((x, y)), snapshot.cell((x, src))) {
                    *dst = s.clone();
                    dst.set_fg(lerp(s.fg, t.bright, q * 0.8));
                }
            }
        }
    } else if p < 0.85 {
        // a bright line shrinking toward the center
        let q = (p - 0.55) / 0.3;
        let half = (1.0 - q) * w / 2.0;
        let y = cy as u16;
        for x in area.x..area.right() {
            if (x as f64 + 0.5 - cx).abs() <= half.max(0.5)
                && let Some(c) = buf.cell_mut((x, y))
            {
                c.set_symbol("━").set_fg(t.bright);
            }
        }
    } else {
        let q = (p - 0.85) / 0.15;
        if let Some(c) = buf.cell_mut((cx as u16, cy as u16)) {
            c.set_symbol("●").set_fg(lerp(t.bright, t.bg, q));
        }
    }
}
