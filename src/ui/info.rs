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
            Constraint::Length(12), // Board info
            Constraint::Length(8),  // Libraries
            Constraint::Min(0),     // Remainder
        ])
        .split(area);

    render_board_block(frame, app, chunks[0]);
    render_libraries_block(frame, chunks[1]);
}

fn render_board_block(frame: &mut Frame, app: &App, area: Rect) {
    let board = &app.stats.board;

    let model_display = if board.model.is_empty() {
        "Unknown".to_string()
    } else {
        board.model.clone()
    };

    let serial_display = if board.serial.is_empty() {
        "N/A".to_string()
    } else {
        board.serial.clone()
    };

    let hostname_display = if board.hostname.is_empty() {
        "N/A".to_string()
    } else {
        board.hostname.clone()
    };

    let l4t_display = if board.l4t_version.is_empty() {
        "N/A".to_string()
    } else {
        board.l4t_version.clone()
    };

    let jetpack_display = if board.jetpack_version.is_empty() {
        "N/A".to_string()
    } else {
        board.jetpack_version.clone()
    };

    let cuda_display = if board.cuda_version.is_empty() {
        "N/A".to_string()
    } else {
        board.cuda_version.clone()
    };

    let kernel_display = if board.kernel_version.is_empty() {
        "N/A".to_string()
    } else {
        board.kernel_version.clone()
    };

    let uptime_display = format_uptime(board.uptime);

    let label_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(Color::White);
    let dim_style = Style::default().fg(Color::DarkGray);

    let lines = vec![
        info_line("  Model    ", &model_display, label_style, value_style),
        info_line("  Serial   ", &serial_display, label_style, dim_style),
        info_line("  Hostname ", &hostname_display, label_style, value_style),
        Line::from(""),
        info_line("  L4T      ", &l4t_display, label_style, value_style),
        info_line("  JetPack  ", &jetpack_display, label_style, value_style),
        info_line("  CUDA     ", &cuda_display, label_style, value_style),
        info_line("  Kernel   ", &kernel_display, label_style, dim_style),
        Line::from(""),
        info_line("  Uptime   ", &uptime_display, label_style, Style::default().fg(Color::Yellow)),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Board Information ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_libraries_block(frame: &mut Frame, area: Rect) {
    let label_style = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(Color::DarkGray);
    let note_style = Style::default().fg(Color::DarkGray);

    let lines = vec![
        info_line("  cuDNN    ", "N/A", label_style, value_style),
        info_line("  TensorRT ", "N/A", label_style, value_style),
        info_line("  OpenCV   ", "N/A", label_style, value_style),
        info_line("  VPI      ", "N/A", label_style, value_style),
        Line::from(Span::styled(
            "  (Library version detection not yet integrated)",
            note_style,
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Libraries ",
            Style::default().fg(Color::Green),
        ));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Format uptime seconds as "Xd Yh Zm Ws"
fn format_uptime(seconds: u64) -> String {
    if seconds == 0 {
        return "N/A".to_string();
    }
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Build a labeled info line with consistent column alignment
fn info_line<'a>(label: &'a str, value: &'a str, label_style: Style, value_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(label, label_style),
        Span::raw(": "),
        Span::styled(value.to_string(), value_style),
    ])
}
