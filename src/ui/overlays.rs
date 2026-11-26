//! Full overlays: the process dossier and the lockdown alarm.

use chrono::{Local, TimeZone};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use super::popup;
use super::widgets::{area_chart, big_text, big_text_width, center};
use crate::app::{App, LOCKDOWN_TIME, REPORT_TIME};
use crate::sysmon::{fmt_bytes, fmt_duration};
use crate::theme::lerp;

const CLASSIFIED: &str = "▒▒▒ CLASSIFIED ▒▒▒";

/// Draws the dossier and returns its box (a click outside it closes it).
pub fn dossier(f: &mut Frame, area: Rect, app: &App) -> Option<Rect> {
    let t = app.theme();
    let Some((pid, data)) = &app.dossier else {
        return None;
    };
    let w = area.width.saturating_sub(8).min(120);
    let h = area.height.saturating_sub(4).min(34);
    let r = popup(area, w, h);
    f.render_widget(Clear, r);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(t.bright))
        .style(Style::new().bg(t.bg))
        .title(
            Line::styled(
                format!(" ╡ DOSSIER ▸ PID {pid} ╞ "),
                Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
            )
            .centered(),
        )
        .title_bottom(
            Line::from(vec![
                Span::styled(
                    " K ",
                    Style::new()
                        .fg(t.bg)
                        .bg(t.crit)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" TERMINATE   ", Style::new().fg(t.dim)),
                Span::styled(
                    " ANY ",
                    Style::new().fg(t.bg).bg(t.dim).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" CLOSE ", Style::new().fg(t.dim)),
            ])
            .centered(),
        );
    let inner = block.inner(r).inner(Margin::new(1, 0));
    f.render_widget(block, r);

    let Some(d) = data else {
        let msg = vec![
            Line::default(),
            Line::styled(
                "SUBJECT NO LONGER ON THE MANIFEST",
                Style::new().fg(t.warn).add_modifier(Modifier::BOLD),
            )
            .centered(),
            Line::styled(
                "The process has ended or was never ours to see.",
                Style::new().fg(t.dim),
            )
            .centered(),
        ];
        f.render_widget(Paragraph::new(msg), inner);
        return Some(r);
    };

    // trend charts along the bottom, when the box is tall enough
    let inner = if inner.height >= 16 {
        let [text, _, charts] = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(5),
        ])
        .areas(inner);
        dossier_charts(f, charts, app);
        text
    } else {
        inner
    };

    let k = Style::new().fg(t.dim);
    let v = Style::new().fg(t.bright);
    let secret = Style::new().fg(t.crit).add_modifier(Modifier::BOLD);
    let field = |key: &str, val: Option<String>| -> Line<'static> {
        let (text, style) = match val {
            Some(s) if !s.is_empty() => (s, v),
            _ => (CLASSIFIED.to_string(), secret),
        };
        Line::from(vec![
            Span::styled(format!("{key:<11}"), k),
            Span::styled(text, style),
        ])
    };
    // paths keep their most specific end when they do not fit
    let path_w = (inner.width as usize).saturating_sub(11);
    let tail = |s: &Option<String>| s.as_ref().map(|p| keep_tail(p, path_w));
    let p = &d.row;
    let started = Local.timestamp_opt(d.started as i64, 0).single().map(|dt| {
        format!(
            "{} ▸ {} AGO",
            dt.format("%Y-%m-%d %H:%M:%S"),
            fmt_duration(p.run_time)
        )
    });
    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("{:<11}", "SUBJECT"), k),
            Span::styled(p.name.to_uppercase(), v.add_modifier(Modifier::BOLD)),
            Span::styled(format!("   STATE {}", p.status), k),
        ]),
        field("OPERATOR", Some(p.user.clone())),
        field(
            "PARENT",
            Some(
                d.parent
                    .as_ref()
                    .map(|(pp, n)| format!("{pp} ({n})"))
                    .unwrap_or_else(|| "NONE ▸ ORIGIN PROCESS".into()),
            ),
        ),
        field("STARTED", started),
        field(
            "LOAD",
            Some(format!(
                "{:.1}% CPU ▸ {} THREADS",
                p.cpu,
                d.threads
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "?".into())
            )),
        ),
        field(
            "MEMORY",
            Some(format!(
                "{} RESIDENT ▸ {} VIRTUAL",
                fmt_bytes(p.mem as f64),
                fmt_bytes(d.vmem as f64)
            )),
        ),
        field(
            "DISK I/O",
            Some(match d.io {
                Some((r, w)) => format!("▼R {}/s ▲W {}/s", fmt_bytes(r), fmt_bytes(w)),
                None => "MEASURING…".into(),
            }),
        ),
        field("EXE", tail(&d.exe)),
        field("CWD", tail(&d.cwd)),
        field("CGROUP", tail(&d.cgroup)),
        field("COMMAND", Some(p.cmd.clone())),
    ];
    let kids = if d.children.is_empty() {
        "NONE".to_string()
    } else {
        d.children
            .iter()
            .map(|(c, n)| format!("{c} ({n})"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    lines.push(field("CHILDREN", Some(kids)));
    lines.push(Line::default());
    match &d.fds {
        None => lines.push(field("OPEN FILES", None)),
        Some((count, targets)) => {
            lines.push(field("OPEN FILES", Some(format!("{count} DESCRIPTORS"))));
            for target in targets {
                lines.push(Line::from(vec![
                    Span::raw(" ".repeat(11)),
                    Span::styled(target.clone(), Style::new().fg(t.fg)),
                ]));
            }
        }
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    Some(r)
}

pub fn lockdown(f: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let Some(l) = &app.lockdown else { return };
    let flash = (app.frame / 4).is_multiple_of(2);
    let edge = if flash { t.crit } else { t.warn };

    // a flashing frame around the whole screen
    let buf = f.buffer_mut();
    for x in area.x..area.right() {
        for y in [area.y, area.bottom() - 1] {
            if let Some(c) = buf.cell_mut((x, y)) {
                c.set_symbol(if (x as u64 + app.frame / 2) % 4 < 2 {
                    "▚"
                } else {
                    " "
                })
                .set_fg(edge);
            }
        }
    }
    for y in area.y..area.bottom() {
        for x in [area.x, area.right() - 1] {
            if let Some(c) = buf.cell_mut((x, y)) {
                c.set_symbol("█").set_fg(edge);
            }
        }
    }

    let scale = (1..=3)
        .rev()
        .find(|&s| big_text_width("LOCKDOWN", s) + 8 <= area.width)
        .unwrap_or(1);
    let logo_w = big_text_width("LOCKDOWN", scale);
    let w = (logo_w + 8).max(64).min(area.width - 4);
    let r = popup(area, w, 13);
    f.render_widget(Clear, r);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::new().fg(edge))
        .style(Style::new().bg(lerp(t.bg, t.crit, if flash { 0.12 } else { 0.04 })));
    let inner = block.inner(r);
    f.render_widget(block, r);

    let buf = f.buffer_mut();
    let lx = inner.x + inner.width.saturating_sub(logo_w) / 2;
    big_text(
        buf,
        lx,
        inner.y + 1,
        "LOCKDOWN",
        scale,
        t.bright,
        t.crit,
        inner,
    );
    let left = LOCKDOWN_TIME.saturating_sub(l.start.elapsed()).as_secs() + 1;
    let lines = [
        (
            "CONTAINMENT BREACH ▸ LEVEL 142 SEALED".to_string(),
            Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
        ),
        (l.reason.clone(), Style::new().fg(t.warn)),
        (String::new(), Style::new()),
        (
            format!("REMAIN AT YOUR STATION ▸ ANY KEY TO ACKNOWLEDGE ▸ {left}s"),
            Style::new().fg(t.dim),
        ),
    ];
    for (i, (text, style)) in lines.iter().enumerate() {
        let y = inner.y + 5 + i as u16;
        if y < inner.bottom() {
            buf.set_string(inner.x, y, center(text, inner.width as usize), *style);
        }
    }
}

fn keep_tail(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if n <= w || w < 2 {
        s.to_string()
    } else {
        format!("…{}", s.chars().skip(n - (w - 1)).collect::<String>())
    }
}

/// CPU and memory of the dossier's process since the dossier was opened, side by side.
fn dossier_charts(f: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let [cpu, mem] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
        .spacing(2)
        .areas(area);
    let samples = app.dossier_cpu.tail(usize::MAX).count();
    for (r, hist, name, unit_max, show) in [
        (
            cpu,
            &app.dossier_cpu,
            "CPU",
            100.0,
            &(|v: f64| format!("{v:.1}%")) as &dyn Fn(f64) -> String,
        ),
        (
            mem,
            &app.dossier_mem,
            "RESIDENT",
            1.0,
            &(|v: f64| fmt_bytes(v)) as &dyn Fn(f64) -> String,
        ),
    ] {
        let [head, plot] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(r);
        // CPU scales to at least one full core; memory to its own peak, so small changes show
        let top = (hist.max() * 1.1).max(unit_max);
        let values: Vec<f64> = hist.tail(plot.width as usize).map(|v| v / top).collect();
        let head_text = if samples < 2 {
            format!("{name} ▸ {} ▸ SAMPLING…", show(hist.last()))
        } else {
            format!("{name} ▸ {} ▸ PEAK {}", show(hist.last()), show(hist.max()))
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("▌", Style::new().fg(t.bright)),
                Span::styled(
                    head_text,
                    Style::new().fg(t.dim).add_modifier(Modifier::BOLD),
                ),
            ])),
            head,
        );
        area_chart(f.buffer_mut(), plot, &values, &t, None);
    }
}

