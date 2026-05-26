use crossterm::event::{Event, KeyCode, KeyEventKind};

use super::app::App;

/// Handle a single terminal event, updating app state.
///
/// Supported keys:
/// - `j` / `Down`: move selection down
/// - `k` / `Up`: move selection up
/// - `h` / `Left`: focus chord list
/// - `l` / `Right` / `Tab`: focus voicing list
/// - `q` / `Esc`: quit
pub fn handle_event(event: Event, app: &mut App) {
    if let Event::Key(key) = event {
        // Ignore key release and repeat events.
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            // Quit.
            KeyCode::Char('q') | KeyCode::Esc => {
                app.running = false;
            }
            // Move down.
            KeyCode::Char('j') | KeyCode::Down => {
                app.screen.move_down();
            }
            // Move up.
            KeyCode::Char('k') | KeyCode::Up => {
                app.screen.move_up();
            }
            // Move focus.
            KeyCode::Char('h') | KeyCode::Left => {
                app.screen.focus_previous();
            }
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => {
                app.screen.focus_next();
            }
            _ => {}
        }
    }
}
