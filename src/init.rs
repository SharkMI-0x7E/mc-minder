use anyhow::Result;
use colored::Colorize;
use dialoguer::{Input, Confirm};

use std::fs;
use std::path::PathBuf;
use super::config::Config;

pub async fn run_init() -> Result<()> {
    println!("{}", "MC-Minder Initialization".green().bold());
    println!("This will help you set up MC-Minder for your Minecraft server.\n");

    let rcon_password: String = Input::new()
        .with_prompt("RCON password (from server.properties)")
        .default(String::new())
        .interact()?;

    if rcon_password.is_empty() {
        println!("{}", "Warning: RCON password is empty. Please set it in server.properties first.".yellow());
    }

    let use_ai = Confirm::new()
        .with_prompt("Enable AI chat features?")
        .default(false)
        .interact()?;

    let (api_url, api_key, use_ollama) = if use_ai {
        let use_ollama = Confirm::new()
            .with_prompt("Use local Ollama instead of OpenAI-compatible API?")
            .default(false)
            .interact()?;

        if use_ollama {
            (String::new(), String::new(), true)
        } else {
            let api_url: String = Input::new()
                .with_prompt("AI API URL (e.g., https://api.openai.com/v1/chat/completions)")
                .default("https://api.openai.com/v1/chat/completions".to_string())
                .interact()?;

            let api_key: String = Input::new()
                .with_prompt("API Key")
                .interact()?;

            (api_url, api_key, false)
        }
    } else {
        (String::new(), String::new(), false)
    };

    let min_mem: String = Input::new()
        .with_prompt("Minimum memory for Minecraft server")
        .default("512M".to_string())
        .interact()?;

    let max_mem: String = Input::new()
        .with_prompt("Maximum memory for Minecraft server")
        .default("1G".to_string())
        .interact()?;

    let session_name: String = Input::new()
        .with_prompt("tmux session name")
        .default("mc_server".to_string())
        .interact()?;

    let config_content = generate_config_content(
        &rcon_password,
        &api_url,
        &api_key,
        use_ai && !use_ollama,
        use_ai && use_ollama,
        &min_mem,
        &max_mem,
        &session_name,
    );

    let config_path = PathBuf::from(super::banner::DEFAULT_CONFIG_PATH);
    fs::write(&config_path, &config_content)?;
    println!("\n{} Created {}", "✓".green(), config_path.display());

    generate_start_script()?;
    generate_backup_script()?;

    println!("\n{}", "Initialization complete!".green().bold());
    println!("\nNext steps:");
    println!("  1. Ensure RCON is enabled in server.properties:");
    println!("     enable-rcon=true");
    println!("     rcon.port=25575");
    println!("     rcon.password=<your_password>");
    println!("  2. Place fabric-server.jar in the current directory");
    println!("  3. Run: ./start-tui.sh");

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn generate_config_content(
    rcon_password: &str,
    api_url: &str,
    api_key: &str,
    enable_ai: bool,
    enable_ollama: bool,
    min_mem: &str,
    max_mem: &str,
    session_name: &str,
) -> String {
    let ai_section = if enable_ai {
        format!(
r#"[ai]
api_url = "{}"
api_key = "{}"
model = "gpt-3.5-turbo"
trigger = "!"
max_tokens = 150
temperature = 0.7
"#, api_url, api_key)
    } else {
        String::new()
    };

    let ollama_section = if enable_ollama {
        String::from(
r#"[ollama]
enabled = true
url = "http://localhost:11434/api/generate"
model = "qwen:0.5b"
"#)
    } else {
        String::from(
r#"[ollama]
enabled = false
url = "http://localhost:11434/api/generate"
model = "qwen:0.5b"
"#)
    };

    format!(
r#"# MC-Minder Configuration File
# MC-Minder 配置文件

# Server Configuration
# 服务器配置
[server]
jar = "fabric-server.jar"
min_mem = "{}"
max_mem = "{}"
session_name = "{}"
log_file = "logs/latest.log"

# RCON Configuration
# RCON 配置
[rcon]
host = "127.0.0.1"
port = 25575
password = "{}"

{}
{}
# Backup Configuration
# 备份配置
[backup]
world_dir = "world"
backup_dest = "../backups"
retain_days = 7

# Notification Configuration
# 通知配置
[notification]
telegram_bot_token = ""
telegram_chat_id = ""
termux_notify = true
"#, min_mem, max_mem, session_name, rcon_password, ai_section, ollama_section)
}

pub fn generate_config(path: &PathBuf) -> Result<()> {
    let content = Config::generate_template();
    fs::write(path, &content)?;
    println!("{} Generated config file: {}", "✓".green(), path.display());
    Ok(())
}

pub fn generate_start_script() -> Result<()> {
    // Write start-tui.sh - now a simple launcher for mc-minder tui
    let script = include_str!("../scripts/start-tui.sh");
    let script_path = PathBuf::from("start-tui.sh");
    fs::write(&script_path, script)?;

    // Set permissions (unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
    }

    println!("{} Generated start-tui.sh", "✓".green());
    Ok(())
}

pub fn generate_backup_script() -> Result<()> {
    let script = include_str!("../scripts/backup.sh");
    let script_path = PathBuf::from("backup.sh");
    fs::write(&script_path, script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
    }

    println!("{} Generated backup.sh", "✓".green());
    Ok(())
}

pub fn show_config(path: &PathBuf) -> Result<()> {
    let config = Config::load(path)?;
    println!("Configuration loaded from: {}", path.display());
    println!("{:#?}", config);
    Ok(())
}

pub fn get_config_value(path: &PathBuf, key: &str) -> Result<()> {
    let config = Config::load(path)?;

    let value = match key {
        "rcon.host" => config.rcon.host.clone(),
        "rcon.port" => config.rcon.port.to_string(),
        "rcon.password" => config.rcon.password.clone(),
        "server.jar" => config.server.jar.clone(),
        "server.min_mem" => config.server.min_mem.clone(),
        "server.max_mem" => config.server.max_mem.clone(),
        "server.session_name" => config.server.session_name.clone(),
        "server.log_file" => config.server.log_file.clone(),
        "backup.world_dir" => config.backup.world_dir.clone(),
        "backup.backup_dest" => config.backup.backup_dest.clone(),
        "backup.retain_days" => config.backup.retain_days.to_string(),
        "jvm.jdk_path" => config.jvm.jdk_path.clone().unwrap_or_default(),
        _ => {
            anyhow::bail!("Unknown config key: {}", key);
        }
    };

    println!("{}", value);
    Ok(())
}
