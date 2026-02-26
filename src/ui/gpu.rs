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

/// Format frequency from Hz to human-readable string.
fn format_freq_hz(hz: u64) -> String {
    if hz == 0 {
        return "N/A".to_string();
    }
    let mhz = hz as f64 / 1_000_000.0;
    if mhz >= 1000.0 {
        format!("{:.2} GHz", mhz / 1000.0)
    } else {
        format!("{:.0} MHz", mhz)
    }
}

pub fn render(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(6)])
        .split(area);

    render_gpu_info(frame, app, chunks[0]);
    render_gpu_history(frame, app, chunks[1]);
}

fn render_gpu_info(frame: &mut Frame, app: &App, area: Rect) {
    let gpu = &app.stats.gpu;

    let block = Block::default()
        .title(" GPU Usage ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    // Load gauge row
    {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(12), Constraint::Min(10)])
            .split(rows[0]);

        frame.render_widget(
            Paragraph::new(" Load:      ").style(Style::default().fg(Color::White)),
            cols[0],
        );

        let pct = gpu.usage.clamp(0.0, 100.0) as u16;
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(usage_color(gpu.usage))
                    .add_modifier(Modifier::BOLD),
            )
            .percent(pct)
            .label(format!("{pct}%"));
        frame.render_widget(gauge, cols[1]);
    }

    // Frequency row
    {
        let freq_line = Line::from(vec![
            Span::styled(" Frequency: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format_freq_hz(gpu.freq_cur),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "  (min: {}, max: {})",
                    format_freq_hz(gpu.freq_min),
                    format_freq_hz(gpu.freq_max)
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(freq_line), rows[1]);
    }

    // Governor row
    {
        let gov_line = Line::from(vec![
            Span::styled(" Governor:  ", Style::default().fg(Color::Gray)),
            Span::styled(
                if gpu.governor.is_empty() {
                    "N/A".to_string()
                } else {
                    gpu.governor.clone()
                },
                Style::default().fg(Color::White),
            ),
        ]);
        frame.render_widget(Paragraph::new(gov_line), rows[2]);
    }

    // Frequency bar (cur / max visual)
    if gpu.freq_max > 0 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(12), Constraint::Min(10)])
            .split(rows[3]);

        frame.render_widget(
            Paragraph::new(" Freq bar:  ").style(Style::default().fg(Color::Gray)),
            cols[0],
        );

        let freq_pct = ((gpu.freq_cur as f64 / gpu.freq_max as f64) * 100.0).clamp(0.0, 100.0)
            as u16;
        let freq_gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent(freq_pct)
            .label(format!(
                "{} / {}",
                format_freq_hz(gpu.freq_cur),
                format_freq_hz(gpu.freq_max)
            ));
        frame.render_widget(freq_gauge, cols[1]);
    }
}

fn render_gpu_history(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" GPU History ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build sparkline data from history
    let mut data: Vec<u64> = app
        .stats_history
        .iter()
        .map(|s| s.gpu.usage.clamp(0.0, 100.0) as u64)
        .collect();
    data.push(app.stats.gpu.usage.clamp(0.0, 100.0) as u64);

    let label_width = 5u16;
    if inner.width > label_width + 2 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(label_width), Constraint::Min(1)])
            .split(inner);

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

        let sparkline = Sparkline::default()
            .data(&data)
            .max(100)
            .style(Style::default().fg(Color::Green));
        frame.render_widget(sparkline, cols[1]);
    } else {
        let sparkline = Sparkline::default()
            .data(&data)
            .max(100)
            .style(Style::default().fg(Color::Green));
        frame.render_widget(sparkline, inner);
    }
}
