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
