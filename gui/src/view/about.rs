//! The About tab: what this app is, who made it, and how to reach them.

use iced::widget::{column, container, row, text};
use iced::{Element, Fill};

use crate::message::Message;
use crate::state::State;
use crate::theme;

pub fn view(_state: &State) -> Element<'_, Message> {
    let header = column![
        text("VidSave").size(28),
        text("Download YouTube playlists and videos.")
            .size(14)
            .color(theme::text_dim()),
        text(format!("Version {}", env!("CARGO_PKG_VERSION")))
            .size(12)
            .color(theme::text_dim()),
    ]
    .spacing(4);

    let developer_card = container(
        column![
            text("Developer").size(13).color(theme::info()),
            info_row("Name", "bel-wadoud"),
            info_row("GitHub", "github.com/bel-wadoud"),
            info_row("Contact", "abdelwadoud.belheraoui@proton.me"),
        ]
        .spacing(6),
    )
    .padding(14)
    .width(Fill)
    .style(container::rounded_box);

    let project_card = container(
        column![
            text("Project").size(13).color(theme::info()),
            info_row("Source", "github.com/bel-wadoud/vidsave"),
            info_row("License", "PolyForm Noncommercial 1.0.0"),
            text(
                "Free to use, modify, and share for any noncommercial \
                 purpose. Commercial use is not permitted."
            )
            .size(11)
            .color(theme::text_dim()),
        ]
        .spacing(6),
    )
    .padding(14)
    .width(Fill)
    .style(container::rounded_box);

    let credits = text(
        "Built with iced (GUI), ratatui (terminal UI), and yt-dlp -- \
         the actual heavy lifting of talking to YouTube.",
    )
    .size(11)
    .color(theme::text_dim());

    column![header, developer_card, project_card, credits]
        .spacing(16)
        .into()
}

fn info_row<'a>(label: &'a str, value: &'a str) -> Element<'a, Message> {
    row![
        text(label).size(12).width(80).color(theme::text_dim()),
        text(value).size(12),
    ]
    .into()
}
