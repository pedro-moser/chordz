use crossterm::event::{Event, KeyCode, KeyEventKind};

use super::app::App;

/// Handle a single terminal event, updating app state.
///
/// Supported keys:
/// - `j` / `Down`: move selection down
/// - `k` / `Up`: move selection up
/// - `h` / `Left`: focus previous field
/// - `l` / `Right` / `Tab`: focus next field
/// - `g` / `Home`: move to first item in focused field
/// - `G` / `End`: move to last item in focused field
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
            // Move to edges.
            KeyCode::Char('g') | KeyCode::Home => {
                app.screen.select_first();
            }
            KeyCode::Char('G') | KeyCode::End => {
                app.screen.select_last();
            }
            _ => {}
        }
    }
}
