//! The facility schematic: an easter egg of three screens, opened by typing 142 in the manifest.
//!
//! 1. ACCESS: a clearance check, typed out.
//! 2. ELEVATION: the silo seen from the side, 144 levels, one band of levels per CPU core, lit by
//!    that core's load, with a scan line sweeping down the shaft.
//! 3. PLAN: one level seen from above, rings and sectors around the central stair, the busiest
//!    processes lighting the outer sectors under a radar sweep.
//! 4. SAFEGUARD: looking was noticed. The announcement flashes and, if a speech synthesizer is
//!    installed, a robotic voice reads it.

use std::f64::consts::TAU;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Circle, Line as CLine, Points};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use super::widgets::{big_text, big_text_width, fit};
use crate::app::{App, SAFEGUARD_PAGE, SCHEMATIC_PAGES};
use crate::theme::Theme;

const LEVELS: usize = 144;
const HERE: usize = 142;
const SECTORS: usize = 16;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let Some(s) = &app.schematic else {
        return;
    };
    let t = app.theme();
    let r = area.inner(Margin::new(2, 1));
    f.render_widget(Clear, r);
    let titles = ["ACCESS REQUEST", "ELEVATION", "PLAN ▸ LEVEL 142", "ANOMALY"];
    let alarm = s.page == SAFEGUARD_PAGE;
    let border = if alarm && (app.frame / 3).is_multiple_of(2) {
        t.crit
    } else {
        t.bright
    };
    let left = SCHEMATIC_PAGES[s.page]
        .saturating_sub(s.page_start.elapsed())
        .as_secs()
        + 1;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(border))
        .style(Style::new().bg(t.bg))
        .title(
            Line::styled(
                format!(" ╡ FACILITY SCHEMATIC ▸ {} ╞ ", titles[s.page]),
                Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
            )
            .centered(),
        )
        .title_bottom(
            Line::styled(
                format!(
                    " SHEET {}/{} ▸ ANY KEY NEXT ▸ ESC SEALS ▸ {left}s ",
                    s.page + 1,
                    SCHEMATIC_PAGES.len()
                ),
                Style::new().fg(t.dim),
            )
            .centered(),
        );
    let inner = block.inner(r).inner(Margin::new(1, 0));
    f.render_widget(block, r);
    let ms = s.page_start.elapsed().as_millis() as u64;
    match s.page {
        0 => access(f, inner, &t, ms),
        1 => elevation(f, inner, app, &t, ms),
        2 => plan(f, inner, app, &t, ms),
        _ => safeguard(f, inner, app, &t, ms),
    }
}

fn access(f: &mut Frame, area: Rect, t: &Theme, ms: u64) {
    let user = std::env::var("USER").unwrap_or_else(|_| "UNKNOWN".into());
    let dots = ".".repeat(((ms / 120) % 12) as usize);
    let steps: [(u64, String); 5] = [
        (0, "REQUEST    ▸ FACILITY SCHEMATIC, ALL LEVELS".into()),
        (300, format!("OPERATOR   ▸ {}", user.to_uppercase())),
        (600, format!("CLEARANCE  ▸ VERIFYING{dots}")),
        (1800, "CLEARANCE  ▸ GRANTED, LEVEL 142 TERMINAL".into()),
        (
            2300,
            "NOTICE     ▸ WHAT YOU SEE HERE IS NOT TO BE DISCUSSED".into(),
        ),
    ];
    let [_, logo, _, text, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Length(5),
        Constraint::Fill(1),
    ])
    .areas(area);
    // the widest "142" that fits
    if let Some(scale) = (1..=4)
        .rev()
        .find(|&s| big_text_width("142", s) <= logo.width)
    {
        let x = logo.x + (logo.width - big_text_width("142", scale)) / 2;
        big_text(
            f.buffer_mut(),
            x,
            logo.y,
            "142",
            scale,
            t.bright,
            t.fg,
            logo,
        );
    }
    let lines: Vec<Line> = steps
        .iter()
        .filter(|(at, _)| ms >= *at)
        .map(|(at, l)| {
            // the verdict line replaces the "verifying" one
            if *at == 600 && ms >= 1800 {
                return Line::default();
            }
            let granted = l.contains("GRANTED");
            Line::styled(
                l.clone(),
                if granted {
                    Style::new().fg(t.bright).add_modifier(Modifier::BOLD)
                } else {
                    Style::new().fg(t.fg)
                },
            )
        })
        .filter(|l| !l.spans.is_empty())
        .collect();
    let w = 58.min(text.width);
    let text = Rect {
        x: text.x + (text.width - w) / 2,
        width: w,
        ..text
    };
    f.render_widget(Paragraph::new(lines), text);
}

