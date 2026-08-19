use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(8),
        Constraint::Min(4),
    ])
    .margin(2)
    .split(area);

    let title = Paragraph::new("VidSave")
        .style(Style::default().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    let input_border_color = if app.history_focused {
        Color::DarkGray
    } else {
        Color::Reset
    };
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(input_border_color))
        .title("Playlist / channel / video URL  (Enter to fetch)");
    let input = Paragraph::new(app.url_input.value()).block(input_block);
    frame.render_widget(input, chunks[1]);
    if !app.history_focused {
        frame.set_cursor_position((
            chunks[1].x + 1 + app.url_input.visual_cursor() as u16,
            chunks[1].y + 1,
        ));
    }

    let ytdlp_line = match (&app.binary_status.ytdlp_version, &app.binary_status.ytdlp) {
        (Some(v), Some(ytdlp)) => Line::from(vec![
            Span::raw("yt-dlp: "),
            Span::styled(v.clone(), Style::default().fg(Color::Green)),
            Span::raw(format!("  ({})", ytdlp.python_path.display())),
        ]),
        _ => Line::from(Span::styled(
            "yt-dlp: NOT FOUND -- bundled Python runtime is missing or broken",
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
        ytdlp_line,
        ffmpeg_line,
        js_runtime_line,
    ];
    if !app.binary_status.ready() {
        lines.push(Line::from(Span::styled(
            "Reinstall via the installer, or see README.md for manual setup",
            Style::default().fg(Color::Red),
        )));
    }

    let info = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Info"));
    frame.render_widget(info, chunks[2]);

    draw_history(frame, app, chunks[3]);
}

/// The "bigger list" below the URL input: every recorded download batch,
/// newest first. A playlist/channel entry drills into its video list on
/// `Enter`; a single-video entry goes straight to that video's details --
/// see `App::open_selected_history_entry`.
fn draw_history(frame: &mut Frame, app: &App, area: Rect) {
    if app.history.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Download history");
        frame.render_widget(
            Paragraph::new("Nothing downloaded yet -- finished batches show up here.").block(block),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = app
        .history
        .iter()
        .map(|entry| {
            let icon = if entry.is_single_video() {
                "•"
            } else {
                "▤"
            };
            let failed = entry.failed_count();
            let failed_suffix = if failed > 0 {
                format!(", {failed} failed")
            } else {
                String::new()
            };
            let line = format!(
                "{icon} {:<16} {}/{} done{failed_suffix}   {}",
                entry.finished_at_label(),
                entry.done_count(),
                entry.videos.len(),
                entry.title,
            );
            ListItem::new(line)
        })
        .collect();

    let border_color = if app.history_focused {
        Color::Reset
    } else {
        Color::DarkGray
    };
    let title = if app.history_focused {
        "Download history  (Enter open, Tab back to URL box)"
    } else {
        "Download history  (Tab to browse)"
    };
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(title),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default();
    if app.history_focused {
        state.select(Some(app.history_cursor));
    }
    frame.render_stateful_widget(list, area, &mut state);
}
