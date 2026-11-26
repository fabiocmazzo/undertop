//! Screen layout. Each panel is a function that draws one instrument of the console.

mod boot;
mod effects;
mod overlays;
mod widgets;

use chrono::{Local, Timelike};
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Flex, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, BorderType, Borders, Cell, Chart, Clear, Dataset, GraphType, Paragraph, Row,
    Table, Wrap,
};

use crate::app::App;
use crate::sysmon::{fmt_bytes, fmt_duration};
use crate::theme::{Theme, lerp};
use widgets::*;

pub const MIN_W: u16 = 80;
pub const MIN_H: u16 = 24;

pub fn draw(f: &mut Frame, app: &mut App) {
    let t = app.theme();
    let area = f.area();
    f.render_widget(Block::new().style(Style::new().bg(t.bg).fg(t.fg)), area);

    if area.width < MIN_W || area.height < MIN_H {
        too_small(f, area, &t);
    } else if app.booting {
        boot::draw(f, area, app);
    } else {
        console(f, area, app);
    }

    if app.report.is_some() && area.width >= MIN_W && area.height >= MIN_H {
        overlays::report(f, area, app);
    }
    if app.effects {
        effects::apply(f, app);
    }
    if let Some(start) = app.shutdown {
        let p = start.elapsed().as_secs_f64() / crate::app::SHUTDOWN_TIME.as_secs_f64();
        effects::power_down(f, p.min(1.0), &t);
    }
}

fn too_small(f: &mut Frame, area: Rect, t: &Theme) {
    let msg = vec![
        Line::styled(
            "▲ VIEWPORT TOO NARROW ▲",
            Style::new().fg(t.warn).add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            format!("NEED {MIN_W}x{MIN_H}, HAVE {}x{}", area.width, area.height),
            Style::new().fg(t.dim),
        ),
    ];
    let [mid] = Layout::vertical([Constraint::Length(2)])
        .flex(Flex::Center)
        .areas(area);
    f.render_widget(Paragraph::new(msg).centered(), mid);
}

