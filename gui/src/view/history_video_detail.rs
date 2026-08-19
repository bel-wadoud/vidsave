//! One video's recorded outcome from a past download batch -- reached
//! either from `history_playlist` (a video within a playlist/channel
//! batch) or straight from `url_input` (a single-video batch has nothing
//! to list, so there's no intermediate screen for it).

use iced::widget::{Space, button, column, container, row, text};
use iced::{Center, Element, Fill};

use vidsave_core::history::HistoryOutcome;

use crate::message::Message;
use crate::state::State;

pub fn view(state: &State) -> Element<'_, Message> {
    let Some(video) = state.current_history_video() else {
        return text("").into();
    };

    let (outcome_text, color) = match &video.outcome {
        HistoryOutcome::Done => ("Done", iced::Color::from_rgb8(0x4C, 0xAF, 0x50)),
        HistoryOutcome::Skipped => (
            "Skipped (already had it)",
            iced::Color::from_rgb8(0x64, 0xB5, 0xF6),
        ),
        HistoryOutcome::Cancelled => ("Cancelled", iced::Color::from_rgb8(0x9E, 0x9E, 0x9E)),
        HistoryOutcome::Failed(_) => ("Failed", iced::Color::from_rgb8(0xE5, 0x73, 0x73)),
    };

    let mut rows = vec![
        detail_row("Title", video.title.clone()),
        detail_row(
            "Uploader",
            video.uploader.clone().unwrap_or_else(|| "--".to_string()),
        ),
        detail_row("Length", video.duration_label()),
        detail_row("Size", video.size_label()),
        detail_row("URL", video.url.clone()),
    ];
    rows.push(
        row![
            text("Result").size(13).width(100),
            text(outcome_text).size(13).color(color),
        ]
        .into(),
    );

    if let HistoryOutcome::Failed(message) = &video.outcome {
        rows.push(
            text(format!("Error: {message}"))
                .size(12)
                .color(iced::Color::from_rgb8(0xE5, 0x73, 0x73))
                .into(),
        );
    }

    let card = container(column(rows).spacing(10))
        .padding(16)
        .width(Fill)
        .style(container::rounded_box);

    let footer = row![
        button(text("← Back").size(14))
            .style(button::secondary)
            .on_press(Message::BackFromHistoryVideoDetail),
        Space::new().width(Fill),
    ]
    .align_y(Center);

    column![card, footer].spacing(14).height(Fill).into()
}

fn detail_row(label: &'static str, value: String) -> Element<'static, Message> {
    row![text(label).size(13).width(100), text(value).size(13)].into()
}
