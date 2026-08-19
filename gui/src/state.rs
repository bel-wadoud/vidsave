//! Application state: everything `view` reads and `update` mutates. Mirrors
//! the TUI's `app.rs` state machine (same screens, same settings-origin
//! concept for where the Settings panel returns to) but adapted for a
//! desktop layout: Settings is a panel you open/close over whichever screen
//! you were on, rather than a distinct screen of its own, and video
//! selection/downloads are driven by clicks rather than a cursor.

use std::collections::HashSet;

use vidsave_core::config::Settings;
use vidsave_core::downloader::DownloadHandle;
use vidsave_core::history::{self, HistoryEntry};
use vidsave_core::models::{DownloadItem, PlaylistInfo};
use vidsave_core::ytdlp::BinaryStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    UrlInput,
    VideoList,
    Downloading,
    /// Drilled into one playlist/channel history entry -- see
    /// `State::history_open`.
    HistoryPlaylist,
    /// One video's recorded outcome, reached either from `HistoryPlaylist`
    /// or straight from `UrlInput` for a single-video entry.
    HistoryVideoDetail,
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

    // -- Settings panel: an overlay-ish toggle rather than its own screen
    // -- (unlike the TUI's `SettingsOrigin`-tracked separate Screen), so
    // -- closing it just returns to whatever `screen` already was -- opening
    // -- it never touches `screen` in the first place.
    pub show_settings: bool,
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

    // -- Download history: a list on the URL input screen, drilling into
    // -- HistoryPlaylist / HistoryVideoDetail -- see `history.rs`.
    pub history: Vec<HistoryEntry>,
    /// Index into `history` of the entry currently drilled into.
    pub history_open: Option<usize>,
    /// Index into `history[history_open].videos` currently shown on
    /// `HistoryVideoDetail`.
    pub history_video_open: Option<usize>,
}

impl State {
    pub fn new(settings: Settings, initial_url: Option<String>) -> Self {
        Self {
            settings,
            binary_status: BinaryStatus::default(),
            tools_checked: false,
            screen: Screen::UrlInput,
            status: None,
            show_settings: false,
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
        }
    }

    pub fn set_status(&mut self, text: impl Into<String>, kind: StatusKind) {
        self.status = Some(StatusMessage {
            text: text.into(),
            kind,
        });
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
