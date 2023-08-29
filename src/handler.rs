use crate::app::{App, AppResult};
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    if cfg!(target_os = "windows") {
        match key_event.kind {
            KeyEventKind::Press => {}
            _ => return Ok(()),
        }
    }

    // println!("{:?}", app.add_feed_state);
    if app.should_render_feed_input() {
        match key_event.code {
            KeyCode::Char('c') | KeyCode::Char('C') if key_event.modifiers == KeyModifiers::CONTROL => {
                app.quit();
            }
            KeyCode::Enter => app.submit_message(),
            KeyCode::Char(to_insert) => {
                app.enter_char(to_insert);
            }
            KeyCode::Backspace => {
                app.delete_char();
            }
            KeyCode::Left => {
                app.move_cursor_left();
            }
            KeyCode::Right => {
                app.move_cursor_right();
            }
            KeyCode::Esc => {
                app.toggle_add_feed();
            }
            _ => {}
        }
        return Ok(());
    }

    if app.show_keybinds {
        match key_event.code {
            // Exit application on `q`
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.quit();
            }
            _ => {
                app.toggle_keybinds();
                return Ok(());
            }
        }
    }

    match key_event.code {
        // Exit application on `q`
        KeyCode::Char('q') | KeyCode::Char('Q') => {
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
        KeyCode::Right | KeyCode::Enter | KeyCode::Char('l') => {
            app.next_view(false);
        }
        KeyCode::Tab => {
            app.next_view(true);
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.prev_view(false);
        }
        KeyCode::BackTab => {
            app.prev_view(true);
        }
        // Other handlers you could add here.
        KeyCode::Esc => {
            app.unselect();
        }
        KeyCode::Char('a') => {
            app.toggle_add_feed();
        }
        KeyCode::Char('o') => {
            app.open();
        }
        KeyCode::Char('r') => {
            app.refresh_all();
        }
        KeyCode::Char('?') => {
            app.toggle_keybinds();
        }
        KeyCode::Char(',') => {
            app.open_config();
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
            app.next_view(false);
        }
        MouseEventKind::ScrollLeft | MouseEventKind::Down(MouseButton::Right) => {
            app.prev_view(false);
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_resize_events(dimensions: (u16, u16), app: &mut App) -> AppResult<()> {
    app.set_dimensions(dimensions);
    Ok(())
}
