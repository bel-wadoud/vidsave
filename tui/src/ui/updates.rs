use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, UpdateStatus};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(format!("Running version {}", env!("CARGO_PKG_VERSION"))),
        Line::from(""),
    ];

    match &app.update_status {
        UpdateStatus::Idle => lines.push(Line::from("Enter/c  Check for updates")),
        UpdateStatus::Checking => lines.push(Line::from("Checking for updates...")),
        UpdateStatus::UpToDate => {
            lines.push(Line::from(ratatui::text::Span::styled(
                "You're up to date.",
                Style::default().fg(Color::Green),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from("Enter/c  Check again"));
        }
        UpdateStatus::Available(info) => {
            lines.push(Line::from(ratatui::text::Span::styled(
                format!("VidSave {} is available", info.version),
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(""));
            for note_line in info.notes.lines().take(10) {
                lines.push(Line::from(note_line.to_string()));
            }
            lines.push(Line::from(""));
            lines.push(Line::from("i  Download and install"));
        }
        UpdateStatus::Installing => {
            lines.push(Line::from(
                "Downloading and launching the installer... VidSave will close automatically.",
            ));
        }
        UpdateStatus::Error(message) => {
            lines.push(Line::from(ratatui::text::Span::styled(
                format!("Couldn't check for updates: {message}"),
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from("Enter/c  Try again"));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from("Esc, F3  Back      q  quit"));

    let block = Block::default().borders(Borders::ALL).title("Updates");
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}
