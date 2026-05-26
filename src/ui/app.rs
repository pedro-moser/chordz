use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

use super::events::handle_event;
use super::screens::browser::BrowserScreen;

/// Application state.
pub struct App {
    /// Whether the app should keep running.
    pub running: bool,
    /// The browser screen.
    pub screen: BrowserScreen,
}

impl App {
    /// Create a new app with an initialized browser screen.
    pub fn new() -> Self {
        Self {
            running: true,
            screen: BrowserScreen::new(),
        }
    }

    /// Run the application event loop inside the given terminal.
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        while self.running {
            terminal.draw(|frame| self.screen.render(frame))?;

            // Poll for events with a short timeout to keep the UI responsive.
            if crossterm::event::poll(Duration::from_millis(100))? {
                let event = crossterm::event::read()?;
                handle_event(event, self);
            }
        }
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
