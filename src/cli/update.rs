//! Self-update functionality.
//!
//! This module handles checking for updates and downloading new versions
//! of the afk binary from GitHub releases.

use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

use reqwest::blocking::Client;
use serde::Deserialize;

/// GitHub repository for releases.
const GITHUB_REPO: &str = "m0nkmaster/afk";

/// GitHub API URL for releases.
const GITHUB_API_URL: &str = "https://api.github.com/repos";

/// Current version from Cargo.toml.
const CURRENT_VERSION: &str = crate::VERSION;

/// GitHub release asset information.
#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

/// GitHub release information.
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    assets: Vec<ReleaseAsset>,
}

/// Result of checking for updates.
#[derive(Debug)]
pub struct UpdateCheckResult {
    /// Current version.
    pub current_version: String,
    /// Latest version available.
    pub latest_version: String,
    /// Whether an update is available.
    pub update_available: bool,
    /// Download URL for the binary.
    pub download_url: Option<String>,
    /// Asset name.
    pub asset_name: Option<String>,
}

/// Error type for update operations.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// HTTP request to GitHub API failed.
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    /// File I/O error during update.
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    /// No release found on GitHub.
    #[error("No release found")]
    NoReleaseFound,
    /// No pre-built binary available for this platform.
    #[error("No binary available for this platform")]
    NoBinaryForPlatform,
    /// Could not determine the current executable path.
    #[error("Failed to determine current executable path")]
    NoExecutablePath,
    /// Self-update not available for pip installations.
    #[error("Self-update not supported when installed via pip")]
    InstalledViaPip,
}

/// Get the platform-specific binary name.
///
/// Binary names must match what's produced by the release workflow:
/// - Linux: afk-linux-x86_64, afk-linux-arm64
/// - macOS: afk-darwin-x86_64, afk-darwin-arm64
/// - Windows: afk-windows-x86_64.exe
pub fn get_platform_binary() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "afk-linux-x86_64";

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "afk-linux-arm64";

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "afk-darwin-x86_64";

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "afk-darwin-arm64";

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "afk-windows-x86_64.exe";

    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    return "afk-unknown";
}

/// Check if running from a pip installation.
fn is_pip_install() -> bool {
    // If the executable is in a site-packages directory, it's a pip install
    if let Ok(exe_path) = env::current_exe() {
        let path_str = exe_path.to_string_lossy();
        return path_str.contains("site-packages") || path_str.contains("dist-packages");
    }
    false
}

/// Create an HTTP client with appropriate headers.
fn create_client() -> Result<Client, UpdateError> {
    Ok(Client::builder()
        .user_agent(format!("afk/{}", CURRENT_VERSION))
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

/// Get the latest release from GitHub that has binaries for this platform.
fn get_latest_release(client: &Client, include_prerelease: bool) -> Result<Release, UpdateError> {
    let url = format!("{}/{}/releases", GITHUB_API_URL, GITHUB_REPO);

    let response: Vec<Release> = client.get(&url).send()?.json()?;

    let platform_binary = get_platform_binary();

    // Find the first release that:
    // 1. Matches prerelease criteria
    // 2. Has a binary for this platform (skip releases still building)
    response
        .into_iter()
        .find(|r| {
            let prerelease_ok = include_prerelease || !r.prerelease;
            let has_binary = r.assets.iter().any(|a| a.name == platform_binary);
            prerelease_ok && has_binary
        })
        .ok_or(UpdateError::NoReleaseFound)
}

/// Parse version string, stripping 'v' prefix if present.
fn parse_version(version: &str) -> &str {
    version.strip_prefix('v').unwrap_or(version)
}

/// Compare versions. Returns true if new_version > current_version.
fn is_newer_version(current: &str, new: &str) -> bool {
    let current = parse_version(current);
    let new = parse_version(new);

    // Simple semver comparison
    let current_parts: Vec<&str> = current.split('.').collect();
    let new_parts: Vec<&str> = new.split('.').collect();

    for i in 0..3 {
        let current_num: u32 = current_parts
            .get(i)
            .and_then(|s| s.split('-').next()) // Strip prerelease suffix
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let new_num: u32 = new_parts
            .get(i)
            .and_then(|s| s.split('-').next()) // Strip prerelease suffix
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        match new_num.cmp(&current_num) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => {}
        }
    }

    // Handle prerelease suffixes (rc, beta, etc.)
    // A release version is always newer than a prerelease of the same base version
    let current_has_suffix = current.contains('-');
    let new_has_suffix = new.contains('-');

    if current_has_suffix && !new_has_suffix {
        return true; // 1.0.0 > 1.0.0-rc1
    }

    false
}

/// Check for available updates.
pub fn check_for_updates(include_prerelease: bool) -> Result<UpdateCheckResult, UpdateError> {
    let client = create_client()?;
    let release = get_latest_release(&client, include_prerelease)?;

    let latest_version = parse_version(&release.tag_name).to_string();
    let update_available = is_newer_version(CURRENT_VERSION, &latest_version);

    // Find the binary for this platform
    let platform_binary = get_platform_binary();
    let asset = release.assets.iter().find(|a| a.name == platform_binary);

    Ok(UpdateCheckResult {
        current_version: CURRENT_VERSION.to_string(),
        latest_version,
        update_available,
        download_url: asset.map(|a| a.browser_download_url.clone()),
        asset_name: asset.map(|a| a.name.clone()),
    })
}

/// Download and install the update.
pub fn perform_update(download_url: &str) -> Result<PathBuf, UpdateError> {
    // Check if running from pip
    if is_pip_install() {
        return Err(UpdateError::InstalledViaPip);
    }

    // Get current executable path
    let current_exe = env::current_exe().map_err(|_| UpdateError::NoExecutablePath)?;

    let client = create_client()?;

    // Download to temp file
    let temp_dir = env::temp_dir();
    let temp_file = temp_dir.join(format!("afk-update-{}", std::process::id()));

    println!("\x1b[2mDownloading update...\x1b[0m");

    let response = client.get(download_url).send()?;
    let bytes = response.bytes()?;

    {
        let mut file = File::create(&temp_file)?;
        file.write_all(&bytes)?;
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_file)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_file, perms)?;
    }

    // Replace current executable
    // On Windows, we need to rename the current exe first
    #[cfg(windows)]
    {
        let backup_path = current_exe.with_extension("exe.old");
        if backup_path.exists() {
            fs::remove_file(&backup_path)?;
        }
        fs::rename(&current_exe, &backup_path)?;
        fs::rename(&temp_file, &current_exe)?;
    }

    #[cfg(not(windows))]
    {
        fs::rename(&temp_file, &current_exe)?;
    }

    Ok(current_exe)
}

