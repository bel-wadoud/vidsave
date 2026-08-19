//! ytb_dl_tui_gui: the desktop GUI frontend. Shares every bit of
//! config/download/yt-dlp logic with the terminal UI via `ytb_dl_tui_core`
//! (see `../core`) -- only how the UI is drawn and driven differs.

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
use ytb_dl_tui_core::config::Settings;

pub fn main() -> iced::Result {
    iced::application(new, update::update, view::view)
        .title("ytb-dl-tui")
        .theme(|_state: &State| Theme::Dark)
        .window(iced::window::Settings {
            size: iced::Size::new(1000.0, 700.0),
            min_size: Some(iced::Size::new(760.0, 480.0)),
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
