use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::App;
use vidsave_core::settings_fields::SettingsField;

enum Row {
    Header(&'static str),
    Field(SettingsField),
}

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .split(area);

    let title = Paragraph::new("Settings").block(
        Block::default()
            .borders(Borders::ALL)
            .title("Settings (F2/Esc back, Shift+S save)"),
    );
    frame.render_widget(title, chunks[0]);

    let mut rows = Vec::new();
    for field in SettingsField::ALL {
        if let Some(header) = field.section() {
            rows.push(Row::Header(header));
        }
        rows.push(Row::Field(field));
    }

    let current = app.current_field();
    let selected_index = rows
        .iter()
        .position(|r| matches!(r, Row::Field(f) if *f == current));

    let items: Vec<ListItem> = rows
        .iter()
        .map(|row| match row {
            Row::Header(h) => ListItem::new(Span::styled(
                format!("-- {h} --"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Row::Field(field) => {
                let relevant = field.relevant_for(app.settings.media_mode);
                let style = if relevant {
                    Style::default()
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let line = format!(
                    "{:<36} {}",
                    field.label(),
                    field.display_value(&app.settings)
                );
                ListItem::new(Span::styled(line, style))
            }
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Options"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(selected_index);
    frame.render_stateful_widget(list, chunks[1], &mut state);

    if app.editing {
        let field = app.current_field();
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Edit: {}  (Enter save, Esc cancel)", field.label()));
        frame.render_widget(
            Paragraph::new(app.edit_input.value()).block(block),
            chunks[2],
        );
        frame.set_cursor_position((
            chunks[2].x + 1 + app.edit_input.visual_cursor() as u16,
            chunks[2].y + 1,
        ));
    } else {
        let hints = "Up/Down move   Enter/Space edit or toggle   Left/Right cycle   Shift+S save to disk   Esc back";
        frame.render_widget(
            Paragraph::new(hints).block(Block::default().borders(Borders::ALL)),
            chunks[2],
        );
    }
}