/// Execute the update command.
pub fn execute_update(beta: bool, check_only: bool) -> Result<(), UpdateError> {
    // Check if running from pip
    if is_pip_install() && !check_only {
        println!("\x1b[33mNote:\x1b[0m Self-update is not available for pip installations.");
        println!();
        println!("To update, use:");
        println!("  pip install --upgrade afk");
        println!();
        println!("Or install the standalone binary:");
        println!(
            "  curl -fsSL https://raw.githubusercontent.com/{}/main/scripts/install.sh | sh",
            GITHUB_REPO
        );
        return Ok(());
    }

    println!("\x1b[36mℹ\x1b[0m Checking for updates...");

    let result = check_for_updates(beta)?;

    println!(
        "  Current version: \x1b[36m{}\x1b[0m",
        result.current_version
    );
    println!(
        "  Latest version:  \x1b[36m{}\x1b[0m",
        result.latest_version
    );
    println!();

    if !result.update_available {
        println!("\x1b[32m✓\x1b[0m You're running the latest version!");
        return Ok(());
    }

    if result.download_url.is_none() {
        println!("\x1b[33m⚠\x1b[0m Update available but no binary for this platform.");
        println!("  Platform: {}", get_platform_binary());
        println!();
        println!("Build from source:");
        println!("  cargo install --git https://github.com/{}", GITHUB_REPO);
        return Ok(());
    }

    if check_only {
        println!(
            "\x1b[33m⚠\x1b[0m Update available: {} → {}",
            result.current_version, result.latest_version
        );
        println!();
        println!("Run 'afk update' to install.");
        return Ok(());
    }

    println!(
        "\x1b[36mℹ\x1b[0m Updating {} → {}...",
        result.current_version, result.latest_version
    );

    let exe_path = perform_update(result.download_url.as_ref().unwrap())?;

    println!();
    println!(
        "\x1b[32m✓\x1b[0m Successfully updated to version {}",
        result.latest_version
    );
    println!("  Binary: {}", exe_path.display());
    println!();
    println!("Restart afk to use the new version.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_platform_binary() {
        let binary = get_platform_binary();
        assert!(binary.starts_with("afk-"));
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("v1.0.0"), "1.0.0");
        assert_eq!(parse_version("1.0.0"), "1.0.0");
        assert_eq!(parse_version("v0.3.2"), "0.3.2");
    }

    #[test]
    fn test_is_newer_version_major() {
        assert!(is_newer_version("1.0.0", "2.0.0"));
        assert!(!is_newer_version("2.0.0", "1.0.0"));
    }

    #[test]
    fn test_is_newer_version_minor() {
        assert!(is_newer_version("1.0.0", "1.1.0"));
        assert!(!is_newer_version("1.1.0", "1.0.0"));
    }

    #[test]
    fn test_is_newer_version_patch() {
        assert!(is_newer_version("1.0.0", "1.0.1"));
        assert!(!is_newer_version("1.0.1", "1.0.0"));
    }

    #[test]
    fn test_is_newer_version_same() {
        assert!(!is_newer_version("1.0.0", "1.0.0"));
    }

    #[test]
    fn test_is_newer_version_with_v_prefix() {
        assert!(is_newer_version("v1.0.0", "v2.0.0"));
        assert!(is_newer_version("1.0.0", "v2.0.0"));
        assert!(is_newer_version("v1.0.0", "2.0.0"));
    }

    #[test]
    fn test_is_newer_version_prerelease() {
        // Release is newer than prerelease of same version
        assert!(is_newer_version("1.0.0-rc1", "1.0.0"));
        // Same prerelease is not newer
        assert!(!is_newer_version("1.0.0-rc1", "1.0.0-rc1"));
    }

    #[test]
    fn test_is_newer_version_complex() {
        assert!(is_newer_version("0.3.2", "1.0.0"));
        assert!(is_newer_version("0.3.2", "0.4.0"));
        assert!(is_newer_version("0.3.2", "0.3.3"));
        assert!(!is_newer_version("1.0.0", "0.9.9"));
    }

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_current_version_defined() {
        assert!(!CURRENT_VERSION.is_empty());
    }

    // Note: Network tests are skipped to avoid external dependencies
    // #[test]
    // fn test_check_for_updates() {
    //     let result = check_for_updates(false);
    //     // May fail if no network or no releases
    //     if let Ok(r) = result {
    //         assert!(!r.current_version.is_empty());
    //     }
    // }
}
