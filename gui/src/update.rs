//! Message handling: mirrors the TUI's `App` methods in `app.rs`
//! (`begin_fetch`, `start_downloads`, `on_download_event`, `save_settings`,
//! ...) function-for-function, since the underlying business logic (all in
//! `ytb_dl_tui_core`) doesn't change between frontends -- only how work gets
//! kicked off (`Task` instead of `tokio::spawn` + a channel poll) and how
//! results come back (a `Message` instead of a direct state mutation).

use iced::Task;
use tokio_stream::wrappers::UnboundedReceiverStream;

use ytb_dl_tui_core::downloader::{self, BinaryPaths, DownloadEvent};
use ytb_dl_tui_core::models::{self, DownloadItem, DownloadState, PlaylistInfo, Video};
use ytb_dl_tui_core::ytdlp::{self, JsRuntime, YtDlp};

use crate::message::Message;
use crate::state::{Screen, State, StatusKind};

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::ToolsChecked(status) => {
            state.binary_status = status;
            if !state.binary_status.ready() {
                state.set_status(
                    "yt-dlp runtime not found -- reinstall the app (see README)",
                    StatusKind::Error,
                );
            }
            match state.initial_url.take() {
                Some(url) if state.binary_status.ready() => begin_fetch(state, url),
                _ => Task::none(),
            }
        }

        // -- URL input --
        Message::UrlChanged(value) => {
            state.url_input = value;
            Task::none()
        }
        Message::FetchPressed => {
            let url = state.url_input.trim().to_string();
            if url.is_empty() {
                state.set_status("Enter a playlist or video URL first", StatusKind::Error);
                return Task::none();
            }
            if !state.binary_status.ready() {
                state.set_status(
                    "yt-dlp runtime not found -- reinstall the app (see README)",
                    StatusKind::Error,
                );
                return Task::none();
            }
            begin_fetch(state, url)
        }
        Message::FetchCompleted(Ok(playlist)) => {
            state.selected = vec![true; playlist.videos.len()];
            state.playlist = Some(playlist);
            state.filter.clear();
            state.fetching = false;
            state.screen = Screen::VideoList;
            Task::none()
        }
        Message::FetchCompleted(Err(message)) => {
            state.fetching = false;
            state.set_status(
                format!("Failed to resolve URL: {message}"),
                StatusKind::Error,
            );
            Task::none()
        }

        // -- Video list --
        Message::ToggleVideo(index) => {
            state.toggle_selected(index);
            Task::none()
        }
        Message::SelectAll => {
            state.set_filtered_selection(true);
            Task::none()
        }
        Message::SelectNone => {
            state.set_filtered_selection(false);
            Task::none()
        }
        Message::InvertSelection => {
            state.invert_filtered_selection();
            Task::none()
        }
        Message::FilterChanged(value) => {
            state.filter = value;
            Task::none()
        }
        Message::BackToUrlInput => {
            state.playlist = None;
            state.selected.clear();
            state.filter.clear();
            state.screen = Screen::UrlInput;
            Task::none()
        }
        Message::StartDownloadsPressed => start_downloads(state),

        // -- Settings panel --
        Message::OpenSettings => {
            state.show_settings = true;
            state.settings_saved_flash = false;
            Task::none()
        }
        Message::CloseSettings => {
            state.show_settings = false;
            Task::none()
        }
        Message::SettingsToggled(field) => {
            field.toggle(&mut state.settings);
            Task::none()
        }
        Message::SettingsTextChanged(field, value) => {
            field.set_text_value(&mut state.settings, &value);
            Task::none()
        }
        Message::MediaModePicked(value) => {
            state.settings.media_mode = value;
            Task::none()
        }
        Message::VideoQualityPicked(value) => {
            state.settings.video_quality = value;
            Task::none()
        }
        Message::VideoContainerPicked(value) => {
            state.settings.video_container = value;
            Task::none()
        }
        Message::AudioFormatPicked(value) => {
            state.settings.audio_format = value;
            Task::none()
        }
        Message::SaveSettingsPressed => {
            match state.settings.save() {
                Ok(()) => {
                    state.set_status("Settings saved", StatusKind::Info);
                    state.settings_saved_flash = true;
                }
                Err(e) => {
                    state.set_status(format!("Could not save settings: {e}"), StatusKind::Error)
                }
            }
            Task::none()
        }

        // -- Downloading --
        Message::DownloadEvent(event) => {
            apply_download_event(state, event);
            Task::none()
        }
        Message::SelectQueueItem(index) => {
            state.selected_queue_item = Some(index);
            Task::none()
        }
        Message::CancelItem(index) => {
            if let Some(handle) = &state.download_handle {
                handle.cancel_item(index);
            }
            Task::none()
        }
        Message::CancelAllPressed => {
            if let Some(handle) = &state.download_handle {
                handle.cancel_all();
            }
            Task::none()
        }
        Message::BackToVideoList => {
            state.screen = Screen::VideoList;
            Task::none()
        }
        Message::StartOverPressed => {
            state.screen = Screen::UrlInput;
            state.playlist = None;
            state.selected.clear();
            state.filter.clear();
            state.items.clear();
            state.download_handle = None;
            state.batch_done = false;
            state.selected_queue_item = None;
            state.url_input.clear();
            Task::none()
        }
    }
}

