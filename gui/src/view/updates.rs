//! The Updates tab: shows whether a newer version is available (checked
//! automatically shortly after startup, see `update::initial_task`) and,
//! if so, lets the user install it with one click -- downloads the real
//! installer and hands off to it (see `update_check::download_and_launch_installer`
//! for why it's not an in-place self-replace) rather than silently
//! rewriting anything behind the user's back.

use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Fill};

use crate::message::Message;
use crate::state::{State, UpdateStatus};
use crate::theme;

pub fn view(state: &State) -> Element<'_, Message> {
    let header = column![
        text("Updates").size(20),
        text(format!(
            "You're running version {}",
            env!("CARGO_PKG_VERSION")
        ))
        .size(13)
        .color(theme::text_dim()),
    ]
    .spacing(4);

    let body = status_body(state);

    column![header, body].spacing(16).into()
}

fn status_body(state: &State) -> Element<'_, Message> {
    match &state.update_status {
        UpdateStatus::Idle => check_button("Check for updates", true),
        UpdateStatus::Checking => status_line("Checking for updates...", theme::text_dim()),
        UpdateStatus::UpToDate => column![
            status_line("You're up to date.", theme::success()),
            check_button("Check again", true),
        ]
        .spacing(10)
        .into(),
        UpdateStatus::Available(info) => {
            let card = container(
                column![
                    text(format!("VidSave {} is available", info.version)).size(15),
                    notes_view(&info.notes),
                    button(text("Download & install").size(14))
                        .style(button::primary)
                        .padding([10, 20])
                        .on_press(Message::InstallUpdatePressed),
                ]
                .spacing(10),
            )
            .padding(14)
            .width(Fill)
            .style(container::rounded_box);
            column![card].spacing(10).into()
        }
        UpdateStatus::Installing => column![
            status_line(
                "Downloading and launching the installer...",
                theme::text_dim()
            ),
            text("VidSave will close automatically once the installer takes over.")
                .size(12)
                .color(theme::text_dim()),
        ]
        .spacing(6)
        .into(),
        UpdateStatus::Error(message) => column![
            status_line(
                format!("Couldn't check for updates: {message}"),
                theme::danger()
            ),
            check_button("Try again", true),
        ]
        .spacing(10)
        .into(),
    }
}

fn notes_view(notes: &str) -> Element<'_, Message> {
    if notes.trim().is_empty() {
        return text("").into();
    }
    container(scrollable(text(notes.to_string()).size(12)).height(160))
        .padding([8, 0])
        .into()
}

fn status_line(text_str: impl Into<String>, color: iced::Color) -> Element<'static, Message> {
    text(text_str.into()).size(13).color(color).into()
}

fn check_button(label: &'static str, enabled: bool) -> Element<'static, Message> {
    let mut b = button(text(label).size(13))
        .style(button::secondary)
        .padding([8, 16]);
    if enabled {
        b = b.on_press(Message::CheckForUpdates);
    }
    b.into()
}
