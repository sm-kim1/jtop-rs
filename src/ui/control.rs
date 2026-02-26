use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Fan control
            Constraint::Length(6), // Power mode
            Constraint::Length(5), // Jetson clocks
            Constraint::Min(0),    // Remainder
        ])
        .split(area);

    render_fan_block(frame, app, chunks[0]);
    render_power_block(frame, app, chunks[1]);
    render_clocks_block(frame, chunks[2]);
    render_help_block(frame, chunks[3]);
}

fn render_fan_block(frame: &mut Frame, app: &App, area: Rect) {
    let fans = &app.stats.fan.fans;

    let mut lines: Vec<Line> = Vec::new();

    if fans.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No fan detected",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for fan in fans {
            // Profile indicator
            let profile_spans: Vec<Span> = ["quiet", "cool", "manual"]
                .iter()
                .map(|&p| {
                    if fan.profile == p {
                        Span::styled(
                            format!(" [{}] ", p),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::styled(
                            format!(" [{}] ", p),
                            Style::default().fg(Color::DarkGray),
                        )
                    }
                })
                .collect();

            let mut profile_line = vec![Span::raw("  Profile: ")];
            profile_line.extend(profile_spans);
            lines.push(Line::from(profile_line));

            // PWM bar
            let pwm_max = if fan.pwm_max == 0 { 255 } else { fan.pwm_max };
            let pct = (fan.pwm * 100 / pwm_max).min(100) as u8;
            let bar = build_bar(pct, 12);

            lines.push(Line::from(vec![
                Span::raw("  Speed:   "),
                Span::styled(bar, Style::default().fg(Color::Cyan)),
                Span::raw(format!(" {:3}%", pct)),
                Span::styled(
                    format!("  PWM: {}/{}", fan.pwm, pwm_max),
                    Style::default().fg(Color::Gray),
                ),
                if fan.speed > 0 {
                    Span::styled(
                        format!("  RPM: {}", fan.speed),
                        Style::default().fg(Color::Yellow),
                    )
                } else {
                    Span::raw("")
                },
            ]));

            lines.push(Line::from(vec![
                Span::raw("  Name:    "),
                Span::styled(&fan.name, Style::default().fg(Color::White)),
            ]));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Fan Control ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_power_block(frame: &mut Frame, app: &App, area: Rect) {
    let rails = &app.stats.power.rails;
    let total_mw = app.stats.power.total_power;

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::raw("  Total:   "),
        Span::styled(
            format!("{:.2} W", total_mw / 1000.0),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    if rails.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No power rails detected",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for rail in rails.iter().take(3) {
            lines.push(Line::from(vec![
                Span::raw(format!("  {:<14}", &rail.name)),
                Span::styled(
                    format!("{:6.2} W", rail.power / 1000.0),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("  {:6.3} V", rail.voltage / 1000.0),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!("  {:6.1} mA", rail.current),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
        if rails.len() > 3 {
            lines.push(Line::from(Span::styled(
                format!("  ... and {} more rails", rails.len() - 3),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Power Rails ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_clocks_block(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(vec![
            Span::raw("  Status:  "),
            Span::styled(
                "N/A",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                "  (jetson_clocks not yet integrated)",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Effect:  "),
            Span::styled(
                "Maximizes CPU / GPU / EMC clocks for best performance",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::raw("  NVP:     "),
            Span::styled(
                "NVP model selection not yet integrated",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Jetson Clocks & NVP Model ",
            Style::default().fg(Color::Green),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_help_block(frame: &mut Frame, area: Rect) {
    if area.height < 2 {
        return;
    }
    let lines = vec![Line::from(vec![
        Span::styled(" Note: ", Style::default().fg(Color::Yellow)),
        Span::raw("Control actions (fan profile, NVP model, jetson_clocks) are read-only in this version."),
    ])];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Build a simple ASCII progress bar of `width` chars filled to `percent` (0-100)
fn build_bar(percent: u8, width: usize) -> String {
    let filled = (percent as usize * width) / 100;
    let empty = width - filled;
    let mut bar = String::with_capacity(width);
    for _ in 0..filled {
        bar.push('\u{2588}');
    }
    for _ in 0..empty {
        bar.push('\u{2591}');
    }
    bar
}
