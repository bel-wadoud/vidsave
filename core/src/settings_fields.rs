//! Declarative description of every row on the Settings screen: how to
//! label it, what kind of control it is (toggle / cycling enum / free text),
//! and how to read/write it on a `Settings` value. Centralizing this here
//! keeps `app.rs` (state transitions) and `ui/settings.rs` (rendering) from
//! duplicating field-specific logic.

use crate::config::Settings;
use crate::models::MediaMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Toggle,
    Cycle,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    OutputDir,
    FilenameTemplate,
    Concurrency,
    MediaMode,
    VideoQuality,
    VideoContainer,
    AudioFormat,
    EmbedThumbnail,
    EmbedMetadata,
    EmbedChapters,
    WriteSubtitles,
    EmbedSubtitles,
    WriteAutoSubs,
    SubtitleLangs,
    UseArchive,
    PlaylistReverse,
    PlaylistStart,
    PlaylistEnd,
    RateLimit,
    Retries,
    Proxy,
    CookiesFile,
    SponsorblockRemove,
    ExtraArgs,
}

impl SettingsField {
    pub const ALL: [SettingsField; 24] = [
        SettingsField::OutputDir,
        SettingsField::FilenameTemplate,
        SettingsField::Concurrency,
        SettingsField::MediaMode,
        SettingsField::VideoQuality,
        SettingsField::VideoContainer,
        SettingsField::AudioFormat,
        SettingsField::EmbedThumbnail,
        SettingsField::EmbedMetadata,
        SettingsField::EmbedChapters,
        SettingsField::WriteSubtitles,
        SettingsField::EmbedSubtitles,
        SettingsField::WriteAutoSubs,
        SettingsField::SubtitleLangs,
        SettingsField::UseArchive,
        SettingsField::PlaylistReverse,
        SettingsField::PlaylistStart,
        SettingsField::PlaylistEnd,
        SettingsField::RateLimit,
        SettingsField::Retries,
        SettingsField::Proxy,
        SettingsField::CookiesFile,
        SettingsField::SponsorblockRemove,
        SettingsField::ExtraArgs,
    ];

