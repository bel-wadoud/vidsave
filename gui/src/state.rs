//! Application state: everything `view` reads and `update` mutates.
//! Navigation is tab-based (`Tab`/`Screen::tab`) -- Download, History,
//! Settings, Updates, and About are always one click away via the top tab
//! bar, rather than Settings being a special overlay and History being
//! squeezed below the URL box like in earlier versions.

use std::collections::HashSet;

use vidsave_core::config::Settings;
use vidsave_core::downloader::DownloadHandle;
use vidsave_core::history::{self, HistoryEntry};
use vidsave_core::models::{DownloadItem, PlaylistInfo};
use vidsave_core::update_check::UpdateInfo;
use vidsave_core::ytdlp::BinaryStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Download,
    History,
    Settings,
    Updates,
    About,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Download,
        Tab::History,
        Tab::Settings,
        Tab::Updates,
        Tab::About,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Download => "Download",
            Tab::History => "History",
            Tab::Settings => "Settings",
            Tab::Updates => "Updates",
            Tab::About => "About",
        }
    }

    /// The screen shown when this tab is first selected.
    fn home_screen(self) -> Screen {
        match self {
            Tab::Download => Screen::UrlInput,
            Tab::History => Screen::HistoryList,
            Tab::Settings => Screen::Settings,
            Tab::Updates => Screen::Updates,
            Tab::About => Screen::About,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    UrlInput,
    VideoList,
    Downloading,
    HistoryList,
    /// Drilled into one playlist/channel history entry -- see
    /// `State::history_open`.
    HistoryPlaylist,
    /// One video's recorded outcome, reached either from `HistoryPlaylist`
    /// or straight from `HistoryList` for a single-video entry.
    HistoryVideoDetail,
    Settings,
    Updates,
    About,
}

impl Screen {
    pub fn tab(self) -> Tab {
        match self {
            Screen::UrlInput | Screen::VideoList | Screen::Downloading => Tab::Download,
            Screen::HistoryList | Screen::HistoryPlaylist | Screen::HistoryVideoDetail => {
                Tab::History
            }
            Screen::Settings => Tab::Settings,
            Screen::Updates => Tab::Updates,
            Screen::About => Tab::About,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Error,
}

pub struct StatusMessage {
    pub text: String,
    pub kind: StatusKind,
}

/// Where a check for updates currently stands -- drives the Updates tab
/// (and a startup notification, see `notify.rs`) without needing separate
/// booleans that could disagree with each other.
#[derive(Default)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(UpdateInfo),
    Installing,
    Error(String),
}

pub struct State {
    pub settings: Settings,
    pub binary_status: BinaryStatus,
    /// Distinguishes "haven't checked yet" from "checked, and it's
    /// genuinely missing" -- without this, the UI would flash a scary
    /// "not found" error for the brief moment at startup before the real
    /// check (an async Task) has had a chance to complete.
    pub tools_checked: bool,
    pub screen: Screen,
    pub status: Option<StatusMessage>,
    pub settings_saved_flash: bool,

    // -- URL input / fetching --
    pub url_input: String,
    pub fetching: bool,
    pub pending_url: String,
    /// Set from the CLI's positional URL argument; fetched automatically
    /// once the initial tool check confirms yt-dlp is ready.
    pub initial_url: Option<String>,

    // -- Video list --
    pub playlist: Option<PlaylistInfo>,
    pub selected: Vec<bool>,
    pub filter: String,

    // -- Downloading --
    pub items: Vec<DownloadItem>,
    pub download_handle: Option<DownloadHandle>,
    /// Which queue rows currently have their raw yt-dlp log expanded --
    /// collapsed (just clean status text) by default, since a normal user
    /// cares about progress, not yt-dlp's internal output.
    pub expanded_items: HashSet<usize>,
    pub batch_done: bool,

    // -- Download history --
    pub history: Vec<HistoryEntry>,
    /// Index into `history` of the entry currently drilled into.
    pub history_open: Option<usize>,
    /// Index into `history[history_open].videos` currently shown on
    /// `HistoryVideoDetail`.
    pub history_video_open: Option<usize>,

    // -- Updates --
    pub update_status: UpdateStatus,
}

impl State {
    pub fn new(settings: Settings, initial_url: Option<String>) -> Self {
        Self {
            settings,
            binary_status: BinaryStatus::default(),
            tools_checked: false,
            screen: Screen::UrlInput,
            status: None,
            settings_saved_flash: false,
            url_input: initial_url.clone().unwrap_or_default(),
            fetching: false,
            pending_url: String::new(),
            initial_url,
            playlist: None,
            selected: Vec::new(),
            filter: String::new(),
            items: Vec::new(),
            download_handle: None,
            expanded_items: HashSet::new(),
            batch_done: false,
            history: history::load_history(),
            history_open: None,
            history_video_open: None,
            update_status: UpdateStatus::default(),
        }
    }

    pub fn set_status(&mut self, text: impl Into<String>, kind: StatusKind) {
        self.status = Some(StatusMessage {
            text: text.into(),
            kind,
        });
    }

    /// Switches to `tab`'s home screen -- e.g. re-selecting a tab you're
    /// already mid-flow on (say, Download while looking at the video list)
    /// resets back to that tab's starting point, same as clicking a tab in
    /// any other app returns you to its top level.
    pub fn switch_tab(&mut self, tab: Tab) {
        if tab == Tab::Settings {
            self.settings_saved_flash = false;
        }
        self.screen = tab.home_screen();
    }

    /// Indices into `playlist.videos` matching the current filter text
    /// (case-insensitive substring match against title/uploader), or every
    /// index if there's no filter -- same semantics as the TUI's
    /// `App::filtered_video_indices`.
    pub fn filtered_video_indices(&self) -> Vec<usize> {
        let Some(playlist) = &self.playlist else {
            return Vec::new();
        };
        let needle = self.filter.trim().to_lowercase();
        if needle.is_empty() {
            return (0..playlist.videos.len()).collect();
        }
        playlist
            .videos
            .iter()
            .enumerate()
            .filter(|(_, v)| {
                v.title.to_lowercase().contains(&needle)
                    || v.uploader
                        .as_deref()
                        .is_some_and(|u| u.to_lowercase().contains(&needle))
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn toggle_selected(&mut self, index: usize) {
        if let Some(sel) = self.selected.get_mut(index) {
            *sel = !*sel;
        }
    }

    pub fn set_filtered_selection(&mut self, value: bool) {
        for index in self.filtered_video_indices() {
            if let Some(sel) = self.selected.get_mut(index) {
                *sel = value;
            }
        }
    }

    pub fn invert_filtered_selection(&mut self) {
        for index in self.filtered_video_indices() {
            if let Some(sel) = self.selected.get_mut(index) {
                *sel = !*sel;
            }
        }
    }

    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|s| **s).count()
    }

    pub fn current_history_entry(&self) -> Option<&HistoryEntry> {
        self.history.get(self.history_open?)
    }

    pub fn current_history_video(&self) -> Option<&history::HistoryVideoEntry> {
        self.current_history_entry()?
            .videos
            .get(self.history_video_open?)
    }
}
