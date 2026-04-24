use std::path::PathBuf;
use std::io;

pub mod app;

pub async fn run(config_path: &PathBuf) -> anyhow::Result<()> {
    use self::app::App;
    use crossterm::event::{self, Event};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
    use crossterm::execute;
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::time::Duration;

    let mut app = App::new(config_path.clone());

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        terminal.draw(|f| {
            app.draw(f);
        })?;

        if app.should_quit {
            break;
        }

        // Handle message timeout
        if let Some(timeout) = &app.message_timeout {
            if timeout.elapsed() > Duration::from_secs(3) {
                app.message = None;
                app.message_timeout = None;
            }
        }

        // Poll for input
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                // Set message timeout on new message
                let had_message = app.message.is_some();
                app.on_key(key);
                if app.message.is_some() && !had_message {
                    app.message_timeout = Some(std::time::Instant::now());
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