fn elevation(f: &mut Frame, area: Rect, app: &App, t: &Theme, ms: u64) {
    let cores = &app.mon.cpu_cores;
    let n = cores.len().max(1);
    let shaft_w = 26.min(area.width / 3).max(8);
    let [ruler, _, shaft, _, notes] = Layout::horizontal([
        Constraint::Length(5),
        Constraint::Length(1),
        Constraint::Length(shaft_w),
        Constraint::Length(3),
        Constraint::Min(10),
    ])
    .areas(area);
    let h = shaft.height.max(1) as usize;
    let level_of = |row: usize| row * LEVELS / h + 1;
    let row_of = |level: usize| ((level - 1) * h / LEVELS).min(h - 1);
    let scan = (ms / 90) as usize % (h + 6);
    let here = row_of(HERE);
    let blink = (app.frame / 4).is_multiple_of(2);
    let buf = f.buffer_mut();

    for row in 0..h {
        let y = shaft.y + row as u16;
        let level = level_of(row);
        // ruler: a tick every row, a number every 12 levels
        let label = if row == 0 || level_of(row.saturating_sub(1)) / 12 != level / 12 {
            format!("{level:>3}─┤")
        } else {
            "   ─┤".into()
        };
        buf.set_string(
            ruler.x,
            y,
            fit(&label, ruler.width as usize),
            Style::new().fg(t.dim),
        );

        let core = ((level - 1) * n / LEVELS).min(n - 1);
        let load = cores.get(core).map(|c| c.last() / 100.0).unwrap_or(0.0);
        let lit = (load * (shaft_w as f64 - 6.0) / 2.0).round() as usize;
        let side = (shaft_w as usize - 6) / 2;
        let stair = if row % 2 == 0 { "╱╲" } else { "╲╱" };
        let fill = |k: usize| -> (String, Style) {
            (
                "▤".repeat(k.min(side)) + &"·".repeat(side - k.min(side)),
                Style::new().fg(t.level(load)),
            )
        };
        let (l, ls) = fill(lit);
        let (rr, rs) = fill(lit);
        let rr: String = rr.chars().rev().collect();
        let wall = Style::new().fg(if row == scan { t.bright } else { t.fg });
        let mut x = shaft.x;
        for (txt, st) in [
            ("╟".to_string(), wall),
            (l, ls),
            ("║".to_string(), wall),
            (stair.to_string(), Style::new().fg(t.dim)),
            ("║".to_string(), wall),
            (rr, rs),
            ("╢".to_string(), wall),
        ] {
            buf.set_string(x, y, &txt, st);
            x += txt.chars().count() as u16;
        }
        if row == scan || row + 1 == scan {
            for xx in shaft.x..shaft.x + shaft_w {
                if let Some(c) = buf.cell_mut((xx, y)) {
                    c.set_fg(t.bright);
                }
            }
        }
        if row == here {
            let mark = if blink { "◀ YOU ARE HERE" } else { "◀" };
            buf.set_string(
                shaft.x + shaft_w + 1,
                y,
                mark,
                Style::new().fg(t.warn).add_modifier(Modifier::BOLD),
            );
        }
    }

    // notes: one line per band of levels
    let k = Style::new().fg(t.dim);
    let mut lines = vec![
        Line::styled(
            format!("{LEVELS} LEVELS ▸ ONE BAND PER REACTOR CORE"),
            Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
        ),
        Line::default(),
    ];
    let room = notes.height.saturating_sub(4) as usize;
    let step = n.div_ceil(room.max(1));
    for core in (0..n).step_by(step.max(1)) {
        let from = core * LEVELS / n + 1;
        let to = ((core + 1) * LEVELS / n).max(from);
        let load = cores.get(core).map(|c| c.last()).unwrap_or(0.0);
        let segs = 10;
        let on = ((load / 100.0) * segs as f64).round() as usize;
        lines.push(Line::from(vec![
            Span::styled(format!("L{from:03}–{to:03} "), k),
            Span::styled(format!("CORE {core:02} "), Style::new().fg(t.fg)),
            Span::styled(
                "▮".repeat(on.min(segs)),
                Style::new().fg(t.level(load / 100.0)),
            ),
            Span::styled("▯".repeat(segs - on.min(segs)), Style::new().fg(t.faint)),
            Span::styled(format!(" {load:>3.0}%"), Style::new().fg(t.bright)),
        ]));
    }
    lines.push(Line::default());
    lines.push(Line::styled(
        "STAIR ▸ SINGLE SPIRAL, ALL LEVELS. NO LIFTS.",
        k,
    ));
    f.render_widget(Paragraph::new(lines), notes);
}

