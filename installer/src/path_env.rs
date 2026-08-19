//! Makes the install directory runnable by name from any new terminal,
//! not just from inside that one folder. This is the part a "just drop
//! tools next to the exe" installer skips -- and the whole point of a
//! *system* install.

use std::path::Path;

#[cfg(unix)]
use anyhow::Context;
use anyhow::Result;

/// What happened when we tried to add `dir` to PATH.
pub enum PathAction {
    /// `dir` (or something resolving to the same place) was on PATH already.
    AlreadyPresent,
    /// `dir` was added; `description` says where/how, for the summary
    /// message, and a new terminal is needed before it takes effect.
    Added(String),
}

#[cfg(windows)]
pub fn ensure_on_path(dir: &Path) -> Result<PathAction> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ};
    use winreg::types::ToRegValue;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env_key = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;

    let current: String = env_key.get_value("Path").unwrap_or_default();
    let already_present = std::env::split_paths(&current).any(|entry| entry == dir);
    if already_present {
        return Ok(PathAction::AlreadyPresent);
    }

    let dir_str = dir.to_string_lossy();
    let trimmed = current.trim_end_matches(';');
    let new_value = if trimmed.is_empty() {
        dir_str.to_string()
    } else {
        format!("{trimmed};{dir_str}")
    };

    // A stock user Path is REG_EXPAND_SZ (so %VAR%-style entries in it
    // keep expanding); preserve whatever type is already there instead of
    // silently downgrading it to REG_SZ, which `set_value` would do.
    // Default to REG_EXPAND_SZ -- the normal type -- if there's no
    // existing value to match.
    let vtype = env_key
        .get_raw_value("Path")
        .map(|v| v.vtype)
        .unwrap_or(REG_EXPAND_SZ);
    let mut reg_value = new_value.to_reg_value();
    reg_value.vtype = vtype;
    env_key.set_raw_value("Path", &reg_value)?;

    broadcast_environment_change();

    Ok(PathAction::Added(
        "your user PATH (HKEY_CURRENT_USER\\Environment)".to_string(),
    ))
}

/// Without this, Explorer (and anything it later spawns, including new
/// terminal windows opened the normal way) keeps using its own cached
/// environment block until you log off and back on. Broadcasting
/// WM_SETTINGCHANGE is the standard way installers avoid making the user
/// reboot just to pick up a PATH change.
#[cfg(windows)]
fn broadcast_environment_change() {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    };

    let param: Vec<u16> = "Environment"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut result: usize = 0;
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST as HWND,
            WM_SETTINGCHANGE,
            0 as WPARAM,
            param.as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            5000,
            &mut result as *mut usize,
        );
    }
}

#[cfg(unix)]
pub fn ensure_on_path(dir: &Path) -> Result<PathAction> {
    use std::io::Write;

    if std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|entry| entry == dir))
        .unwrap_or(false)
    {
        return Ok(PathAction::AlreadyPresent);
    }

    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("$HOME is not set"))?;

    const MARKER_START: &str = "# >>> VidSave PATH >>>";
    const MARKER_END: &str = "# <<< VidSave PATH <<<";
    let block = format!(
        "\n{MARKER_START}\nexport PATH=\"{}:$PATH\"\n{MARKER_END}\n",
        dir.display()
    );

    // Cover both "opened a new terminal tab" (usually sources .bashrc /
    // .zshrc) and "logged in fresh" (usually sources .profile) without
    // trying to be clever about which shell/session type the user has.
    let candidates = [".bashrc", ".zshrc", ".profile"];
    let mut any_file_existed = false;
    let mut already_had_marker = false;
    let mut newly_touched = Vec::new();
    for name in candidates {
        let rc_path = home.join(name);
        if !rc_path.is_file() {
            continue; // don't invent shell configs the user never had
        }
        any_file_existed = true;
        let contents = std::fs::read_to_string(&rc_path)
            .with_context(|| format!("reading {}", rc_path.display()))?;
        if contents.contains(MARKER_START) {
            already_had_marker = true;
            continue; // this file's already set up from a previous run
        }
        // Append-only: never truncate/rewrite a file that already has
        // content, or a re-run could destroy whatever else was in it.
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&rc_path)
            .with_context(|| format!("opening {}", rc_path.display()))?;
        file.write_all(block.as_bytes())
            .with_context(|| format!("writing to {}", rc_path.display()))?;
        newly_touched.push(rc_path);
    }

    if !any_file_existed {
        // None of the usual files existed at all -- create .profile, read
        // by login shells across bash/zsh/dash/sh. Safe to create fresh
        // since nothing was there to lose.
        let rc_path = home.join(".profile");
        std::fs::write(&rc_path, block.trim_start())
            .with_context(|| format!("writing {}", rc_path.display()))?;
        newly_touched.push(rc_path);
    } else if newly_touched.is_empty() && already_had_marker {
        // Every existing candidate already had our block; nothing to do.
        return Ok(PathAction::AlreadyPresent);
    }

    let names: Vec<String> = newly_touched
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    Ok(PathAction::Added(names.join(", ")))
}
