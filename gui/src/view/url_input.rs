use iced::widget::{button, column, container, row, text, text_input};
use iced::{Center, Element, Fill};

use crate::message::Message;
use crate::state::State;

pub fn view(state: &State) -> Element<'_, Message> {
    let title = text("VidSave").size(32);
    let subtitle = text("Paste a YouTube playlist, channel, or video URL").size(14);

    let input = text_input("https://www.youtube.com/...", &state.url_input)
        .on_input(Message::UrlChanged)
        .on_submit(Message::FetchPressed)
        .padding(10)
        .size(16);

    let ready = state.tools_checked && state.binary_status.ready();
    let fetch_label = if state.fetching {
        "Resolving..."
    } else {
        "Fetch"
    };
    let mut fetch_button = button(text(fetch_label).size(16)).padding([10, 20]);
    if !state.fetching && ready {
        fetch_button = fetch_button.on_press(Message::FetchPressed);
    }

    let url_row = row![input, fetch_button].spacing(10).align_y(Center);

    let settings_button = button(text("⚙ Settings").size(14))
        .style(button::secondary)
        .on_press(Message::OpenSettings);

    let status = setup_status(state);

    let content = column![
        title,
        subtitle,
        url_row,
        row![settings_button].width(Fill),
        status,
    ]
    .spacing(16);

    container(container(content).max_width(720))
        .width(Fill)
        .center_x(Fill)
        .into()
}

/// Plain-language setup status -- a normal user doesn't need to know what
/// "yt-dlp" or "ffmpeg" are, just whether the app is ready to go, and if
/// not, roughly why. Always shows *something*: never leaves the screen
/// looking like it's doing nothing when it's actually still checking.
fn setup_status(state: &State) -> Element<'_, Message> {
    let ok = iced::Color::from_rgb8(0x4C, 0xAF, 0x50);
    let warn = iced::Color::from_rgb8(0xFF, 0xB7, 0x4D);
    let err = iced::Color::from_rgb8(0xE5, 0x73, 0x73);
    let dim = iced::Color::from_rgb8(0x9E, 0x9E, 0x9E);

    if !state.tools_checked {
        return text("Checking your setup...").size(13).color(dim).into();
    }

    if !state.binary_status.ready() {
        return text("Something needed to download videos is missing. Try reinstalling VidSave.")
            .size(13)
            .color(err)
            .into();
    }

    let mut warnings: Vec<Element<'_, Message>> = Vec::new();
    if state.binary_status.ffmpeg_path.is_none() {
        warnings.push(
            text("⚠ Merging video/audio and embedding thumbnails is unavailable right now.")
                .size(12)
                .color(warn)
                .into(),
        );
    }
    if state.binary_status.js_runtime.is_none() {
        warnings.push(
            text("⚠ Some videos may fail to download until VidSave is reinstalled.")
                .size(12)
                .color(warn)
                .into(),
        );
    }

    if warnings.is_empty() {
        text("Ready").size(12).color(ok).into()
    } else {
        column(warnings).spacing(4).into()
    }
}
