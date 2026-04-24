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
        // Process async update messages first
        app.process_update_messages();

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

    // Restore terminal before starting foreground server
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    // If foreground server was requested, start it now
    if app.foreground_requested {
        let jar = app.get_jar();
        let min_mem = app.get_min_mem();
        let max_mem = app.get_max_mem();

        println!();
        println!("Starting Minecraft server in foreground...");
        println!("Command: java -Xms{} -Xmx{} -jar {} nogui", min_mem, max_mem, jar);
        println!("Press Ctrl+C to stop the server");
        println!();

        // Use exec to replace current process with java
        let status = std::process::Command::new("java")
            .args(["-Xms".to_owned() + &min_mem, "-Xmx".to_owned() + &max_mem, "-jar".to_owned(), jar, "nogui".to_owned()])
            .status();

        match status {
            Ok(exit_code) => {
                if exit_code.success() {
                    println!("\nServer stopped normally.");
                } else {
                    println!("\nServer exited with code: {:?}", exit_code.code());
                }
            }
            Err(e) => {
                println!("\nFailed to start server: {}", e);
            }
        }
    }

    Ok(())
}
