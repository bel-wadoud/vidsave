use iced::widget::{Space, button, column, container, progress_bar, row, scrollable, text};
use iced::{Center, Element, Fill};

use ytb_dl_tui_core::models::{DownloadItem, DownloadState};

use crate::message::Message;
use crate::state::State;

pub fn view(state: &State) -> Element<'_, Message> {
    let overall = overall_percent(&state.items);
    let done = state
        .items
        .iter()
        .filter(|i| matches!(i.state, DownloadState::Done | DownloadState::Skipped))
        .count();
    let failed = state
        .items
        .iter()
        .filter(|i| matches!(i.state, DownloadState::Failed(_)))
        .count();
    let cancelled = state
        .items
        .iter()
        .filter(|i| matches!(i.state, DownloadState::Cancelled))
        .count();

    let progress = column![
        progress_bar(0.0..=100.0, overall as f32),
        text(format!(
            "{done}/{} done   {failed} failed   {cancelled} cancelled   {overall:.0}%",
            state.items.len()
        ))
        .size(13),
    ]
    .spacing(6);

    let queue_rows = state
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| queue_row(state, i, item));
    let queue = scrollable(column(queue_rows).spacing(4)).height(Fill);

    let log = log_pane(state);

    let body = row![
        container(queue).width(380),
        container(log).width(Fill).height(Fill),
    ]
    .spacing(14)
    .height(Fill);

    let has_selection = state.selected_queue_item.is_some();
    let mut cancel_item = button(text("Cancel selected").size(13)).style(button::danger);
    if has_selection && !state.batch_done {
        cancel_item =
            cancel_item.on_press(Message::CancelItem(state.selected_queue_item.unwrap_or(0)));
    }
    let mut cancel_all = button(text("Cancel all").size(13)).style(button::danger);
    if !state.batch_done {
        cancel_all = cancel_all.on_press(Message::CancelAllPressed);
    }

    let footer = if state.batch_done {
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
    } else {
        row![
            cancel_item,
            cancel_all,
            Space::new().width(Fill),
            button(text("← Back (keeps running)").size(14))
                .style(button::secondary)
                .on_press(Message::BackToVideoList),
        ]
    }
    .spacing(10)
    .align_y(Center);

    column![progress, body, footer]
        .spacing(14)
        .height(Fill)
        .into()
}

fn queue_row<'a>(state: &'a State, index: usize, item: &'a DownloadItem) -> Element<'a, Message> {
    let (icon, color) = state_style(&item.state);
    let extra = match &item.state {
        DownloadState::Downloading(p) => {
            let speed = p.speed.clone().unwrap_or_else(|| "--".to_string());
            let eta = p.eta.clone().unwrap_or_else(|| "--".to_string());
            format!("{speed}  eta {eta}")
        }
        DownloadState::Failed(msg) => msg.clone(),
        _ => String::new(),
    };

    let selected = state.selected_queue_item == Some(index);
    let label = column![
        text(format!(
            "{icon} {}  {}",
            item.state.label(),
            item.video.title
        ))
        .size(13)
        .color(color),
        text(extra).size(11),
    ]
    .spacing(2);

    let style = if selected {
        button::secondary
    } else {
        button::text
    };
    button(label)
        .style(style)
        .width(Fill)
        .padding(6)
        .on_press(Message::SelectQueueItem(index))
        .into()
}

fn log_pane(state: &State) -> Element<'_, Message> {
    let Some(index) = state.selected_queue_item else {
        return container(text("Select an item to see its log").size(13)).into();
    };
    let Some(item) = state.items.get(index) else {
        return container(text("")).into();
    };
    let title = format!("{} [{}]", item.video.title, item.video.id);
    let log_text = item.log.join("\n");
    column![
        text(title).size(14),
        scrollable(text(log_text).size(12).font(iced::Font::MONOSPACE)).height(Fill),
    ]
    .spacing(8)
    .into()
}

fn state_style(state: &DownloadState) -> (&'static str, iced::Color) {
    let gray = iced::Color::from_rgb8(0x9E, 0x9E, 0x9E);
    let yellow = iced::Color::from_rgb8(0xFF, 0xB7, 0x4D);
    let green = iced::Color::from_rgb8(0x4C, 0xAF, 0x50);
    let blue = iced::Color::from_rgb8(0x64, 0xB5, 0xF6);
    let red = iced::Color::from_rgb8(0xE5, 0x73, 0x73);
    match state {
        DownloadState::Queued => ("○", gray),
        DownloadState::Starting => ("▶", yellow),
        DownloadState::Downloading(_) => ("▼", yellow),
        DownloadState::PostProcessing => ("~", yellow),
        DownloadState::Done => ("✓", green),
        DownloadState::Skipped => ("−", blue),
        DownloadState::Cancelled => ("✗", gray),
        DownloadState::Failed(_) => ("!", red),
    }
}

fn overall_percent(items: &[DownloadItem]) -> f64 {
    if items.is_empty() {
        return 0.0;
    }
    let sum: f64 = items
        .iter()
        .map(|item| match &item.state {
            DownloadState::Queued | DownloadState::Starting => 0.0,
            DownloadState::Downloading(p) => (p.percent as f64 / 100.0).clamp(0.0, 1.0),
            DownloadState::PostProcessing => 0.95,
            DownloadState::Done
            | DownloadState::Skipped
            | DownloadState::Cancelled
            | DownloadState::Failed(_) => 1.0,
        })
        .sum();
    (sum / items.len() as f64) * 100.0
}
