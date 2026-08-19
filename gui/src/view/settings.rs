//! The Settings panel: one row per `SettingsField`, grouped under the same
//! section headers as the TUI (`SettingsField::section`), each row rendered
//! according to `SettingsField::kind()` -- a checkbox for `Toggle`, a
//! dropdown for `Cycle` (an actual dropdown here, rather than the TUI's
//! left/right cycling, since a GUI can show every option at once), and a
//! text field for `Text`.

use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Center, Element, Fill};

use ytb_dl_tui_core::config::Settings;
use ytb_dl_tui_core::models::{AudioFormat, MediaMode, VideoContainer, VideoQuality};
use ytb_dl_tui_core::settings_fields::{FieldKind, SettingsField};

use crate::message::Message;
use crate::state::State;

const DIM: iced::Color = iced::Color {
    r: 0.55,
    g: 0.55,
    b: 0.55,
    a: 1.0,
};

pub fn view(state: &State) -> Element<'_, Message> {
    let mut rows: Vec<Element<'_, Message>> = Vec::new();
    for field in SettingsField::ALL {
        if let Some(header) = field.section() {
            rows.push(section_header(header));
        }
        rows.push(field_row(field, &state.settings));
    }

    let title = text("Settings").size(24);

    let save_label = if state.settings_saved_flash {
        "Saved ✓"
    } else {
        "Save to disk"
    };
    let footer = row![
        button(text("← Back").size(14))
            .style(button::secondary)
            .on_press(Message::CloseSettings),
        iced::widget::Space::new().width(Fill),
        button(text(save_label).size(14))
            .style(button::primary)
            .on_press(Message::SaveSettingsPressed),
    ]
    .align_y(Center);

    column![
        title,
        scrollable(column(rows).spacing(10).padding(iced::Padding {
            top: 0.0,
            right: 4.0,
            bottom: 0.0,
            left: 0.0,
        }),)
        .height(Fill),
        footer,
    ]
    .spacing(14)
    .height(Fill)
    .into()
}

fn section_header(title: &'static str) -> Element<'static, Message> {
    container(
        text(title.to_uppercase())
            .size(12)
            .color(iced::Color::from_rgb8(0x64, 0xB5, 0xF6)),
    )
    .padding(iced::Padding {
        top: 10.0,
        right: 0.0,
        bottom: 2.0,
        left: 0.0,
    })
    .into()
}

fn field_row(field: SettingsField, settings: &Settings) -> Element<'_, Message> {
    let relevant = field.relevant_for(settings.media_mode);
    let label_color = if relevant { None } else { Some(DIM) };
    let mut label = text(field.label()).size(14).width(230);
    if let Some(c) = label_color {
        label = label.color(c);
    }

    let control: Element<'_, Message> = match field.kind() {
        FieldKind::Toggle => checkbox(field.bool_value(settings))
            .on_toggle(move |_| Message::SettingsToggled(field))
            .into(),
        FieldKind::Text => text_input("", &field.text_value(settings))
            .on_input(move |value| Message::SettingsTextChanged(field, value))
            .padding(6)
            .width(280)
            .into(),
        FieldKind::Cycle => cycle_control(field, settings),
    };

    row![label, control].spacing(12).align_y(Center).into()
}

fn cycle_control(field: SettingsField, settings: &Settings) -> Element<'_, Message> {
    match field {
        SettingsField::MediaMode => pick_list(
            [MediaMode::VideoAudio, MediaMode::AudioOnly],
            Some(settings.media_mode),
            Message::MediaModePicked,
        )
        .into(),
        SettingsField::VideoQuality => pick_list(
            VideoQuality::ALL,
            Some(settings.video_quality),
            Message::VideoQualityPicked,
        )
        .into(),
        SettingsField::VideoContainer => pick_list(
            VideoContainer::ALL,
            Some(settings.video_container),
            Message::VideoContainerPicked,
        )
        .into(),
        SettingsField::AudioFormat => pick_list(
            AudioFormat::ALL,
            Some(settings.audio_format),
            Message::AudioFormatPicked,
        )
        .into(),
        _ => text("").into(),
    }
}
