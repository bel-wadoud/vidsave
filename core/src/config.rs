//! Persistent, user-editable settings. Loaded from (and saved to) a TOML file
//! in the platform config directory so choices survive between runs.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::models::{AudioFormat, MediaMode, VideoContainer, VideoQuality};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub output_dir: PathBuf,
    pub media_mode: MediaMode,
    pub video_quality: VideoQuality,
    pub video_container: VideoContainer,
    pub audio_format: AudioFormat,
    /// Output filename template, relative to `output_dir`. `{index}`
    /// (rendered by us, not yt-dlp -- see `ytdlp::render_filename_template`)
    /// expands to `"N - "` for playlist entries or nothing for a lone
    /// video; everything else uses yt-dlp's own `%(...)s` fields.
    pub filename_template: String,
    /// Number of simultaneous downloads.
    pub concurrency: usize,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub embed_chapters: bool,
    pub write_subtitles: bool,
    pub embed_subtitles: bool,
    pub write_auto_subs: bool,
    /// Comma-separated language codes, e.g. "en,fr" or "all".
    pub subtitle_langs: String,
    /// Skip items already recorded in the download archive file.
    pub use_download_archive: bool,
    pub playlist_reverse: bool,
    pub playlist_start: Option<u32>,
    pub playlist_end: Option<u32>,
    /// e.g. "2M" / "500K", empty string = unlimited.
    pub rate_limit: String,
    pub retries: u32,
    /// Empty string = no proxy.
    pub proxy: String,
    /// Path to a cookies file (Netscape format), empty = none.
    pub cookies_file: String,
    /// Strip sponsor/self-promo segments via the SponsorBlock API.
    pub sponsorblock_remove: bool,
    /// Free-form extra yt-dlp CLI arguments appended verbatim (space separated).
    pub extra_args: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            media_mode: MediaMode::VideoAudio,
            video_quality: VideoQuality::Best,
            video_container: VideoContainer::Mp4,
            audio_format: AudioFormat::Mp3,
            filename_template: "{index}%(title)s.%(ext)s".to_string(),
            concurrency: 3,
            embed_thumbnail: true,
            embed_metadata: true,
            embed_chapters: false,
            write_subtitles: false,
            embed_subtitles: false,
            write_auto_subs: false,
            subtitle_langs: "en".to_string(),
            // Off by default: a played-list can legitimately list the same
            // video more than once (reposts, re-ordered "best of" lists,
            // etc.), and the least surprising behavior for "download this
            // playlist" is to fetch every listed entry rather than silently
            // dedup by video ID. Power users who want incremental re-runs
            // to skip what they already have can switch this back on.
            use_download_archive: false,
            playlist_reverse: false,
            playlist_start: None,
            playlist_end: None,
            rate_limit: String::new(),
            retries: 10,
            proxy: String::new(),
            cookies_file: String::new(),
            sponsorblock_remove: false,
            extra_args: String::new(),
        }
    }
}

fn default_output_dir() -> PathBuf {
    if let Some(user_dirs) = directories::UserDirs::new() {
        if let Some(video_dir) = user_dirs.video_dir() {
            return video_dir.join("vidsave");
        }
        return user_dirs.home_dir().join("Videos").join("vidsave");
    }
    PathBuf::from("./downloads")
}

impl Settings {
    fn project_dirs() -> Option<directories::ProjectDirs> {
        directories::ProjectDirs::from("dev", "vidsave", "vidsave")
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::project_dirs().map(|d| d.config_dir().join("config.toml"))
    }

    /// The `--download-archive` file used to skip already-downloaded videos.
    ///
    /// Deliberately independent of `output_dir`: it lives in the app's own
    /// data directory rather than inside the user's media folder, so
    /// per-playlist output subfolders (see `App::start_downloads`) don't
    /// accidentally reset or fragment the dedup history, and the download
    /// folder doesn't get a stray dotfile mixed into it.
    pub fn archive_path(&self) -> PathBuf {
        match Self::project_dirs() {
            Some(dirs) => dirs.data_dir().join("download_archive.txt"),
            None => self.output_dir.join(".vidsave-archive.txt"),
        }
    }

    /// Load settings from disk, falling back to defaults if the file is
    /// missing, unreadable, or fails to parse.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()
            .ok_or_else(|| anyhow::anyhow!("could not determine config directory"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }

    pub fn extra_args_list(&self) -> Vec<String> {
        self.extra_args
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }
}
