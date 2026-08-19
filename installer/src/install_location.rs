//! Where everything gets installed: a dedicated per-user directory, not
//! wherever the installer happens to be run from. Per-user (not
//! machine-wide) deliberately -- it needs no admin/root privileges, which
//! matches every other part of this app's "just works, no elevation"
//! design.

use std::path::PathBuf;

use anyhow::{Context, Result};

#[cfg(windows)]
pub fn resolve() -> Result<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").context("%LOCALAPPDATA% is not set")?;
    Ok(PathBuf::from(local_app_data)
        .join("Programs")
        .join("Playloader"))
}

#[cfg(unix)]
pub fn resolve() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("$HOME is not set")?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("playloader"))
}
