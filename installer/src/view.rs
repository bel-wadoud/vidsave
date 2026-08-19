use iced::widget::{Space, button, checkbox, column, container, row, scrollable, text};
use iced::{Center, Element, Fill};

use crate::install_logic::InstallEvent;
use crate::{Message, Screen, State};

pub fn view(state: &State) -> Element<'_, Message> {
    let page = match state.screen {
        Screen::Welcome => welcome(),
        Screen::Components => components(state),
        Screen::Installing => installing(state),
        Screen::Finish => finish(state),
    };

    container(container(page).max_width(560))
        .width(Fill)
        .height(Fill)
        .padding(28)
        .center_x(Fill)
        .into()
}

fn welcome<'a>() -> Element<'a, Message> {
    column![
        text("ytb-dl-tui Setup").size(28),
        text(
            "This sets up ytb-dl-tui: a terminal UI and/or desktop GUI for \
             downloading YouTube playlists and videos, plus everything it \
             needs (a bundled Python runtime + yt-dlp, ffmpeg, and a JS \
             runtime) -- no admin rights required, installed just for you."
        )
        .size(14),
        row![Space::new().width(Fill), next_button("Next")].width(Fill),
    ]
    .spacing(20)
    .into()
}

fn components(state: &State) -> Element<'_, Message> {
    let tui_row = column![
        checkbox(state.install_tui)
            .label("Terminal UI  (ytb_dl_tui)")
            .on_toggle(Message::ToggleTui),
        text("Run from any terminal. Fully keyboard-driven.")
            .size(12)
            .color(dim()),
    ]
    .spacing(2);

    let gui_row = column![
        checkbox(state.install_gui)
            .label("Desktop GUI  (ytb-dl-tui)")
            .on_toggle(Message::ToggleGui),
        text("A normal windowed app, launchable from your Start Menu / app launcher.")
            .size(12)
            .color(dim()),
    ]
    .spacing(2);

    let can_continue = state.install_tui || state.install_gui;
    let mut next = button(text("Install").size(15))
        .style(button::primary)
        .padding([10, 24]);
    if can_continue {
        next = next.on_press(Message::NextPressed);
    }

    let warning: Element<'_, Message> = if can_continue {
        text("").into()
    } else {
        text("Pick at least one to continue.")
            .size(12)
            .color(warn_color())
            .into()
    };

    column![
        text("Choose what to install").size(24),
        tui_row,
        gui_row,
        warning,
        row![
            button(text("Back").size(14))
                .style(button::secondary)
                .on_press(Message::BackPressed),
            Space::new().width(Fill),
            next,
        ]
        .align_y(Center),
    ]
    .spacing(18)
    .into()
}

fn installing(state: &State) -> Element<'_, Message> {
    let lines = state.log.iter().map(log_line);
    column![
        text("Installing...").size(24),
        scrollable(column(lines).spacing(4)).height(360),
    ]
    .spacing(16)
    .into()
}

fn log_line(event: &InstallEvent) -> Element<'_, Message> {
    match event {
        InstallEvent::Step(s) => text(s.clone()).size(14).into(),
        InstallEvent::Detail(s) => text(format!("    {s}")).size(13).color(dim()).into(),
        InstallEvent::Warning(s) => text(format!("    {s}")).size(13).color(warn_color()).into(),
    }
}

fn finish(state: &State) -> Element<'_, Message> {
    let Some(outcome) = &state.outcome else {
        return text("").into();
    };

    let headline = if outcome.success {
        text("Done -- ytb-dl-tui is installed.")
            .size(22)
            .color(ok_color())
    } else {
        text("Finished with some problems -- see the log above.")
            .size(20)
            .color(warn_color())
    };

    let mut notes: Vec<Element<'_, Message>> = Vec::new();
    if state.install_tui && outcome.needs_new_terminal {
        notes.push(
            text("Open a NEW terminal window and run:  ytb_dl_tui")
                .size(13)
                .into(),
        );
    } else if state.install_tui {
        notes.push(
            text("Run it from any terminal:  ytb_dl_tui")
                .size(13)
                .into(),
        );
    }
    if state.install_gui && outcome.gui_shortcut_created {
        notes.push(
            text("A shortcut was added to your Start Menu / app launcher.")
                .size(13)
                .into(),
        );
    }

    let mut actions = row![].spacing(10).align_y(Center);
    if state.install_gui && outcome.success {
        actions = actions.push(
            button(text("Launch ytb-dl-tui").size(14))
                .style(button::primary)
                .padding([10, 20])
                .on_press(Message::LaunchGuiPressed),
        );
    }
    actions = actions.push(Space::new().width(Fill));
    actions = actions.push(
        button(text("Finish").size(14))
            .style(button::secondary)
            .padding([10, 20])
            .on_press(Message::FinishPressed),
    );

    column![headline, column(notes).spacing(6), actions]
        .spacing(20)
        .into()
}

fn next_button<'a>(label: &'a str) -> Element<'a, Message> {
    button(text(label).size(15))
        .style(button::primary)
        .padding([10, 24])
        .on_press(Message::NextPressed)
        .into()
}

fn dim() -> iced::Color {
    iced::Color::from_rgb8(0x9E, 0x9E, 0x9E)
}
fn warn_color() -> iced::Color {
    iced::Color::from_rgb8(0xFF, 0xB7, 0x4D)
}
fn ok_color() -> iced::Color {
    iced::Color::from_rgb8(0x4C, 0xAF, 0x50)
}
