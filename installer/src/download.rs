//! A small blocking HTTP GET with a printed progress bar. Buffers fully in
//! memory (these downloads top out around ~100MB) so extraction can just
//! wrap the bytes in a `Cursor` rather than juggling temp files.

use std::io::{Read, Write};

use anyhow::{Context, Result, bail};

/// Generous but not unbounded: large enough for the biggest of our
/// archives (ffmpeg's Windows zip) with plenty of headroom, small enough to
/// refuse to stream an unbounded response into memory forever.
const MAX_DOWNLOAD_SIZE: u64 = 300 * 1024 * 1024;

pub fn fetch(url: &str) -> Result<Vec<u8>> {
    let mut response = ureq::get(url)
        .header("User-Agent", "vidsave-install")
        .call()
        .with_context(|| format!("requesting {url}"))?;

    if !response.status().is_success() {
        bail!("server returned HTTP {}", response.status());
    }

    let total = response.body().content_length();
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD_SIZE)
        .reader();

    let mut buf = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    let mut last_pct: i32 = -1;
    loop {
        let n = reader.read(&mut chunk).context("reading response body")?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(total) = total.filter(|t| *t > 0) {
            let pct = ((buf.len() as u64 * 100) / total) as i32;
            if pct != last_pct {
                print!(
                    "\r   {pct:>3}%  ({} / {} KiB)",
                    buf.len() / 1024,
                    total / 1024
                );
                let _ = std::io::stdout().flush();
                last_pct = pct;
            }
        }
    }
    println!();
    Ok(buf)
}
