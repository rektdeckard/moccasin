use crate::app::{App, AppResult};
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    #[cfg(windows)]
    match key_event.kind {
        KeyEventKind::Press => {}
        _ => return Ok(()),
    }

    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => {
            app.quit();
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
        }
        // Arrow handlers
        KeyCode::Down | KeyCode::Char('j') => {
            app.next();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.prev();
        }
        KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => {
            app.next_view();
        }
        KeyCode::Left | KeyCode::BackTab | KeyCode::Char('h') => {
            app.prev_view();
        }
        // Other handlers you could add here.
        KeyCode::Enter => {
            app.enter();
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_mouse_events(mouse_event: MouseEvent, app: &mut App) -> AppResult<()> {
    match mouse_event.kind {
        MouseEventKind::ScrollDown => {
            app.next();
        }
        MouseEventKind::ScrollUp => {
            app.prev();
        }
        MouseEventKind::ScrollRight | MouseEventKind::Down(MouseButton::Left) => {
            app.next_view();
        }
        MouseEventKind::ScrollLeft | MouseEventKind::Down(MouseButton::Right) => {
            app.prev_view();
        }
        _ => {}
    }
    Ok(())
}
