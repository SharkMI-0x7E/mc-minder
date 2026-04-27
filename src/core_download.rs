use anyhow::{Context, Result};
use log::info;
use serde::Deserialize;
use std::path::Path;

/// Supported server core types.
#[derive(Debug, Clone, PartialEq)]
pub enum CoreType {
    Fabric,
    Paper,
    Vanilla,
}

impl CoreType {
    pub fn all() -> Vec<CoreType> {
        vec![CoreType::Fabric, CoreType::Vanilla, CoreType::Paper]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CoreType::Fabric => "Fabric (Mod Loader)",
            CoreType::Vanilla => "Vanilla (Official Mojang)",
            CoreType::Paper => "Paper (Performance Fork)",
        }
    }

    pub fn display_name_cn(&self) -> &'static str {
        match self {
            CoreType::Fabric => "Fabric (模组加载器)",
            CoreType::Vanilla => "Vanilla (Mojang 官方)",
            CoreType::Paper => "Paper (性能优化版)",
        }
    }
}

// ============================================================
// FabricMC Meta API
// ============================================================

#[derive(Debug, Deserialize)]
struct FabricGameVersion {
    version: String,
    stable: bool,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderVersion {
    version: String,
    stable: bool,
}

/// Fetch available Minecraft versions from FabricMC.
pub async fn fetch_fabric_game_versions() -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let resp: Vec<FabricGameVersion> = client
        .get("https://meta.fabricmc.net/v2/versions/game")
        .header("User-Agent", "mc-minder")
        .send()
        .await
        .context("Failed to fetch FabricMC game versions")?
        .json()
        .await
        .context("Failed to parse FabricMC game versions")?;

    // Return stable versions, newest first
    let mut versions: Vec<String> = resp
        .into_iter()
        .filter(|v| v.stable)
        .map(|v| v.version)
        .collect();
    versions.reverse(); // newest first
    Ok(versions)
}

/// Fetch stable Fabric loader version for a game version.
pub async fn fetch_fabric_loader(game_version: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let resp: Vec<FabricLoaderVersion> = client
        .get(format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}",
            game_version
        ))
        .header("User-Agent", "mc-minder")
        .send()
        .await
        .context("Failed to fetch Fabric loader versions")?
        .json()
        .await
        .context("Failed to parse Fabric loader versions")?;

    resp.into_iter()
        .find(|v| v.stable)
        .map(|v| v.version)
        .ok_or_else(|| anyhow::anyhow!("No stable Fabric loader found for {}", game_version))
}

/// Download Fabric server jar.
pub async fn download_fabric_server(
    game_version: &str,
    loader_version: &str,
    output_dir: &Path,
) -> Result<String> {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/server/jar",
        game_version, loader_version
    );
    let filename = format!("fabric-server-{}.jar", game_version);
    download_to_file(&url, output_dir, &filename).await
}

// ============================================================
// Vanilla (Mojang) API
// ============================================================

#[derive(Debug, Deserialize)]
struct VersionManifest {
    versions: Vec<VersionEntry>,
}

#[derive(Debug, Deserialize)]
struct VersionEntry {
    id: String,
    #[serde(rename = "type")]
    release_type: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct VersionInfo {
    downloads: VersionDownloads,
}

#[derive(Debug, Deserialize)]
struct VersionDownloads {
    server: DownloadInfo,
}

#[derive(Debug, Deserialize)]
struct DownloadInfo {
    url: String,
}

/// Fetch available Vanilla Minecraft versions.
pub async fn fetch_vanilla_versions() -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let manifest: VersionManifest = client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest.json")
        .header("User-Agent", "mc-minder")
        .send()
        .await
        .context("Failed to fetch Mojang version manifest")?
        .json()
        .await
        .context("Failed to parse Mojang version manifest")?;

    let mut versions: Vec<String> = manifest
        .versions
        .into_iter()
        .filter(|v| v.release_type == "release")
        .map(|v| v.id)
        .collect();
    versions.reverse(); // newest first
    Ok(versions)
}

/// Download Vanilla Minecraft server jar.
pub async fn download_vanilla_server(
    game_version: &str,
    output_dir: &Path,
) -> Result<String> {
    // First, get the manifest to find the version URL
    let client = reqwest::Client::new();
    let manifest: VersionManifest = client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest.json")
        .header("User-Agent", "mc-minder")
        .send()
        .await?
        .json()
        .await?;

    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == game_version)
        .ok_or_else(|| anyhow::anyhow!("Version {} not found in Mojang manifest", game_version))?;

    // Get version info to find download URL
    let info: VersionInfo = client
        .get(&entry.url)
        .header("User-Agent", "mc-minder")
        .send()
        .await?
        .json()
        .await?;

    let filename = format!("minecraft-server-{}.jar", game_version);
    download_to_file(&info.downloads.server.url, output_dir, &filename).await
}

// ============================================================
// PaperMC API
// ============================================================

