//! Application state machine: the `App` struct holds everything needed to
//! render the current screen and react to input; screens are advanced by
//! `App::on_key` (keyboard) and `App::on_tick` / event-channel handlers
//! (background work completing).

use std::time::Instant;

use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use tokio::sync::mpsc;

use vidsave_core::config::Settings;
use vidsave_core::downloader::{self, DownloadEvent, DownloadHandle};
use vidsave_core::history::{self, HistoryEntry};
use vidsave_core::models::{DownloadItem, DownloadState, PlaylistInfo, Video};
use vidsave_core::settings_fields::{FieldKind, SettingsField};
use vidsave_core::ytdlp::BinaryStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    UrlInput,
    Fetching,
    VideoList,
    Settings,
    Downloading,
    /// Drilled into one playlist/channel history entry, showing its videos
    /// -- see `App::open_history_entry`.
    HistoryPlaylist,
    /// One video's recorded outcome -- reached either directly from
    /// `HistoryPlaylist` -> a video, or straight from `UrlInput` for a
    /// single-video history entry (no point showing a list of one).
    HistoryVideoDetail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Info,
    Error,
}

pub struct StatusMessage {
    pub text: String,
    pub kind: MessageKind,
}

/// Which screen a `Settings` visit was entered from, so `Esc`/save returns
/// the user to the right place (initial setup vs. mid-flow adjustment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsOrigin {
    UrlInput,
    VideoList,
}

/// A batch's cancellation handle and its event receiver, regrouped for our
/// own convenience -- `core::downloader::start` returns them separately
/// (see its doc comment) since the GUI frontend wants to hand the receiver
/// off to its async runtime whole, but the TUI's own event loop is happy
/// polling both off one place.
pub struct DownloadSession {
    pub handle: DownloadHandle,
    pub events: mpsc::UnboundedReceiver<DownloadEvent>,
}

pub struct App {
    pub settings: Settings,
    pub binary_status: BinaryStatus,
    pub screen: Screen,
    pub should_quit: bool,
    pub show_help: bool,
    pub status: Option<StatusMessage>,

    // -- URL input screen --
    pub url_input: Input,
    /// Recent download batches, newest first -- shown as a list below the
    /// URL input box. Loaded once at startup and appended to as batches
    /// finish; see `push_history_entry`.
    pub history: Vec<HistoryEntry>,
    /// Whether Up/Down/Enter on the URL input screen act on `history`
    /// (toggled with `Tab`) rather than the URL text box.
    pub history_focused: bool,
    pub history_cursor: usize,
    /// Index into `history` of the entry currently drilled into, on either
    /// `HistoryPlaylist` or `HistoryVideoDetail`.
    pub history_open: Option<usize>,
    pub history_video_cursor: usize,

    // -- Fetching screen --
    pub fetch_rx: Option<tokio::sync::oneshot::Receiver<anyhow::Result<PlaylistInfo>>>,
    pub fetch_spinner_frame: usize,
    pub pending_url: String,

    // -- Video list screen --
    pub playlist: Option<PlaylistInfo>,
    pub selected: Vec<bool>,
    pub video_cursor: usize,
    pub filtering: bool,
    pub filter_input: Input,

    // -- Settings screen --
    pub settings_origin: SettingsOrigin,
    pub settings_cursor: usize,
    pub editing: bool,
    pub edit_input: Input,

    // -- Downloading screen --
    pub downloader: Option<DownloadSession>,
    pub items: Vec<DownloadItem>,
    pub download_cursor: usize,
    pub download_started_at: Option<Instant>,
    pub batch_done: bool,
}

impl App {
    pub fn new(
        settings: Settings,
        binary_status: BinaryStatus,
        initial_url: Option<String>,
    ) -> Self {
        let mut app = Self {
            settings,
            binary_status,
            screen: Screen::UrlInput,
            should_quit: false,
            show_help: false,
            status: None,
            url_input: Input::default(),
            history: history::load_history(),
            history_focused: false,
            history_cursor: 0,
            history_open: None,
            history_video_cursor: 0,
            fetch_rx: None,
            fetch_spinner_frame: 0,
            pending_url: String::new(),
            playlist: None,
            selected: Vec::new(),
            video_cursor: 0,
            filtering: false,
            filter_input: Input::default(),
            settings_origin: SettingsOrigin::UrlInput,
            settings_cursor: 0,
            editing: false,
            edit_input: Input::default(),
            downloader: None,
            items: Vec::new(),
            download_cursor: 0,
            download_started_at: None,
            batch_done: false,
        };
        if let Some(url) = initial_url {
            app.url_input = Input::new(url.clone());
            app.begin_fetch(url);
        }
        app
    }

    pub fn set_status(&mut self, text: impl Into<String>, kind: MessageKind) {
        self.status = Some(StatusMessage {
            text: text.into(),
            kind,
        });
    }

    // ---------------------------------------------------------------
    // Fetching
    // ---------------------------------------------------------------

