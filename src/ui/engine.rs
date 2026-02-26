use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;

/// Known Jetson hardware engines with display names
const ENGINES: &[(&str, &str)] = &[
    ("NVDEC", "Video Decoder"),
    ("NVENC", "Video Encoder"),
    ("NVJPG", "JPEG Encoder/Decoder"),
    ("OFA", "Optical Flow Accelerator"),
    ("VIC", "Video Image Compositor"),
    ("APE", "Audio Processing Engine"),
    ("SE", "Security Engine"),
];

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(ENGINES.len() as u16 + 2), // engines block
            Constraint::Min(0),                            // note block
        ])
        .split(area);

    render_engines_block(frame, app, chunks[0]);
    render_note_block(frame, chunks[1]);
}

fn render_engines_block(frame: &mut Frame, app: &App, area: Rect) {
    // Check temperature zones for any engine-related names as proxy for activity
    let temp_zones = &app.stats.temperature.zones;

    let items: Vec<ListItem> = ENGINES
        .iter()
        .map(|(engine_key, engine_desc)| {
            // Look for a temperature zone whose name contains the engine key (case-insensitive)
            let temp_str = temp_zones
                .iter()
                .find(|z| {
                    z.name
                        .to_uppercase()
                        .contains(&engine_key.to_uppercase())
                })
                .map(|z| format!("  Temp: {:.1}°C", z.temp))
                .unwrap_or_default();

            // Build a simple progress bar placeholder (no live utilization data yet)
            let bar = build_bar(0, 12);

            let line = Line::from(vec![
                Span::styled(
                    format!("{:<8}", engine_key),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(": "),
                Span::styled(bar, Style::default().fg(Color::DarkGray)),
                Span::raw("  "),
                Span::styled(
                    format!("{:<30}", engine_desc),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(temp_str, Style::default().fg(Color::Yellow)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Hardware Engines ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_note_block(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(vec![
            Span::styled("Note: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("Live engine utilization data requires Jetson-specific tegrastats integration."),
        ]),
        Line::from(vec![
            Span::raw("      Engine frequency data is available via "),
            Span::styled(
                "/sys/kernel/debug/bpmp/debug/clk/",
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" on Jetson devices."),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Engines: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("NVDEC · NVENC · NVJPG · OFA · VIC · APE · SE"),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Engine Status ",
            Style::default().fg(Color::Green),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Build a simple ASCII progress bar of `width` characters filled to `percent` (0-100)
fn build_bar(percent: u8, width: usize) -> String {
    let filled = (percent as usize * width) / 100;
    let empty = width - filled;
    let mut bar = String::with_capacity(width);
    for _ in 0..filled {
        bar.push('\u{2588}'); // full block
    }
    for _ in 0..empty {
        bar.push('\u{2591}'); // light shade
    }
    bar
}