fn console(f: &mut Frame, area: Rect, app: &mut App) {
    let t = app.theme();
    app.hits = crate::app::Hits::default();
    let tall = area.height >= 40;
    let [header, top, mid, procs, footer] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(if tall { 14 } else { 11 }),
        Constraint::Length(if tall { 10 } else { 8 }),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .areas(area);

    draw_header(f, header, app);

    let [cpu, mem] =
        Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)]).areas(top);
    draw_cpu(f, cpu, app);
    draw_memory(f, mem, app);

    // the GPU station only joins the middle row when there is room for four panels
    if !app.mon.gpu_rows.is_empty() && area.width >= 140 {
        let [thermal, comms, vaults, gpu] = Layout::horizontal([
            Constraint::Percentage(24),
            Constraint::Percentage(30),
            Constraint::Percentage(24),
            Constraint::Percentage(22),
        ])
        .areas(mid);
        draw_thermal(f, thermal, app);
        draw_comms(f, comms, app);
        draw_vaults(f, vaults, app);
        draw_gpu(f, gpu, app);
    } else {
        let [thermal, comms, vaults] = Layout::horizontal([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .areas(mid);
        draw_thermal(f, thermal, app);
        draw_comms(f, comms, app);
        draw_vaults(f, vaults, app);
    }

    draw_procs(f, procs, app);
    draw_footer(f, footer, app);

    if app.show_help {
        draw_help(f, area, &t);
    }
    if app.dossier.is_some() {
        app.hits.dossier = overlays::dossier(f, area, app);
    }
    if app.kill.is_some() {
        draw_kill(f, area, app);
    }
    if app.lockdown.is_some() {
        overlays::lockdown(f, area, app);
    }
}

// ---------------------------------------------------------------------------------------------

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let m = &app.mon;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(t.fg))
        .title(Line::from(vec![
            Span::styled("╡ ", Style::new().fg(t.fg)),
            Span::styled(
                "SUBTERRANEAN OPERATIONS TERMINAL",
                Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ╞", Style::new().fg(t.fg)),
        ]))
        .title(
            Line::from(Span::styled(
                format!("╡ {} ╞", m.os.to_uppercase()),
                Style::new().fg(t.dim),
            ))
            .right_aligned(),
        )
        .title_bottom(
            Line::from(Span::styled(
                format!("╡ PALETTE {} ╞", t.name),
                Style::new().fg(t.dim),
            ))
            .right_aligned(),
        );
    let inner = block.inner(area);
    f.render_widget(block, area);

    // wide: big logo and clock; medium: small logo; narrow: no room for the clock
    let wide = inner.width >= 150;
    let narrow = inner.width < 100;
    let logo_scale = if wide { 2 } else { 1 };
    let logo_w = big_text_width("UNDERTOP", logo_scale);
    let now = Local::now();
    let time = now.format("%H:%M:%S").to_string();
    let clock_w = if narrow {
        0
    } else {
        big_text_width(&time, 1) + 4
    };
    let [logo, info, stripes, clk] = Layout::horizontal([
        Constraint::Length(logo_w + 2),
        Constraint::Min(30),
        Constraint::Max(18),
        Constraint::Length(clock_w),
    ])
    .areas(inner);

    big_text(
        f.buffer_mut(),
        logo.x + 1,
        logo.y,
        "UNDERTOP",
        logo_scale,
        t.bright,
        t.warn,
        logo,
    );

    let k = Style::new().fg(t.dim);
    let v = Style::new().fg(t.bright);
    let sep = Span::styled(" ░ ", Style::new().fg(t.faint));
    let lines = vec![
        Line::from(vec![
            Span::styled("NODE ", k),
            Span::styled(m.host.to_uppercase(), v.add_modifier(Modifier::BOLD)),
            sep.clone(),
            Span::styled("KERNEL ", k),
            Span::styled(m.kernel.clone(), v),
        ]),
        Line::from(vec![
            Span::styled("UPTIME ", k),
            Span::styled(fmt_duration(m.uptime), v),
            sep.clone(),
            Span::styled("LOAD ", k),
            Span::styled(
                format!("{:.2} {:.2} {:.2}", m.load.0, m.load.1, m.load.2),
                v,
            ),
            sep.clone(),
            Span::styled("TASKS ", k),
            Span::styled(m.procs.len().to_string(), v),
        ]),
        Line::from(vec![
            Span::styled("SHIFT ", k),
            Span::styled(shift_name(now.hour()), v),
            sep,
            Span::styled("STATUS ", k),
            status_span(app),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), info.inner(Margin::new(1, 0)));

    // hazard stripes, scrolling slowly
    let buf = f.buffer_mut();
    for y in stripes.y..stripes.bottom() {
        for x in stripes.x..stripes.right() {
            let on = ((x as u64 + y as u64 + app.frame / 3) % 4) < 2;
            if let Some(c) = buf.cell_mut((x, y)) {
                c.set_symbol(if on { "▚" } else { " " })
                    .set_fg(lerp(t.bg, t.warn, 0.55));
            }
        }
    }

    if !narrow {
        let blink = (app.frame / 6).is_multiple_of(2);
        let shown = if blink {
            time.clone()
        } else {
            time.replace(':', ";")
        };
        big_text(
            f.buffer_mut(),
            clk.x + 2,
            clk.y,
            &shown,
            1,
            t.fg,
            t.bright,
            clk,
        );
    }
}

fn shift_name(hour: u32) -> String {
    match hour {
        6..=13 => "FIRST (DAY)".into(),
        14..=21 => "SECOND (SWING)".into(),
        _ => "THIRD (GRAVEYARD)".into(),
    }
}

fn status_span(app: &App) -> Span<'static> {
    let t = app.theme();
    let cpu = app.mon.cpu_total.last();
    let hot = app
        .mon
        .temps
        .first()
        .map(|s| s.temp / s.crit)
        .unwrap_or(0.0);
    let (txt, c) = if app.paused {
        ("TELEMETRY FROZEN", t.warn)
    } else if cpu > 90.0 || hot > 0.9 {
        ("▲ CRITICAL LOAD ▲", t.crit)
    } else if cpu > 65.0 || hot > 0.75 {
        ("ELEVATED", t.warn)
    } else {
        ("ALL SYSTEMS NOMINAL", t.fg)
    };
    let blink = c != t.fg && (app.frame / 4).is_multiple_of(2);
    Span::styled(
        txt,
        Style::new()
            .fg(if blink { t.bg } else { c })
            .bg(if blink { c } else { t.bg })
            .add_modifier(Modifier::BOLD),
    )
}

// ---------------------------------------------------------------------------------------------