    pub fn begin_fetch(&mut self, url: String) {
        // Guards the CLI `<url>` fast path too (it skips the URL screen's
        // own readiness check), so we never spawn fetch_playlist without a
        // resolved yt-dlp.
        let Some(ytdlp) = self.binary_status.ytdlp.clone() else {
            self.set_status(
                "yt-dlp runtime not found -- reinstall the app (see README)",
                MessageKind::Error,
            );
            return;
        };
        let settings = self.settings.clone();
        let js_runtime = self.binary_status.js_runtime.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_url = url.clone();
        self.fetch_rx = Some(rx);
        self.screen = Screen::Fetching;
        tokio::spawn(async move {
            let result =
                vidsave_core::ytdlp::fetch_playlist(&url, &settings, &ytdlp, js_runtime.as_ref())
                    .await;
            let _ = tx.send(result);
        });
    }

    pub fn on_fetch_result(&mut self, result: anyhow::Result<PlaylistInfo>) {
        match result {
            Ok(playlist) => {
                self.selected = vec![true; playlist.videos.len()];
                self.video_cursor = 0;
                self.playlist = Some(playlist);
                self.screen = Screen::VideoList;
            }
            Err(e) => {
                self.set_status(format!("Failed to resolve URL: {e}"), MessageKind::Error);
                self.screen = Screen::UrlInput;
            }
        }
    }

    // ---------------------------------------------------------------
    // Video list screen: filtering + selection
    // ---------------------------------------------------------------

