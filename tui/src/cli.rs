//! Command-line entry points, mainly so the TUI can be launched straight
//! into a specific playlist (`playloader-tui <url>`) instead of always
//! starting at the blank URL prompt.

use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "playloader-tui",
    version,
    about = "Terminal UI for downloading YouTube playlists and videos via yt-dlp"
)]
pub struct Cli {
    /// Playlist, channel, or single video URL to resolve immediately on launch.
    pub url: Option<String>,

    /// Override the configured output directory for this run only.
    #[arg(short = 'o', long)]
    pub output_dir: Option<PathBuf>,
}