fn draw_cpu(f: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let m = &app.mon;
    let total = m.cpu_total.last();
    let block = panel(
        &t,
        area.width,
        "01",
        "REACTOR CORE",
        format!("{} ▸ {} CORES", short_brand(&m.cpu_brand), m.core_count()),
        total > 50.0 && (app.frame / 5).is_multiple_of(2),
    );
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 4 {
        return;
    }

    let [stat, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(inner);
    let k = Style::new().fg(t.dim);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("OUTPUT ", k),
            Span::styled(
                format!("{total:5.1}%"),
                Style::new()
                    .fg(t.level(total / 100.0))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  FREQ ", k),
            Span::styled(
                format!("{:.2} GHz", m.avg_freq as f64 / 1000.0),
                Style::new().fg(t.bright),
            ),
            Span::styled("  PEAK ", k),
            Span::styled(
                format!("{:.0}%", m.cpu_total.max()),
                Style::new().fg(t.bright),
            ),
            Span::styled("  RATE ", k),
            Span::styled(
                format!("{}ms", app.interval.as_millis()),
                Style::new().fg(t.bright),
            ),
        ])),
        stat,
    );

    let n = m.core_count().max(1) as u16;
    let avail = body.width / 2;
    let bw = (avail / n).clamp(1, 4);
    let gap = if bw >= 2 { 1 } else { 0 };
    let bars_w = n * (bw + gap);
    let [bars, _, chart] = Layout::horizontal([
        Constraint::Length(bars_w),
        Constraint::Length(1),
        Constraint::Min(10),
    ])
    .areas(body);

    let buf = f.buffer_mut();
    let bar_h = bars.height.saturating_sub(1);
    for (i, h) in m.cpu_cores.iter().enumerate() {
        let x = bars.x + i as u16 * (bw + gap);
        let r = Rect::new(x, bars.y, bw, bar_h);
        vbar(buf, r, h.last() / 100.0, &t);
        let label = format!("{:X}", i % 16);
        let lx = x + bw.saturating_sub(1) / 2;
        buf.set_string(lx, bars.y + bar_h, label, Style::new().fg(t.dim));
    }

    let values: Vec<f64> = m
        .cpu_total
        .tail(chart.width as usize)
        .map(|v| v / 100.0)
        .collect();
    area_chart(buf, chart, &values, &t, None);
    buf.set_string(chart.x, chart.y, "100", Style::new().fg(t.dim));
    buf.set_string(chart.x, chart.bottom() - 1, "0", Style::new().fg(t.dim));
}

fn short_brand(b: &str) -> String {
    b.replace("(R)", "")
        .replace("(TM)", "")
        .replace(" CPU", "")
        .replace("  ", " ")
        .to_uppercase()
}

// ---------------------------------------------------------------------------------------------

