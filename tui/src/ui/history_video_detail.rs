use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let Some(video) = app.current_history_video() else {
        return;
    };

    let (outcome_text, outcome_color) = match &video.outcome {
        vidsave_core::history::HistoryOutcome::Done => ("Done", Color::Green),
        vidsave_core::history::HistoryOutcome::Skipped => ("Skipped (already had it)", Color::Blue),
        vidsave_core::history::HistoryOutcome::Cancelled => ("Cancelled", Color::DarkGray),
        vidsave_core::history::HistoryOutcome::Failed(_) => ("Failed", Color::Red),
    };

    let mut lines = vec![
        Line::from(vec![
            Span::raw("Title:      "),
            Span::raw(video.title.clone()),
        ]),
        Line::from(vec![
            Span::raw("Uploader:   "),
            Span::raw(video.uploader.clone().unwrap_or_else(|| "--".to_string())),
        ]),
        Line::from(vec![
            Span::raw("Length:     "),
            Span::raw(video.duration_label()),
        ]),
        Line::from(vec![
            Span::raw("Size:       "),
            Span::raw(video.size_label()),
        ]),
        Line::from(vec![
            Span::raw("URL:        "),
            Span::raw(video.url.clone()),
        ]),
        Line::from(vec![
            Span::raw("Result:     "),
            Span::styled(outcome_text, Style::default().fg(outcome_color)),
        ]),
    ];

    if let vidsave_core::history::HistoryOutcome::Failed(message) = &video.outcome {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Error: {message}"),
            Style::default().fg(Color::Red),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from("Esc  back      q  quit"));

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Video details");
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}
