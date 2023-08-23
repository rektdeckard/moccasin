use std::io;
use tabss::app::{App, AppResult};
use tabss::event::{Event, EventHandler};
use tabss::handler::handle_key_events;
use tabss::tui::Tui;
use tui::backend::CrosstermBackend;
use tui::Terminal;

#[tokio::main]
async fn main() -> AppResult<()> {
    // Read or create config
    let config = tabss::config::Config::read_from_path(None)?;

    // Create an application.
    let mut app = App::init(config).await?;

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
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
