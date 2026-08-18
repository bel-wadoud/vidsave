use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::App;
use crate::models::{DownloadItem, DownloadState};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .split(area);

    draw_overall_gauge(frame, app, chunks[0]);

    let body = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);
    draw_queue_list(frame, app, body[0]);
    draw_log_pane(frame, app, body[1]);

    let hints = if app.batch_done {
        "All downloads finished   Esc back to list   q quit"
    } else {
        "Up/Down select   c cancel item   C cancel all   Esc back (keeps running)   q quit"
    };
    frame.render_widget(
        Paragraph::new(hints).block(Block::default().borders(Borders::ALL)),
        chunks[2],
    );
}

fn draw_overall_gauge(frame: &mut Frame, app: &App, area: Rect) {
    let total = app.items.len();
    let done = app
        .items
        .iter()
        .filter(|i| matches!(i.state, DownloadState::Done | DownloadState::Skipped))
        .count();
    let failed = app
        .items
        .iter()
        .filter(|i| matches!(i.state, DownloadState::Failed(_)))
        .count();
    let cancelled = app
        .items
        .iter()
        .filter(|i| matches!(i.state, DownloadState::Cancelled))
        .count();
    let overall = overall_percent(&app.items);

    let label =
        format!("{done}/{total} done   {failed} failed   {cancelled} cancelled   {overall:.0}%");
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Overall progress"),
        )
        .gauge_style(Style::default().fg(Color::Green))
        .ratio((overall / 100.0).clamp(0.0, 1.0))
        .label(label);
    frame.render_widget(gauge, area);
}

fn draw_queue_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .items
        .iter()
        .map(|item| {
            let (icon, color) = state_style(&item.state);
            let extra = match &item.state {
                DownloadState::Downloading(p) => {
                    let speed = p.speed.clone().unwrap_or_else(|| "--".to_string());
                    let eta = p.eta.clone().unwrap_or_else(|| "--".to_string());
                    format!("  {speed}  eta {eta}")
                }
                DownloadState::Failed(msg) => format!("  {msg}"),
                _ => String::new(),
            };
            let line = format!(
                "{icon} {:<10} {}{}",
                item.state.label(),
                item.video.title,
                extra
            );
            ListItem::new(Span::styled(line, Style::default().fg(color)))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Queue"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(if app.items.is_empty() {
        None
    } else {
        Some(app.download_cursor)
    });
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_log_pane(frame: &mut Frame, app: &App, area: Rect) {
    let Some(item) = app.items.get(app.download_cursor) else {
        frame.render_widget(Block::default().borders(Borders::ALL).title("Log"), area);
        return;
    };
    let title = format!("Log: {} [{}]", item.video.title, item.video.id);
    let visible_lines = area.height.saturating_sub(2) as usize;
    let start = item.log.len().saturating_sub(visible_lines);
    let text = item.log[start..].join("\n");
    let para = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn state_style(state: &DownloadState) -> (&'static str, Color) {
    match state {
        DownloadState::Queued => ("o", Color::DarkGray),
        DownloadState::Starting => (">", Color::Yellow),
        DownloadState::Downloading(_) => ("v", Color::Yellow),
        DownloadState::PostProcessing => ("~", Color::Yellow),
        DownloadState::Done => ("+", Color::Green),
        DownloadState::Skipped => ("-", Color::Blue),
        DownloadState::Cancelled => ("x", Color::DarkGray),
        DownloadState::Failed(_) => ("!", Color::Red),
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