fn draw_memory(f: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let m = &app.mon;
    let frac = if m.mem_total > 0 {
        m.mem_used as f64 / m.mem_total as f64
    } else {
        0.0
    };
    let swap = if m.swap_total > 0 {
        m.swap_used as f64 / m.swap_total as f64
    } else {
        0.0
    };
    let block = panel(
        &t,
        area.width,
        "02",
        "RESERVOIR",
        format!("{} TOTAL", fmt_bytes(m.mem_total as f64)),
        frac > 0.85,
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let [mem_tank, swap_tank, _, stats] = Layout::horizontal([
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Length(1),
        Constraint::Min(10),
    ])
    .areas(inner);
    tank(
        f.buffer_mut(),
        mem_tank,
        frac,
        &format!("MEM {:.0}%", frac * 100.0),
        &t,
        app.frame,
    );
    tank(
        f.buffer_mut(),
        swap_tank,
        swap,
        &format!("SWP {:.0}%", swap * 100.0),
        &t,
        app.frame + 17,
    );

    let k = Style::new().fg(t.dim);
    let v = Style::new().fg(t.bright);
    let row = |a: &str, b: String| {
        Line::from(vec![Span::styled(format!("{a:<7}"), k), Span::styled(b, v)])
    };
    let mut lines = vec![
        row("USED", fmt_bytes(m.mem_used as f64)),
        row(
            "FREE",
            fmt_bytes(m.mem_total.saturating_sub(m.mem_used) as f64),
        ),
        row(
            "SWAP",
            format!(
                "{} / {}",
                fmt_bytes(m.swap_used as f64),
                fmt_bytes(m.swap_total as f64)
            ),
        ),
        Line::default(),
    ];
    if let Some(b) = &m.battery {
        lines.push(Line::styled("AUX POWER CELL", k));
        lines.push(Line::styled(format!("{}% ▸ {}", b.capacity, b.status), v));
    }
    let stats_h = lines.len() as u16;
    f.render_widget(Paragraph::new(lines), stats);

    if let Some(b) = &m.battery {
        let y = stats.y + stats_h;
        if y < stats.bottom() {
            seg_bar(
                f.buffer_mut(),
                Rect::new(stats.x, y, stats.width.min(20), 1),
                b.capacity as f64 / 100.0,
                &t,
            );
        }
    }

    // memory history in what is left at the bottom
    let used = stats_h + 2;
    if stats.height > used + 1 {
        let r = Rect::new(stats.x, stats.y + used, stats.width, stats.height - used);
        let values: Vec<f64> = m.mem.tail(r.width as usize).map(|v| v / 100.0).collect();
        area_chart(f.buffer_mut(), r, &values, &t, Some(t.dim));
    }
}

// ---------------------------------------------------------------------------------------------

fn draw_thermal(f: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let m = &app.mon;
    let hottest = m.temps.iter().map(|s| s.temp).fold(0.0, f32::max);
    let block = panel(
        &t,
        area.width,
        "03",
        "THERMAL",
        format!("MAX {hottest:.0}°C"),
        hottest > 80.0,
    );
    let inner = block.inner(area);
    f.render_widget(block, area);
    if m.temps.is_empty() {
        f.render_widget(
            Paragraph::new(Line::styled(
                "NO SENSORS RESPONDING",
                Style::new().fg(t.dim),
            )),
            inner,
        );
        return;
    }
    let label_w = 12u16.min(inner.width / 3);
    let value_w = 6u16;
    for (i, s) in m.temps.iter().take(inner.height as usize).enumerate() {
        let y = inner.y + i as u16;
        let buf = f.buffer_mut();
        buf.set_string(
            inner.x,
            y,
            fit(&s.label.to_uppercase(), label_w as usize),
            Style::new().fg(t.dim),
        );
        let bar_w = inner.width.saturating_sub(label_w + value_w + 2);
        let frac = (s.temp / s.crit) as f64;
        seg_bar(buf, Rect::new(inner.x + label_w + 1, y, bar_w, 1), frac, &t);
        buf.set_string(
            inner.right() - value_w,
            y,
            format!("{:>4.0}°C", s.temp),
            Style::new().fg(t.level(frac)).add_modifier(Modifier::BOLD),
        );
    }
}

// ---------------------------------------------------------------------------------------------

fn draw_comms(f: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let m = &app.mon;
    let block = panel(
        &t,
        area.width,
        "04",
        "COMMS",
        format!(
            "Σ▼ {}  Σ▲ {}",
            fmt_bytes(m.rx_total as f64),
            fmt_bytes(m.tx_total as f64)
        ),
        m.rx.last() + m.tx.last() > 1024.0 * 1024.0,
    );
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 3 {
        return;
    }
    let [stat, scope] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(inner);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("▼ RX ", Style::new().fg(t.fg)),
            Span::styled(
                format!("{:<12}", format!("{}/s", fmt_bytes(m.rx.last()))),
                Style::new().fg(t.bright),
            ),
            Span::styled("▲ TX ", Style::new().fg(t.warn)),
            Span::styled(
                format!("{}/s", fmt_bytes(m.tx.last())),
                Style::new().fg(t.bright),
            ),
        ])),
        stat,
    );

    // Braille gives the scope 2x4 dots per cell.
    let n = (scope.width as usize * 2).max(2);
    let rx: Vec<(f64, f64)> =
        m.rx.tail(n)
            .enumerate()
            .map(|(i, v)| (i as f64, v))
            .collect();
    let tx: Vec<(f64, f64)> =
        m.tx.tail(n)
            .enumerate()
            .map(|(i, v)| (i as f64, v))
            .collect();
    let top = m.rx.max().max(m.tx.max()).max(1024.0) * 1.15;
    let x_max = (rx.len().max(2) - 1) as f64;
    let chart = Chart::new(vec![
        Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::new().fg(t.fg))
            .data(&rx),
        Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::new().fg(t.warn))
            .data(&tx),
    ])
    .style(Style::new().bg(t.bg))
    .x_axis(Axis::default().bounds([0.0, x_max]))
    .y_axis(
        Axis::default()
            .bounds([0.0, top])
            .style(Style::new().fg(t.faint))
            .labels(vec![
                Span::styled("0", Style::new().fg(t.dim)),
                Span::styled(fmt_bytes(top), Style::new().fg(t.dim)),
            ]),
    );
    // dotted grid first, chart on top
    area_chart(f.buffer_mut(), scope, &[], &t, None);
    f.render_widget(chart, scope);
}

// ---------------------------------------------------------------------------------------------

fn draw_vaults(f: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let m = &app.mon;
    let block = panel(
        &t,
        area.width,
        "05",
        "VAULTS",
        format!(
            "▼R {}/s ▲W {}/s",
            fmt_bytes(m.disk_r.last()),
            fmt_bytes(m.disk_w.last())
        ),
        m.disk_r.last() + m.disk_w.last() > 50.0 * 1024.0 * 1024.0,
    );
    let inner = block.inner(area);
    f.render_widget(block, area);
    let mut y = inner.y;
    for d in &m.disks_rows {
        if y + 1 >= inner.bottom() {
            break;
        }
        let frac = if d.total > 0 {
            d.used as f64 / d.total as f64
        } else {
            0.0
        };
        let buf = f.buffer_mut();
        let size = format!(
            "{} / {}",
            fmt_bytes(d.used as f64),
            fmt_bytes(d.total as f64)
        );
        let mount_w = inner.width.saturating_sub(size.len() as u16 + 1) as usize;
        buf.set_string(
            inner.x,
            y,
            fit(&d.mount, mount_w),
            Style::new().fg(t.bright),
        );
        buf.set_string(
            inner.right() - size.len() as u16,
            y,
            &size,
            Style::new().fg(t.dim),
        );
        let pct = format!("{:>3.0}%", frac * 100.0);
        seg_bar(
            buf,
            Rect::new(inner.x, y + 1, inner.width.saturating_sub(5), 1),
            frac,
            &t,
        );
        buf.set_string(
            inner.right() - 4,
            y + 1,
            pct,
            Style::new().fg(t.level(frac)),
        );
        y += 2;
    }
    // I/O history in the rows left over: reads and writes added, scaled to the peak
    if inner.bottom() > y + 1 {
        let r = Rect::new(inner.x, y, inner.width, inner.bottom() - y);
        let total: Vec<f64> = m
            .disk_r
            .tail(r.width as usize)
            .zip(m.disk_w.tail(r.width as usize))
            .map(|(a, b)| a + b)
            .collect();
        let peak = total.iter().copied().fold(1024.0 * 1024.0, f64::max);
        let values: Vec<f64> = total.iter().map(|v| v / peak).collect();
        area_chart(f.buffer_mut(), r, &values, &t, Some(t.dim));
        let io = format!(
            "I/O ▼R {}/s ▲W {}/s",
            fmt_bytes(m.disk_r.last()),
            fmt_bytes(m.disk_w.last())
        );
        f.buffer_mut().set_string(
            r.x,
            r.y,
            fit(&io, r.width as usize),
            Style::new().fg(t.bright),
        );
    }
}

