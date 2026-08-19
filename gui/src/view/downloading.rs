//! The downloading screen: one clearly-labeled overall-progress card at the
//! top, and a scrollable queue below where every video gets its own small
//! progress bar plus pause/resume/cancel controls. Raw yt-dlp output is
//! collapsed by default per item -- a normal user wants to know "how far
//! along is this" and "did it work", not read a log -- and can be expanded
//! with the "Details" toggle if they go looking for it.

use iced::widget::{Space, button, column, container, progress_bar, row, scrollable, text};
use iced::{Center, Element, Fill};

use vidsave_core::models::{DownloadItem, DownloadProgress, DownloadState};

use crate::message::Message;
use crate::state::State;

pub fn view(state: &State) -> Element<'_, Message> {
    let queue_rows = state
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| queue_row(state, i, item));
    let queue = scrollable(column(queue_rows).spacing(8)).height(Fill);

    column![overall_card(&state.items), queue, footer(state)]
        .spacing(14)
        .height(Fill)
        .into()
}

/// The whole-playlist progress card: a title (so it's never mistaken for one
/// video's bar), a single overall bar, and a plain-language stats line. Each
/// video's own bar lives down in `queue_row` instead -- this one only ever
/// shows the total.
fn overall_card(items: &[DownloadItem]) -> Element<'_, Message> {
    let overall = overall_percent(items);
    let done = items
        .iter()
        .filter(|i| matches!(i.state, DownloadState::Done | DownloadState::Skipped))
        .count();
    let failed = items
        .iter()
        .filter(|i| matches!(i.state, DownloadState::Failed(_)))
        .count();
    let cancelled = items
        .iter()
        .filter(|i| matches!(i.state, DownloadState::Cancelled))
        .count();

    let content = column![
        text("Overall progress").size(15),
        progress_bar(0.0..=100.0, overall as f32)
            .length(Fill)
            .girth(10),
        text(format!(
            "{done}/{} done   {failed} failed   {cancelled} cancelled   {overall:.0}% overall",
            items.len()
        ))
        .size(12),
    ]
    .spacing(6);

    container(content)
        .padding(12)
        .width(Fill)
        .style(container::rounded_box)
        .into()
}

fn queue_row<'a>(state: &'a State, index: usize, item: &'a DownloadItem) -> Element<'a, Message> {
    let (icon, color) = state_style(&item.state);
    let percent = item_percent(item);
    let expanded = state.expanded_items.contains(&index);

    let title_line = text(format!("{icon} {}", item.video.title))
        .size(13)
        .color(color);
    let status_line = text(format!("{}  {}", item.state.label(), extra_info(item))).size(11);

    let header = column![title_line, status_line].spacing(2).width(Fill);
    let bar = progress_bar(0.0..=100.0, percent as f32)
        .length(Fill)
        .girth(6);

    let controls = row![
        pause_resume_button(index, &item.state),
        cancel_button(index, &item.state),
        details_button(index, expanded),
    ]
    .spacing(6)
    .align_y(Center);

    let mut card = column![row![header, controls].spacing(10).align_y(Center), bar,].spacing(6);

    if expanded {
        card = card.push(item_log(item));
    }

    container(card)
        .padding(10)
        .width(Fill)
        .style(container::bordered_box)
        .into()
}

/// Pause while downloading, resume while paused -- one button, icon and
/// action swap together so it's never showing the wrong one. Hidden (as a
/// disabled placeholder) once the item has reached a terminal state, since
/// there's nothing left to pause or resume.
fn pause_resume_button(index: usize, state: &DownloadState) -> Element<'static, Message> {
    match state {
        DownloadState::Paused => button(text("▶ Resume").size(12))
            .style(button::success)
            .on_press(Message::ResumeItem(index))
            .into(),
        DownloadState::Queued
        | DownloadState::Starting
        | DownloadState::Downloading(_)
        | DownloadState::PostProcessing => button(text("‖ Pause").size(12))
            .style(button::secondary)
            .on_press(Message::PauseItem(index))
            .into(),
        DownloadState::Done
        | DownloadState::Skipped
        | DownloadState::Cancelled
        | DownloadState::Failed(_) => Space::new().width(0).into(),
    }
}

fn cancel_button(index: usize, state: &DownloadState) -> Element<'static, Message> {
    if state.is_terminal() {
        return Space::new().width(0).into();
    }
    button(text("✕ Cancel").size(12))
        .style(button::danger)
        .on_press(Message::CancelItem(index))
        .into()
}

