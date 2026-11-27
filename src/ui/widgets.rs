//! Hand-drawn instruments: block font, segmented bars, vertical gauges, tanks and area charts.
//! They draw straight into the buffer so every cell can get its own color.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders};

use crate::theme::{Theme, lerp};

const EIGHTHS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// A panel frame: thick border, numbered title on the left, a subtitle on the right.
/// The subtitle is dropped when the panel is too narrow for both.
pub fn panel<'a>(
    t: &Theme,
    width: u16,
    code: &str,
    title: &str,
    sub: String,
    lit: bool,
) -> Block<'a> {
    let lamp = if lit { "◉" } else { "○" };
    let needed = code.chars().count() + title.chars().count() + sub.chars().count() + 16;
    let sub = if needed <= width as usize {
        sub
    } else {
        String::new()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::new().fg(t.dim))
        .title(Line::from(vec![
            Span::styled("┫", Style::new().fg(t.dim)),
            Span::styled(
                format!(" {lamp} "),
                Style::new().fg(if lit { t.warn } else { t.faint }),
            ),
            Span::styled(
                format!("{code} "),
                Style::new().fg(t.bg).bg(t.dim).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {title} "),
                Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
            ),
            Span::styled("┣", Style::new().fg(t.dim)),
        ]));
    if sub.is_empty() {
        return block;
    }
    block.title(
        Line::from(vec![
            Span::styled("┫ ", Style::new().fg(t.dim)),
            Span::styled(sub, Style::new().fg(t.dim)),
            Span::styled(" ┣", Style::new().fg(t.dim)),
        ])
        .right_aligned(),
    )
}

// ---------------------------------------------------------------------------------------------
// Block font: 3x5 pixel glyphs drawn with half blocks, so one glyph is 3 cells tall.

fn glyph(c: char) -> [&'static str; 5] {
    match c {
        '0' | 'O' => ["###", "# #", "# #", "# #", "###"],
        '1' => [" # ", "## ", " # ", " # ", "###"],
        '2' => ["###", "  #", "###", "#  ", "###"],
        '3' => ["###", "  #", "###", "  #", "###"],
        '4' => ["# #", "# #", "###", "  #", "  #"],
        '5' => ["###", "#  ", "###", "  #", "###"],
        '6' => ["###", "#  ", "###", "# #", "###"],
        '7' => ["###", "  #", "  #", "  #", "  #"],
        '8' => ["###", "# #", "###", "# #", "###"],
        '9' => ["###", "# #", "###", "  #", "###"],
        'B' => ["## ", "# #", "## ", "# #", "## "],
        'L' => ["#  ", "#  ", "#  ", "#  ", "###"],
        'I' => ["###", " # ", " # ", " # ", "###"],
        'T' => ["###", " # ", " # ", " # ", " # "],
        'P' => ["###", "# #", "###", "#  ", "#  "],
        'U' => ["# #", "# #", "# #", "# #", "###"],
        'C' => ["###", "#  ", "#  ", "#  ", "###"],
        'K' => ["# #", "# #", "## ", "# #", "# #"],
        'W' => ["# #", "# #", "###", "###", "# #"],
        'N' => ["###", "# #", "# #", "# #", "# #"],
        'D' => ["## ", "# #", "# #", "# #", "## "],
        'E' => ["###", "#  ", "###", "#  ", "###"],
        'R' => ["###", "# #", "## ", "# #", "# #"],
        'S' => ["###", "#  ", "###", "  #", "###"],
        'A' => ["###", "# #", "###", "# #", "# #"],
        'F' => ["###", "#  ", "## ", "#  ", "#  "],
        'G' => ["###", "#  ", "# #", "# #", "###"],
        ':' => [" ", "#", " ", "#", " "],
        // a colon that is switched off: same width as ':' so blinking never moves the text
        ';' => [" ", " ", " ", " ", " "],
        _ => ["   ", "   ", "   ", "   ", "   "],
    }
}

