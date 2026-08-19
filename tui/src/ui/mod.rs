//! Top-level render dispatch: picks the widget tree for the active screen
//! and draws the shared status bar / help overlay on top.

mod downloading;
mod help;
mod settings;
mod url_input;
mod video_list;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;

use crate::app::{App, MessageKind, Screen};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    match app.screen {
        Screen::UrlInput => url_input::draw(frame, app, chunks[0]),
        Screen::Fetching => draw_fetching(frame, app, chunks[0]),
        Screen::VideoList => video_list::draw(frame, app, chunks[0]),
        Screen::Settings => settings::draw(frame, app, chunks[0]),
        Screen::Downloading => downloading::draw(frame, app, chunks[0]),
    }

    draw_status_bar(frame, app, chunks[1]);

    if app.show_help {
        help::draw(frame, app.screen);
    }
}

fn draw_fetching(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::widgets::{Block, Borders};

    const FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
    let spinner = FRAMES[(app.fetch_spinner_frame / 2) % FRAMES.len()];
    let text = format!(
        "{spinner} Resolving {} ...\n\nEsc to cancel",
        app.pending_url
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Fetching playlist");
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (text, style) = if let Some(status) = &app.status {
        let color = match status.kind {
            MessageKind::Info => Color::Green,
            MessageKind::Error => Color::Red,
        };
        (status.text.clone(), Style::default().fg(color))
    } else {
        ("F1 help".to_string(), Style::default().fg(Color::DarkGray))
    };
    frame.render_widget(Paragraph::new(text).style(style), area);
}
