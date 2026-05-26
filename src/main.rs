use chordz::ui::app::App;
use std::io;
use std::panic;

use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

fn main() -> io::Result<()> {
    // Initialize terminal.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Set panic hook to restore terminal state on panic.
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        hook(info);
    }));

    // Run the app.
    let mut app = App::new();
    let result = app.run(&mut terminal);

    // Restore terminal.
    terminal.show_cursor()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;

    result
}
