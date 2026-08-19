//! The History tab: every recorded download batch, newest first. Clicking
//! a playlist/channel entry drills into its video list (`history_playlist`);
//! clicking a single-video entry goes straight to that video's details
//! (`history_video_detail`) -- see `update::update`'s `OpenHistoryEntry`.

use iced::widget::{button, column, scrollable, text};
use iced::{Element, Fill};

use vidsave_core::history::HistoryEntry;

use crate::message::Message;
use crate::state::State;
use crate::theme;

pub fn view(state: &State) -> Element<'_, Message> {
    if state.history.is_empty() {
        return column![
            text("Download history").size(20),
            text("Nothing downloaded yet -- finished batches show up here.")
                .size(13)
                .color(theme::text_dim()),
        ]
        .spacing(10)
        .into();
    }

    let rows = state
        .history
        .iter()
        .enumerate()
        .map(|(i, entry)| history_row(i, entry));

    column![
        text("Download history").size(20),
        scrollable(column(rows).spacing(6)).height(Fill),
    ]
    .spacing(10)
    .height(Fill)
    .into()
}

fn history_row(index: usize, entry: &HistoryEntry) -> Element<'_, Message> {
    let icon = if entry.is_single_video() {
        "•"
    } else {
        "▤"
    };
    let failed = entry.failed_count();
    let failed_suffix = if failed > 0 {
        format!(", {failed} failed")
    } else {
        String::new()
    };
    let label = column![
        text(format!("{icon} {}", entry.title)).size(13),
        text(format!(
            "{}/{} done{failed_suffix}   {}",
            entry.done_count(),
            entry.videos.len(),
            entry.finished_at_label(),
        ))
        .size(11)
        .color(theme::text_dim()),
    ]
    .spacing(2);

    button(label)
        .style(button::text)
        .width(Fill)
        .padding(8)
        .on_press(Message::OpenHistoryEntry(index))
        .into()
}
