//! Archive handling for the tools that don't ship as a bare executable.
//!
//! Both the ffmpeg and deno release archives bury the binary we actually
//! want inside a versioned subfolder alongside other files (docs, ffprobe,
//! ffplay, licenses...). Rather than hardcoding those paths -- which shift
//! whenever the upstream version changes -- we extract the whole archive
//! to a scratch directory and search it for a file named exactly like the
//! executable we're after, preferring the largest match if there's more
//! than one (a real binary will always dwarf a stray text file that
//! happens to share a name).

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
    let scratch = dest.with_extension("extract-tmp");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch)
        .with_context(|| format!("creating scratch directory {}", scratch.display()))?;

    let result = (|| -> Result<()> {
        if source_url.ends_with(".zip") {
            extract_zip(bytes, &scratch)?;
        } else if source_url.ends_with(".tar.xz") || source_url.ends_with(".xz") {
            extract_tar_xz(bytes, &scratch)?;
        } else {
            bail!("don't know how to extract {source_url} (unrecognized extension)");
        }

        let found = find_file_by_name(&scratch, target_name).with_context(|| {
            format!("could not find a file named '{target_name}' inside the downloaded archive")
        })?;
        std::fs::copy(&found, dest)
            .with_context(|| format!("copying {} to {}", found.display(), dest.display()))?;
        Ok(())
    })();

    let _ = std::fs::remove_dir_all(&scratch);
    result
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
