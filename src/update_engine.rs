// Update engine - handles version check, download and binary replacement
// Used by both CLI self-update command and TUI update flow

use anyhow::Result;
use serde_json;
use tokio::sync::mpsc;

/// Messages sent from update engine to UI
#[derive(Debug, Clone)]
pub enum UpdateMsg {
    UpdateAvailable { current: String, latest: String, download_url: String },
    UpToDate,
    DownloadProgress { downloaded: u64, total: Option<u64> },
    Installing,
    Done { new_version: String },
    Failed(String),
}

/// Detect target platform string for GitHub releases
pub fn detect_target() -> String {
    if cfg!(target_os = "android") {
        "termux-aarch64".to_string()
    } else if cfg!(target_arch = "x86_64") {
        "x86_64-linux".to_string()
    } else {
        "aarch64-linux".to_string()
    }
}

/// Build download URL for a specific version and target
pub fn build_download_url(version: &str, target: &str) -> String {
    format!(
        "https://github.com/SharkMI-0x7E/mc-minder/releases/download/v{}/mc-minder-{}",
        version, target
    )
}

/// Compare version strings, return true if latest > current
#[allow(dead_code)]
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    fn parse_ver(v: &str) -> Vec<u32> {
        v.split('.').filter_map(|s| s.parse().ok()).collect()
    }
    parse_ver(latest) > parse_ver(current)
}

/// Format bytes into human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    if bytes as f64 >= MB {
        format!("{:.1} MB", bytes as f64 / MB)
    } else if bytes as f64 >= KB {
        format!("{:.1} KB", bytes as f64 / KB)
    } else {
        format!("{} B", bytes)
    }
}

pub struct UpdateEngine {
    client: reqwest::Client,
}

impl UpdateEngine {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }

    pub async fn check_update(&self, current_version: &str) -> UpdateMsg {
        let response = match self.client
            .get("https://api.github.com/repos/SharkMI-0x7E/mc-minder/releases/latest")
            .header("User-Agent", "mc-minder")
            .send()
            .await {
            Ok(r) => r,
            Err(e) => return UpdateMsg::Failed(format!("Network error: {}", e)),
        };

        if !response.status().is_success() {
            return UpdateMsg::Failed(format!("HTTP error: {}", response.status()));
        }

        let release: serde_json::Value = match response.json().await {
            Ok(v) => v,
            Err(e) => return UpdateMsg::Failed(format!("Parse error: {}", e)),
        };

        let latest = release["tag_name"]
            .as_str()
            .unwrap_or("unknown")
            .trim_start_matches('v')
            .to_string();

        let target = detect_target();
        let download_url = build_download_url(&latest, &target);

        if latest == current_version {
            UpdateMsg::UpToDate
        } else {
            UpdateMsg::UpdateAvailable {
                current: current_version.to_string(),
                latest,
                download_url,
            }
        }
    }

    pub async fn download_and_install(
        &self,
        download_url: &str,
        latest_version: &str,
        tx: mpsc::Sender<UpdateMsg>,
    ) -> Result<(), String> {
        let response = self.client
            .get(download_url)
            .send()
            .await
            .map_err(|e| format!("Download failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let total_size = response.content_length();
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        let mut data = Vec::new();

        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    downloaded += bytes.len() as u64;
                    data.extend_from_slice(&bytes);
                    let _ = tx.send(UpdateMsg::DownloadProgress {
                        downloaded,
                        total: total_size,
                    }).await;
                }
                Err(e) => {
                    return Err(format!("Download error: {}", e));
                }
            }
        }

        let _ = tx.send(UpdateMsg::Installing).await;

        // Replace binary
        let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
        let backup_path = format!("{}.old", exe_path.display());

        // Rename old to .old
        std::fs::rename(&exe_path, &backup_path).map_err(|e| format!("Backup failed: {}", e))?;

        // Write new binary
        std::fs::write(&exe_path, &data).map_err(|e| format!("Write failed: {}", e))?;

        // Set executable permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&exe_path, fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Permission failed: {}", e))?;
        }

        let _ = tx.send(UpdateMsg::Done { new_version: latest_version.to_string() }).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_download_url() {
        let url = build_download_url("0.4.7", "x86_64-linux");
        assert_eq!(url, "https://github.com/SharkMI-0x7E/mc-minder/releases/download/v0.4.7/mc-minder-x86_64-linux");
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.4.6", "0.4.7"));
        assert!(!is_newer_version("0.4.7", "0.4.6"));
        assert!(!is_newer_version("0.4.7", "0.4.7"));
        assert!(is_newer_version("0.3.9", "0.4.0"));
        assert!(is_newer_version("0.4.6", "1.0.0"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1572864), "1.5 MB");
    }

    #[test]
    fn test_detect_target() {
        let t = detect_target();
        assert!(t == "termux-aarch64" || t == "x86_64-linux" || t == "aarch64-linux");
    }
}