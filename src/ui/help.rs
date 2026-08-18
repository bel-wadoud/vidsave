use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::Screen;

pub fn draw(frame: &mut Frame, screen: Screen) {
    let area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, area);

    let mut lines = vec![Line::from("Global: F1 help   Ctrl+C quit"), Line::from("")];
    match screen {
        Screen::UrlInput => lines.extend([
            Line::from("Enter          Fetch playlist / video"),
            Line::from("F2             Open settings"),
        ]),
        Screen::Fetching => lines.push(Line::from("Esc            Cancel fetch")),
        Screen::VideoList => lines.extend([
            Line::from("Up/Down, j/k   Move cursor"),
            Line::from("Space          Toggle selection"),
            Line::from("a / n / i      Select all / none / invert"),
            Line::from("/              Filter by title or uploader"),
            Line::from("s, F2          Settings"),
            Line::from("Enter          Start downloading the selection"),
            Line::from("Esc            Back to URL screen"),
            Line::from("q              Quit"),
        ]),
        Screen::Settings => lines.extend([
            Line::from("Up/Down, j/k   Move cursor"),
            Line::from("Left/Right     Cycle enum value"),
            Line::from("Enter/Space    Toggle, cycle, or edit the field"),
            Line::from("Shift+S        Save settings to disk"),
            Line::from("Esc, F2        Back"),
        ]),
        Screen::Downloading => lines.extend([
            Line::from("Up/Down, j/k   Move cursor"),
            Line::from("c              Cancel the selected item"),
            Line::from("C              Cancel everything"),
            Line::from("Esc            Back to video list (downloads keep running)"),
            Line::from("q              Quit"),
        ]),
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Help (F1 or Esc to close)");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}
