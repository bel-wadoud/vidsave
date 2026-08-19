//! Core data types shared across the app: playlist/video metadata, user-facing
//! enums for download options, and the state machine for an in-flight download.

use serde::{Deserialize, Serialize};

/// Whether to grab full video+audio or extract audio only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaMode {
    VideoAudio,
    AudioOnly,
}

impl MediaMode {
    pub fn label(self) -> &'static str {
        match self {
            MediaMode::VideoAudio => "Video + Audio",
            MediaMode::AudioOnly => "Audio only",
        }
    }

    pub fn next(self) -> Self {
        match self {
            MediaMode::VideoAudio => MediaMode::AudioOnly,
            MediaMode::AudioOnly => MediaMode::VideoAudio,
        }
    }
}

impl std::fmt::Display for MediaMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Target vertical resolution cap for video downloads (`Best` = no cap).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoQuality {
    Best,
    P2160,
    P1440,
    P1080,
    P720,
    P480,
    P360,
    Worst,
}

impl VideoQuality {
    pub const ALL: [VideoQuality; 8] = [
        VideoQuality::Best,
        VideoQuality::P2160,
        VideoQuality::P1440,
        VideoQuality::P1080,
        VideoQuality::P720,
        VideoQuality::P480,
        VideoQuality::P360,
        VideoQuality::Worst,
    ];

    pub fn label(self) -> &'static str {
        match self {
            VideoQuality::Best => "Best available",
            VideoQuality::P2160 => "2160p (4K)",
            VideoQuality::P1440 => "1440p (2K)",
            VideoQuality::P1080 => "1080p",
            VideoQuality::P720 => "720p",
            VideoQuality::P480 => "480p",
            VideoQuality::P360 => "360p",
            VideoQuality::Worst => "Worst available",
        }
    }

    pub fn height_cap(self) -> Option<u32> {
        match self {
            VideoQuality::Best | VideoQuality::Worst => None,
            VideoQuality::P2160 => Some(2160),
            VideoQuality::P1440 => Some(1440),
            VideoQuality::P1080 => Some(1080),
            VideoQuality::P720 => Some(720),
            VideoQuality::P480 => Some(480),
            VideoQuality::P360 => Some(360),
        }
    }

    pub fn next(self) -> Self {
        let all = Self::ALL;
        let idx = all.iter().position(|q| *q == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub fn prev(self) -> Self {
        let all = Self::ALL;
        let idx = all.iter().position(|q| *q == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
}

impl std::fmt::Display for VideoQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Output container for combined video+audio downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoContainer {
    Mp4,
    Mkv,
    Webm,
}

impl VideoContainer {
    pub const ALL: [VideoContainer; 3] = [
        VideoContainer::Mp4,
        VideoContainer::Mkv,
        VideoContainer::Webm,
    ];

    pub fn label(self) -> &'static str {
        match self {
            VideoContainer::Mp4 => "MP4",
            VideoContainer::Mkv => "MKV",
            VideoContainer::Webm => "WebM",
        }
    }

    pub fn ytdlp_name(self) -> &'static str {
        match self {
            VideoContainer::Mp4 => "mp4",
            VideoContainer::Mkv => "mkv",
            VideoContainer::Webm => "webm",
        }
    }

    pub fn next(self) -> Self {
        let all = Self::ALL;
        let idx = all.iter().position(|q| *q == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }
}

impl std::fmt::Display for VideoContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Output codec/container for audio-only extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    Mp3,
    M4a,
    Opus,
    Flac,
    Wav,
    Best,
}

impl AudioFormat {
    pub const ALL: [AudioFormat; 6] = [
        AudioFormat::Mp3,
        AudioFormat::M4a,
        AudioFormat::Opus,
        AudioFormat::Flac,
        AudioFormat::Wav,
        AudioFormat::Best,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "MP3",
            AudioFormat::M4a => "M4A (AAC)",
            AudioFormat::Opus => "Opus",
            AudioFormat::Flac => "FLAC",
            AudioFormat::Wav => "WAV",
            AudioFormat::Best => "Best (source codec)",
        }
    }

    /// Value passed to yt-dlp's `--audio-format`, if any (`Best` keeps source codec).
    pub fn ytdlp_name(self) -> Option<&'static str> {
        match self {
            AudioFormat::Mp3 => Some("mp3"),
            AudioFormat::M4a => Some("m4a"),
            AudioFormat::Opus => Some("opus"),
            AudioFormat::Flac => Some("flac"),
            AudioFormat::Wav => Some("wav"),
            AudioFormat::Best => None,
        }
    }

    pub fn next(self) -> Self {
        let all = Self::ALL;
        let idx = all.iter().position(|q| *q == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// A single entry discovered while resolving a playlist (or a lone video URL).
#[derive(Debug, Clone)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration_secs: Option<u64>,
    /// 1-based position within the source playlist, if any.
    pub playlist_index: Option<u64>,
    pub url: String,
}

impl Video {
    pub fn duration_label(&self) -> String {
        match self.duration_secs {
            Some(s) => format_duration(s),
            None => "--:--".to_string(),
        }
    }
}

pub fn format_duration(total_secs: u64) -> String {
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Metadata about the playlist (or single-video pseudo-playlist) that was resolved.
#[derive(Debug, Clone)]
pub struct PlaylistInfo {
    pub title: String,
    pub uploader: Option<String>,
    pub videos: Vec<Video>,
    /// True for an actual playlist/channel (yt-dlp reported an `entries`
    /// list, even if it only had one item); false for a lone video URL.
    /// Drives whether downloads land in a `title`-named subfolder.
    pub is_playlist: bool,
}

/// Makes `name` safe to use as a single path component (a directory or file
/// name, not a full path) on Windows/macOS/Linux: strips characters that are
/// reserved on any of them, trims trailing dots/spaces (invalid on
/// Windows), caps the length, and dodges Windows' reserved device names.
pub fn sanitize_path_component(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();

    let mut cleaned = replaced.trim().to_string();
    while matches!(cleaned.chars().last(), Some('.') | Some(' ')) {
        cleaned.pop();
    }

    const MAX_LEN: usize = 150;
    let cleaned: String = cleaned.chars().take(MAX_LEN).collect();
    let cleaned = cleaned.trim().to_string();

    // A name made entirely of reserved characters (e.g. "///") sanitizes to
    // a string of underscores rather than an empty one -- still meaningless
    // as a folder name, so treat it the same as truly empty input.
    if cleaned.is_empty() || cleaned.chars().all(|c| c == '_') {
        return "playlist".to_string();
    }

    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(&cleaned)) {
        format!("_{cleaned}")
    } else {
        cleaned
    }
}

/// Lifecycle state of one queued download, driven by parsed yt-dlp progress output.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum DownloadState {
    #[default]
    Queued,
    Starting,
    Downloading(DownloadProgress),
    PostProcessing,
    /// Stopped by the user (not by an error) -- resumable via
    /// `DownloadHandle::resume_item`, which picks up from yt-dlp's own
    /// partial-file continuation rather than starting over. Deliberately
    /// *not* terminal (see `is_terminal`): the batch isn't "done" while
    /// something in it is just waiting for the user to resume it.
    Paused,
    Done,
    Skipped,
    Cancelled,
    Failed(String),
}

