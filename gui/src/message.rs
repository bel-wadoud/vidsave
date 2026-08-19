use vidsave_core::downloader::DownloadEvent;
use vidsave_core::models::{AudioFormat, MediaMode, PlaylistInfo, VideoContainer, VideoQuality};
use vidsave_core::settings_fields::SettingsField;
use vidsave_core::update_check::UpdateInfo;
use vidsave_core::ytdlp::BinaryStatus;

use crate::state::Tab;

#[derive(Debug, Clone)]
pub enum Message {
    /// The startup tool probe (yt-dlp/ffmpeg/JS runtime) completed.
    ToolsChecked(BinaryStatus),

    /// Switches the active tab -- see `state::Tab`.
    TabSelected(Tab),

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

    // -- Settings tab --
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

    // -- Download history --
    /// A single-video entry opens its detail directly; a playlist/channel
    /// entry opens its video list instead -- see `update::open_history_entry`.
    OpenHistoryEntry(usize),
    OpenHistoryVideo(usize),
    BackFromHistoryPlaylist,
    BackFromHistoryVideoDetail,

    // -- Updates tab --
    /// Fired once automatically shortly after startup, and again any time
    /// "Check for updates" is pressed.
    CheckForUpdates,
    UpdateCheckCompleted(Result<Option<UpdateInfo>, String>),
    InstallUpdatePressed,
    /// `Ok` means the installer was launched successfully -- the app exits
    /// right after (see `update::update`) so the installer can overwrite
    /// files this process currently has open.
    UpdateInstallResult(Result<(), String>),
}