fn details_button(index: usize, expanded: bool) -> Element<'static, Message> {
    let label = if expanded { "Hide details" } else { "Details" };
    button(text(label).size(12))
        .style(button::text)
        .on_press(Message::ToggleItemDetails(index))
        .into()
}

fn item_log(item: &DownloadItem) -> Element<'_, Message> {
    if item.log.is_empty() {
        return container(text("No output yet").size(11))
            .padding([4, 0])
            .into();
    }
    let log_text = item.log.join("\n");
    container(scrollable(text(log_text).size(11).font(iced::Font::MONOSPACE)).height(120))
        .padding([4, 0])
        .into()
}

fn extra_info(item: &DownloadItem) -> String {
    match &item.state {
        DownloadState::Downloading(p) => speed_eta(p),
        DownloadState::Paused => item
            .last_progress
            .as_ref()
            .map(speed_eta)
            .unwrap_or_default(),
        DownloadState::Failed(msg) => msg.clone(),
        _ => String::new(),
    }
}

fn speed_eta(p: &DownloadProgress) -> String {
    let speed = p.speed.clone().unwrap_or_else(|| "--".to_string());
    let eta = p.eta.clone().unwrap_or_else(|| "--".to_string());
    format!("{speed}  eta {eta}")
}

fn footer(state: &State) -> Element<'_, Message> {
    if state.batch_done {
        row![
            text("All downloads finished").size(14),
            Space::new().width(Fill),
            button(text("← Back to list").size(14))
                .style(button::secondary)
                .on_press(Message::BackToVideoList),
            button(text("Start over").size(14))
                .style(button::primary)
                .on_press(Message::StartOverPressed),
        ]
        .spacing(10)
        .align_y(Center)
        .into()
    } else {
        let cancel_all = button(text("Cancel all").size(13))
            .style(button::danger)
            .on_press(Message::CancelAllPressed);
        row![
            cancel_all,
            Space::new().width(Fill),
            button(text("← Back (keeps running)").size(14))
                .style(button::secondary)
                .on_press(Message::BackToVideoList),
        ]
        .spacing(10)
        .align_y(Center)
        .into()
    }
}

fn state_style(state: &DownloadState) -> (&'static str, iced::Color) {
    let gray = iced::Color::from_rgb8(0x9E, 0x9E, 0x9E);
    let yellow = iced::Color::from_rgb8(0xFF, 0xB7, 0x4D);
    let green = iced::Color::from_rgb8(0x4C, 0xAF, 0x50);
    let blue = iced::Color::from_rgb8(0x64, 0xB5, 0xF6);
    let cyan = iced::Color::from_rgb8(0x4D, 0xD0, 0xE1);
    let red = iced::Color::from_rgb8(0xE5, 0x73, 0x73);
    match state {
        DownloadState::Queued => ("○", gray),
        DownloadState::Starting => ("▶", yellow),
        DownloadState::Downloading(_) => ("▼", yellow),
        DownloadState::PostProcessing => ("~", yellow),
        DownloadState::Paused => ("‖", cyan),
        DownloadState::Done => ("✓", green),
        DownloadState::Skipped => ("−", blue),
        DownloadState::Cancelled => ("✗", gray),
        DownloadState::Failed(_) => ("!", red),
    }
}

/// One item's own progress, 0-100 -- keeps showing its last known position
/// while paused instead of dropping back to 0 just because it isn't
/// actively downloading right now.
fn item_percent(item: &DownloadItem) -> f64 {
    match &item.state {
        DownloadState::Downloading(p) => p.percent as f64,
        DownloadState::Paused => item
            .last_progress
            .as_ref()
            .map(|p| p.percent as f64)
            .unwrap_or(0.0),
        DownloadState::PostProcessing => 95.0,
        DownloadState::Done
        | DownloadState::Skipped
        | DownloadState::Cancelled
        | DownloadState::Failed(_) => 100.0,
        DownloadState::Queued | DownloadState::Starting => 0.0,
    }
}

fn overall_percent(items: &[DownloadItem]) -> f64 {
    if items.is_empty() {
        return 0.0;
    }
    let sum: f64 = items.iter().map(|item| item_percent(item) / 100.0).sum();
    (sum / items.len() as f64) * 100.0
}
