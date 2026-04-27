mod cli;
mod init;
mod self_update;
mod server_run;
mod notification;
mod banner;

mod config;
mod monitor;
mod api;
mod tui;
mod update_engine;
mod command_sender;
mod foreground_process;
mod core_download;

use anyhow::Result;
use clap::Parser;
use cli::{Args, handle_command};
use banner::{init_logger, print_banner};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(ref cmd) = args.command {
        return handle_command(cmd.clone(), &args).await;
    }

    init_logger(args.verbose)?;
    print_banner();

    server_run::run_server_bg(args).await
}