//! Command-line entry points, mirroring the TUI's `Cli` (see `tui/src/cli.rs`):
//! launch straight into a playlist, and/or override the output directory for
//! this run only.

use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "ytb-dl-tui-gui",
    version,
    about = "Desktop GUI for downloading YouTube playlists and videos via yt-dlp"
)]
pub struct Cli {
    /// Playlist, channel, or single video URL to resolve immediately on launch.
    pub url: Option<String>,

    /// Override the configured output directory for this run only.
    #[arg(short = 'o', long)]
    pub output_dir: Option<PathBuf>,
}
