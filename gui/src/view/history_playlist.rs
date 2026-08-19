//! A drilled-into playlist/channel history entry: its videos, each
//! clickable to see that video's recorded outcome (`history_video_detail`).

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Center, Element, Fill};

use vidsave_core::history::{HistoryEntry, HistoryOutcome, HistoryVideoEntry};

use crate::message::Message;
use crate::state::State;

pub fn view(state: &State) -> Element<'_, Message> {
    let Some(entry) = state.current_history_entry() else {
        return text("").into();
    };

    let header = header_card(entry);
    let rows = entry
        .videos
        .iter()
        .enumerate()
        .map(|(i, v)| video_row(i, v));
    let list = scrollable(column(rows).spacing(6)).height(Fill);

    let footer = row![
        button(text("← Back").size(14))
            .style(button::secondary)
            .on_press(Message::BackFromHistoryPlaylist),
        Space::new().width(Fill),
    ]
    .align_y(Center);

    column![header, list, footer]
        .spacing(14)
        .height(Fill)
        .into()
}

fn header_card(entry: &HistoryEntry) -> Element<'_, Message> {
    let uploader_suffix = entry
        .uploader
        .as_ref()
        .map(|u| format!("  --  {u}"))
        .unwrap_or_default();
    let content = column![
        text(&entry.title).size(18),
        text(format!(
            "{} videos, {} done, {} failed{}   --   {}",
            entry.videos.len(),
            entry.done_count(),
            entry.failed_count(),
            uploader_suffix,
            entry.finished_at_label(),
        ))
        .size(12),
    ]
    .spacing(4);

    container(content)
        .padding(12)
        .width(Fill)
        .style(container::rounded_box)
        .into()
}

fn video_row(index: usize, video: &HistoryVideoEntry) -> Element<'_, Message> {
    let (icon, color) = match &video.outcome {
        HistoryOutcome::Done => ("✓", iced::Color::from_rgb8(0x4C, 0xAF, 0x50)),
        HistoryOutcome::Skipped => ("−", iced::Color::from_rgb8(0x64, 0xB5, 0xF6)),
        HistoryOutcome::Cancelled => ("✗", iced::Color::from_rgb8(0x9E, 0x9E, 0x9E)),
        HistoryOutcome::Failed(_) => ("!", iced::Color::from_rgb8(0xE5, 0x73, 0x73)),
    };

    let label = column![
        text(format!("{icon} {}", video.title))
            .size(13)
            .color(color),
        text(format!(
            "{}   {}   {}",
            video.duration_label(),
            video.size_label(),
            video.outcome.label(),
        ))
        .size(11),
    ]
    .spacing(2);

    button(label)
        .style(button::text)
        .width(Fill)
        .padding(8)
        .on_press(Message::OpenHistoryVideo(index))
        .into()
}
