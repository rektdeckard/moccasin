use clap::Parser;
use crossterm::terminal;
use std::io;
use tabss::app::{App, AppResult, Args};
use tabss::event::{Event, EventHandler};
use tabss::handler::{handle_key_events, handle_mouse_events, handle_resize_events};
use tabss::tui::Tui;
use tui::backend::CrosstermBackend;
use tui::Terminal;

#[tokio::main]
async fn main() -> AppResult<()> {
    // Parse arguments
    let args = Args::parse();

    // Read or create config
    let config = tabss::config::Config::new(args)?;

    // Create an application.
    let mut app = App::init(terminal::size().unwrap(), config).await?;

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            Event::Mouse(mouse_event) => handle_mouse_events(mouse_event, &mut app)?,
            Event::Resize(w, h) => handle_resize_events((w, h), &mut app)?,
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
