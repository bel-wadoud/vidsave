//! Archive handling for the tools that don't ship as a bare executable.
//!
//! Two shapes are needed:
//! - `install_from_archive`: the ffmpeg and deno release archives bury the
//!   binary we actually want inside a versioned subfolder alongside other
//!   files (docs, ffprobe, ffplay, licenses...). Rather than hardcoding
//!   those paths -- which shift whenever the upstream version changes -- we
//!   extract the whole archive to a scratch directory and search it for a
//!   file named exactly like the executable we're after, preferring the
//!   largest match if there's more than one (a real binary will always
//!   dwarf a stray text file that happens to share a name).
//! - `install_dir_from_archive`: the portable Python build is a whole
//!   directory tree (interpreter + standard library), not one file, so we
//!   unpack everything rather than cherry-picking a single name.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Extracts `bytes` (a .zip or .tar.xz, inferred from `source_url`'s
/// extension) and copies whichever contained file is named `target_name`
/// to `dest`.
pub fn install_from_archive(
    bytes: &[u8],
    source_url: &str,
    target_name: &str,
    dest: &Path,
) -> Result<()> {
    install_many_from_archive(bytes, source_url, &[(target_name, dest)])
}

/// Like `install_from_archive`, but pulls more than one named file out of a
/// single downloaded archive -- e.g. ffmpeg's release tarball bundles both
/// `ffmpeg` and `ffprobe`; yt-dlp needs both to be findable in the same
/// directory (see `--ffmpeg-location` in yt-dlp's own docs), so a single
/// download should install both rather than fetching the same archive
/// twice for two different filenames.
pub fn install_many_from_archive(
    bytes: &[u8],
    source_url: &str,
    targets: &[(&str, &Path)],
) -> Result<()> {
    let Some((_, first_dest)) = targets.first() else {
        return Ok(());
    };
    with_scratch_dir(first_dest, |scratch| {
        extract_archive(bytes, source_url, scratch)?;
        for (name, dest) in targets {
            let found = find_file_by_name(scratch, name).with_context(|| {
                format!("could not find a file named '{name}' inside the downloaded archive")
            })?;
            std::fs::copy(&found, dest)
                .with_context(|| format!("copying {} to {}", found.display(), dest.display()))?;
        }
        Ok(())
    })
}

/// Extracts `bytes` in full into `dest_dir`, first stripping a single
/// top-level wrapper directory if the archive has exactly one (the shape
/// python-build-standalone's releases use: everything under one `python/`
/// folder). `dest_dir` is replaced atomically-ish: built up in a scratch
/// location, then swapped in, so a failed/interrupted install can't leave a
/// half-extracted runtime behind that a later run mistakes for a good one.
pub fn install_dir_from_archive(bytes: &[u8], source_url: &str, dest_dir: &Path) -> Result<()> {
    with_scratch_dir(dest_dir, |scratch| {
        extract_archive(bytes, source_url, scratch)?;

        let root = match single_subdirectory(scratch)? {
            Some(only_dir) => only_dir,
            None => scratch.to_path_buf(),
        };

        let _ = std::fs::remove_dir_all(dest_dir);
        if let Some(parent) = dest_dir.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::rename(&root, dest_dir).with_context(|| {
            format!(
                "moving extracted {} to {}",
                root.display(),
                dest_dir.display()
            )
        })?;
        Ok(())
    })
}

/// Runs `f` with a fresh scratch directory (derived from `dest`, so retries
/// for different tools don't collide), cleaning it up afterward either way.
fn with_scratch_dir(dest: &Path, f: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
    let scratch = dest.with_extension("extract-tmp");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch)
        .with_context(|| format!("creating scratch directory {}", scratch.display()))?;

    let result = f(&scratch);

    let _ = std::fs::remove_dir_all(&scratch);
    result
}

fn extract_archive(bytes: &[u8], source_url: &str, dest_dir: &Path) -> Result<()> {
    if source_url.ends_with(".zip") {
        extract_zip(bytes, dest_dir)
    } else if source_url.ends_with(".tar.xz") || source_url.ends_with(".xz") {
        extract_tar_xz(bytes, dest_dir)
    } else if source_url.ends_with(".tar.gz") || source_url.ends_with(".tgz") {
        extract_tar_gz(bytes, dest_dir)
    } else {
        bail!("don't know how to extract {source_url} (unrecognized extension)")
    }
}

fn extract_zip(bytes: &[u8], dest_dir: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("reading zip archive")?;
    archive
        .extract(dest_dir)
        .context("extracting zip archive")?;
    Ok(())
}

#[cfg(unix)]
fn extract_tar_xz(bytes: &[u8], dest_dir: &Path) -> Result<()> {
    let mut decompressed = Vec::new();
    let mut reader = std::io::BufReader::new(Cursor::new(bytes));
    lzma_rs::xz_decompress(&mut reader, &mut decompressed)
        .map_err(|e| anyhow::anyhow!("xz decompression failed: {e}"))?;
    let mut archive = tar::Archive::new(Cursor::new(decompressed));
    archive.unpack(dest_dir).context("extracting tar archive")?;
    Ok(())
}

#[cfg(windows)]
fn extract_tar_xz(_bytes: &[u8], _dest_dir: &Path) -> Result<()> {
    bail!("tar.xz archives are not expected on Windows")
}

fn extract_tar_gz(bytes: &[u8], dest_dir: &Path) -> Result<()> {
    let decoder = flate2::read::GzDecoder::new(Cursor::new(bytes));
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(dest_dir)
        .context("extracting tar.gz archive")?;
    Ok(())
}

/// If `dir` contains exactly one entry and it's a directory, returns it.
fn single_subdirectory(dir: &Path) -> Result<Option<PathBuf>> {
    let mut entries = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    if entries.len() != 1 {
        return Ok(None);
    }
    let only = entries.remove(0);
    if only.file_type()?.is_dir() {
        Ok(Some(only.path()))
    } else {
        Ok(None)
    }
}

fn find_file_by_name(root: &Path, name: &str) -> Option<PathBuf> {
    let mut best: Option<(PathBuf, u64)> = None;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if path.file_name().and_then(|n| n.to_str()) != Some(name) {
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if best.as_ref().is_none_or(|(_, best_size)| size > *best_size) {
                best = Some((path, size));
            }
        }
    }
    best.map(|(path, _)| path)
}

/// Extracts an in-memory zip (the embedded vendored yt-dlp source, or
/// anything else we already have as bytes rather than a downloaded archive)
/// straight into `dest_dir`, replacing whatever was there.
pub fn install_embedded_zip(bytes: &[u8], dest_dir: &Path) -> Result<()> {
    let _ = std::fs::remove_dir_all(dest_dir);
    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("creating {}", dest_dir.display()))?;
    extract_zip(bytes, dest_dir)
}
