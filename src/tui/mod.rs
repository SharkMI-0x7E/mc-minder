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
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use crate::api::McStatusSnapshot;

    let mut app = App::new(config_path.clone());

    // Spawn background MC status polling (so TUI can show server status)
    let mc_cache: Arc<RwLock<Option<(McStatusSnapshot, std::time::Instant)>>> = Arc::new(RwLock::new(None));
    app.mc_status_cache = Some(mc_cache.clone());

    // Load config to get port + interval
    if let Ok(config) = crate::config::Config::load(config_path) {
        let mc_port = crate::config::discover_minecraft_port(
            config_path.parent().unwrap_or(std::path::Path::new("."))
        ).0;
        let interval = Duration::from_secs(config.mc_status.ping_interval_secs);
        let timeout = Duration::from_secs(config.mc_status.ping_timeout_secs);

        tokio::spawn(async move {
            loop {
                let result = mc_status_probe::ping("127.0.0.1", mc_port, timeout, None).await;
                let snapshot = match result {
                    Ok(r) => McStatusSnapshot {
                        online: true,
                        players_online: r.players_online,
                        players_max: r.players_max,
                        version: r.version_name,
                        latency_ms: r.latency_ms,
                        motd: r.description,
                        error: None,
                        tps: None,
                        alert: None,
                    },
                    Err(e) => McStatusSnapshot::offline(&e.to_string()),
                };
                let mut cache = mc_cache.write().await;
                *cache = Some((snapshot, std::time::Instant::now()));
                tokio::time::sleep(interval).await;
            }
        });
    }

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

        // Refresh MC status snapshot from shared cache (non-blocking)
        if let Some(ref cache) = app.mc_status_cache {
            if let Ok(guard) = cache.try_read() {
                if let Some((ref snapshot, _)) = *guard {
                    app.mc_status_snapshot = Some(snapshot.clone());
                    // Record TPS history for chart (P6-2)
                    if let Some(tps) = snapshot.tps {
                        app.tps_history.push_back(tps);
                        if app.tps_history.len() > 30 {
                            app.tps_history.pop_front();
                        }
                    }
                }
            }
        }

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
                // Skip key Release events on Windows to avoid double-processing
                // (Windows terminals send both Press and Release)
                if key.kind == crossterm::event::KeyEventKind::Release {
                    continue;
                }
                // Also skip Repeat events for the same reason
                if key.kind == crossterm::event::KeyEventKind::Repeat {
                    continue;
                }
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

    // If foreground server was requested, exit TUI and exec Java directly
    if app.foreground_requested {
        let config = crate::config::Config::load(config_path).ok();
        let jar = config.as_ref().map(|c| c.server.jar.clone()).unwrap_or_default();
        let min_mem = config.as_ref().map(|c| c.server.min_mem.clone()).unwrap_or_else(|| "512M".to_string());
        let max_mem = config.as_ref().map(|c| c.server.max_mem.clone()).unwrap_or_else(|| "1G".to_string());

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