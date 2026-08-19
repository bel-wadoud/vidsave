use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;

pub fn draw(frame: &mut Frame, _app: &App, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(
            "VidSave",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from("Download YouTube playlists and videos."),
        Line::from(format!("Version {}", env!("CARGO_PKG_VERSION"))),
        Line::from(""),
        Line::from("Developer"),
        Line::from("  Name:     bel-wadoud"),
        Line::from("  GitHub:   github.com/bel-wadoud"),
        Line::from("  Contact:  abdelwadoud.belheraoui@proton.me"),
        Line::from(""),
        Line::from("Project"),
        Line::from("  Source:   github.com/bel-wadoud/vidsave"),
        Line::from("  License:  PolyForm Noncommercial 1.0.0"),
        Line::from("  Free to use, modify, and share for any noncommercial purpose."),
        Line::from(""),
        Line::from("Built with ratatui (terminal UI), iced (desktop UI), and yt-dlp."),
        Line::from(""),
        Line::from("Esc, F4  Back      q  quit"),
    ];

    let block = Block::default().borders(Borders::ALL).title("About");
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}
