use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline},
    Frame,
};

use crate::app::App;

fn usage_color(pct: f32) -> Color {
    if pct > 80.0 {
        Color::Red
    } else if pct > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

/// Format CPU frequency from kHz to human-readable string.
fn format_freq_khz(khz: u64) -> String {
    if khz == 0 {
        return "N/A".to_string();
    }
    let mhz = khz as f64 / 1000.0;
    if mhz >= 1000.0 {
        format!("{:.2} GHz", mhz / 1000.0)
    } else {
        format!("{:.0} MHz", mhz)
    }
}

pub fn render(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let cpu = &app.stats.cpu;

    // Split vertically: CPU cores block on top, history sparkline below
    let core_height = (cpu.cores.len() as u16 + 3).max(6);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(core_height),
            Constraint::Min(6),
        ])
        .split(area);

    render_cores(frame, app, chunks[0]);
    render_history(frame, app, chunks[1]);
}

fn render_cores(frame: &mut Frame, app: &App, area: Rect) {
    let cpu = &app.stats.cpu;

    let block = Block::default()
        .title(" CPU Usage ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if cpu.cores.is_empty() {
        frame.render_widget(
            Paragraph::new(" No CPU data").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    // Each core gets one row, plus one row for the total/governor line
    let num_rows = cpu.cores.len() + 1;
    let constraints: Vec<Constraint> = (0..num_rows).map(|_| Constraint::Length(1)).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, core) in cpu.cores.iter().enumerate() {
        if i >= rows.len() {
            break;
        }
        let row_area = rows[i];

        if !core.online {
            let line = Line::from(vec![
                Span::styled(
                    format!(" Core {:>2} [offline]                                        ", core.id),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            frame.render_widget(Paragraph::new(line), row_area);
            continue;
        }

        // Columns: label(14) | gauge(Min) | freq(14)
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(14),
                Constraint::Min(10),
                Constraint::Length(14),
            ])
            .split(row_area);

        // Label: " Core  0 [on]"
        let label = Paragraph::new(format!(" Core {:>2} [on] ", core.id))
            .style(Style::default().fg(Color::White));
        frame.render_widget(label, cols[0]);

        // Gauge
        let pct = core.usage.clamp(0.0, 100.0) as u16;
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(usage_color(core.usage))
                    .add_modifier(Modifier::BOLD),
            )
            .percent(pct)
            .label(format!("{:>3}%", pct));
        frame.render_widget(gauge, cols[1]);

        // Frequency (kHz)
        let freq_str = format_freq_khz(core.freq_cur);
        let freq_label = Paragraph::new(format!("  {:<11}", freq_str))
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(freq_label, cols[2]);
    }

    // Total + governor line at the bottom
    if let Some(total_row) = rows.get(cpu.cores.len()) {
        // Pick governor from first online core
        let governor = cpu
            .cores
            .iter()
            .find(|c| c.online && !c.governor.is_empty())
            .map(|c| c.governor.as_str())
            .unwrap_or("unknown");

        let total_pct = cpu.total_usage.clamp(0.0, 100.0);
        let line = Line::from(vec![
            Span::styled(" Total: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.1}%", total_pct),
                Style::default()
                    .fg(usage_color(total_pct))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Governor: ", Style::default().fg(Color::Gray)),
            Span::styled(governor.to_string(), Style::default().fg(Color::White)),
        ]);
        frame.render_widget(Paragraph::new(line), *total_row);
    }
}

fn render_history(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" CPU History ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build sparkline data from history (total CPU usage, 0-100 scaled to u64)
    let data: Vec<u64> = app
        .stats_history
        .iter()
        .map(|s| s.cpu.total_usage.clamp(0.0, 100.0) as u64)
        .collect();

    // Current value appended
    let mut all_data = data;
    all_data.push(app.stats.cpu.total_usage.clamp(0.0, 100.0) as u64);

    // Sparkline renders left-to-right with rightmost being most recent
    let sparkline = Sparkline::default()
        .data(&all_data)
        .max(100)
        .style(Style::default().fg(Color::Cyan));

    // Add percentage labels on left
    let label_area_width = 5u16;
    if inner.width > label_area_width + 2 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(label_area_width),
                Constraint::Min(1),
            ])
            .split(inner);

        // Labels at top and bottom
        let label_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(cols[0]);

        frame.render_widget(
            Paragraph::new("100% ").style(Style::default().fg(Color::DarkGray)),
            label_rows[0],
        );
        frame.render_widget(
            Paragraph::new("  0% ").style(Style::default().fg(Color::DarkGray)),
            label_rows[2],
        );

        frame.render_widget(sparkline, cols[1]);
    } else {
        frame.render_widget(sparkline, inner);
    }
}
