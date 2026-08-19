//! Top-level render dispatch: picks the screen (or the Settings panel, which
//! overlays whichever screen was active -- see `state::State::show_settings`)
//! and draws the shared status bar underneath it. Mirrors the TUI's
//! `ui/mod.rs` in spirit.

mod downloading;
mod settings;
mod url_input;
mod video_list;

use iced::widget::{column, container, text};
use iced::{Element, Fill, Length};

use crate::message::Message;
use crate::state::{Screen, State, StatusKind};

pub fn view(state: &State) -> Element<'_, Message> {
    let content = if state.show_settings {
        settings::view(state)
    } else {
        match state.screen {
            Screen::UrlInput => url_input::view(state),
            Screen::VideoList => video_list::view(state),
            Screen::Downloading => downloading::view(state),
        }
    };

    column![
        container(content).width(Fill).height(Fill).padding(16),
        status_bar(state),
    ]
    .into()
}

/// The status bar always shows *something* concrete about what's going on --
/// an explicit status message (e.g. an error) takes priority, but absent
/// that it falls back to a plain description of the current screen/activity
/// rather than a static app name, so the user is never left guessing
/// whether the app is fetching, downloading, or just idle.
fn status_bar(state: &State) -> Element<'_, Message> {
    let dim = iced::Color::from_rgb8(0x77, 0x77, 0x77);
    let (message, color) = match &state.status {
        Some(status) => {
            let color = match status.kind {
                StatusKind::Info => iced::Color::from_rgb8(0x4C, 0xAF, 0x50),
                StatusKind::Error => iced::Color::from_rgb8(0xE5, 0x73, 0x73),
            };
            (status.text.clone(), color)
        }
        None => (activity_text(state), dim),
    };
    container(text(message).size(13).color(color))
        .width(Length::Fill)
        .padding([6, 16])
        .into()
}

fn activity_text(state: &State) -> String {
    if state.show_settings {
        return "Settings".to_string();
    }
    match state.screen {
        Screen::UrlInput if !state.tools_checked => "Starting up...".to_string(),
        Screen::UrlInput if state.fetching => "Fetching playlist info...".to_string(),
        Screen::UrlInput => "Ready".to_string(),
        Screen::VideoList => "Choose videos, then start the download".to_string(),
        Screen::Downloading if state.batch_done => "All downloads finished".to_string(),
        Screen::Downloading => "Downloading...".to_string(),
    }
}
