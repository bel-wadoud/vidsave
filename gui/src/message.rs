use ytb_dl_tui_core::downloader::DownloadEvent;
use ytb_dl_tui_core::models::{AudioFormat, MediaMode, PlaylistInfo, VideoContainer, VideoQuality};
use ytb_dl_tui_core::settings_fields::SettingsField;
use ytb_dl_tui_core::ytdlp::BinaryStatus;

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
    SelectQueueItem(usize),
    CancelItem(usize),
    CancelAllPressed,
    BackToVideoList,
    StartOverPressed,
}
