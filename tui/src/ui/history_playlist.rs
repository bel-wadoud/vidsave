use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::App;
use vidsave_core::history::HistoryOutcome;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let Some(entry) = app.current_history_entry() else {
        return;
    };

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .split(area);

    let uploader_suffix = entry
        .uploader
        .as_ref()
        .map(|u| format!("  --  {u}"))
        .unwrap_or_default();
    let header = format!(
        "{}   ({} videos, {} done, {} failed{})   {}",
        entry.title,
        entry.videos.len(),
        entry.done_count(),
        entry.failed_count(),
        uploader_suffix,
        entry.finished_at_label(),
    );
    frame.render_widget(
        Paragraph::new(header).block(Block::default().borders(Borders::ALL).title("Playlist")),
        chunks[0],
    );

    let items: Vec<ListItem> = entry
        .videos
        .iter()
        .map(|v| {
            let (icon, color) = match &v.outcome {
                HistoryOutcome::Done => ("✓", Color::Green),
                HistoryOutcome::Skipped => ("−", Color::Blue),
                HistoryOutcome::Cancelled => ("✗", Color::DarkGray),
                HistoryOutcome::Failed(_) => ("!", Color::Red),
            };
            let line = format!(
                "{icon} {:<8} {:<9} {}",
                v.duration_label(),
                v.size_label(),
                v.title
            );
            ListItem::new(line).style(Style::default().fg(color))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Videos"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(app.history_video_cursor));
    frame.render_stateful_widget(list, chunks[1], &mut state);

    let hints = "Up/Down move   Enter view details   Esc back   q quit";
    frame.render_widget(
        Paragraph::new(hints).block(Block::default().borders(Borders::ALL)),
        chunks[2],
    );
}
