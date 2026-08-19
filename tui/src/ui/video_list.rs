use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let Some(playlist) = &app.playlist else {
        return;
    };
    let filtered = app.filtered_video_indices();

    let mut constraints = vec![Constraint::Length(3)];
    if app.filtering {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Min(0));
    constraints.push(Constraint::Length(3));
    let chunks = Layout::vertical(constraints).split(area);

    let mut next = 0;
    let header_area = chunks[next];
    next += 1;
    let filter_area = if app.filtering {
        let a = chunks[next];
        next += 1;
        Some(a)
    } else {
        None
    };
    let list_area = chunks[next];
    next += 1;
    let hints_area = chunks[next];

    let uploader_suffix = playlist
        .uploader
        .as_ref()
        .map(|u| format!("  --  {u}"))
        .unwrap_or_default();
    let header_text = format!(
        "{}   ({} videos, {} selected{})",
        playlist.title,
        playlist.videos.len(),
        app.selected_count(),
        uploader_suffix
    );
    frame.render_widget(
        Paragraph::new(header_text).block(Block::default().borders(Borders::ALL).title("Playlist")),
        header_area,
    );

    if let Some(filter_area) = filter_area {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Filter (Enter to apply, Esc to clear)");
        frame.render_widget(
            Paragraph::new(app.filter_input.value()).block(block),
            filter_area,
        );
        frame.set_cursor_position((
            filter_area.x + 1 + app.filter_input.visual_cursor() as u16,
            filter_area.y + 1,
        ));
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|&idx| {
            let video = &playlist.videos[idx];
            let checked = if app.selected.get(idx).copied().unwrap_or(false) {
                "[x]"
            } else {
                "[ ]"
            };
            let index_label = video
                .playlist_index
                .map(|i| format!("{i:>3}"))
                .unwrap_or_else(|| "  -".to_string());
            let line = format!(
                "{checked} {index_label}  {:<8} {:<9} {}",
                video.duration_label(),
                video.size_label(),
                video.title
            );
            ListItem::new(line)
        })
        .collect();

    let list_title = if filtered.is_empty() && !app.filter_input.value().is_empty() {
        "Videos (no matches)".to_string()
    } else {
        "Videos".to_string()
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(list_title))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(app.video_cursor));
    frame.render_stateful_widget(list, list_area, &mut state);

    let hints = "Enter start   Space toggle   a all   n none   i invert   / filter   s settings   Esc back   q quit";
    frame.render_widget(
        Paragraph::new(hints).block(Block::default().borders(Borders::ALL)),
        hints_area,
    );
}
