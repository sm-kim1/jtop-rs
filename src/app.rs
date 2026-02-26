use anyhow::Result;
use crossterm::event::KeyCode;

use crate::event::{Event, EventHandler};
use crate::hardware::{HardwareReader, JetsonStats, LocalReader};
use crate::remote::SshReader;
use crate::ui::Tab;
use crate::Args;

/// Application state
pub struct App {
    pub running: bool,
    pub current_tab: Tab,
    pub stats: JetsonStats,
    pub stats_history: Vec<JetsonStats>,
    pub max_history: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            current_tab: Tab::All,
            stats: JetsonStats::default(),
            stats_history: Vec::new(),
            max_history: 120, // 60 seconds at 500ms interval
        }
    }

    pub fn next_tab(&mut self) {
        let idx = self.current_tab.index();
        let next = (idx + 1) % Tab::ALL_TABS.len();
        self.current_tab = Tab::ALL_TABS[next];
    }

    pub fn prev_tab(&mut self) {
        let idx = self.current_tab.index();
        let prev = if idx == 0 {
            Tab::ALL_TABS.len() - 1
        } else {
            idx - 1
        };
        self.current_tab = Tab::ALL_TABS[prev];
    }

    pub fn update_stats(&mut self, stats: JetsonStats) {
        self.stats_history.push(self.stats.clone());
        if self.stats_history.len() > self.max_history {
            self.stats_history.remove(0);
        }
        self.stats = stats;
    }
}

/// Main application run loop
pub async fn run(args: Args) -> Result<()> {
    // Create hardware reader based on args
    let mut reader: Box<dyn HardwareReader> = if let Some(ref host) = args.host {
        Box::new(SshReader::new(
            host.clone(),
            args.user.clone(),
            args.port,
            args.key.clone(),
        ))
    } else {
        Box::new(LocalReader::new())
    };

    // Setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = App::new();
    let event_handler = EventHandler::new(args.interval);

    // Main loop
    while app.running {
        // Read hardware stats
        match reader.read_stats() {
            Ok(stats) => app.update_stats(stats),
            Err(e) => tracing::warn!("Failed to read stats: {e}"),
        }

        // Render
        terminal.draw(|frame| {
            crate::ui::render_tab(frame, &app);
        })?;

        // Handle events
        match event_handler.next()? {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => app.running = false,
                KeyCode::Tab | KeyCode::Right => app.next_tab(),
                KeyCode::BackTab | KeyCode::Left => app.prev_tab(),
                KeyCode::Char('1') => app.current_tab = Tab::All,
                KeyCode::Char('2') => app.current_tab = Tab::Cpu,
                KeyCode::Char('3') => app.current_tab = Tab::Gpu,
                KeyCode::Char('4') => app.current_tab = Tab::Memory,
                KeyCode::Char('5') => app.current_tab = Tab::Engine,
                KeyCode::Char('6') => app.current_tab = Tab::Control,
                KeyCode::Char('7') => app.current_tab = Tab::Info,
                _ => {}
            },
            Event::Tick => {}
            Event::Resize(_, _) => {}
        }
    }

    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