// ---------------------------------------------------------------------------------------------

fn draw_gpu(f: &mut Frame, area: Rect, app: &App) {
    use crate::gpu::GpuState;
    let t = app.theme();
    let gpus = &app.mon.gpu_rows;
    let busy = gpus
        .iter()
        .any(|g| g.load.unwrap_or(0.0) > 0.5 && g.state == GpuState::Active);
    let block = panel(
        &t,
        area.width,
        "07",
        "AUX REACTOR",
        format!("{} UNITS", gpus.len()),
        busy,
    );
    let inner = block.inner(area);
    f.render_widget(block, area);
    let mut y = inner.y;
    for g in gpus {
        if y + 2 > inner.bottom() {
            break;
        }
        let buf = f.buffer_mut();
        let (tag, tag_style) = match g.state {
            GpuState::Active => ("ONLINE", Style::new().fg(t.fg)),
            GpuState::Dormant => ("◌ DORMANT", Style::new().fg(t.faint)),
            GpuState::Unknown => ("PROBING", Style::new().fg(t.dim)),
        };
        let name_w = inner.width.saturating_sub(tag.chars().count() as u16 + 1) as usize;
        buf.set_string(
            inner.x,
            y,
            fit(&g.name, name_w),
            Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
        );
        buf.set_string(
            inner.right() - tag.chars().count() as u16,
            y,
            tag,
            tag_style,
        );
        let bar = Rect::new(inner.x, y + 1, inner.width.saturating_sub(5), 1);
        if g.state == GpuState::Dormant {
            // a slow breathing glow while it sleeps
            let breath = ((app.frame as f64 * 0.08).sin() + 1.0) / 2.0;
            let line: String = "▯".repeat(bar.width as usize);
            buf.set_string(
                bar.x,
                bar.y,
                line,
                Style::new().fg(lerp(t.bg, t.faint, 0.4 + 0.6 * breath)),
            );
            if y + 2 < inner.bottom() {
                buf.set_string(
                    inner.x,
                    y + 2,
                    fit("ASLEEP ▸ NOT TO BE DISTURBED", inner.width as usize),
                    Style::new().fg(t.faint),
                );
            }
        } else {
            let load = g.load.unwrap_or(0.0);
            seg_bar(buf, bar, load, &t);
            let pct = g
                .load
                .map(|l| format!("{:>3.0}%", l * 100.0))
                .unwrap_or_else(|| "  ?%".into());
            buf.set_string(
                inner.right() - 4,
                y + 1,
                pct,
                Style::new().fg(t.level(load)),
            );
            let mut parts = Vec::new();
            match g.freq_mhz {
                // 0 MHz: the iGPU is parked in its RC6 power-saving state
                Some(0) => parts.push("RC6 IDLE".into()),
                Some(mhz) => parts.push(format!("{:.2} GHz", mhz as f64 / 1000.0)),
                None => {}
            }
            if let Some(c) = g.temp {
                parts.push(format!("{c:.0}°C"));
            }
            if let Some((u, tot)) = g.vram {
                parts.push(format!("VRAM {u}/{tot} MiB"));
            }
            if let Some(age) = g.sampled.map(|s| s.elapsed().as_secs()).filter(|a| *a >= 3) {
                parts.push(format!("{age}s AGO"));
            }
            if y + 2 < inner.bottom() {
                buf.set_string(
                    inner.x,
                    y + 2,
                    fit(&parts.join(" ▸ "), inner.width as usize),
                    Style::new().fg(t.dim),
                );
            }
        }
        y += 4;
    }
}

// ---------------------------------------------------------------------------------------------

