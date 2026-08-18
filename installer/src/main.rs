//! ytb-dl-tui-install: a single, self-contained installer. It embeds a
//! real build of `ytb_dl_tui` (see `build.rs`) plus our own vendored copy of
//! yt-dlp's source (see `../vendor/`), and installs those together with a
//! bundled Python runtime, ffmpeg, and a JS runtime (deno) into one
//! dedicated per-user folder (`%LOCALAPPDATA%\Programs\ytb-dl-tui` on
//! Windows, `~/.local/share/ytb-dl-tui` on Linux -- no admin/root needed),
//! then registers that folder on PATH so `ytb_dl_tui` runs from any
//! terminal, any directory, without the user having to keep files together
//! by hand or separately install yt-dlp themselves.

mod download;
mod extract;
mod install_location;
mod path_env;
mod python_runtime;
mod tools;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use path_env::PathAction;
use tools::Tool;

/// The exact `ytb_dl_tui` binary for this platform, embedded at compile
/// time -- see `build.rs`, which requires it to exist in `embed/` before
/// letting this crate build at all.
static APP_BINARY: &[u8] = include_bytes!(env!("YTB_DL_TUI_BINARY_PATH"));

/// Our vendored copy of yt-dlp's Python source (see `../vendor/`), zipped
/// up at compile time by `build.rs`. We ship this instead of downloading
/// yt-dlp's own release binary, so the exact version running is always the
/// one this installer was built with.
static YTDLP_VENDOR_ZIP: &[u8] = include_bytes!(env!("YTDLP_VENDOR_ZIP_PATH"));

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

    println!("-- yt-dlp (bundled Python runtime + vendored source) --");
    let mut missing_required = false;
    match ensure_ytdlp(&install_dir) {
        Ok(YtDlpStatus::AlreadyInstalled) => println!("   already installed"),
        Ok(YtDlpStatus::Installed) => println!("   installed"),
        Err(e) => {
            println!("   FAILED: {e:#}");
            missing_required = true;
        }
    }
    println!();

    println!("-- ffmpeg --");
    match ensure_ffmpeg(&install_dir) {
        Ok(Status::AlreadyOnPath(path)) => {
            println!("   already available on PATH: {}", path.display())
        }
        Ok(Status::AlreadyInstalled(path)) => println!("   already installed: {}", path.display()),
        Ok(Status::Installed(path)) => println!("   installed: {}", path.display()),
        Err(e) => {
            println!("   FAILED: {e:#}");
            println!("   (not required -- ytb_dl_tui will still run, with reduced features)");
        }
    }
    println!();

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
        println!("The bundled Python runtime + yt-dlp could not be installed");
        println!("automatically; ytb_dl_tui cannot run without them. Check your");
        println!("network connection and try running this installer again.");
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

/// Where `ytb_dl_tui` itself expects to find the bundled interpreter and
/// vendored yt-dlp source -- keep in sync with `resolve_ytdlp` in the main
/// app's `src/ytdlp.rs`.
fn runtime_dir(install_dir: &Path) -> PathBuf {
    install_dir.join("python-runtime")
}
fn ytdlp_src_dir(install_dir: &Path) -> PathBuf {
    install_dir.join("yt_dlp_src")
}

enum YtDlpStatus {
    AlreadyInstalled,
    Installed,
}

/// Installs the bundled Python interpreter (downloaded, unless a working
/// copy from a previous run is already sitting there) and our vendored
/// yt-dlp source (always re-extracted from what's embedded in this
/// installer, so re-running always leaves you with exactly the version this
/// installer shipped -- same reasoning as `install_app`).
fn ensure_ytdlp(install_dir: &Path) -> Result<YtDlpStatus> {
    let runtime_dir = runtime_dir(install_dir);
    let python_path = python_runtime::python_exe_path(&runtime_dir);
    let src_dir = ytdlp_src_dir(install_dir);

    let already_installed = python_path.is_file() && verify_python(&python_path);

    if !already_installed {
        println!(
            "   downloading Python {} (build {})",
            python_runtime::PYTHON_VERSION,
            python_runtime::RELEASE_TAG
        );
        let bytes = download::fetch(python_runtime::download_url())?;
        extract::install_dir_from_archive(&bytes, python_runtime::download_url(), &runtime_dir)?;
        make_executable(&python_path)?;
        if !verify_python(&python_path) {
            bail!(
                "downloaded Python runtime to {} but it did not run correctly",
                runtime_dir.display()
            );
        }
    }

    extract::install_embedded_zip(YTDLP_VENDOR_ZIP, &src_dir)
        .with_context(|| format!("extracting vendored yt-dlp source to {}", src_dir.display()))?;

    if !verify_ytdlp(&python_path, &src_dir) {
        bail!("installed yt-dlp (via the bundled Python runtime) but it did not run correctly");
    }

    Ok(if already_installed {
        YtDlpStatus::AlreadyInstalled
    } else {
        YtDlpStatus::Installed
    })
}

fn verify_python(python_path: &Path) -> bool {
    Command::new(python_path)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn verify_ytdlp(python_path: &Path, src_dir: &Path) -> bool {
    Command::new(python_path)
        .env("PYTHONPATH", src_dir)
        .args(["-m", "yt_dlp", "--version"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

enum Status {
    AlreadyOnPath(PathBuf),
    AlreadyInstalled(PathBuf),
    Installed(PathBuf),
}

/// Installs ffmpeg *and* ffprobe from one download. yt-dlp is given
/// ffmpeg's path via `--ffmpeg-location` and, from that, looks for ffprobe
/// in the very same directory (see yt-dlp's `postprocessor/ffmpeg.py`) --
/// so unlike `ensure_tool`'s other callers, cherry-picking just the one
/// named executable out of the archive isn't enough here.
fn ensure_ffmpeg(install_dir: &Path) -> Result<Status> {
    let tool = Tool::Ffmpeg;
    if let Ok(path) = which::which(tool.exe_stem())
        && verify_runs(&path, tool)
    {
        return Ok(Status::AlreadyOnPath(path));
    }

    let dest = install_dir.join(tool.exe_filename());
    let companion_dest = tool
        .companion_exe_filename()
        .map(|name| install_dir.join(name));
    let companion_ok = companion_dest.as_deref().is_none_or(Path::is_file);
    if dest.is_file() && companion_ok && verify_runs(&dest, tool) {
        return Ok(Status::AlreadyInstalled(dest));
    }

    println!("   downloading {}", tool.download_url());
    let bytes = download::fetch(tool.download_url())?;
    let mut targets = vec![(tool.exe_filename(), dest.clone())];
    if let Some(companion_dest) = &companion_dest {
        targets.push((
            tool.companion_exe_filename().unwrap(),
            companion_dest.clone(),
        ));
    }
    let targets_ref: Vec<(&str, &Path)> = targets
        .iter()
        .map(|(name, path)| (name.as_str(), path.as_path()))
        .collect();
    extract::install_many_from_archive(&bytes, tool.download_url(), &targets_ref)?;
    make_executable(&dest)?;
    if let Some(companion_dest) = &companion_dest {
        make_executable(companion_dest)?;
    }

    if !verify_runs(&dest, tool) {
        bail!(
            "downloaded to {} but it did not run correctly",
            dest.display()
        );
    }

    Ok(Status::Installed(dest))
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
    extract::install_from_archive(&bytes, tool.download_url(), &tool.exe_filename(), &dest)?;
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