    /// Section header to draw above this row, if it starts a new group.
    pub fn section(self) -> Option<&'static str> {
        match self {
            SettingsField::OutputDir => Some("Output"),
            SettingsField::MediaMode => Some("Media"),
            SettingsField::EmbedThumbnail => Some("Embedding"),
            SettingsField::WriteSubtitles => Some("Subtitles"),
            SettingsField::UseArchive => Some("Playlist"),
            SettingsField::RateLimit => Some("Network"),
            SettingsField::ExtraArgs => Some("Advanced"),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SettingsField::OutputDir => "Output directory",
            SettingsField::FilenameTemplate => "Filename template",
            SettingsField::Concurrency => "Concurrent downloads",
            SettingsField::MediaMode => "Media mode",
            SettingsField::VideoQuality => "Video quality cap",
            SettingsField::VideoContainer => "Video container",
            SettingsField::AudioFormat => "Audio format",
            SettingsField::EmbedThumbnail => "Embed thumbnail",
            SettingsField::EmbedMetadata => "Embed metadata",
            SettingsField::EmbedChapters => "Embed chapters",
            SettingsField::WriteSubtitles => "Write subtitles",
            SettingsField::EmbedSubtitles => "Embed subtitles",
            SettingsField::WriteAutoSubs => "Write auto-generated subs",
            SettingsField::SubtitleLangs => "Subtitle languages",
            SettingsField::UseArchive => "Skip already-downloaded (archive)",
            SettingsField::PlaylistReverse => "Reverse playlist order",
            SettingsField::PlaylistStart => "Playlist start index",
            SettingsField::PlaylistEnd => "Playlist end index",
            SettingsField::RateLimit => "Rate limit (e.g. 2M)",
            SettingsField::Retries => "Retries",
            SettingsField::Proxy => "Proxy URL",
            SettingsField::CookiesFile => "Cookies file",
            SettingsField::SponsorblockRemove => "Strip sponsor segments (SponsorBlock)",
            SettingsField::ExtraArgs => "Extra yt-dlp arguments",
        }
    }

    pub fn kind(self) -> FieldKind {
        match self {
            SettingsField::MediaMode
            | SettingsField::VideoQuality
            | SettingsField::VideoContainer
            | SettingsField::AudioFormat => FieldKind::Cycle,

            SettingsField::EmbedThumbnail
            | SettingsField::EmbedMetadata
            | SettingsField::EmbedChapters
            | SettingsField::WriteSubtitles
            | SettingsField::EmbedSubtitles
            | SettingsField::WriteAutoSubs
            | SettingsField::UseArchive
            | SettingsField::PlaylistReverse
            | SettingsField::SponsorblockRemove => FieldKind::Toggle,

            _ => FieldKind::Text,
        }
    }

    /// Current value of a `FieldKind::Toggle` field (meaningless for any
    /// other kind -- returns `false`). Split out from `display_value`
    /// (which renders "On"/"Off" as a string) so a GUI frontend's checkbox
    /// widget can bind to a real `bool` instead of string-matching it back.
    pub fn bool_value(self, s: &Settings) -> bool {
        match self {
            SettingsField::EmbedThumbnail => s.embed_thumbnail,
            SettingsField::EmbedMetadata => s.embed_metadata,
            SettingsField::EmbedChapters => s.embed_chapters,
            SettingsField::WriteSubtitles => s.write_subtitles,
            SettingsField::EmbedSubtitles => s.embed_subtitles,
            SettingsField::WriteAutoSubs => s.write_auto_subs,
            SettingsField::UseArchive => s.use_download_archive,
            SettingsField::PlaylistReverse => s.playlist_reverse,
            SettingsField::SponsorblockRemove => s.sponsorblock_remove,
            _ => false,
        }
    }

    pub fn toggle(self, s: &mut Settings) {
        match self {
            SettingsField::EmbedThumbnail => s.embed_thumbnail = !s.embed_thumbnail,
            SettingsField::EmbedMetadata => s.embed_metadata = !s.embed_metadata,
            SettingsField::EmbedChapters => s.embed_chapters = !s.embed_chapters,
            SettingsField::WriteSubtitles => s.write_subtitles = !s.write_subtitles,
            SettingsField::EmbedSubtitles => s.embed_subtitles = !s.embed_subtitles,
            SettingsField::WriteAutoSubs => s.write_auto_subs = !s.write_auto_subs,
            SettingsField::UseArchive => s.use_download_archive = !s.use_download_archive,
            SettingsField::PlaylistReverse => s.playlist_reverse = !s.playlist_reverse,
            SettingsField::SponsorblockRemove => s.sponsorblock_remove = !s.sponsorblock_remove,
            _ => {}
        }
    }

    /// `dir`: +1 cycles forward, -1 cycles backward.
    pub fn cycle(self, s: &mut Settings, dir: i32) {
        match self {
            SettingsField::MediaMode => s.media_mode = s.media_mode.next(),
            SettingsField::VideoQuality => {
                s.video_quality = if dir >= 0 {
                    s.video_quality.next()
                } else {
                    s.video_quality.prev()
                }
            }
            SettingsField::VideoContainer => s.video_container = s.video_container.next(),
            SettingsField::AudioFormat => s.audio_format = s.audio_format.next(),
            _ => {}
        }
    }

    /// Raw value used to seed the text-edit box when entering edit mode.
    pub fn text_value(self, s: &Settings) -> String {
        match self {
            SettingsField::OutputDir => s.output_dir.to_string_lossy().to_string(),
            SettingsField::FilenameTemplate => s.filename_template.clone(),
            SettingsField::Concurrency => s.concurrency.to_string(),
            SettingsField::SubtitleLangs => s.subtitle_langs.clone(),
            SettingsField::PlaylistStart => {
                s.playlist_start.map(|v| v.to_string()).unwrap_or_default()
            }
            SettingsField::PlaylistEnd => s.playlist_end.map(|v| v.to_string()).unwrap_or_default(),
            SettingsField::RateLimit => s.rate_limit.clone(),
            SettingsField::Retries => s.retries.to_string(),
            SettingsField::Proxy => s.proxy.clone(),
            SettingsField::CookiesFile => s.cookies_file.clone(),
            SettingsField::ExtraArgs => s.extra_args.clone(),
            _ => String::new(),
        }
    }

    pub fn set_text_value(self, s: &mut Settings, value: &str) {
        let trimmed = value.trim();
        match self {
            SettingsField::OutputDir => {
                if !trimmed.is_empty() {
                    s.output_dir = std::path::PathBuf::from(trimmed);
                }
            }
            SettingsField::FilenameTemplate => {
                if !trimmed.is_empty() {
                    s.filename_template = trimmed.to_string();
                }
            }
            SettingsField::Concurrency => {
                if let Ok(n) = trimmed.parse::<usize>() {
                    s.concurrency = n.max(1);
                }
            }
            SettingsField::SubtitleLangs => s.subtitle_langs = trimmed.to_string(),
            SettingsField::PlaylistStart => s.playlist_start = trimmed.parse().ok(),
            SettingsField::PlaylistEnd => s.playlist_end = trimmed.parse().ok(),
            SettingsField::RateLimit => s.rate_limit = trimmed.to_string(),
            SettingsField::Retries => {
                if let Ok(n) = trimmed.parse::<u32>() {
                    s.retries = n;
                }
            }
            SettingsField::Proxy => s.proxy = trimmed.to_string(),
            SettingsField::CookiesFile => s.cookies_file = trimmed.to_string(),
            SettingsField::ExtraArgs => s.extra_args = value.trim().to_string(),
            _ => {}
        }
    }

    /// Human-friendly rendering of the current value for the settings list.
    pub fn display_value(self, s: &Settings) -> String {
        match self {
            SettingsField::MediaMode => s.media_mode.label().to_string(),
            SettingsField::VideoQuality => s.video_quality.label().to_string(),
            SettingsField::VideoContainer => s.video_container.label().to_string(),
            SettingsField::AudioFormat => s.audio_format.label().to_string(),
            SettingsField::EmbedThumbnail => bool_label(s.embed_thumbnail),
            SettingsField::EmbedMetadata => bool_label(s.embed_metadata),
            SettingsField::EmbedChapters => bool_label(s.embed_chapters),
            SettingsField::WriteSubtitles => bool_label(s.write_subtitles),
            SettingsField::EmbedSubtitles => bool_label(s.embed_subtitles),
            SettingsField::WriteAutoSubs => bool_label(s.write_auto_subs),
            SettingsField::UseArchive => bool_label(s.use_download_archive),
            SettingsField::PlaylistReverse => bool_label(s.playlist_reverse),
            SettingsField::SponsorblockRemove => bool_label(s.sponsorblock_remove),
            SettingsField::PlaylistStart => s
                .playlist_start
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(none)".into()),
            SettingsField::PlaylistEnd => s
                .playlist_end
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(none)".into()),
            SettingsField::RateLimit => {
                if s.rate_limit.is_empty() {
                    "unlimited".into()
                } else {
                    s.rate_limit.clone()
                }
            }
            SettingsField::Proxy => {
                if s.proxy.is_empty() {
                    "(none)".into()
                } else {
                    s.proxy.clone()
                }
            }
            SettingsField::CookiesFile => {
                if s.cookies_file.is_empty() {
                    "(none)".into()
                } else {
                    s.cookies_file.clone()
                }
            }
            SettingsField::ExtraArgs => {
                if s.extra_args.is_empty() {
                    "(none)".into()
                } else {
                    s.extra_args.clone()
                }
            }
            _ => self.text_value(s),
        }
    }

    /// Whether this row is meaningful in the given media mode; irrelevant
    /// rows are dimmed rather than hidden so the layout stays stable.
    pub fn relevant_for(self, mode: MediaMode) -> bool {
        match self {
            SettingsField::VideoQuality
            | SettingsField::VideoContainer
            | SettingsField::EmbedChapters => mode == MediaMode::VideoAudio,
            SettingsField::AudioFormat => mode == MediaMode::AudioOnly,
            _ => true,
        }
    }
}

fn bool_label(b: bool) -> String {
    if b {
        "On".to_string()
    } else {
        "Off".to_string()
    }
}
