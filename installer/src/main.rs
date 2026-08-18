//! ytb-dl-tui-install: a single, self-contained installer. It embeds a
//! real build of `ytb_dl_tui` (see `build.rs`), installs it plus yt-dlp,
//! ffmpeg, and a JS runtime (deno) into one dedicated per-user folder
//! (`%LOCALAPPDATA%\Programs\ytb-dl-tui` on Windows,
//! `~/.local/share/ytb-dl-tui` on Linux -- no admin/root needed), and
//! registers that folder on PATH so `ytb_dl_tui` runs from any terminal,
//! any directory, without the user having to keep files together by hand.

mod download;
mod extract;
mod install_location;
mod path_env;
mod tools;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use path_env::PathAction;
use tools::{Payload, Tool};

/// The exact `ytb_dl_tui` binary for this platform, embedded at compile
/// time -- see `build.rs`, which requires it to exist in `embed/` before
/// letting this crate build at all.
static APP_BINARY: &[u8] = include_bytes!(env!("YTB_DL_TUI_BINARY_PATH"));

fn app_exe_filename() -> &'static str {
    if cfg!(windows) {
        "ytb_dl_tui.exe"
    } else {
        "ytb_dl_tui"
    }
}

fn main() {
    let code = run();
    pause_before_exit();
    std::process::exit(code);
}

fn run() -> i32 {
    println!("ytb-dl-tui installer");
    println!("=====================");
    println!();

    let install_dir = match install_location::resolve() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Could not determine where to install: {e:#}");
            return 1;
        }
    };
    if let Err(e) = std::fs::create_dir_all(&install_dir) {
        eprintln!("Could not create {}: {e}", install_dir.display());
        return 1;
    }
    println!("Installing to: {}", install_dir.display());
    println!();

    println!("-- ytb_dl_tui --");
    let app_dest = install_dir.join(app_exe_filename());
    if let Err(e) = install_app(&app_dest) {
        eprintln!("   FAILED: {e:#}");
        eprintln!("Cannot continue without the app itself installed.");
        return 1;
    }
    println!("   installed: {}", app_dest.display());
    println!();

    let mut missing_required = false;
    for tool in Tool::ALL {
        println!("-- {} --", tool.display_name());
        match ensure_tool(tool, &install_dir) {
            Ok(Status::AlreadyOnPath(path)) => {
                println!("   already available on PATH: {}", path.display());
            }
            Ok(Status::AlreadyInstalled(path)) => {
                println!("   already installed: {}", path.display());
            }
            Ok(Status::Installed(path)) => {
                println!("   installed: {}", path.display());
            }
            Err(e) => {
                println!("   FAILED: {e:#}");
                if tool.required() {
                    missing_required = true;
                } else {
                    println!(
                        "   (not required -- ytb_dl_tui will still run, with reduced features)"
                    );
                }
            }
        }
        println!();
    }

    println!("-- PATH --");
    let mut needs_new_terminal = false;
    match path_env::ensure_on_path(&install_dir) {
        Ok(PathAction::AlreadyPresent) => {
            println!("   already on PATH");
        }
        Ok(PathAction::Added(where_)) => {
            println!("   added to {where_}");
            needs_new_terminal = true;
        }
        Err(e) => {
            println!("   could not update PATH automatically: {e:#}");
            println!("   add this folder to PATH yourself to run ytb_dl_tui from anywhere:");
            println!("     {}", install_dir.display());
        }
    }
    println!();

    if missing_required {
        println!("yt-dlp could not be installed automatically; ytb_dl_tui cannot run");
        println!("without it. Try running this installer again, or download it");
        println!("yourself and place it in:");
        println!("  {}", install_dir.display());
        return 1;
    }

    println!("Done. ytb_dl_tui is installed.");
    if needs_new_terminal {
        println!("Open a NEW terminal window and run:  ytb_dl_tui");
        println!("(a window that was already open won't see the PATH change)");
    } else {
        println!("Run it from any terminal:  ytb_dl_tui");
    }
    0
}

/// Writes the embedded app binary to `dest`, always overwriting -- running
/// the installer again should always leave you with exactly the version it
/// shipped, not whatever happened to already be there.
fn install_app(dest: &Path) -> Result<()> {
    std::fs::write(dest, APP_BINARY).with_context(|| format!("writing {}", dest.display()))?;
    make_executable(dest)?;
    Ok(())
}

enum Status {
    AlreadyOnPath(PathBuf),
    AlreadyInstalled(PathBuf),
    Installed(PathBuf),
}

fn ensure_tool(tool: Tool, install_dir: &Path) -> Result<Status> {
    if let Ok(path) = which::which(tool.exe_stem())
        && verify_runs(&path, tool)
    {
        return Ok(Status::AlreadyOnPath(path));
    }

    let dest = install_dir.join(tool.exe_filename());
    if dest.is_file() && verify_runs(&dest, tool) {
        return Ok(Status::AlreadyInstalled(dest));
    }

    println!("   downloading {}", tool.download_url());
    let bytes = download::fetch(tool.download_url())?;

    match tool.payload() {
        Payload::RawExecutable => {
            std::fs::write(&dest, &bytes).with_context(|| format!("writing {}", dest.display()))?;
        }
        Payload::Archive => {
            extract::install_from_archive(
                &bytes,
                tool.download_url(),
                &tool.exe_filename(),
                &dest,
            )?;
        }
    }
    make_executable(&dest)?;

    if !verify_runs(&dest, tool) {
        bail!(
            "downloaded to {} but it did not run correctly",
            dest.display()
        );
    }

    Ok(Status::Installed(dest))
}

fn verify_runs(path: &Path, tool: Tool) -> bool {
    Command::new(path)
        .args(tool.version_args())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(windows)]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// Windows: double-clicking closes the console the instant the process
/// exits, so without this the user never gets to read the result.
#[cfg(windows)]
fn pause_before_exit() {
    use std::io::Write;
    println!("Press Enter to close this window...");
    let _ = std::io::stdout().flush();
    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf);
}

#[cfg(not(windows))]
fn pause_before_exit() {}
