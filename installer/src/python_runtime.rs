//! Fetches a portable, redistributable CPython build instead of relying on
//! whatever Python the user may or may not already have. We pair it with a
//! specific pinned copy of yt-dlp (see `../vendor/`), so we need a known,
//! consistent interpreter -- not just "some `python3` that happens to be on
//! PATH" -- to keep that pairing actually reliable. Builds come from
//! astral-sh's python-build-standalone project (PSF-licensed, redistributable,
//! the same builds tools like `uv` bundle for this exact purpose).

use std::path::{Path, PathBuf};

/// The python-build-standalone release to fetch. There's no "latest" URL
/// alias for this project, so this is pinned explicitly -- bump deliberately
/// at https://github.com/astral-sh/python-build-standalone/releases.
pub const RELEASE_TAG: &str = "20260814";
pub const PYTHON_VERSION: &str = "3.12.14";

pub fn download_url() -> &'static str {
    if cfg!(windows) {
        "https://github.com/astral-sh/python-build-standalone/releases/download/20260814/cpython-3.12.14+20260814-x86_64-pc-windows-msvc-install_only_stripped.tar.gz"
    } else {
        "https://github.com/astral-sh/python-build-standalone/releases/download/20260814/cpython-3.12.14+20260814-x86_64-unknown-linux-gnu-install_only_stripped.tar.gz"
    }
}

/// Where the interpreter ends up inside `runtime_dir` (the directory
/// `extract::install_dir_from_archive` unpacks the download into, with the
/// archive's single top-level `python/` wrapper already stripped).
pub fn python_exe_path(runtime_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        runtime_dir.join("python.exe")
    } else {
        runtime_dir.join("bin").join("python3")
    }
}
