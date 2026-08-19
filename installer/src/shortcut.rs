//! A clickable launcher for the GUI app, so it shows up in the Start Menu
//! (Windows) or the application launcher (Linux) like a normal desktop app,
//! rather than only being runnable by typing its name in a terminal.
//!
//! Windows: deliberately a plain-text `.url` "Internet Shortcut" rather
//! than a real `.lnk` -- a `.lnk` needs raw COM FFI (`IShellLinkW`), which
//! this project has no way to test at all (no Windows machine anywhere in
//! its development loop, and CI only proves it *compiles*, not that the
//! unsafe calls are actually correct). A `.url` file still launches the
//! exe with a real icon, entirely in safe, trivially-correct plain text.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Creates the shortcut and returns where it was written.
#[cfg(windows)]
pub fn create(exe_path: &Path) -> Result<PathBuf> {
    let app_data = std::env::var_os("APPDATA").context("%APPDATA% is not set")?;
    let start_menu = PathBuf::from(app_data)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs");
    std::fs::create_dir_all(&start_menu)
        .with_context(|| format!("creating {}", start_menu.display()))?;

    // Windows shows a .url shortcut's *filename* (there's no separate
    // display-name field like .desktop has), so this capitalization is
    // what actually appears in the Start Menu.
    let target = start_menu.join("VidSave.url");
    let exe_str = exe_path.to_string_lossy().replace('\\', "/");
    let contents = format!(
        "[InternetShortcut]\r\nURL=file:///{exe_str}\r\nIconFile={}\r\nIconIndex=0\r\n",
        exe_path.display()
    );
    std::fs::write(&target, contents).with_context(|| format!("writing {}", target.display()))?;
    Ok(target)
}

/// A standard XDG desktop entry in the per-user applications directory --
/// no root needed, picked up by GNOME/KDE/most Linux app launchers.
#[cfg(unix)]
pub fn create(exe_path: &Path) -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("$HOME is not set")?;
    let apps_dir = PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("applications");
    std::fs::create_dir_all(&apps_dir)
        .with_context(|| format!("creating {}", apps_dir.display()))?;

    let target = apps_dir.join("vidsave.desktop");
    // `install_logic::run_install` writes `icon.png` next to the exe before
    // calling this -- a `.desktop` entry's `Icon=` accepts a plain absolute
    // path just as well as an icon-theme name, so no theme installation is
    // needed for the app-launcher to show a real icon instead of a generic
    // one.
    let icon_path = exe_path
        .parent()
        .map(|dir| dir.join("icon.png"))
        .unwrap_or_default();
    let contents = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=VidSave\n\
         Comment=Download YouTube playlists and videos\n\
         Exec=\"{}\"\n\
         Icon={}\n\
         Terminal=false\n\
         Categories=AudioVideo;Network;\n",
        exe_path.display(),
        icon_path.display(),
    );
    std::fs::write(&target, contents).with_context(|| format!("writing {}", target.display()))?;

    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&target)?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(&target, perms)?;

    Ok(target)
}
