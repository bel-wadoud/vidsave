//! VidSave's desktop GUI frontend. Shares every bit of config/download/
//! yt-dlp logic with the terminal version via `vidsave_core` (see
//! `../core`) -- only how the UI is drawn and driven differs.

// No console window on Windows: this is a windowed app, not a terminal one
// (unlike the TUI, which keeps the default "console" subsystem on purpose).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod cli;
mod message;
mod state;
mod update;
mod view;

use clap::Parser;
use iced::{Task, Theme};

use cli::Cli;
use message::Message;
use state::State;
use vidsave_core::config::Settings;

/// Raw 64x64 RGBA pixels for the window/taskbar icon (see `../../assets/`)
/// -- embedded straight as raw pixels rather than a PNG so setting it
/// doesn't need an image-decoding dependency just for this one startup
/// call. The `.exe` file/shortcut icon on Windows is a separate thing,
/// embedded as a resource by `build.rs`.
static ICON_RGBA: &[u8] = include_bytes!("../../assets/icon-64.rgba");

pub fn main() -> iced::Result {
    iced::application(new, update::update, view::view)
        .title("VidSave")
        .theme(|_state: &State| Theme::Dark)
        .window(iced::window::Settings {
            size: iced::Size::new(1000.0, 700.0),
            min_size: Some(iced::Size::new(760.0, 480.0)),
            icon: iced::window::icon::from_rgba(ICON_RGBA.to_vec(), 64, 64).ok(),
            ..Default::default()
        })
        .run()
}

fn new() -> (State, Task<Message>) {
    let cli = Cli::parse();
    let mut settings = Settings::load();
    if let Some(dir) = cli.output_dir {
        settings.output_dir = dir;
    }
    let state = State::new(settings, cli.url);
    (state, update::initial_task())
}