/// Width in cells of `text` drawn by [`big_text`] with horizontal scale `sx`.
pub fn big_text_width(text: &str, sx: u16) -> u16 {
    let n = text.chars().count() as u16;
    text.chars()
        .map(|c| glyph(c)[0].len() as u16 * sx)
        .sum::<u16>()
        + n.saturating_sub(1)
}

/// Draws `text` in the block font. Each pixel is `sx` cells wide; the color fades along x.
#[allow(clippy::too_many_arguments)]
pub fn big_text(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    text: &str,
    sx: u16,
    from: Color,
    to: Color,
    clip: Rect,
) {
    let total = big_text_width(text, sx).max(1);
    let mut cx = x;
    for ch in text.chars() {
        let g = glyph(ch);
        let w = g[0].len() as u16;
        for gx in 0..w {
            for row in 0..3u16 {
                let top = pixel(&g, gx as usize, row as usize * 2);
                let bot = pixel(&g, gx as usize, row as usize * 2 + 1);
                let sym = match (top, bot) {
                    (true, true) => "█",
                    (true, false) => "▀",
                    (false, true) => "▄",
                    (false, false) => continue,
                };
                for s in 0..sx {
                    let px = cx + gx * sx + s;
                    let py = y + row;
                    if px < clip.right() && py < clip.bottom() && px >= clip.x && py >= clip.y {
                        let color = lerp(from, to, (px - x) as f64 / total as f64);
                        if let Some(cell) = buf.cell_mut((px, py)) {
                            cell.set_symbol(sym).set_fg(color);
                        }
                    }
                }
            }
        }
        cx += w * sx + 1;
    }
}

fn pixel(g: &[&str; 5], x: usize, y: usize) -> bool {
    y < 5 && g[y].as_bytes().get(x) == Some(&b'#')
}

// ---------------------------------------------------------------------------------------------

/// A horizontal bar made of discrete segments: `▮▮▮▮▯▯▯`. Each lit segment is colored by its position.
pub fn seg_bar(buf: &mut Buffer, area: Rect, frac: f64, t: &Theme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let n = area.width as usize;
    let lit = (frac.clamp(0.0, 1.0) * n as f64).round() as usize;
    for i in 0..n {
        let pos = (i as f64 + 0.5) / n as f64;
        let (sym, color) = if i < lit {
            ("▮", t.level(pos))
        } else {
            ("▯", t.faint)
        };
        if let Some(cell) = buf.cell_mut((area.x + i as u16, area.y)) {
            cell.set_symbol(sym).set_fg(color);
        }
    }
}

/// A vertical gauge with sub-cell resolution, filled from the bottom.
pub fn vbar(buf: &mut Buffer, area: Rect, frac: f64, t: &Theme) {
    let h = area.height as f64;
    let filled = frac.clamp(0.0, 1.0) * h * 8.0;
    for row in 0..area.height {
        // row 0 is the bottom
        let y = area.bottom() - 1 - row;
        let eighths = (filled - row as f64 * 8.0).clamp(0.0, 8.0) as usize;
        let pos = (row as f64 + 0.5) / h;
        for x in area.x..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                if eighths == 0 {
                    cell.set_symbol(if row % 2 == 0 { "·" } else { " " })
                        .set_fg(t.faint);
                } else {
                    cell.set_symbol(EIGHTHS[eighths]).set_fg(t.level(pos));
                }
            }
        }
    }
}

