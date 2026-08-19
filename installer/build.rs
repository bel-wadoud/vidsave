//! Two build-time guards, both feeding `include_bytes!` in `src/main.rs`:
//!
//! 1. Makes sure a freshly built `vidsave-tui` binary for the target platform
//!    is sitting in `embed/` before compiling. Run the app's own build (for
//!    the same target) first -- see `../build-installer.sh`, which does
//!    exactly that.
//! 2. Zips `../vendor/yt_dlp` (our vendored copy of yt-dlp's Python source
//!    -- see `../vendor/update.sh`) into `$OUT_DIR/yt_dlp_src.zip`, which the
//!    installer extracts into the install directory at install time instead
//!    of downloading yt-dlp from anywhere.
//!
//! Windows only, additionally: embeds the app icon (`../assets/icon.ico`)
//! as a resource on the built `.exe`, same as `../gui/build.rs`.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

fn main() {
    if let Err(e) = check_embedded_app_binary("vidsave-tui", "VIDSAVE_TUI_BINARY_PATH") {
        panic!("\n\n{e:#}\n\n");
    }
    if let Err(e) = check_embedded_app_binary("vidsave", "VIDSAVE_GUI_BINARY_PATH") {
        panic!("\n\n{e:#}\n\n");
    }
    if let Err(e) = zip_vendored_ytdlp() {
        panic!("\n\ninstaller build.rs: failed to package vendored yt-dlp: {e:#}\n\n");
    }
    embed_windows_icon();
}

#[cfg(windows)]
fn embed_windows_icon() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../assets/icon.ico");
    if let Err(e) = res.compile() {
        panic!("\n\nfailed to embed the Windows .exe icon: {e:#}\n\n");
    }
}

#[cfg(not(windows))]
fn embed_windows_icon() {}

/// Makes sure a freshly built `{stem}` binary for the target platform is
/// sitting in `embed/`, and exposes its path via `env_var` so `main.rs` can
/// `include_bytes!` it. Called once for the TUI binary and once for the
/// GUI binary -- the installer embeds both, regardless of which one(s) the
/// user ends up choosing on the wizard's Components page, same as any
/// installer that bundles optional components inside one package.
fn check_embedded_app_binary(stem: &str, env_var: &str) -> Result<()> {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    let filename = if target_os == "windows" {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    };
    let path = Path::new(&manifest_dir).join("embed").join(&filename);

    if !path.is_file() {
        bail!(
            "missing {path}\n\n\
             The installer embeds real {stem} binaries at compile time.\n\
             Build the app/gui crates for this same target first, e.g.:\n\
             \n    cargo build --release --target <target-triple>\n\
             \n\
             ... then copy target/<target-triple>/release/{filename} to\n\
             installer/embed/{filename} before building the installer.\n\
             (build-installer.sh at the repo root does all of this in order.)",
            path = path.display(),
            stem = stem,
            filename = filename,
        );
    }

    println!("cargo:rustc-env={env_var}={}", path.display());
    println!("cargo:rerun-if-changed={}", path.display());
    Ok(())
}

fn zip_vendored_ytdlp() -> Result<()> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let vendor_dir = manifest_dir.join("..").join("vendor").join("yt_dlp");

    if !vendor_dir.is_dir() {
        bail!(
            "missing {}\n\n\
             The installer embeds our vendored copy of yt-dlp's source instead of\n\
             downloading yt-dlp from anywhere -- see ../vendor/update.sh.",
            vendor_dir.display()
        );
    }

    let zip_path = out_dir.join("yt_dlp_src.zip");
    let file =
        File::create(&zip_path).with_context(|| format!("creating {}", zip_path.display()))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    add_dir_recursive(&mut writer, &vendor_dir, &vendor_dir, options)
        .with_context(|| format!("zipping {}", vendor_dir.display()))?;
    writer.finish().context("finalizing zip archive")?;

    println!(
        "cargo:rustc-env=YTDLP_VENDOR_ZIP_PATH={}",
        zip_path.display()
    );
    println!("cargo:rerun-if-changed={}", vendor_dir.display());
    Ok(())
}

/// Adds every file under `dir` to `writer`, with archive paths relative to
/// `base` (e.g. `yt_dlp/extractor/youtube.py`, not an absolute host path).
fn add_dir_recursive(
    writer: &mut zip::ZipWriter<File>,
    base: &Path,
    dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            // Skip compiled bytecode caches -- not source, and stale ones
            // from a previous local `python -m yt_dlp` run shouldn't ship.
            if path.file_name().and_then(|n| n.to_str()) == Some("__pycache__") {
                continue;
            }
            add_dir_recursive(writer, base, &path, options)?;
            continue;
        }

        let rel = path
            .strip_prefix(base.parent().unwrap())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        writer.start_file(rel, options)?;
        let mut buf = Vec::new();
        File::open(&path)
            .with_context(|| format!("reading {}", path.display()))?
            .read_to_end(&mut buf)?;
        writer.write_all(&buf)?;
    }
    Ok(())
}