fn draw_procs(f: &mut Frame, area: Rect, app: &mut App) {
    let t = app.theme();
    let arrow = if app.sort_desc { "▼" } else { "▲" };
    let filter = if app.filter.is_empty() && !app.filter_mode {
        String::new()
    } else {
        let cursor = if app.filter_mode && (app.frame / 4).is_multiple_of(2) {
            "█"
        } else {
            " "
        };
        format!("FILTER ▸ {}{} ░ ", app.filter, cursor)
    };
    let block = panel(
        &t,
        area.width,
        "06",
        if app.tree { "LINEAGE" } else { "MANIFEST" },
        format!(
            "{filter}SORT ▸ {}{arrow} ░ {} ENTRIES",
            app.sort.label(),
            app.visible.len()
        ),
        app.filter_mode,
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mem_total = app.mon.mem_total.max(1) as f64;
    let head = |s: &'static str, key| {
        let st = if app.sort == key {
            Style::new().fg(t.bg).bg(t.fg).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(t.dim).add_modifier(Modifier::BOLD)
        };
        Cell::from(Span::styled(s, st))
    };
    use crate::app::SortKey::*;
    let header = Row::new(vec![
        head("PID", Pid),
        Cell::from(Span::styled(
            "OPERATOR",
            Style::new().fg(t.dim).add_modifier(Modifier::BOLD),
        )),
        head("CPU%", Cpu),
        Cell::from(Span::styled(
            "LOAD",
            Style::new().fg(t.dim).add_modifier(Modifier::BOLD),
        )),
        head("MEM", Mem),
        Cell::from(Span::styled(
            "ST",
            Style::new().fg(t.dim).add_modifier(Modifier::BOLD),
        )),
        head("TIME", Time),
        head("COMMAND", Name),
    ]);

    let rows = app.visible.iter().map(|p| {
        // one fully busy core fills the bar
        let load = (p.cpu / 100.0).clamp(0.0, 1.0) as f64;
        let segs = 6;
        let lit = (load * segs as f64).ceil() as usize;
        let bar = Line::from(vec![
            Span::styled("▮".repeat(lit), Style::new().fg(t.level(load))),
            Span::styled("▯".repeat(segs - lit), Style::new().fg(t.faint)),
        ]);
        let mem_frac = p.mem as f64 / mem_total;
        let st_color = match p.status {
            "RUN" => t.bright,
            "ZMB" | "DED" => t.crit,
            "STP" | "DSK" => t.warn,
            _ => t.dim,
        };
        Row::new(vec![
            Cell::from(Span::styled(
                format!("{:>7}", p.pid),
                Style::new().fg(t.dim),
            )),
            Cell::from(Span::styled(p.user.clone(), Style::new().fg(t.fg))),
            Cell::from(Span::styled(
                format!("{:>5.1}", p.cpu),
                Style::new()
                    .fg(t.level(load))
                    .add_modifier(if p.cpu > 50.0 {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            )),
            Cell::from(bar),
            Cell::from(Span::styled(
                format!("{:>9}", fmt_bytes(p.mem as f64)),
                Style::new().fg(t.level((mem_frac * 8.0).min(1.0))),
            )),
            Cell::from(Span::styled(p.status, Style::new().fg(st_color))),
            Cell::from(Span::styled(
                fmt_duration(p.run_time),
                Style::new().fg(t.dim),
            )),
            Cell::from(Line::from(vec![
                Span::styled(p.tree.clone(), Style::new().fg(t.dim)),
                Span::styled(p.cmd.clone(), Style::new().fg(t.fg)),
            ])),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(5),
            Constraint::Length(6),
            Constraint::Length(9),
            Constraint::Length(3),
            Constraint::Length(11),
            Constraint::Min(10),
        ],
    )
    .header(header.bottom_margin(0))
    .column_spacing(1)
    .row_highlight_style(Style::new().fg(t.bg).bg(t.fg).add_modifier(Modifier::BOLD))
    .highlight_symbol("▶ ");
    f.render_stateful_widget(table, inner, &mut app.table);

    // where a click lands: the header line, then one line per row
    app.hits.header_y = inner.y;
    app.hits.rows = Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    };
    let mut x = inner.x + 2; // the "▶ " highlight column
    for (w, key) in [
        (7, Some(Pid)),
        (10, None),
        (5, Some(Cpu)),
        (6, None),
        (9, Some(Mem)),
        (3, None),
        (11, Some(Time)),
        (u16::MAX, Some(Name)),
    ] {
        let end = x.saturating_add(w).min(inner.right());
        if let Some(key) = key {
            app.hits.columns.push((x, end, key));
        }
        x = end + 1;
    }
}

// ---------------------------------------------------------------------------------------------

const TICKER: &[&str] = &[
    "REMAIN CALM. REMAIN BELOW.",
    "REPORT ANY UNSCHEDULED PROCESS TO YOUR SHIFT SUPERVISOR",
    "THE MAINFRAME SEES ALL THREADS",
    "HYDROPONICS ▸ YIELD NOMINAL",
    "DO NOT TAMPER WITH THE CONTROL GROUP",
    "SCHEDULED MAINTENANCE OF THE PAGE CACHE IS NOT YOUR CONCERN",
    "A CLEAN SWAP IS A HAPPY SWAP",
    "LEVEL 142 ▸ PRESSURE SEALS HOLDING",
    "IN CASE OF CONTAINMENT BREACH, DO NOT PRESS !",
];

fn draw_footer(f: &mut Frame, area: Rect, app: &mut App) {
    let t = app.theme();
    let keys = [
        ("Q", "QUIT", KeyCode::Char('q')),
        ("?", "HELP", KeyCode::Char('?')),
        ("↵", "DOSSIER", KeyCode::Enter),
        ("V", "TREE", KeyCode::Char('v')),
        ("T", "PALETTE", KeyCode::Char('t')),
        ("S", "SORT", KeyCode::Char('s')),
        ("/", "FILTER", KeyCode::Char('/')),
        ("K", "TERMINATE", KeyCode::Char('K')),
        ("X", "CRT", KeyCode::Char('x')),
    ];
    app.hits.footer_y = area.y;
    let mut spans = Vec::new();
    let mut used = 0usize;
    for (k, label, code) in keys {
        // keep at least 30 columns for the ticker
        let w = k.chars().count() + label.chars().count() + 4;
        if used + w + 32 > area.width as usize && used > 0 {
            break;
        }
        let x = area.x + used as u16;
        app.hits.keys.push((x, x + w as u16, code));
        used += w;
        spans.push(Span::styled(
            format!(" {k} "),
            Style::new().fg(t.bg).bg(t.dim).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {label} "), Style::new().fg(t.dim)));
    }
    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let rest = (area.width as usize).saturating_sub(used + 2);

    let msg = if let Some((s, _)) = &app.status {
        Span::styled(
            fit(&format!("▶ {s}"), rest),
            Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
        )
    } else if let Some((a, _)) = &app.alert {
        let lamp = if (app.frame / 4).is_multiple_of(2) {
            "▲"
        } else {
            " "
        };
        Span::styled(
            fit(&format!("{lamp} ALERT ▸ {a}"), rest),
            Style::new().fg(t.warn).add_modifier(Modifier::BOLD),
        )
    } else {
        let tape: String = TICKER.iter().map(|s| format!("{s}   ◆   ")).collect();
        let chars: Vec<char> = tape.chars().collect();
        let off = (app.frame / 2) as usize % chars.len();
        let shown: String = (0..rest).map(|i| chars[(off + i) % chars.len()]).collect();
        Span::styled(shown, Style::new().fg(t.faint))
    };
    spans.push(Span::raw("  "));
    spans.push(msg);
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

pub(crate) fn popup(area: Rect, w: u16, h: u16) -> Rect {
    let [v] = Layout::vertical([Constraint::Length(h)])
        .flex(Flex::Center)
        .areas(area);
    let [r] = Layout::horizontal([Constraint::Length(w)])
        .flex(Flex::Center)
        .areas(v);
    r
}

fn draw_help(f: &mut Frame, area: Rect, t: &Theme) {
    let r = popup(area, 62, 22);
    f.render_widget(Clear, r);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(t.bright))
        .style(Style::new().bg(t.bg))
        .title(
            Line::styled(
                " ╡ OPERATOR MANUAL ╞ ",
                Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
            )
            .centered(),
        );
    let k = Style::new().fg(t.bg).bg(t.fg).add_modifier(Modifier::BOLD);
    let d = Style::new().fg(t.fg);
    let entry = |key: &str, desc: &str| {
        Line::from(vec![
            Span::styled(format!(" {key:^9} "), k),
            Span::styled(format!("  {desc}"), d),
        ])
    };
    let lines = vec![
        entry("↑ ↓ j k", "move through the manifest"),
        entry("PgUp PgDn", "move one page"),
        entry("g G", "first / last entry"),
        entry("c m p n", "sort by cpu / memory / pid / name"),
        entry("s  Tab", "next sort key"),
        entry("r", "reverse sort order"),
        entry("/  f", "filter (Enter keeps, Esc clears)"),
        entry("Enter", "open the dossier of the selected process"),
        entry("v  F5", "tree view (lineage) on / off"),
        entry("K  Del", "terminate the selected process"),
        entry("t", "next phosphor palette"),
        entry("x", "CRT emulation on / off"),
        entry("+ -", "slower / faster sampling"),
        entry("Space", "freeze telemetry"),
        entry("q  Esc", "leave the terminal (with a shift report)"),
        entry("mouse", "click selects, double-click opens, wheel"),
        Line::default(),
        Line::styled("PRESS ANY KEY TO RETURN TO DUTY", Style::new().fg(t.dim)).centered(),
    ];
    let inner = block.inner(r);
    f.render_widget(block, r);
    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        inner.inner(Margin::new(1, 1)),
    );
}

fn draw_kill(f: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let Some(k) = &app.kill else { return };
    let r = popup(area, 56, 9);
    f.render_widget(Clear, r);
    let flash = (app.frame / 3).is_multiple_of(2);
    let edge = if flash { t.crit } else { t.warn };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::new().fg(edge))
        .style(Style::new().bg(t.bg))
        .title(
            Line::styled(
                " ▲ AUTHORIZATION REQUIRED ▲ ",
                Style::new().fg(t.bg).bg(edge).add_modifier(Modifier::BOLD),
            )
            .centered(),
        );
    let inner = block.inner(r);
    f.render_widget(block, r);
    let stripes: String = (0..inner.width)
        .map(|i| {
            if (i as u64 + app.frame / 2) % 4 < 2 {
                '▚'
            } else {
                ' '
            }
        })
        .collect();
    let lines = vec![
        Line::styled(stripes.clone(), Style::new().fg(t.warn)),
        Line::default(),
        Line::from(vec![
            Span::styled("TERMINATE PROCESS ", Style::new().fg(t.fg)),
            Span::styled(
                format!("{} ", k.pid),
                Style::new().fg(t.bright).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("({})", k.name), Style::new().fg(t.bright)),
            Span::styled(" ?", Style::new().fg(t.fg)),
        ])
        .centered(),
        Line::default(),
        Line::from(vec![
            Span::styled(
                " Y ",
                Style::new().fg(t.bg).bg(t.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" SIGTERM   ", Style::new().fg(t.fg)),
            Span::styled(
                " 9 ",
                Style::new()
                    .fg(t.bg)
                    .bg(t.crit)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" SIGKILL   ", Style::new().fg(t.crit)),
            Span::styled(
                " ANY ",
                Style::new().fg(t.bg).bg(t.dim).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ABORT", Style::new().fg(t.dim)),
        ])
        .centered(),
        Line::default(),
        Line::styled(stripes, Style::new().fg(t.warn)),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use ratatui::{Terminal, backend::TestBackend};

    use crate::app::{App, KillPrompt};
    use crate::config::Settings;

    const SCREENS: [&str; 9] = [
        "console",
        "boot",
        "help",
        "kill",
        "dossier",
        "lockdown",
        "tree",
        "power-down",
        "report",
    ];

    fn test_app() -> App {
        App::new(Settings {
            boot: false,
            ..Settings::default()
        })
        .unwrap()
    }

    /// Puts the app on screen `n` of `SCREENS`.
    fn set_screen(app: &mut App, n: usize) {
        app.booting = n == 1;
        app.show_help = n == 2;
        app.kill = (n == 3).then(|| KillPrompt {
            pid: 1,
            name: "init".into(),
        });
        let me = std::process::id();
        app.dossier = (n == 4).then(|| (me, app.mon.dossier(me)));
        app.lockdown = None;
        if n == 5 {
            app.start_lockdown("TEST ▸ DRILL".into());
        }
        app.tree = n == 6;
        app.rebuild();
        app.shutdown = (n == 7).then(|| Instant::now() - Duration::from_millis(300));
        app.report = (n == 8).then(|| Instant::now() - Duration::from_secs(3));
    }

    fn dump(term: &Terminal<TestBackend>, w: u16, h: u16) -> String {
        let buf = term.backend().buffer();
        let mut out = String::new();
        for y in 0..h {
            for x in 0..w {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    /// Renders every screen at a few sizes; fails on panics (out-of-range drawing, etc.).
    /// `UNDERTOP_DUMP=<screen> cargo test -- --nocapture` prints a screen, by name or number
    /// (console, boot, help, kill, dossier, lockdown, tree, power-down, report).
    #[test]
    fn renders_at_many_sizes() {
        let mut app = test_app();
        app.effects = false;
        let wanted = std::env::var("UNDERTOP_DUMP").ok();
        for (w, h) in [(80, 24), (120, 36), (160, 45), (240, 60), (60, 20)] {
            for (n, name) in SCREENS.iter().enumerate() {
                set_screen(&mut app, n);
                let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
                term.draw(|f| super::draw(f, &mut app)).unwrap();
                if wanted
                    .as_deref()
                    .is_some_and(|v| v == *name || v == n.to_string())
                {
                    println!("--- {name} {w}x{h}\n{}", dump(&term, w, h));
                }
            }
        }
    }

    /// The power-down animation and the CRT pass at every stage of their timing.
    #[test]
    fn animations_never_draw_out_of_bounds() {
        let mut app = test_app();
        let mut term = Terminal::new(TestBackend::new(120, 36)).unwrap();
        for _ in 0..50 {
            term.draw(|f| super::draw(f, &mut app)).unwrap();
        }
        for ms in (0..=950).step_by(50) {
            app.shutdown = Some(Instant::now() - Duration::from_millis(ms));
            term.draw(|f| super::draw(f, &mut app)).unwrap();
        }
    }
}