/// Column chart of `values` (0..=1), newest on the right, over a dotted grid.
pub fn area_chart(buf: &mut Buffer, area: Rect, values: &[f64], t: &Theme, color: Option<Color>) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let h = area.height as f64;
    // grid
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let gy = (area.bottom() - 1 - y).is_multiple_of(3);
            let gx = (area.right() - 1 - x).is_multiple_of(6);
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(if gx && gy {
                    "┼"
                } else if gy {
                    "·"
                } else {
                    " "
                })
                .set_fg(t.faint);
            }
        }
    }
    let w = area.width as usize;
    let start = values.len().saturating_sub(w);
    let shown = &values[start..];
    let offset = w - shown.len();
    for (i, v) in shown.iter().enumerate() {
        let x = area.x + (offset + i) as u16;
        let filled = v.clamp(0.0, 1.0) * h * 8.0;
        for row in 0..area.height {
            let eighths = (filled - row as f64 * 8.0).clamp(0.0, 8.0) as usize;
            if eighths == 0 {
                continue;
            }
            let y = area.bottom() - 1 - row;
            let c = color.unwrap_or_else(|| t.level((row as f64 + 1.0) / h));
            // The newest column glows.
            let c = if i + 1 == shown.len() { t.bright } else { c };
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(EIGHTHS[eighths]).set_fg(c);
            }
        }
    }
}

/// A liquid tank: walls, a fill level and an animated surface.
pub fn tank(buf: &mut Buffer, area: Rect, frac: f64, label: &str, t: &Theme, frame: u64) {
    if area.width < 4 || area.height < 4 {
        return;
    }
    let style = Style::new().fg(t.dim);
    let (l, r, b) = (area.x, area.right() - 1, area.bottom() - 1);
    let top = area.y + 1;
    buf.set_string(
        area.x,
        area.y,
        center(label, area.width as usize),
        Style::new().fg(t.bright),
    );
    for y in top..b {
        buf.set_string(l, y, "▐", style);
        buf.set_string(r, y, "▌", style);
    }
    buf.set_string(l, b, "▝", style);
    buf.set_string(r, b, "▘", style);
    for x in l + 1..r {
        buf.set_string(x, b, "▀", style);
    }
    let inner_h = (b - top) as f64;
    let level = frac.clamp(0.0, 1.0) * inner_h;
    let full_rows = level.floor() as u16;
    let color = t.level(frac);
    let deep = lerp(t.bg, color, 0.55);
    // measurement ticks on the left wall
    for i in 0..(b - top) {
        if i % 2 == 0 {
            buf.set_string(l, b - 1 - i, "▐", Style::new().fg(t.dim));
        }
    }
    for row in 0..(b - top) {
        let y = b - 1 - row;
        for x in l + 1..r {
            let cell = buf.cell_mut((x, y)).unwrap();
            if row < full_rows {
                // lower water is darker
                let depth = 1.0 - row as f64 / inner_h.max(1.0);
                cell.set_symbol(if (x + y + (frame / 4) as u16).is_multiple_of(7) {
                    "░"
                } else {
                    "█"
                })
                .set_fg(lerp(color, deep, depth * 0.8));
            } else if row == full_rows {
                let rem = level - full_rows as f64;
                let wave = ((x as f64 * 0.9 + frame as f64 * 0.35).sin() * 1.5 + rem * 8.0).round();
                let e = wave.clamp(1.0, 7.0) as usize;
                if level > 0.02 {
                    cell.set_symbol(EIGHTHS[e]).set_fg(color);
                }
            }
        }
    }
}

pub fn center(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if n >= w {
        s.chars().take(w).collect()
    } else {
        let pad = (w - n) / 2;
        format!("{}{}{}", " ".repeat(pad), s, " ".repeat(w - n - pad))
    }
}

/// Text that is cut to `w` chars, with an ellipsis when it does not fit.
pub fn fit(s: &str, w: usize) -> String {
    if s.chars().count() <= w {
        format!("{s:<w$}")
    } else if w == 0 {
        String::new()
    } else {
        let mut out: String = s.chars().take(w - 1).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::big_text_width;

    #[test]
    fn blinking_clock_keeps_its_width() {
        assert_eq!(big_text_width("12:35:04", 1), big_text_width("12;35;04", 1));
    }
}
