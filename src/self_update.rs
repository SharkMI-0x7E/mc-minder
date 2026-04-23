// Self-update module
// Extracted from main.rs for modularity

use anyhow::{Result, Context, bail};

use std::fs;
use std::env;

use serde_json;
use colored::Colorize;
use dialoguer::Confirm;

pub async fn run_self_update() -> Result<()> {
    println!("Checking for updates...");

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/SharkMI-0x7E/mc-minder/releases/latest")
        .header("User-Agent", "mc-minder")
        .send()
        .await
        .context("Failed to check for updates. Please check your network connection.")?;

    if !response.status().is_success() {
        bail!("Failed to check for updates: HTTP {}", response.status());
    }

    let release: serde_json::Value = response.json().await?;
    let latest_version = release["tag_name"]
        .as_str()
        .unwrap_or("unknown")
        .trim_start_matches('v');

    let current_version = env!("CARGO_PKG_VERSION");

    println!("Current version: {}", current_version);
    println!("Latest version: {}", latest_version);

    if latest_version == current_version {
        println!("You are already on the latest version!");
        return Ok(());
    }

    let confirm = Confirm::new()
        .with_prompt("Update to the latest version?")
        .default(true)
        .interact()?;

    if !confirm {
        println!("Update cancelled.");
        return Ok(());
    }

    let target = if cfg!(target_os = "android") {
        "termux-aarch64"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64-linux"
    } else {
        "aarch64-linux"
    };

    let download_url = format!(
        "https://github.com/SharkMI-0x7E/mc-minder/releases/download/v{}/mc-minder-{}",
        latest_version, target
    );

    println!("Downloading from: {}", download_url);

    let binary_response = client
        .get(&download_url)
        .send()
        .await
        .context("Failed to download update")?;

    if !binary_response.status().is_success() {
        bail!("Failed to download binary: HTTP {}", binary_response.status());
    }

    let binary_data = binary_response.bytes().await?;

    let exe_path = std::env::current_exe()?;
    let backup_path = format!("{}.old", exe_path.display());

    fs::rename(&exe_path, &backup_path)?;
    fs::write(&exe_path, &binary_data)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&exe_path, fs::Permissions::from_mode(0o755))?;
    }

    println!("{} Updated to version {}!", "✓".green(), latest_version);
    println!("Please restart MC-Minder to use the new version.");

    Ok(())
}
