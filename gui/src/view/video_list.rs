use iced::widget::{Space, button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{Center, Element, Fill};

use crate::message::Message;
use crate::state::State;

pub fn view(state: &State) -> Element<'_, Message> {
    let Some(playlist) = &state.playlist else {
        return text("No playlist loaded").into();
    };

    let uploader_suffix = playlist
        .uploader
        .as_ref()
        .map(|u| format!("  --  {u}"))
        .unwrap_or_default();
    let header = column![
        text(&playlist.title).size(22),
        text(format!(
            "{} videos, {} selected{}",
            playlist.videos.len(),
            state.selected_count(),
            uploader_suffix
        ))
        .size(13),
    ]
    .spacing(2);

    let filter = text_input("Filter by title or uploader...", &state.filter)
        .on_input(Message::FilterChanged)
        .padding(8);

    let select_row = row![
        button(text("All").size(13))
            .style(button::secondary)
            .on_press(Message::SelectAll),
        button(text("None").size(13))
            .style(button::secondary)
            .on_press(Message::SelectNone),
        button(text("Invert").size(13))
            .style(button::secondary)
            .on_press(Message::InvertSelection),
    ]
    .spacing(8);

    let filtered = state.filtered_video_indices();
    let list: Element<'_, Message> = if filtered.is_empty() {
        text(if state.filter.is_empty() {
            "This playlist has no videos"
        } else {
            "No videos match that filter"
        })
        .size(14)
        .into()
    } else {
        let rows = filtered.into_iter().map(|index| video_row(state, index));
        scrollable(column(rows).spacing(4)).height(Fill).into()
    };

    let selected_count = state.selected_count();
    let download_label = if selected_count == 0 {
        "Download".to_string()
    } else {
        format!("Download {selected_count} selected")
    };
    let mut download_button = button(text(download_label).size(15))
        .style(button::primary)
        .padding([10, 20]);
    if selected_count > 0 {
        download_button = download_button.on_press(Message::StartDownloadsPressed);
    }

    let footer = row![
        button(text("← Back").size(14))
            .style(button::secondary)
            .on_press(Message::BackToUrlInput),
        Space::new().width(Fill),
        download_button,
    ]
    .spacing(10)
    .align_y(Center);

    column![header, filter, select_row, list, footer]
        .spacing(14)
        .height(Fill)
        .into()
}

fn video_row(state: &State, index: usize) -> Element<'_, Message> {
    let Some(playlist) = &state.playlist else {
        return text("").into();
    };
    let video = &playlist.videos[index];
    let checked = state.selected.get(index).copied().unwrap_or(false);

    let index_label = video
        .playlist_index
        .map(|i| format!("{i:>3}. "))
        .unwrap_or_default();
    let label = format!(
        "{index_label}{}  ({}, {})",
        video.title,
        video.duration_label(),
        video.size_label()
    );

    container(
        checkbox(checked)
            .label(label)
            .on_toggle(move |_| Message::ToggleVideo(index))
            .width(Fill),
    )
    .padding([4, 8])
    .into()
}
