use iced::widget::{button, column, container, row, text, text_input};
use iced::{Center, Element, Fill};

use crate::message::Message;
use crate::state::State;

pub fn view(state: &State) -> Element<'_, Message> {
    let title = text("ytb-dl-tui").size(32);
    let subtitle = text("Paste a YouTube playlist, channel, or video URL").size(14);

    let input = text_input("https://www.youtube.com/...", &state.url_input)
        .on_input(Message::UrlChanged)
        .on_submit(Message::FetchPressed)
        .padding(10)
        .size(16);

    let fetch_label = if state.fetching {
        "Resolving..."
    } else {
        "Fetch"
    };
    let mut fetch_button = button(text(fetch_label).size(16)).padding([10, 20]);
    if !state.fetching {
        fetch_button = fetch_button.on_press(Message::FetchPressed);
    }

    let url_row = row![input, fetch_button].spacing(10).align_y(Center);

    let settings_button = button(text("⚙ Settings").size(14))
        .style(button::secondary)
        .on_press(Message::OpenSettings);

    let tools = tool_status(state);

    let content = column![
        title,
        subtitle,
        url_row,
        row![settings_button].width(Fill),
        tools,
    ]
    .spacing(16);

    container(container(content).max_width(720))
        .width(Fill)
        .center_x(Fill)
        .into()
}

fn tool_status(state: &State) -> Element<'_, Message> {
    let ok = iced::Color::from_rgb8(0x4C, 0xAF, 0x50);
    let warn = iced::Color::from_rgb8(0xFF, 0xB7, 0x4D);
    let err = iced::Color::from_rgb8(0xE5, 0x73, 0x73);

    let ytdlp_line = match (
        &state.binary_status.ytdlp_version,
        &state.binary_status.ytdlp,
    ) {
        (Some(v), Some(_)) => text(format!("✓ yt-dlp {v}")).color(ok),
        _ => text("✗ yt-dlp runtime not found -- reinstall the app").color(err),
    };
    let ffmpeg_line = match &state.binary_status.ffmpeg_path {
        Some(_) => text("✓ ffmpeg found").color(ok),
        None => text("⚠ ffmpeg not found -- merging/thumbnail embedding disabled").color(warn),
    };
    let js_line = match &state.binary_status.js_runtime {
        Some(js) => text(format!("✓ JS runtime: {}", js.name)).color(ok),
        None => text("⚠ no JS runtime found -- some videos may wrongly report as unavailable")
            .color(warn),
    };

    column![ytdlp_line.size(13), ffmpeg_line.size(13), js_line.size(13)]
        .spacing(4)
        .into()
}
