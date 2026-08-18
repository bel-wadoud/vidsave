use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .margin(2)
    .split(area);

    let title = Paragraph::new("YouTube Playlist Downloader")
        .style(Style::default().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Playlist / channel / video URL  (Enter to fetch)");
    let input = Paragraph::new(app.url_input.value()).block(input_block);
    frame.render_widget(input, chunks[1]);
    frame.set_cursor_position((
        chunks[1].x + 1 + app.url_input.visual_cursor() as u16,
        chunks[1].y + 1,
    ));

    let ytdlp_line = match (
        &app.binary_status.ytdlp_version,
        &app.binary_status.ytdlp_path,
    ) {
        (Some(v), Some(path)) => Line::from(vec![
            Span::raw("yt-dlp: "),
            Span::styled(v.clone(), Style::default().fg(Color::Green)),
            Span::raw(format!("  ({})", path.display())),
        ]),
        _ => Line::from(Span::styled(
            "yt-dlp: NOT FOUND on PATH or next to this program",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
    };
    let ffmpeg_line = if let Some(path) = &app.binary_status.ffmpeg_path {
        Line::from(vec![
            Span::styled("ffmpeg: found", Style::default().fg(Color::Green)),
            Span::raw(format!("  ({})", path.display())),
        ])
    } else {
        Line::from(Span::styled(
            "ffmpeg: NOT FOUND (merging/thumbnail embedding disabled)",
            Style::default().fg(Color::Yellow),
        ))
    };
    let js_runtime_line = if let Some(js) = &app.binary_status.js_runtime {
        Line::from(vec![
            Span::styled(
                format!("JS runtime: {}", js.name),
                Style::default().fg(Color::Green),
            ),
            Span::raw(format!("  ({})", js.path.display())),
        ])
    } else {
        Line::from(Span::styled(
            "JS runtime: NOT FOUND -- some videos may wrongly report as unavailable",
            Style::default().fg(Color::Yellow),
        ))
    };

    let mut lines = vec![
        Line::from("Paste a YouTube playlist, channel, or single video URL above."),
        Line::from(""),
        ytdlp_line,
        ffmpeg_line,
        js_runtime_line,
        Line::from(""),
        Line::from("Enter  fetch      F2  settings      F1  help      Ctrl+C  quit"),
    ];
    if !app.binary_status.ready() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Get yt-dlp (no Python needed) and place it on PATH or next to this program -- see README.md",
            Style::default().fg(Color::Red),
        )));
    }

    let info = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Info"));
    frame.render_widget(info, chunks[2]);
}
