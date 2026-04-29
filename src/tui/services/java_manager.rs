// Java version management — detection, installation, switching.
// Pure logic with no UI dependency.

use std::path::Path;
// Public API surface
/// Detects all Java installations on the system.
/// Returns Vec of (path_or_label, version_string).
pub fn detect_java_versions(config: Option<&crate::config::Config>) -> Vec<(String, String)> {
    let mut versions: Vec<(String, String)> = Vec::new();

    // Helper: try get version from a java binary path
    fn add_version(versions: &mut Vec<(String, String)>, path: &str) {
        if versions.iter().any(|(p, _)| p == path) {
            return;
        }
        if path.is_empty() {
            return;
        }
        if let Ok(out) = std::process::Command::new(path).arg("-version").output() {
            let ver = String::from_utf8_lossy(&out.stderr);
            if let Some(line) = ver.lines().next() {
                versions.push((path.to_string(), line.to_string()));
            }
        }
    }

    // 1. Check system default java
    if let Ok(out) = std::process::Command::new("java").arg("-version").output() {
        let ver = String::from_utf8_lossy(&out.stderr);
        if let Some(line) = ver.lines().next() {
            versions.push(("system default (java)".to_string(), line.to_string()));
        }
    }

    // 2. Search PATH for all java binaries (which -a / where)
    let which_cmd = if cfg!(target_os = "windows") {
        std::process::Command::new("where").arg("java").output()
    } else {
        std::process::Command::new("which").args(["-a", "java"]).output()
    };
    if let Ok(out) = which_cmd {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if !line.is_empty() && line != "java" && !line.contains("no java") {
                add_version(&mut versions, line);
            }
        }
    }

    // 3. Check update-alternatives (Linux)
    if cfg!(target_os = "linux") {
        if let Ok(out) = std::process::Command::new("update-alternatives").args(["--list", "java"]).output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if !line.is_empty() {
                    add_version(&mut versions, line);
                }
            }
        }
    }

    // 4. Check custom JDK path from config
    if let Some(cfg) = config {
        if let Some(ref jdk) = cfg.jvm.jdk_path {
            if !jdk.is_empty() {
                add_version(&mut versions, jdk);
            }
        }
    }

    // 5. Search common installation directories
    let common_paths: Vec<String> = if cfg!(target_os = "android") {
        vec![
            "/data/data/com.termux/files/usr/lib/jvm".to_string(),
            "/data/data/com.termux/files/usr/bin".to_string(),
        ]
    } else {
        let mut paths = vec![
            "/usr/lib/jvm".to_string(),
            "/usr/java".to_string(),
            "/opt/java".to_string(),
            "/opt/jdk".to_string(),
            "/usr/local/lib/jvm".to_string(),
            "/snap/openjdk".to_string(),
        ];
        if let Ok(jh) = std::env::var("JAVA_HOME") {
            paths.push(jh);
        }
        if let Ok(jh) = std::env::var("JDK_HOME") {
            paths.push(jh);
        }
        paths
    };

    for base in common_paths {
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.contains("jdk") || name.contains("jre") || name.contains("openjdk") || name.contains("java") {
                        let full_path = entry.path();
                        let java_bin = full_path.join("bin").join("java");
                        if java_bin.exists() {
                            add_version(&mut versions, java_bin.to_str().unwrap_or(""));
                        } else if full_path.join("java").exists() {
                            add_version(&mut versions, full_path.join("java").to_str().unwrap_or(""));
                        } else {
                            versions.push((full_path.to_string_lossy().to_string(), name.to_string()));
                        }
                    }
                }
            }
        }
    }

    versions
}

/// Returns platform-specific Java installation options.
/// Returns Vec of (label, install_command, needs_confirmation).
pub fn java_install_options() -> Vec<(String, String, bool)> {
    let is_termux = std::env::var("TERMUX_VERSION").is_ok()
        || std::path::Path::new("/data/data/com.termux").exists();

    if is_termux {
        vec![
            ("OpenJDK 17".to_string(), "pkg install openjdk-17 -y".to_string(), false),
            ("OpenJDK 21".to_string(), "pkg install openjdk-21 -y".to_string(), false),
        ]
    } else if cfg!(target_os = "linux") {
        // Detect package manager
        let has_apt = std::process::Command::new("which").arg("apt").output().map(|o| o.status.success()).unwrap_or(false);
        let has_dnf = std::process::Command::new("which").arg("dnf").output().map(|o| o.status.success()).unwrap_or(false);
        let has_pacman = std::process::Command::new("which").arg("pacman").output().map(|o| o.status.success()).unwrap_or(false);

        if has_apt {
            vec![
                ("OpenJDK 17 (apt)".to_string(), "sudo apt install -y openjdk-17-jre".to_string(), true),
                ("OpenJDK 21 (apt)".to_string(), "sudo apt install -y openjdk-21-jre".to_string(), true),
            ]
        } else if has_dnf {
            vec![
                ("OpenJDK 17 (dnf)".to_string(), "sudo dnf install -y java-17-openjdk".to_string(), true),
                ("OpenJDK 21 (dnf)".to_string(), "sudo dnf install -y java-21-openjdk".to_string(), true),
            ]
        } else if has_pacman {
            vec![
                ("OpenJDK 17 (pacman)".to_string(), "sudo pacman -S --noconfirm jre17-openjdk".to_string(), true),
                ("OpenJDK 21 (pacman)".to_string(), "sudo pacman -S --noconfirm jre21-openjdk".to_string(), true),
            ]
        } else {
            vec![
                ("OpenJDK 17 (manual)".to_string(), "".to_string(), false),
                ("OpenJDK 21 (manual)".to_string(), "".to_string(), false),
            ]
        }
    } else {
        // macOS fallback
        vec![
            ("OpenJDK 17 (brew)".to_string(), "brew install openjdk@17".to_string(), false),
            ("OpenJDK 21 (brew)".to_string(), "brew install openjdk@21".to_string(), false),
        ]
    }
}

/// Locate the mc-minder binary nearby the provided config path.
pub fn find_mcminder_bin(config_path: &Path) -> String {
    // Check MC_MINDER_BIN env
    if let Ok(bin) = std::env::var("MC_MINDER_BIN") {
        if std::path::Path::new(&bin).exists() {
            return bin;
        }
    }

    // Check current executable directory (most reliable)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let bin = exe_dir.join("mc-minder");
            if bin.exists() {
                return bin.to_string_lossy().to_string();
            }
            // Also check for platform-specific names
            let platform_bin = if cfg!(target_os = "android") {
                exe_dir.join("mc-minder-termux-aarch64")
            } else {
                exe_dir.join("mc-minder-x86_64-linux")
            };
            if platform_bin.exists() {
                return platform_bin.to_string_lossy().to_string();
            }
        }
    }

    // Check current dir
    if std::path::Path::new("./mc-minder").exists() {
        return "./mc-minder".to_string();
    }

    // Check config path parent
    if let Some(parent) = config_path.parent() {
        let bin = parent.join("mc-minder");
        if bin.exists() {
            return bin.to_string_lossy().to_string();
        }
    }

    // Fallback to PATH
    "mc-minder".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_install_options_returns_something() {
        let options = java_install_options();
        assert!(!options.is_empty(), "Should return at least one install option");
    }
}
