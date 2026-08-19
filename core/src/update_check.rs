//! Checking GitHub Releases for a newer version, and handing off to a
//! freshly-downloaded installer to actually perform the update.
//!
//! The update itself is deliberately *not* an in-place self-replace: on
//! Windows a running `.exe` can't be overwritten while it's open, so the
//! only reliable pattern is downloading the real installer (the same one
//! anyone would run by hand) and handing off to it -- see
//! `download_and_launch_installer`'s doc comment.

use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/bel-wadoud/vidsave/releases/latest";

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// e.g. "3.0.1" (the release tag with its leading `v` stripped).
    pub version: String,
    /// Release notes (Markdown), shown as-is.
    pub notes: String,
    pub asset_url: String,
    pub asset_name: String,
}

#[derive(Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    #[serde(default)]
    body: String,
    assets: Vec<AssetResponse>,
}

#[derive(Deserialize)]
struct AssetResponse {
    name: String,
    browser_download_url: String,
}

/// Compares `current` (pass `env!("CARGO_PKG_VERSION")`) against the
/// latest GitHub release. `Ok(None)` means already up to date (or ahead of
/// the latest release, e.g. a local dev build).
pub async fn check_for_update(current: &str) -> Result<Option<UpdateInfo>> {
    let current = current.to_string();
    tokio::task::spawn_blocking(move || check_blocking(&current))
        .await
        .context("update check task panicked")?
}

fn check_blocking(current: &str) -> Result<Option<UpdateInfo>> {
    let mut response = ureq::get(LATEST_RELEASE_URL)
        .header("User-Agent", "vidsave-update-check")
        .header("Accept", "application/vnd.github+json")
        .call()
        .context("checking for updates")?;
    if !response.status().is_success() {
        bail!("update check returned HTTP {}", response.status());
    }
    let body = response
        .body_mut()
        .read_to_string()
        .context("reading release info")?;
    let release: ReleaseResponse = serde_json::from_str(&body).context("parsing release info")?;

    let latest_version = release.tag_name.trim_start_matches('v');
    if !is_newer(latest_version, current) {
        return Ok(None);
    }

    let asset_name = platform_asset_name();
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .with_context(|| format!("the latest release has no {asset_name} asset"))?;

    Ok(Some(UpdateInfo {
        version: latest_version.to_string(),
        notes: release.body,
        asset_url: asset.browser_download_url.clone(),
        asset_name: asset.name.clone(),
    }))
}

fn platform_asset_name() -> &'static str {
    if cfg!(windows) {
        "vidsave-install-windows-x86_64.exe"
    } else {
        "vidsave-install-linux-x86_64"
    }
}

/// Plain `MAJOR.MINOR.PATCH` comparison -- sufficient for our own tags,
/// which are always exactly this shape (see `build-installer.sh` /
/// `release.yml`); no need to pull in a full semver-parsing dependency for
/// three integers.
fn is_newer(latest: &str, current: &str) -> bool {
    parse_version(latest) > parse_version(current)
}

fn parse_version(v: &str) -> (u32, u32, u32) {
    let mut parts = v.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// Which app(s) are installed next to the currently-running executable --
/// used so an update reinstalls the same components the user already has,
/// rather than silently adding the other one.
pub struct InstalledComponents {
    pub tui: bool,
    pub gui: bool,
}

pub fn detect_installed_components() -> InstalledComponents {
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
    else {
        // Can't tell -- reinstall both rather than risk silently dropping one.
        return InstalledComponents {
            tui: true,
            gui: true,
        };
    };
    let exe_name = |stem: &str| {
        if cfg!(windows) {
            format!("{stem}.exe")
        } else {
            stem.to_string()
        }
    };
    InstalledComponents {
        tui: dir.join(exe_name("vidsave-tui")).is_file(),
        gui: dir.join(exe_name("vidsave")).is_file(),
    }
}

/// Downloads the update's installer to a temp file and launches it
/// (`--silent`, matching whichever component(s) are already installed),
/// then returns -- it's on the caller to exit the current process right
/// after (e.g. the GUI's `iced::exit()`), since the installer needs to
/// overwrite files this process currently has open. yt-dlp/ffmpeg/the JS
/// runtime aren't touched by this at all; the installer's own update path
/// already only replaces the app binaries and re-verifies the rest.
pub async fn download_and_launch_installer(
    info: &UpdateInfo,
    components: &InstalledComponents,
) -> Result<()> {
    let url = info.asset_url.clone();
    let bytes = tokio::task::spawn_blocking(move || fetch(&url))
        .await
        .context("download task panicked")??;

    let temp_dir = std::env::temp_dir().join("vidsave-update");
    std::fs::create_dir_all(&temp_dir).context("creating temp update directory")?;
    let installer_path = temp_dir.join(&info.asset_name);
    std::fs::write(&installer_path, &bytes)
        .with_context(|| format!("writing {}", installer_path.display()))?;
    make_executable(&installer_path)?;

    let mut cmd = std::process::Command::new(&installer_path);
    cmd.arg("--silent");
    if !components.tui {
        cmd.arg("--no-tui");
    }
    if !components.gui {
        cmd.arg("--no-gui");
    }
    cmd.spawn().context("launching the downloaded installer")?;
    Ok(())
}

fn fetch(url: &str) -> Result<Vec<u8>> {
    const MAX_SIZE: u64 = 300 * 1024 * 1024;
    let mut response = ureq::get(url)
        .header("User-Agent", "vidsave-update-check")
        .call()
        .with_context(|| format!("downloading {url}"))?;
    if !response.status().is_success() {
        bail!("server returned HTTP {}", response.status());
    }
    let mut buf = Vec::new();
    response
        .body_mut()
        .with_config()
        .limit(MAX_SIZE)
        .reader()
        .read_to_end(&mut buf)
        .context("reading installer download")?;
    Ok(buf)
}

#[cfg(unix)]
fn make_executable(path: &PathBuf) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &PathBuf) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_compares_major_minor_patch() {
        assert!(is_newer("3.0.1", "3.0.0"));
        assert!(is_newer("3.1.0", "3.0.9"));
        assert!(is_newer("4.0.0", "3.9.9"));
        assert!(!is_newer("3.0.0", "3.0.0"));
        assert!(!is_newer("2.9.9", "3.0.0"));
    }

    #[test]
    fn parse_version_defaults_missing_parts_to_zero() {
        assert_eq!(parse_version("3.0"), (3, 0, 0));
        assert_eq!(parse_version("3"), (3, 0, 0));
        assert_eq!(parse_version("not-a-version"), (0, 0, 0));
    }
}
