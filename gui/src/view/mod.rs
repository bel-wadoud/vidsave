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

fn status_bar(state: &State) -> Element<'_, Message> {
    let (message, color) = match &state.status {
        Some(status) => {
            let color = match status.kind {
                StatusKind::Info => iced::Color::from_rgb8(0x4C, 0xAF, 0x50),
                StatusKind::Error => iced::Color::from_rgb8(0xE5, 0x73, 0x73),
            };
            (status.text.clone(), color)
        }
        None => (
            "ytb-dl-tui".to_string(),
            iced::Color::from_rgb8(0x77, 0x77, 0x77),
        ),
    };
    container(text(message).size(13).color(color))
        .width(Length::Fill)
        .padding([6, 16])
        .into()
}