async fn check_tools() -> ytb_dl_tui_core::ytdlp::BinaryStatus {
    ytdlp::check_binaries().await
}

pub fn initial_task() -> Task<Message> {
    Task::perform(check_tools(), Message::ToolsChecked)
}

async fn fetch(
    url: String,
    settings: ytb_dl_tui_core::config::Settings,
    ytdlp: YtDlp,
    js_runtime: Option<JsRuntime>,
) -> Result<PlaylistInfo, String> {
    ytdlp::fetch_playlist(&url, &settings, &ytdlp, js_runtime.as_ref())
        .await
        .map_err(|e| format!("{e:#}"))
}

fn begin_fetch(state: &mut State, url: String) -> Task<Message> {
    let Some(ytdlp) = state.binary_status.ytdlp.clone() else {
        state.set_status(
            "yt-dlp runtime not found -- reinstall the app (see README)",
            StatusKind::Error,
        );
        return Task::none();
    };
    let settings = state.settings.clone();
    let js_runtime = state.binary_status.js_runtime.clone();
    state.pending_url = url.clone();
    state.fetching = true;
    state.status = None;
    Task::perform(
        fetch(url, settings, ytdlp, js_runtime),
        Message::FetchCompleted,
    )
}

fn start_downloads(state: &mut State) -> Task<Message> {
    let Some(ytdlp) = state.binary_status.ytdlp.clone() else {
        state.set_status(
            "yt-dlp runtime not found -- reinstall the app (see README)",
            StatusKind::Error,
        );
        return Task::none();
    };
    let Some(playlist) = &state.playlist else {
        return Task::none();
    };
    let videos: Vec<Video> = playlist
        .videos
        .iter()
        .zip(state.selected.iter())
        .filter(|(_, selected)| **selected)
        .map(|(v, _)| v.clone())
        .collect();
    if videos.is_empty() {
        state.set_status("Select at least one video first", StatusKind::Error);
        return Task::none();
    }

    // Playlists/channels get their own subfolder (named after the
    // playlist/channel title) so their videos land together rather than
    // mixed into the shared output directory; a lone video URL doesn't need
    // that extra nesting -- see the TUI's `App::start_downloads` for the
    // identical reasoning.
    let mut download_settings = state.settings.clone();
    if playlist.is_playlist {
        let folder = models::sanitize_path_component(&playlist.title);
        download_settings.output_dir = state.settings.output_dir.join(folder);
    }

    if let Err(e) = std::fs::create_dir_all(&download_settings.output_dir) {
        state.set_status(
            format!("Could not create output directory: {e}"),
            StatusKind::Error,
        );
        return Task::none();
    }
    if download_settings.use_download_archive {
        let archive_path = download_settings.archive_path();
        if let Some(parent) = archive_path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            state.set_status(
                format!("Could not create archive directory: {e}"),
                StatusKind::Error,
            );
            return Task::none();
        }
    }

    state.items = videos.iter().cloned().map(DownloadItem::new).collect();
    state.selected_queue_item = (!state.items.is_empty()).then_some(0);
    state.batch_done = false;
    let binaries = BinaryPaths {
        ytdlp,
        ffmpeg: state.binary_status.ffmpeg_path.clone(),
        js_runtime: state.binary_status.js_runtime.clone(),
    };
    let (handle, events) = downloader::start(videos, download_settings, binaries);
    state.download_handle = Some(handle);
    state.screen = Screen::Downloading;

    Task::run(UnboundedReceiverStream::new(events), Message::DownloadEvent)
}

fn apply_download_event(state: &mut State, event: DownloadEvent) {
    match event {
        DownloadEvent::Started(i) => set_item_state(state, i, DownloadState::Starting),
        DownloadEvent::Progress(i, p) => set_item_state(state, i, DownloadState::Downloading(p)),
        DownloadEvent::PostProcessing(i) => set_item_state(state, i, DownloadState::PostProcessing),
        DownloadEvent::Log(i, line) => {
            if let Some(item) = state.items.get_mut(i) {
                item.push_log(line);
            }
        }
        DownloadEvent::Finished(i, Ok(())) => set_item_state(state, i, DownloadState::Done),
        DownloadEvent::Finished(i, Err(msg)) => {
            set_item_state(state, i, DownloadState::Failed(msg))
        }
        DownloadEvent::Skipped(i) => set_item_state(state, i, DownloadState::Skipped),
        DownloadEvent::Cancelled(i) => set_item_state(state, i, DownloadState::Cancelled),
    }
    if !state.items.is_empty() && state.items.iter().all(|i| i.state.is_terminal()) {
        state.batch_done = true;
    }
}

fn set_item_state(state: &mut State, index: usize, new_state: DownloadState) {
    if let Some(item) = state.items.get_mut(index) {
        item.state = new_state;
    }
}
