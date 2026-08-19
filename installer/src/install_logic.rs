//! The actual install sequence, factored out of the old console `main.rs`
//! so both a GUI wizard (and, in principle, any other frontend) can drive
//! it: reports progress by sending `InstallEvent`s instead of printing,
//! and installs the TUI and/or GUI app binaries depending on `Components`
//! rather than unconditionally installing one fixed binary.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use tokio::sync::mpsc::UnboundedSender;

use crate::{download, extract, install_location, path_env, python_runtime, shortcut, tools};
use path_env::PathAction;
use tools::Tool;

/// Which app binaries to install, chosen on the wizard's Components page.
#[derive(Debug, Clone, Copy)]
pub struct Components {
    pub tui: bool,
    pub gui: bool,
}

/// One line of progress. `Step` starts a new labeled section (mirrors the
/// old console output's `-- Section --` headers); `Detail`/`Warning` are
/// lines within the current section.
#[derive(Debug, Clone)]
pub enum InstallEvent {
    Step(String),
    Detail(String),
    Warning(String),
}

fn step(tx: &UnboundedSender<InstallEvent>, s: impl Into<String>) {
    let _ = tx.send(InstallEvent::Step(s.into()));
}
fn detail(tx: &UnboundedSender<InstallEvent>, s: impl Into<String>) {
    let _ = tx.send(InstallEvent::Detail(s.into()));
}
fn warning(tx: &UnboundedSender<InstallEvent>, s: impl Into<String>) {
    let _ = tx.send(InstallEvent::Warning(s.into()));
}

#[derive(Debug, Clone)]
pub struct InstallOutcome {
    pub success: bool,
    pub install_dir: PathBuf,
    pub needs_new_terminal: bool,
    pub gui_shortcut_created: bool,
}

/// Runs the whole install sequence. Blocking (network/filesystem I/O
/// throughout) -- callers should run this on a dedicated thread (e.g.
/// `tokio::task::spawn_blocking`) rather than on an async/UI executor.
pub fn run_install(components: Components, tx: UnboundedSender<InstallEvent>) -> InstallOutcome {
    let install_dir = match install_location::resolve() {
        Ok(dir) => dir,
        Err(e) => {
            warning(&tx, format!("Could not determine where to install: {e:#}"));
            return InstallOutcome {
                success: false,
                install_dir: PathBuf::new(),
                needs_new_terminal: false,
                gui_shortcut_created: false,
            };
        }
    };
    if let Err(e) = std::fs::create_dir_all(&install_dir) {
        warning(
            &tx,
            format!("Could not create {}: {e}", install_dir.display()),
        );
        return InstallOutcome {
            success: false,
            install_dir,
            needs_new_terminal: false,
            gui_shortcut_created: false,
        };
    }
    step(&tx, format!("Installing to: {}", install_dir.display()));

    let mut ok = true;

    if components.tui {
        step(&tx, "Terminal version");
        let dest = install_dir.join(app_exe_filename("vidsave-tui"));
        match install_embedded(crate::TUI_BINARY, &dest) {
            Ok(()) => detail(&tx, format!("installed: {}", dest.display())),
            Err(e) => {
                warning(&tx, format!("FAILED: {e:#}"));
                ok = false;
            }
        }
    }

    if components.gui {
        step(&tx, "Desktop app");
        let dest = install_dir.join(app_exe_filename("vidsave"));
        match install_embedded(crate::GUI_BINARY, &dest) {
            Ok(()) => detail(&tx, format!("installed: {}", dest.display())),
            Err(e) => {
                warning(&tx, format!("FAILED: {e:#}"));
                ok = false;
            }
        }
    }

    step(&tx, "yt-dlp (bundled Python runtime + vendored source)");
    match ensure_ytdlp(&install_dir, &tx) {
        Ok(true) => detail(&tx, "already installed"),
        Ok(false) => detail(&tx, "installed"),
        Err(e) => {
            warning(&tx, format!("FAILED: {e:#}"));
            ok = false;
        }
    }

    step(&tx, "ffmpeg");
    match ensure_ffmpeg(&install_dir) {
        Ok(Status::AlreadyOnPath(p)) => {
            detail(&tx, format!("already available on PATH: {}", p.display()))
        }
        Ok(Status::AlreadyInstalled(p)) => {
            detail(&tx, format!("already installed: {}", p.display()))
        }
        Ok(Status::Installed(p)) => detail(&tx, format!("installed: {}", p.display())),
        Err(e) => warning(
            &tx,
            format!("FAILED: {e:#}  (not required -- reduced features without it)"),
        ),
    }

    for tool in Tool::ALL {
        step(&tx, tool.display_name().to_string());
        match ensure_tool(tool, &install_dir) {
            Ok(Status::AlreadyOnPath(p)) => {
                detail(&tx, format!("already available on PATH: {}", p.display()))
            }
            Ok(Status::AlreadyInstalled(p)) => {
                detail(&tx, format!("already installed: {}", p.display()))
            }
            Ok(Status::Installed(p)) => detail(&tx, format!("installed: {}", p.display())),
            Err(e) => warning(
                &tx,
                format!("FAILED: {e:#}  (not required -- reduced features without it)"),
            ),
        }
    }

    step(&tx, "PATH");
    let mut needs_new_terminal = false;
    match path_env::ensure_on_path(&install_dir) {
        Ok(PathAction::AlreadyPresent) => detail(&tx, "already on PATH"),
        Ok(PathAction::Added(where_)) => {
            detail(&tx, format!("added to {where_}"));
            needs_new_terminal = true;
        }
        Err(e) => warning(&tx, format!("could not update PATH automatically: {e:#}")),
    }

    let mut gui_shortcut_created = false;
    if components.gui {
        step(&tx, "Application shortcut");
        let gui_exe = install_dir.join(app_exe_filename("vidsave"));
        // Linux's `.desktop` entry needs an actual icon file to point at
        // (unlike Windows, where the shortcut just points `IconFile` at the
        // exe itself, which already has the icon baked in as a resource --
        // see `build.rs`) -- so drop one next to the app first.
        const ICON_PNG: &[u8] = include_bytes!("../../assets/icon-512.png");
        if let Some(dir) = gui_exe.parent()
            && let Err(e) = std::fs::write(dir.join("icon.png"), ICON_PNG)
        {
            warning(&tx, format!("could not write app icon: {e:#}"));
        }
        match shortcut::create(&gui_exe) {
            Ok(location) => {
                detail(&tx, format!("created: {}", location.display()));
                gui_shortcut_created = true;
            }
            Err(e) => warning(&tx, format!("could not create a shortcut: {e:#}")),
        }
    }

    InstallOutcome {
        success: ok,
        install_dir,
        needs_new_terminal,
        gui_shortcut_created,
    }
}