fn plan(f: &mut Frame, area: Rect, app: &App, t: &Theme, ms: u64) {
    let [map, _, notes] = Layout::horizontal([
        Constraint::Fill(3),
        Constraint::Length(2),
        Constraint::Fill(2),
    ])
    .areas(area);
    let [map, ruler] = Layout::vertical([Constraint::Min(4), Constraint::Length(1)]).areas(map);

    // the busiest processes take the outer sectors
    let mut busy: Vec<_> = app.mon.procs.iter().filter(|p| p.cpu >= 0.5).collect();
    busy.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
    busy.truncate(SECTORS / 2);
    let lit: Vec<(usize, f64)> = busy
        .iter()
        .enumerate()
        .map(|(i, p)| {
            (
                (i * 2 + 1) % SECTORS,
                (p.cpu as f64 / 100.0).clamp(0.15, 1.0),
            )
        })
        .collect();

    // braille dots are about square, so the x range follows the area's shape
    let ry = 1.08;
    let rx = ry * map.width as f64 / (map.height.max(1) as f64 * 2.0);
    let sweep = (ms as f64 / 2600.0 * TAU) % TAU;
    let (fg, dim, bright, warn) = (t.fg, t.dim, t.bright, t.warn);
    let lit_sectors = lit.clone();
    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .background_color(t.bg)
        .x_bounds([-rx, rx])
        .y_bounds([-ry, ry])
        .paint(move |ctx| {
            for (r, c) in [
                (1.0, fg),
                (0.97, dim),
                (0.72, fg),
                (0.46, fg),
                (0.2, bright),
                (0.1, dim),
            ] {
                ctx.draw(&Circle {
                    x: 0.0,
                    y: 0.0,
                    radius: r,
                    color: c,
                });
            }
            // walls between sectors: 16 outside, 8 in the middle ring
            for i in 0..SECTORS {
                let a = i as f64 / SECTORS as f64 * TAU;
                let inner = if i % 2 == 0 { 0.46 } else { 0.72 };
                ctx.draw(&CLine {
                    x1: inner * a.cos(),
                    y1: inner * a.sin(),
                    x2: a.cos(),
                    y2: a.sin(),
                    color: dim,
                });
            }
            // the two pump rooms, set into the middle ring
            for side in [-1.0, 1.0] {
                ctx.draw(&Circle {
                    x: side * 0.59,
                    y: 0.0,
                    radius: 0.1,
                    color: fg,
                });
                ctx.draw(&Circle {
                    x: side * 0.59,
                    y: 0.0,
                    radius: 0.04,
                    color: bright,
                });
            }
            // lit sectors, filled with dots
            for &(sector, load) in &lit_sectors {
                let a0 = sector as f64 / SECTORS as f64 * TAU;
                let mut pts = Vec::new();
                let steps = (6.0 + load * 10.0) as usize;
                for i in 1..steps {
                    for j in 1..8 {
                        let a = a0 + i as f64 / steps as f64 * TAU / SECTORS as f64;
                        let r = 0.72 + j as f64 / 8.0 * 0.27;
                        pts.push((r * a.cos(), r * a.sin()));
                    }
                }
                ctx.draw(&Points {
                    coords: &pts,
                    color: if load > 0.7 { warn } else { bright },
                });
            }
            // radar sweep, with a short fading trail
            for (k, c) in [(0.0, bright), (0.06, fg), (0.12, dim)] {
                let a = sweep - k;
                ctx.draw(&CLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 0.99 * a.cos(),
                    y2: 0.99 * a.sin(),
                    color: c,
                });
            }
        });
    f.render_widget(canvas, map);

    // a dimension line under the plan, as on a drawing
    let w = ruler.width as usize;
    if w >= 12 {
        let label = " 1:400 ";
        let side = (w - 2 - label.len()) / 2;
        let line = format!(
            "├{}{label}{}┤",
            "─".repeat(side),
            "─".repeat(w - 2 - label.len() - side)
        );
        f.buffer_mut()
            .set_string(ruler.x, ruler.y, line, Style::new().fg(t.dim));
    }

    let k = Style::new().fg(t.dim);
    let mut lines = vec![
        Line::styled(
            "LEVEL 142 ▸ CROSS-SECTION",
            Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
        ),
        Line::styled("CENTRAL STAIR ▸ 2 PUMP ROOMS ▸ 16 SECTORS", k),
        Line::default(),
        Line::styled(
            "OCCUPIED SECTORS",
            Style::new().fg(t.fg).add_modifier(Modifier::BOLD),
        ),
    ];
    if busy.is_empty() {
        lines.push(Line::styled("NONE ▸ THE LEVEL IS QUIET", k));
    }
    let name_w = (notes.width as usize).saturating_sub(15).max(4);
    for (p, (sector, _)) in busy.iter().zip(&lit) {
        lines.push(Line::from(vec![
            Span::styled(format!("S{:02} ", sector + 1), k),
            Span::styled(
                format!("{:<name_w$}", fit(&p.name.to_uppercase(), name_w)),
                Style::new().fg(t.fg),
            ),
            Span::styled(
                format!(" {:>5.1}%", p.cpu),
                Style::new().fg(t.level(p.cpu as f64 / 100.0)),
            ),
        ]));
    }
    f.render_widget(Paragraph::new(lines), notes);
}

