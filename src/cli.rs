// CLI argument parsing and command dispatching
// Extracted from main.rs for modularity

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mc-minder")]
#[command(author = "SharkMI-0x7E")]
#[command(version)]
#[command(about = "A smart management suite for Minecraft Fabric servers")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, value_name = "PATH", default_value = super::banner::DEFAULT_CONFIG_PATH)]
    pub config: PathBuf,

    #[arg(short, long)]
    pub verbose: bool,

    #[arg(long, default_value = "8080")]
    pub http_port: u16,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    #[command(about = "Initialize configuration interactively")]
    Init,

    #[command(about = "Generate default config file")]
    GenConfig,

    #[command(about = "Generate start-tui.sh script")]
    GenStart,

    #[command(about = "Generate backup.sh script")]
    GenBackup,

    #[command(about = "Update to the latest version")]
    SelfUpdate,

    #[command(about = "Show current configuration")]
    Config,

    #[command(about = "Get a config value by key")]
    ConfigGet {
        #[arg(required = true)]
        key: String,
    },

    #[command(about = "Start TUI management interface")]
    Tui,
}

use anyhow::Result;

pub async fn handle_command(cmd: Commands, args: &Args) -> Result<()> {
    match cmd {
        Commands::Init => super::init::run_init().await,
        Commands::GenConfig => super::init::generate_config(&args.config),
        Commands::GenStart => super::init::generate_start_script(),
        Commands::GenBackup => super::init::generate_backup_script(),
        Commands::SelfUpdate => super::self_update::run_self_update().await,
        Commands::Config => super::init::show_config(&args.config),
        Commands::ConfigGet { key } => super::init::get_config_value(&args.config, &key),
        Commands::Tui => super::tui::run(&args.config).await,
    }
}
