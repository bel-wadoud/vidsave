//! Top-level render dispatch: a persistent tab bar (Download / History /
//! Settings / Updates / About -- see `state::Tab`) plus whichever screen is
//! active within the selected tab, with a shared status bar underneath
//! that always describes what's currently happening.

mod about;
mod downloading;
mod history;
mod history_playlist;
mod history_video_detail;
mod settings;
mod updates;
mod url_input;
mod video_list;

use iced::widget::{button, column, container, row, text};
use iced::{Center, Element, Fill, Length};

use crate::message::Message;
use crate::state::{Screen, State, StatusKind, Tab};
use crate::theme;

pub fn view(state: &State) -> Element<'_, Message> {
    let content = match state.screen {
        Screen::UrlInput => url_input::view(state),
        Screen::VideoList => video_list::view(state),
        Screen::Downloading => downloading::view(state),
        Screen::HistoryList => history::view(state),
        Screen::HistoryPlaylist => history_playlist::view(state),
        Screen::HistoryVideoDetail => history_video_detail::view(state),
        Screen::Settings => settings::view(state),
        Screen::Updates => updates::view(state),
        Screen::About => about::view(state),
    };

    column![
        tab_bar(state),
        container(content).width(Fill).height(Fill).padding(16),
        status_bar(state),
    ]
    .into()
}

/// The persistent tab bar: always visible, always the same five tabs, so
/// Settings/History/Updates/About are never more than one click away --
/// unlike the old design, where Settings was a modal-ish overlay and
/// History was squeezed below the URL box on just one screen.
fn tab_bar(state: &State) -> Element<'_, Message> {
    let active = state.screen.tab();
    let tabs = Tab::ALL.map(|tab| tab_button(tab, tab == active));

    container(row(tabs).spacing(4).align_y(Center))
        .width(Fill)
        .padding([8, 16])
        .style(|t: &iced::Theme| {
            let palette = t.extended_palette();
            container::Style {
                background: Some(palette.background.weak.color.into()),
                ..container::Style::default()
            }
        })
        .into()
}

fn tab_button(tab: Tab, active: bool) -> Element<'static, Message> {
    let icon = match tab {
        Tab::Download => "⬇",
        Tab::History => "▤",
        Tab::Settings => "⚙",
        Tab::Updates => "↻",
        Tab::About => "i",
    };
    let label = text(format!("{icon} {}", tab.label())).size(14);
    let style = move |t: &iced::Theme, status: button::Status| {
        if active {
            active_tab_style(status)
        } else {
            button::text(t, status)
        }
    };
    button(label)
        .style(style)
        .padding([8, 14])
        .on_press(Message::TabSelected(tab))
        .into()
}

/// The active tab's own look: a filled pill in the app's accent color
/// (matching the icon), distinct from every other button style in the app
/// so the current tab is unmistakable at a glance.
fn active_tab_style(status: button::Status) -> button::Style {
    let base = if status == button::Status::Pressed {
        theme::accent_dim()
    } else {
        theme::accent()
    };
    button::Style {
        background: Some(base.into()),
        text_color: iced::Color::WHITE,
        border: iced::Border {
            radius: 6.0.into(),
            ..iced::Border::default()
        },
        ..button::Style::default()
    }
}

/// The status bar always shows *something* concrete about what's going on --
/// an explicit status message (e.g. an error) takes priority, but absent
/// that it falls back to a plain description of the current screen/activity
/// rather than a static app name, so the user is never left guessing
/// whether the app is fetching, downloading, or just idle.
fn status_bar(state: &State) -> Element<'_, Message> {
    let dim = theme::text_dim();
    let (message, color) = match &state.status {
        Some(status) => {
            let color = match status.kind {
                StatusKind::Info => theme::success(),
                StatusKind::Error => theme::danger(),
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
    match state.screen {
        Screen::UrlInput if !state.tools_checked => "Starting up...".to_string(),
        Screen::UrlInput if state.fetching => "Fetching playlist info...".to_string(),
        Screen::UrlInput => "Ready".to_string(),
        Screen::VideoList => "Choose videos, then start the download".to_string(),
        Screen::Downloading if state.batch_done => "All downloads finished".to_string(),
        Screen::Downloading => "Downloading...".to_string(),
        Screen::HistoryList | Screen::HistoryPlaylist | Screen::HistoryVideoDetail => {
            "Browsing download history".to_string()
        }
        Screen::Settings => "Settings".to_string(),
        Screen::Updates => "Updates".to_string(),
        Screen::About => "About VidSave".to_string(),
    }
}
