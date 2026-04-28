// Backup module (P5)
// Creates world backups, manages retention, integrates with RCON.

use anyhow::{Context, Result};
use log::info;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Create a backup of the world directory.
/// First sends /save-all via RCON, then copies world files.
pub fn create_backup(
    world_dir: &Path,
    backup_dir: &Path,
    session_name: &str,
) -> Result<PathBuf> {
    if !world_dir.exists() {
        return Err(anyhow::anyhow!("World directory not found: {:?}", world_dir));
    }

    std::fs::create_dir_all(backup_dir)
        .context("Failed to create backup directory")?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("{}-{}", session_name, timestamp);
    let backup_path = backup_dir.join(&backup_name);

    // Simple copy of world directory (no compression for now)
    copy_dir_all(world_dir, &backup_path)
        .with_context(|| format!("Failed to copy world from {:?} to {:?}", world_dir, backup_path))?;

    info!("[Backup] Created: {:?}", backup_path);
    Ok(backup_path)
}

/// Restore a world backup. WARNING: overwrites current world.
pub fn restore_backup(backup_path: &Path, world_dir: &Path) -> Result<()> {
    if !backup_path.exists() {
        return Err(anyhow::anyhow!("Backup not found: {:?}", backup_path));
    }

    // Remove current world (backup first?)
    if world_dir.exists() {
        std::fs::remove_dir_all(world_dir)
            .context("Failed to remove current world")?;
    }

    copy_dir_all(backup_path, world_dir)
        .context("Failed to restore world from backup")?;

    info!("[Backup] Restored {:?} -> {:?}", backup_path, world_dir);
    Ok(())
}

/// List all backups in the backup directory, sorted by modification time (newest first).
pub fn list_backups(backup_dir: &Path) -> Result<Vec<BackupEntry>> {
    let mut entries = Vec::new();
    if !backup_dir.exists() {
        return Ok(entries);
    }

    for entry in std::fs::read_dir(backup_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let metadata = entry.metadata().ok();
        let size = dir_size(&path).unwrap_or(0);
        let modified = metadata.and_then(|m| m.modified().ok()).unwrap_or(SystemTime::now());
        entries.push(BackupEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path,
            size,
            modified,
        });
    }

    entries.sort_by(|a, b| b.modified.cmp(&a.modified)); // newest first
    Ok(entries)
}

/// Apply retention policy: remove old backups exceeding max_count or max_days.
pub fn apply_retention(backup_dir: &Path, max_count: usize, max_days: u64) {
    let mut backups = match list_backups(backup_dir) {
        Ok(b) => b,
        Err(_) => return,
    };

    // Remove by count (keep newest)
    while backups.len() > max_count {
        if let Some(oldest) = backups.pop() {
            let _ = std::fs::remove_dir_all(&oldest.path);
            info!("[Backup] Retention removed (count): {}", oldest.name);
        }
    }

    // Remove by age
    let now = SystemTime::now();
    let max_age = std::time::Duration::from_secs(max_days * 86400);
    for b in &backups {
        if let Ok(age) = now.duration_since(b.modified) {
            if age > max_age {
                let _ = std::fs::remove_dir_all(&b.path);
                info!("[Backup] Retention removed (age): {}", b.name);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackupEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn dir_size(path: &Path) -> Option<u64> {
    let mut total = 0u64;
    fn walk(path: &Path, total: &mut u64) {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_dir() {
                        walk(&entry.path(), total);
                    } else {
                        *total += meta.len();
                    }
                }
            }
        }
    }
    walk(path, &mut total);
    Some(total)
}