/// The end-of-shift report, typed out one line at a time.
pub fn report(f: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let Some(start) = app.report else {
        return;
    };
    let elapsed = start.elapsed();
    let items = app.shift.lines(&app.mon);
    let w = area.width.saturating_sub(4).min(72);
    // the lines, a blank before the verdict, the borders and a margin of one
    let h = (items.len() as u16 + 5).min(area.height);
    let r = popup(area, w, h);
    f.render_widget(Clear, r);
    let left = REPORT_TIME.saturating_sub(elapsed).as_secs() + 1;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(t.bright))
        .style(Style::new().bg(t.bg))
        .title(
            Line::styled(
                " ╡ END OF SHIFT REPORT ╞ ",
                Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
            )
            .centered(),
        )
        .title_bottom(
            Line::styled(
                format!(" ANY KEY TO SIGN OUT ▸ AUTOMATIC IN {left}s "),
                Style::new().fg(t.dim),
            )
            .centered(),
        );
    let inner = block.inner(r).inner(Margin::new(2, 1));
    f.render_widget(block, r);

    // one line every 150 ms; the line being typed gets a cursor
    let shown = (elapsed.as_millis() / 150) as usize;
    let mut lines = Vec::new();
    for (i, (k, v)) in items.into_iter().enumerate() {
        if i > shown {
            break;
        }
        let verdict = k == "VERDICT";
        if verdict {
            lines.push(Line::default());
        }
        let style = if verdict {
            Style::new().fg(t.bright).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(t.fg)
        };
        let mut spans = vec![
            Span::styled(
                format!("{k:<10} "),
                Style::new().fg(t.dim).add_modifier(Modifier::BOLD),
            ),
            Span::styled(v, style),
        ];
        if i == shown && (app.frame / 3).is_multiple_of(2) {
            spans.push(Span::styled("█", Style::new().fg(t.bright)));
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