    /// Indices into `playlist.videos` that match the current filter text
    /// (case-insensitive substring match against title/uploader), or every
    /// index if there's no filter.
    pub fn filtered_video_indices(&self) -> Vec<usize> {
        let Some(playlist) = &self.playlist else {
            return Vec::new();
        };
        let needle = self.filter_input.value().trim().to_lowercase();
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

    // ---------------------------------------------------------------
    // Download lifecycle
    // ---------------------------------------------------------------

    pub fn start_downloads(&mut self) {
        let Some(ytdlp) = self.binary_status.ytdlp.clone() else {
            self.set_status(
                "yt-dlp runtime not found -- reinstall the app (see README)",
                MessageKind::Error,
            );
            return;
        };
        let Some(playlist) = &self.playlist else {
            return;
        };
        let videos: Vec<Video> = playlist
            .videos
            .iter()
            .zip(self.selected.iter())
            .filter(|(_, sel)| **sel)
            .map(|(v, _)| v.clone())
            .collect();
        if videos.is_empty() {
            self.set_status("Select at least one video first", MessageKind::Error);
            return;
        }

        // Playlists/channels get their own subfolder (named after the
        // playlist/channel title) so their videos land together rather than
        // mixed into the shared output directory; a lone video URL doesn't
        // need that extra nesting. This only affects where files for *this*
        // batch land, not the persisted output-dir setting itself.
        let mut download_settings = self.settings.clone();
        if playlist.is_playlist {
            let folder = vidsave_core::models::sanitize_path_component(&playlist.title);
            download_settings.output_dir = self.settings.output_dir.join(folder);
        }

        if let Err(e) = std::fs::create_dir_all(&download_settings.output_dir) {
            self.set_status(
                format!("Could not create output directory: {e}"),
                MessageKind::Error,
            );
            return;
        }
        if download_settings.use_download_archive {
            let archive_path = download_settings.archive_path();
            if let Some(parent) = archive_path.parent()
                && let Err(e) = std::fs::create_dir_all(parent)
            {
                self.set_status(
                    format!("Could not create archive directory: {e}"),
                    MessageKind::Error,
                );
                return;
            }
        }

        self.items = videos.iter().cloned().map(DownloadItem::new).collect();
        self.download_cursor = 0;
        self.download_started_at = Some(Instant::now());
        self.batch_done = false;
        let binaries = vidsave_core::downloader::BinaryPaths {
            ytdlp,
            ffmpeg: self.binary_status.ffmpeg_path.clone(),
            js_runtime: self.binary_status.js_runtime.clone(),
        };
        let (handle, events) = downloader::start(videos, download_settings, binaries);
        self.downloader = Some(DownloadSession { handle, events });
        self.screen = Screen::Downloading;
    }

    pub fn on_download_event(&mut self, event: DownloadEvent) {
        let (index, state) = match event {
            DownloadEvent::Started(i) => (i, DownloadState::Starting),
            DownloadEvent::Progress(i, progress) => (i, DownloadState::Downloading(progress)),
            DownloadEvent::PostProcessing(i) => (i, DownloadState::PostProcessing),
            DownloadEvent::Log(i, line) => {
                if let Some(item) = self.items.get_mut(i) {
                    item.push_log(line);
                }
                return;
            }
            DownloadEvent::Finished(i, Ok(())) => (i, DownloadState::Done),
            DownloadEvent::Finished(i, Err(msg)) => (i, DownloadState::Failed(msg)),
            DownloadEvent::Skipped(i) => (i, DownloadState::Skipped),
            DownloadEvent::Cancelled(i) => (i, DownloadState::Cancelled),
            DownloadEvent::Paused(i) => (i, DownloadState::Paused),
        };
        if let Some(item) = self.items.get_mut(index) {
            item.set_state(state);
        }
        if !self.batch_done
            && !self.items.is_empty()
            && self.items.iter().all(|i| i.state.is_terminal())
        {
            self.batch_done = true;
            self.record_history();
        }
    }

    pub fn on_download_channel_closed(&mut self) {
        self.downloader = None;
        if !self.batch_done {
            self.batch_done = true;
            self.record_history();
        }
    }

    /// Appends this just-finished batch to `history.json` and to the
    /// in-memory list shown on the URL input screen -- called exactly once
    /// per batch, right as `batch_done` flips to `true`.
    fn record_history(&mut self) {
        let Some(playlist) = &self.playlist else {
            return;
        };
        let Some(entry) = HistoryEntry::from_batch(playlist, &self.items) else {
            return;
        };
        if let Err(e) = history::push_history_entry(entry.clone()) {
            self.set_status(
                format!("Could not save download history: {e}"),
                MessageKind::Error,
            );
        }
        self.history.insert(0, entry);
    }

    // ---------------------------------------------------------------
    // History screen (embedded list on UrlInput, drilling into
    // HistoryPlaylist / HistoryVideoDetail)
    // ---------------------------------------------------------------

    /// `Enter` on the currently-selected history row: a single-video entry
    /// goes straight to its detail (a "list" of one video is pointless), a
    /// playlist/channel entry opens its video list.
    pub fn open_selected_history_entry(&mut self) {
        let Some(entry) = self.history.get(self.history_cursor) else {
            return;
        };
        self.history_open = Some(self.history_cursor);
        self.history_video_cursor = 0;
        self.screen = if entry.is_single_video() {
            Screen::HistoryVideoDetail
        } else {
            Screen::HistoryPlaylist
        };
    }

    pub fn open_selected_history_video(&mut self) {
        self.screen = Screen::HistoryVideoDetail;
    }

    pub fn current_history_entry(&self) -> Option<&HistoryEntry> {
        self.history.get(self.history_open?)
    }

    pub fn current_history_video(&self) -> Option<&history::HistoryVideoEntry> {
        self.current_history_entry()?
            .videos
            .get(self.history_video_cursor)
    }

    /// `Esc` from `HistoryVideoDetail`: back to the video list for a
    /// playlist/channel entry, or straight back to `UrlInput` for a
    /// single-video entry (there's no list screen to return to).
    pub fn back_from_history_video_detail(&mut self) {
        let is_single = self
            .current_history_entry()
            .is_some_and(HistoryEntry::is_single_video);
        if is_single {
            self.history_open = None;
            self.screen = Screen::UrlInput;
        } else {
            self.screen = Screen::HistoryPlaylist;
        }
    }

    pub fn back_from_history_playlist(&mut self) {
        self.history_open = None;
        self.screen = Screen::UrlInput;
    }

    // ---------------------------------------------------------------
    // Ticking / spinner
    // ---------------------------------------------------------------

    pub fn on_tick(&mut self) {
        self.fetch_spinner_frame = self.fetch_spinner_frame.wrapping_add(1);
    }

    // ---------------------------------------------------------------
    // Settings field access (shared by settings screen + persistence)
    // ---------------------------------------------------------------

    pub fn current_field(&self) -> SettingsField {
        SettingsField::ALL[self.settings_cursor]
    }

    pub fn apply_settings_action(&mut self, field: SettingsField, action: FieldAction) {
        match (field.kind(), action) {
            (FieldKind::Toggle, FieldAction::Activate) => field.toggle(&mut self.settings),
            (FieldKind::Cycle, FieldAction::Left) => field.cycle(&mut self.settings, -1),
            (FieldKind::Cycle, FieldAction::Right) => field.cycle(&mut self.settings, 1),
            _ => {}
        }
    }

    pub fn begin_edit_field(&mut self, field: SettingsField) {
        let current = field.text_value(&self.settings);
        self.edit_input = Input::new(current);
        self.editing = true;
    }

    pub fn commit_edit_field(&mut self, field: SettingsField) {
        field.set_text_value(&mut self.settings, self.edit_input.value());
        self.editing = false;
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
    }

    pub fn save_settings(&mut self) {
        match self.settings.save() {
            Ok(()) => self.set_status("Settings saved", MessageKind::Info),
            Err(e) => self.set_status(format!("Could not save settings: {e}"), MessageKind::Error),
        }
    }
}

pub enum FieldAction {
    Activate,
    Left,
    Right,
}

/// Feed a crossterm key event into whichever `tui_input::Input` is currently
/// focused (URL bar / filter box / settings edit box).
pub fn feed_input(input: &mut Input, event: &crossterm::event::Event) {
    input.handle_event(event);
}