impl DownloadState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            DownloadState::Done
                | DownloadState::Skipped
                | DownloadState::Cancelled
                | DownloadState::Failed(_)
        )
    }

    pub fn label(&self) -> String {
        match self {
            DownloadState::Queued => "Queued".to_string(),
            DownloadState::Starting => "Starting".to_string(),
            DownloadState::Downloading(p) => format!("{:>5.1}%", p.percent),
            DownloadState::PostProcessing => "Processing".to_string(),
            DownloadState::Paused => "Paused".to_string(),
            DownloadState::Done => "Done".to_string(),
            DownloadState::Skipped => "Skipped".to_string(),
            DownloadState::Cancelled => "Cancelled".to_string(),
            DownloadState::Failed(_) => "Failed".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DownloadProgress {
    pub percent: f32,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
}

/// One row in the download queue/progress screen: a video plus its live state and log tail.
#[derive(Debug, Clone)]
pub struct DownloadItem {
    pub video: Video,
    pub state: DownloadState,
    pub log: Vec<String>,
    /// The most recent `DownloadProgress` seen for this item, kept even
    /// after `state` moves on to something that doesn't carry one itself
    /// (`Paused`, `PostProcessing`, ...) -- so a progress bar has something
    /// sensible to show ("last known position") instead of resetting to 0
    /// the moment a download is paused.
    pub last_progress: Option<DownloadProgress>,
}

impl DownloadItem {
    pub fn new(video: Video) -> Self {
        Self {
            video,
            state: DownloadState::Queued,
            log: Vec::new(),
            last_progress: None,
        }
    }

    /// Updates `state`, additionally remembering the progress if this state
    /// carries one -- see `last_progress`'s doc comment.
    pub fn set_state(&mut self, state: DownloadState) {
        if let DownloadState::Downloading(progress) = &state {
            self.last_progress = Some(progress.clone());
        }
        self.state = state;
    }

    pub fn push_log(&mut self, line: String) {
        const MAX_LOG_LINES: usize = 200;
        self.log.push(line);
        if self.log.len() > MAX_LOG_LINES {
            let excess = self.log.len() - MAX_LOG_LINES;
            self.log.drain(0..excess);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_reserved_characters() {
        assert_eq!(
            sanitize_path_component("Best of: Cats/Dogs? \"Vol 1\""),
            "Best of_ Cats_Dogs_ _Vol 1_"
        );
    }

    #[test]
    fn sanitize_trims_trailing_dots_and_spaces() {
        assert_eq!(sanitize_path_component("My Playlist. . "), "My Playlist");
    }

    #[test]
    fn sanitize_avoids_reserved_windows_names() {
        assert_eq!(sanitize_path_component("con"), "_con");
        assert_eq!(sanitize_path_component("COM1"), "_COM1");
    }

    #[test]
    fn sanitize_falls_back_when_empty() {
        assert_eq!(sanitize_path_component("///"), "playlist");
        assert_eq!(sanitize_path_component("   "), "playlist");
    }

    #[test]
    fn sanitize_caps_length() {
        let long = "a".repeat(500);
        assert_eq!(sanitize_path_component(&long).chars().count(), 150);
    }
}
