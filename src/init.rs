use anyhow::Result;
use colored::Colorize;
use dialoguer::Input;

use std::fs;
use std::path::PathBuf;
use super::config::Config;

/// Detect Java installation and return version info
fn detect_java() -> Option<(String, String)> {
    // Try "java" first
    if let Ok(output) = std::process::Command::new("java")
        .arg("-version")
        .output()
    {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{} {}", stdout, stderr);
        
        // Extract version string
        for line in combined.lines() {
            if line.contains("version") {
                let version = line.trim().to_string();
                return Some(("java".to_string(), version));
            }
        }
        return Some(("java".to_string(), "unknown version".to_string()));
    }
    
    None
}

/// Check if running in Termux
fn is_termux() -> bool {
    std::env::var("TERMUX_VERSION").is_ok()
        || std::path::Path::new("/data/data/com.termux").exists()
}

pub async fn run_init() -> Result<()> {
    println!("{}", "MC-Minder Initialization".green().bold());
    println!("This will help you set up MC-Minder for your Minecraft server.\n");

    // === Java Detection ===
    println!("{}", "Checking Java installation...".cyan());
    let java_info = detect_java();
    
    let jdk_path = if let Some((_cmd, version)) = &java_info {
        println!("{} Java found: {}", "✓".green(), version);
        String::new() // Use system default
    } else {
        println!("{}", "✗ Java not found".red());
        
        if is_termux() {
            println!("\n{}", "Termux detected. You can install Java with:".yellow());
            println!("  pkg install openjdk-17");
            println!("  pkg install ecj");
        } else {
            println!("\n{}", "Please install Java 17+ for your system:".yellow());
            println!("  Ubuntu/Debian: sudo apt install openjdk-17-jre");
            println!("  Fedora: sudo dnf install java-17-openjdk");
            println!("  Arch: sudo pacman -S jre17-openjdk");
        }
        
        let custom_jdk: String = Input::new()
            .with_prompt("Custom JDK path (leave empty to use system default after installing)")
            .default(String::new())
            .interact()?;
        
        custom_jdk
    };

    // === RCON Configuration ===
    println!("\n{}", "RCON Configuration".cyan().bold());
    println!("RCON is required for MC-Minder to communicate with your Minecraft server.");
    println!("Make sure you have set enable-rcon=true in server.properties\n");

    let rcon_password: String = Input::new()
        .with_prompt("RCON password (from server.properties)")
        .default(String::new())
        .interact()?;

    if rcon_password.is_empty() {
        println!("{}", "Warning: RCON password is empty. Please set it in server.properties first.".yellow());
    }

    // === Server Memory ===
    let min_mem: String = Input::new()
        .with_prompt("Minimum memory for Minecraft server")
        .default("512M".to_string())
        .interact()?;

    let max_mem: String = Input::new()
        .with_prompt("Maximum memory for Minecraft server")
        .default("1G".to_string())
        .interact()?;

    // === Session Name ===
    let session_name: String = Input::new()
        .with_prompt("tmux session name")
        .default("mc_server".to_string())
        .interact()?;

    // === Server Jar ===
    let jar: String = Input::new()
        .with_prompt("Server jar filename")
        .default("fabric-server.jar".to_string())
        .interact()?;

    // === JVM Flags ===
    let jvm_flags: String = Input::new()
        .with_prompt("Extra JVM flags (optional)")
        .default(String::new())
        .interact()?;

    // Generate config
    let config_content = generate_config_content(
        &rcon_password,
        &min_mem,
        &max_mem,
        &session_name,
        &jar,
        &jvm_flags,
        &jdk_path,
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
    println!("  2. Place {} in the current directory", jar);
    if java_info.is_none() {
        println!("  3. Install Java (see instructions above)");
    }
    println!("  4. Run: ./start-tui.sh");

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn generate_config_content(
    rcon_password: &str,
    min_mem: &str,
    max_mem: &str,
    session_name: &str,
    jar: &str,
    jvm_flags: &str,
    jdk_path: &str,
) -> String {
    let jvm_section = if jdk_path.is_empty() && jvm_flags.is_empty() {
        String::from(
r#"[jvm]
gc = "G1GC"
extra_flags = ""
# jdk_path = "/usr/lib/jvm/java-17-openjdk/bin/java"
"#)
    } else {
        format!(
r#"[jvm]
gc = "G1GC"
extra_flags = "{}"
jdk_path = "{}"
"#, jvm_flags, jdk_path)
    };

    format!(
r#"# MC-Minder Configuration File
# MC-Minder 配置文件

# Server Configuration
# 服务器配置
[server]
jar = "{}"
min_mem = "{}"
max_mem = "{}"
session_name = "{}"
log_file = "logs/latest.log"
server_type = "fabric"

# RCON Configuration
# RCON 配置
[rcon]
host = "127.0.0.1"
port = 25575
password = "{}"

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

# MC Status Probe Configuration
# MC 状态查询配置
[mc_status]
ping_interval_secs = 60
ping_timeout_secs = 3
"#, jar, min_mem, max_mem, session_name, rcon_password, jvm_section)
}

pub fn generate_config(path: &PathBuf) -> Result<()> {
    let content = Config::generate_template();
    fs::write(path, &content)?;
    println!("{} Generated config file: {}", "✓".green(), path.display());
    Ok(())
}

pub fn generate_start_script() -> Result<()> {
    let script = include_str!("../scripts/start-tui.sh");
    let script_path = PathBuf::from("start-tui.sh");
    fs::write(&script_path, script)?;

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
        "jvm.gc" => config.jvm.gc.clone(),
        "jvm.extra_flags" => config.jvm.extra_flags.clone(),
        _ => {
            anyhow::bail!("Unknown config key: {}", key);
        }
    };

    println!("{}", value);
    Ok(())
}