#[derive(Debug, Deserialize)]
struct PaperProject {
    versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PaperVersionBuilds {
    builds: Vec<i64>,
}

#[derive(Debug, Deserialize)]
struct PaperBuildInfo {
    downloads: PaperDownloads,
}

#[derive(Debug, Deserialize)]
struct PaperDownloads {
    application: PaperDownloadFile,
}

#[derive(Debug, Deserialize)]
struct PaperDownloadFile {
    name: String,
}

/// Fetch available PaperMC versions.
pub async fn fetch_paper_versions() -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let project: PaperProject = client
        .get("https://api.papermc.io/v2/projects/paper")
        .header("User-Agent", "mc-minder")
        .send()
        .await
        .context("Failed to fetch PaperMC versions")?
        .json()
        .await
        .context("Failed to parse PaperMC versions")?;

    let mut versions = project.versions;
    versions.reverse(); // newest first
    Ok(versions)
}

/// Download PaperMC server jar (latest build for a version).
pub async fn download_paper_server(
    game_version: &str,
    output_dir: &Path,
) -> Result<String> {
    let client = reqwest::Client::new();

    // Get latest build
    let builds: PaperVersionBuilds = client
        .get(format!(
            "https://api.papermc.io/v2/projects/paper/versions/{}",
            game_version
        ))
        .header("User-Agent", "mc-minder")
        .send()
        .await
        .context("Failed to fetch PaperMC builds")?
        .json()
        .await
        .context("Failed to parse PaperMC builds")?;

    let latest_build = builds
        .builds
        .last()
        .ok_or_else(|| anyhow::anyhow!("No builds found for PaperMC {}", game_version))?;

    // Get download URL
    let build_info: PaperBuildInfo = client
        .get(format!(
            "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}",
            game_version, latest_build
        ))
        .header("User-Agent", "mc-minder")
        .send()
        .await?
        .json()
        .await?;

    let download_name = &build_info.downloads.application.name;
    let url = format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/{}",
        game_version, latest_build, download_name
    );

    let filename = format!("paper-{}.jar", game_version);
    download_to_file(&url, output_dir, &filename).await
}

// ============================================================
// Modrinth API (Mod platform integration)
// ============================================================

#[derive(Debug, Deserialize)]
pub struct ModrinthSearchResult {
    pub hits: Vec<ModrinthHit>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModrinthHit {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub downloads: i64,
    pub project_type: String,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
pub struct ModrinthVersion {
    pub files: Vec<ModrinthFile>,
    pub version_number: String,
}

#[derive(Debug, Deserialize)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
}

/// Search Modrinth for mods.
pub async fn search_modrinth(query: &str, limit: u32) -> Result<Vec<ModrinthHit>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.modrinth.com/v2/search?query={}&limit={}&facets=[[\"project_type:mod\"],[\"categories:fabric\"]]",
        urlencoding(query),
        limit
    );
    let resp: ModrinthSearchResult = client
        .get(&url)
        .header("User-Agent", "mc-minder")
        .send()
        .await
        .context("Failed to search Modrinth")?
        .json()
        .await
        .context("Failed to parse Modrinth search")?;

    Ok(resp.hits)
}

/// Get the latest stable version file for a Modrinth project.
pub async fn get_modrinth_latest_version(project_id: &str, game_version: &str) -> Result<ModrinthFile> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.modrinth.com/v2/project/{}/version?loaders=[\"fabric\"]&game_versions=[\"{}\"]",
        project_id, game_version
    );
    let versions: Vec<ModrinthVersion> = client
        .get(&url)
        .header("User-Agent", "mc-minder")
        .send()
        .await
        .context("Failed to fetch Modrinth versions")?
        .json()
        .await
        .context("Failed to parse Modrinth versions")?;

    let version = versions.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("No compatible version found for {}", game_version))?;

    version.files.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("No download file for Modrinth project {}", project_id))
}

/// Download a mod from Modrinth URL to the mods folder.
pub async fn download_modrinth_mod(url: &str, filename: &str, output_dir: &Path) -> Result<String> {
    let mods_dir = output_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).ok();
    download_to_file(url, &mods_dir, filename).await
}

/// Popular Fabric mods for quick install.
pub fn popular_mods() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("fabric-api", "Fabric API", "P7dR8mSH"),
        ("sodium", "Sodium", "AANobbMI"),
        ("lithium", "Lithium", "gvQqBUqZ"),
        ("phosphor", "Phosphor", "fQEb0iXm"),
        ("iris", "Iris Shaders", "YL57xq9U"),
        ("modmenu", "Mod Menu", "mOgUt4GM"),
    ]
}

/// Simple URL encoding (just replaces space with %20).
fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('&', "%26")
        .replace('?', "%3F")
        .replace('#', "%23")
}


async fn download_to_file(url: &str, output_dir: &Path, filename: &str) -> Result<String> {
    let output_path = output_dir.join(filename);
    info!("Downloading {} from {}", filename, url);

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "mc-minder")
        .send()
        .await
        .with_context(|| format!("Failed to download from {}", url))?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Download failed: HTTP {} from {}",
            response.status(),
            url
        ));
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read download response")?;

    std::fs::write(&output_path, &bytes)
        .with_context(|| format!("Failed to write to {:?}", output_path))?;

    info!("Downloaded {} ({} bytes)", filename, bytes.len());
    Ok(output_path.to_string_lossy().to_string())
}
