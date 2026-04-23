// Banner and logger initialization
// Extracted from main.rs for modularity

use anyhow::Result;
use colored::Colorize;

use std::fs::{self, OpenOptions};
use std::path::PathBuf;

pub const DEFAULT_CONFIG_PATH: &str = "config.toml";
pub const LOG_FILE_PATH: &str = "logs/mc-minder.log";
pub const LOG_FILE_MAX_SIZE: u64 = 50 * 1024 * 1024;

pub fn print_banner() {
    println!("{}", format!("MC-Minder v{}", env!("CARGO_PKG_VERSION")).green().bold());
    println!("{}", "A smart management suite for Minecraft Fabric servers".dimmed());
    println!();
}

pub fn init_logger(verbose: bool) -> Result<()> {
    let log_dir = PathBuf::from("logs");
    if !log_dir.exists() {
        fs::create_dir_all(&log_dir)?;
    }

    let log_file = PathBuf::from(LOG_FILE_PATH);
    if log_file.exists() {
        let metadata = fs::metadata(&log_file)?;
        if metadata.len() > LOG_FILE_MAX_SIZE {
            let backup_path = format!("{}.old", LOG_FILE_PATH);
            let _ = fs::rename(&log_file, &backup_path);
        }
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;

    let log_level = if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::Builder::new()
        .filter_level(log_level)
        .format(|buf, record| {
            use std::io::Write;
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %z");
            writeln!(buf, "[{} {}] {}", timestamp, record.level(), record.args())
        })
        .target(env_logger::Target::Pipe(Box::new(file)))
        .init();

    Ok(())
}