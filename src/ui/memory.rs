use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use crate::app::App;

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn usage_color(pct: f32) -> Color {
    if pct > 85.0 {
        Color::Red
    } else if pct > 60.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn format_freq_hz(hz: u64) -> String {
    if hz == 0 {
        return "N/A".to_string();
    }
    let mhz = hz as f64 / 1_000_000.0;
    if mhz >= 1000.0 {
        format!("{:.0} GHz", mhz / 1000.0)
    } else {
        format!("{:.0} MHz", mhz)
    }
}

pub fn render(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mem = &app.stats.memory;

    // Determine how many sections we have
    let has_zram = mem.zram.is_some();
    let has_emc = mem.emc.is_some();

    let mut constraints = vec![
        Constraint::Length(5), // RAM block
        Constraint::Length(4), // Swap block
    ];
    if has_zram {
        constraints.push(Constraint::Length(4)); // ZRAM block
    }
    if has_emc {
        constraints.push(Constraint::Length(4)); // EMC block
    }
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // RAM block
    render_ram(frame, app, chunks[chunk_idx]);
    chunk_idx += 1;

    // Swap block
    render_swap(frame, app, chunks[chunk_idx]);
    chunk_idx += 1;

    // ZRAM block (optional)
    if has_zram {
        render_zram(frame, app, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // EMC block (optional)
    if has_emc {
        render_emc(frame, app, chunks[chunk_idx]);
    }
}

fn render_ram(frame: &mut Frame, app: &App, area: Rect) {
    let ram = &app.stats.memory.ram;

    let block = Block::default()
        .title(" RAM ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Usage gauge
    let pct = if ram.total > 0 {
        (ram.used as f64 / ram.total as f64 * 100.0) as f32
    } else {
        0.0
    };
    let label = format!(
        "{} / {} ({:.1}%)",
        format_bytes(ram.used),
        format_bytes(ram.total),
        pct
    );
    let gauge = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(usage_color(pct))
                .add_modifier(Modifier::BOLD),
        )
        .percent(pct.clamp(0.0, 100.0) as u16)
        .label(label);
    frame.render_widget(gauge, rows[0]);

    // Used / Free / Cached line
    let info_line = Line::from(vec![
        Span::styled(" Used: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_bytes(ram.used),
            Style::default().fg(Color::White),
        ),
        Span::styled("  Free: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_bytes(ram.free),
            Style::default().fg(Color::White),
        ),
        Span::styled("  Cached: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_bytes(ram.cached),
            Style::default().fg(Color::White),
        ),
    ]);
    frame.render_widget(Paragraph::new(info_line), rows[1]);

    // Buffers line
    let buf_line = Line::from(vec![
        Span::styled(" Buffers: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_bytes(ram.buffers),
            Style::default().fg(Color::White),
        ),
    ]);
    frame.render_widget(Paragraph::new(buf_line), rows[2]);
}

fn render_swap(frame: &mut Frame, app: &App, area: Rect) {
    let swap = &app.stats.memory.swap;

    let block = Block::default()
        .title(" Swap ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let pct = if swap.total > 0 {
        (swap.used as f64 / swap.total as f64 * 100.0) as f32
    } else {
        0.0
    };

    let label = format!(
        "{} / {} ({:.1}%)",
        format_bytes(swap.used),
        format_bytes(swap.total),
        pct
    );
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(usage_color(pct)))
        .percent(pct.clamp(0.0, 100.0) as u16)
        .label(label);
    frame.render_widget(gauge, rows[0]);

    let info_line = Line::from(vec![
        Span::styled(" Used: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_bytes(swap.used),
            Style::default().fg(Color::White),
        ),
        Span::styled("  Free: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_bytes(swap.free),
            Style::default().fg(Color::White),
        ),
    ]);
    frame.render_widget(Paragraph::new(info_line), rows[1]);
}

fn render_zram(frame: &mut Frame, app: &App, area: Rect) {
    let zram = match &app.stats.memory.zram {
        Some(z) => z,
        None => return,
    };

    let block = Block::default()
        .title(" ZRAM ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let pct = if zram.total > 0 {
        (zram.used as f64 / zram.total as f64 * 100.0) as f32
    } else {
        0.0
    };

    let label = format!(
        "{} / {} ({:.1}%)",
        format_bytes(zram.used),
        format_bytes(zram.total),
        pct
    );
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(usage_color(pct)))
        .percent(pct.clamp(0.0, 100.0) as u16)
        .label(label);
    frame.render_widget(gauge, rows[0]);

    let info_line = Line::from(vec![
        Span::styled(" Compressed: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_bytes(zram.used),
            Style::default().fg(Color::White),
        ),
        Span::styled("  Original: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_bytes(zram.total),
            Style::default().fg(Color::White),
        ),
    ]);
    frame.render_widget(Paragraph::new(info_line), rows[1]);
}

fn render_emc(frame: &mut Frame, app: &App, area: Rect) {
    let emc = match &app.stats.memory.emc {
        Some(e) => e,
        None => return,
    };

    let block = Block::default()
        .title(" EMC (External Memory Controller) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    // Usage gauge
    let pct = emc.usage.clamp(0.0, 100.0);
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(usage_color(pct)))
        .percent(pct as u16)
        .label(format!("{:.1}%", pct));
    frame.render_widget(gauge, rows[0]);

    // Frequency info
    let freq_line = Line::from(vec![
        Span::styled(" Usage: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{:.0}%", pct),
            Style::default()
                .fg(usage_color(pct))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  Freq: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_freq_hz(emc.freq_cur),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(" / ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format_freq_hz(emc.freq_max),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(freq_line), rows[1]);
}
