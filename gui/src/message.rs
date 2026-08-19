use vidsave_core::downloader::DownloadEvent;
use vidsave_core::models::{AudioFormat, MediaMode, PlaylistInfo, VideoContainer, VideoQuality};
use vidsave_core::settings_fields::SettingsField;
use vidsave_core::ytdlp::BinaryStatus;

#[derive(Debug, Clone)]
pub enum Message {
    /// The startup tool probe (yt-dlp/ffmpeg/JS runtime) completed.
    ToolsChecked(BinaryStatus),

    // -- URL input --
    UrlChanged(String),
    FetchPressed,
    FetchCompleted(Result<PlaylistInfo, String>),

    // -- Video list --
    ToggleVideo(usize),
    SelectAll,
    SelectNone,
    InvertSelection,
    FilterChanged(String),
    BackToUrlInput,
    StartDownloadsPressed,

    // -- Settings panel --
    OpenSettings,
    CloseSettings,
    SettingsToggled(SettingsField),
    SettingsTextChanged(SettingsField, String),
    MediaModePicked(MediaMode),
    VideoQualityPicked(VideoQuality),
    VideoContainerPicked(VideoContainer),
    AudioFormatPicked(AudioFormat),
    SaveSettingsPressed,

    // -- Downloading --
    DownloadEvent(DownloadEvent),
    PauseItem(usize),
    ResumeItem(usize),
    CancelItem(usize),
    CancelAllPressed,
    /// Per-item "show the raw log" toggle -- off by default, since a normal
    /// user just wants to see progress, not yt-dlp's internal output.
    ToggleItemDetails(usize),
    BackToVideoList,
    StartOverPressed,
}
