pub mod all;
pub mod control;
pub mod cpu;
pub mod engine;
pub mod gpu;
pub mod info;
pub mod memory;

use ratatui::Frame;

use crate::app::App;

/// Tab definitions for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    All,
    Cpu,
    Gpu,
    Memory,
    Engine,
    Control,
    Info,
}

impl Tab {
    pub const ALL_TABS: &[Tab] = &[
        Tab::All,
        Tab::Cpu,
        Tab::Gpu,
        Tab::Memory,
        Tab::Engine,
        Tab::Control,
        Tab::Info,
    ];

    pub fn title(&self) -> &str {
        match self {
            Tab::All => "ALL",
            Tab::Cpu => "CPU",
            Tab::Gpu => "GPU",
            Tab::Memory => "MEM",
            Tab::Engine => "ENGINE",
            Tab::Control => "CTRL",
            Tab::Info => "INFO",
        }
    }

    pub fn index(&self) -> usize {
        *self as usize
    }
}

/// Render the current tab with tab bar header and status footer
pub fn render_tab(frame: &mut Frame, app: &App) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Status bar
        ])
        .split(frame.area());

    // Tab bar
    let titles: Vec<Line> = Tab::ALL_TABS
        .iter()
        .map(|t| Line::from(Span::styled(t.title(), Style::default().fg(Color::White))))
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" jtop-rs "))
        .select(app.current_tab.index())
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::raw(" | "));
    frame.render_widget(tabs, chunks[0]);

    // Status bar
    let status = Line::from(vec![
        Span::styled(
            " q",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(":Quit "),
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(":Next "),
        Span::styled(
            "1-7",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(":Select "),
    ]);
    frame.render_widget(Paragraph::new(status), chunks[2]);

    // Render current tab content into the middle area
    let content_area = chunks[1];
    match app.current_tab {
        Tab::All => all::render(frame, app, content_area),
        Tab::Cpu => cpu::render(frame, app, content_area),
        Tab::Gpu => gpu::render(frame, app, content_area),
        Tab::Memory => memory::render(frame, app, content_area),
        Tab::Engine => engine::render(frame, app, content_area),
        Tab::Control => control::render(frame, app, content_area),
        Tab::Info => info::render(frame, app, content_area),
    }
}