fn safeguard(f: &mut Frame, area: Rect, app: &App, t: &Theme, ms: u64) {
    let flash = (app.frame / 3).is_multiple_of(2);
    let word = "SAFEGUARD";
    let big_h = 3;
    let [_, logo, _, text] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(big_h),
        Constraint::Length(1),
        Constraint::Fill(2),
    ])
    .areas(area);
    if let Some(scale) = (1..=4)
        .rev()
        .find(|&s| big_text_width(word, s) <= logo.width)
    {
        let x = logo.x + (logo.width - big_text_width(word, scale)) / 2;
        let (a, b) = if flash {
            (t.crit, t.warn)
        } else {
            (t.warn, t.crit)
        };
        big_text(f.buffer_mut(), x, logo.y, word, scale, a, b, logo);
    }
    let lines = [
        (
            0,
            "ANOMALY DETECTED ▸ UNAUTHORIZED SCHEMATIC ACCESS, LEVEL 142",
            t.fg,
        ),
        (700, "SAFEGUARD WILL BE ACTIVATED", t.crit),
        (
            1900,
            "REMAIN IN YOUR QUARTERS. THIS IS FOR YOUR PROTECTION.",
            t.dim,
        ),
    ];
    let shown: Vec<Line> = lines
        .iter()
        .filter(|(at, _, _)| ms >= *at)
        .map(|(_, l, c)| {
            let style = Style::new().fg(*c);
            let style = if *c == t.crit {
                style.add_modifier(Modifier::BOLD)
            } else {
                style
            };
            Line::styled(*l, style).centered()
        })
        .collect();
    f.render_widget(Paragraph::new(shown), text);
}