fn app_exe_filename(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

/// Writes an embedded app binary to `dest`, always overwriting -- running
/// the installer again should always leave you with exactly the version it
/// shipped, not whatever happened to already be there.
fn install_embedded(bytes: &'static [u8], dest: &Path) -> Result<()> {
    std::fs::write(dest, bytes).with_context(|| format!("writing {}", dest.display()))?;
    make_executable(dest)?;
    Ok(())
}

fn runtime_dir(install_dir: &Path) -> PathBuf {
    install_dir.join("python-runtime")
}
fn ytdlp_src_dir(install_dir: &Path) -> PathBuf {
    install_dir.join("yt_dlp_src")
}

/// Installs the bundled Python interpreter (downloaded, unless a working
/// copy from a previous run is already there) and our vendored yt-dlp
/// source (always re-extracted from what's embedded in this installer).
/// Returns `Ok(true)` if the interpreter was already installed.
fn ensure_ytdlp(install_dir: &Path, tx: &UnboundedSender<InstallEvent>) -> Result<bool> {
    let runtime_dir = runtime_dir(install_dir);
    let python_path = python_runtime::python_exe_path(&runtime_dir);
    let src_dir = ytdlp_src_dir(install_dir);

    let already_installed = python_path.is_file() && verify_python(&python_path);

    if !already_installed {
        detail(
            tx,
            format!(
                "downloading Python {} (build {})",
                python_runtime::PYTHON_VERSION,
                python_runtime::RELEASE_TAG
            ),
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

    extract::install_embedded_zip(crate::YTDLP_VENDOR_ZIP, &src_dir)
        .with_context(|| format!("extracting vendored yt-dlp source to {}", src_dir.display()))?;

    if !verify_ytdlp(&python_path, &src_dir) {
        bail!("installed yt-dlp (via the bundled Python runtime) but it did not run correctly");
    }

    Ok(already_installed)
}

fn verify_python(python_path: &Path) -> bool {
    let mut cmd = Command::new(python_path);
    cmd.arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    suppress_console_window(&mut cmd);
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

fn verify_ytdlp(python_path: &Path, src_dir: &Path) -> bool {
    let mut cmd = Command::new(python_path);
    cmd.env("PYTHONPATH", src_dir)
        .args(["-m", "yt_dlp", "--version"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    suppress_console_window(&mut cmd);
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

pub enum Status {
    AlreadyOnPath(PathBuf),
    AlreadyInstalled(PathBuf),
    Installed(PathBuf),
}

/// Installs ffmpeg *and* ffprobe from one download. yt-dlp is given
/// ffmpeg's path via `--ffmpeg-location` and, from that, looks for ffprobe
/// in the very same directory (see yt-dlp's `postprocessor/ffmpeg.py`).
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
        .map(|(n, p)| (n.as_str(), p.as_path()))
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
    let mut cmd = Command::new(path);
    cmd.args(tool.version_args())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    suppress_console_window(&mut cmd);
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

/// Without this, every version-check subprocess the installer runs pops up
/// its own console window on Windows -- the wizard has no console of its
/// own (see main.rs's windows_subsystem attribute), but Windows still
/// creates one by default for a spawned console subprocess unless told not
/// to. CREATE_NO_WINDOW (0x0800_0000) suppresses that.
#[cfg(windows)]
fn suppress_console_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn suppress_console_window(_cmd: &mut Command) {}

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
