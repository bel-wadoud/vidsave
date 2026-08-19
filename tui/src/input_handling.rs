//! Translates raw crossterm key events into `App` state changes, one
//! function per screen. Kept separate from `app.rs` so state/transition
//! logic and "what does pressing this key do" stay easy to scan
//! independently.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tui_input::Input;

use crate::app::{App, FieldAction, MessageKind, Screen, SettingsOrigin, feed_input};
use vidsave_core::settings_fields::{FieldKind, SettingsField};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind == KeyEventKind::Release {
        return;
    }

    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return;
    }
    if key.code == KeyCode::F(1) {
        app.show_help = !app.show_help;
        return;
    }
    if app.show_help {
        if matches!(key.code, KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('q')) {
            app.show_help = false;
        }
        return;
    }

    match app.screen {
        Screen::UrlInput => handle_url_input(app, key),
        Screen::Fetching => handle_fetching(app, key),
        Screen::VideoList => handle_video_list(app, key),
        Screen::Settings => handle_settings(app, key),
        Screen::Downloading => handle_downloading(app, key),
        Screen::HistoryPlaylist => handle_history_playlist(app, key),
        Screen::HistoryVideoDetail => handle_history_video_detail(app, key),
    }
}

fn handle_url_input(app: &mut App, key: KeyEvent) {
    // Tab moves focus between the URL box and the history list below it --
    // only meaningful when there's actually history to browse.
    if key.code == KeyCode::Tab && !app.history.is_empty() {
        app.history_focused = !app.history_focused;
        return;
    }

    if app.history_focused {
        let max_cursor = app.history.len().saturating_sub(1);
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.history_cursor = app.history_cursor.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.history_cursor = (app.history_cursor + 1).min(max_cursor)
            }
            KeyCode::Enter => app.open_selected_history_entry(),
            KeyCode::Esc => app.history_focused = false,
            KeyCode::F(2) => {
                app.settings_origin = SettingsOrigin::UrlInput;
                app.screen = Screen::Settings;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Enter => {
            let url = app.url_input.value().trim().to_string();
            if url.is_empty() {
                app.set_status("Enter a playlist or video URL first", MessageKind::Error);
            } else if !app.binary_status.ready() {
                app.set_status(
                    "yt-dlp was not found on PATH -- install it before fetching",
                    MessageKind::Error,
                );
            } else {
                app.begin_fetch(url);
            }
        }
        KeyCode::F(2) => {
            app.settings_origin = SettingsOrigin::UrlInput;
            app.screen = Screen::Settings;
        }
        _ => feed_input(&mut app.url_input, &Event::Key(key)),
    }
}

fn handle_history_playlist(app: &mut App, key: KeyEvent) {
    let max_cursor = app
        .current_history_entry()
        .map(|e| e.videos.len().saturating_sub(1))
        .unwrap_or(0);
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.history_video_cursor = app.history_video_cursor.saturating_sub(1)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.history_video_cursor = (app.history_video_cursor + 1).min(max_cursor)
        }
        KeyCode::Enter => app.open_selected_history_video(),
        KeyCode::Esc => app.back_from_history_playlist(),
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_history_video_detail(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.back_from_history_video_detail(),
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_fetching(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::Esc {
        app.fetch_rx = None;
        app.screen = Screen::UrlInput;
        app.set_status("Fetch cancelled", MessageKind::Info);
    }
}

fn handle_video_list(app: &mut App, key: KeyEvent) {
    if app.filtering {
        match key.code {
            KeyCode::Enter => app.filtering = false,
            KeyCode::Esc => {
                app.filter_input = Input::default();
                app.filtering = false;
            }
            _ => {
                feed_input(&mut app.filter_input, &Event::Key(key));
                app.video_cursor = 0;
            }
        }
        return;
    }

    let filtered = app.filtered_video_indices();
    let max_cursor = filtered.len().saturating_sub(1);

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.video_cursor = app.video_cursor.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => {
            app.video_cursor = (app.video_cursor + 1).min(max_cursor)
        }
        KeyCode::Home => app.video_cursor = 0,
        KeyCode::End => app.video_cursor = max_cursor,
        KeyCode::Char(' ') => {
            if let Some(&index) = filtered.get(app.video_cursor) {
                app.toggle_selected(index);
            }
        }
        KeyCode::Char('a') => app.set_filtered_selection(true),
        KeyCode::Char('n') => app.set_filtered_selection(false),
        KeyCode::Char('i') => app.invert_filtered_selection(),
        KeyCode::Char('/') => app.filtering = true,
        KeyCode::F(2) | KeyCode::Char('s') => {
            app.settings_origin = SettingsOrigin::VideoList;
            app.screen = Screen::Settings;
        }
        KeyCode::Esc => {
            app.playlist = None;
            app.selected.clear();
            app.screen = Screen::UrlInput;
        }
        KeyCode::Enter => app.start_downloads(),
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_settings(app: &mut App, key: KeyEvent) {
    if app.editing {
        match key.code {
            KeyCode::Enter => {
                let field = app.current_field();
                app.commit_edit_field(field);
            }
            KeyCode::Esc => app.cancel_edit(),
            _ => feed_input(&mut app.edit_input, &Event::Key(key)),
        }
        return;
    }

    let last = SettingsField::ALL.len() - 1;
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.settings_cursor = app.settings_cursor.saturating_sub(1)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.settings_cursor = (app.settings_cursor + 1).min(last)
        }
        KeyCode::Left | KeyCode::Char('h') => {
            let field = app.current_field();
            app.apply_settings_action(field, FieldAction::Left);
        }
        KeyCode::Right | KeyCode::Char('l') => {
            let field = app.current_field();
            app.apply_settings_action(field, FieldAction::Right);
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let field = app.current_field();
            match field.kind() {
                FieldKind::Toggle => app.apply_settings_action(field, FieldAction::Activate),
                FieldKind::Cycle => app.apply_settings_action(field, FieldAction::Right),
                FieldKind::Text => app.begin_edit_field(field),
            }
        }
        KeyCode::Char('S') => app.save_settings(),
        KeyCode::Esc | KeyCode::F(2) => {
            app.screen = match app.settings_origin {
                SettingsOrigin::UrlInput => Screen::UrlInput,
                SettingsOrigin::VideoList => Screen::VideoList,
            };
        }
        _ => {}
    }
}

fn handle_downloading(app: &mut App, key: KeyEvent) {
    let max_cursor = app.items.len().saturating_sub(1);
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.download_cursor = app.download_cursor.saturating_sub(1)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.download_cursor = (app.download_cursor + 1).min(max_cursor)
        }
        KeyCode::Char('c') => {
            if let Some(dm) = &app.downloader {
                dm.handle.cancel_item(app.download_cursor);
            }
        }
        KeyCode::Char('C') => {
            if let Some(dm) = &app.downloader {
                dm.handle.cancel_all();
            }
        }
        KeyCode::Char('p') => {
            if let Some(dm) = &app.downloader {
                dm.handle.pause_item(app.download_cursor);
            }
        }
        KeyCode::Char('r') => {
            if let Some(dm) = &app.downloader {
                dm.handle.resume_item(app.download_cursor);
            }
        }
        KeyCode::Esc => app.screen = Screen::VideoList,
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}
