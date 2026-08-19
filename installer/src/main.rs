//! ytb-dl-tui-install: a graphical setup wizard. Lets you choose the
//! terminal UI, the desktop GUI, or both, then installs whichever you
//! picked plus a bundled Python runtime + vendored yt-dlp (`../vendor/`),
//! ffmpeg, and a JS runtime (deno) into one dedicated per-user folder
//! (`%LOCALAPPDATA%\Programs\ytb-dl-tui` on Windows, `~/.local/share/
//! ytb-dl-tui` on Linux -- no admin/root needed), registers that folder on
//! PATH, and adds a Start Menu / app-launcher shortcut for the GUI.
//!
//! `--silent` skips the window entirely and installs both (or, combined
//! with `--no-tui` / `--no-gui`, just one) non-interactively, printing the
//! same progress the wizard would show -- for scripted/unattended installs.

mod download;
mod extract;
mod install_location;
mod install_logic;
mod path_env;
mod python_runtime;
mod shortcut;
mod tools;
mod view;

use iced::{Task, Theme};
use tokio_stream::wrappers::UnboundedReceiverStream;

use install_logic::{Components, InstallEvent, InstallOutcome};

/// The TUI and GUI binaries for this platform, embedded at compile time --
/// see `build.rs`, which requires both to exist in `embed/` before letting
/// this crate build at all. Always embedded regardless of what the user
/// ends up choosing on the Components page, same as any installer that
/// bundles optional components inside one package.
static TUI_BINARY: &[u8] = include_bytes!(env!("YTB_DL_TUI_TUI_BINARY_PATH"));
static GUI_BINARY: &[u8] = include_bytes!(env!("YTB_DL_TUI_GUI_BINARY_PATH"));

/// Our vendored copy of yt-dlp's Python source (see `../vendor/`), zipped
/// up at compile time by `build.rs`.
static YTDLP_VENDOR_ZIP: &[u8] = include_bytes!(env!("YTDLP_VENDOR_ZIP_PATH"));

pub fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--silent") {
        // No window, no runtime, no interaction: does the same install a
        // click-through of the wizard would, printing the same progress the
        // wizard would show, and exits with a status code -- for scripted/
        // unattended installs, and for CI to build the "portable bundle"
        // release asset without anything there to click Next.
        let components = Components {
            tui: !args.iter().any(|a| a == "--no-tui"),
            gui: !args.iter().any(|a| a == "--no-gui"),
        };
        std::process::exit(run_silent(components));
    }

    iced::application(State::default, update, view::view)
        .title("ytb-dl-tui Setup")
        .theme(|_state: &State| Theme::Dark)
        .window(iced::window::Settings {
            size: iced::Size::new(560.0, 520.0),
            resizable: true,
            ..Default::default()
        })
        .run()
}

/// `blocking_recv` (rather than needing any async runtime at all here) is
/// exactly what it's for: bridging a channel fed from synchronous code
/// (`run_install`, on its own thread) back to synchronous code (this one).
fn run_silent(components: Components) -> i32 {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let install_thread = std::thread::spawn(move || install_logic::run_install(components, tx));

    while let Some(event) = rx.blocking_recv() {
        match event {
            InstallEvent::Step(s) => println!("-- {s} --"),
            InstallEvent::Detail(s) => println!("   {s}"),
            InstallEvent::Warning(s) => eprintln!("   {s}"),
        }
    }

    let outcome = install_thread.join().expect("install thread panicked");
    if outcome.success { 0 } else { 1 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Welcome,
    Components,
    Installing,
    Finish,
}

struct State {
    screen: Screen,
    install_tui: bool,
    install_gui: bool,
    log: Vec<InstallEvent>,
    outcome: Option<InstallOutcome>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            screen: Screen::Welcome,
            install_tui: true,
            install_gui: true,
            log: Vec::new(),
            outcome: None,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    NextPressed,
    BackPressed,
    ToggleTui(bool),
    ToggleGui(bool),
    InstallProgress(InstallEvent),
    InstallFinished(InstallOutcome),
    LaunchGuiPressed,
    FinishPressed,
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::NextPressed => match state.screen {
            Screen::Welcome => {
                state.screen = Screen::Components;
                Task::none()
            }
            Screen::Components if state.install_tui || state.install_gui => {
                state.screen = Screen::Installing;
                begin_install(state)
            }
            _ => Task::none(),
        },
        Message::BackPressed => {
            state.screen = Screen::Welcome;
            Task::none()
        }
        Message::ToggleTui(value) => {
            state.install_tui = value;
            Task::none()
        }
        Message::ToggleGui(value) => {
            state.install_gui = value;
            Task::none()
        }
        Message::InstallProgress(event) => {
            state.log.push(event);
            Task::none()
        }
        Message::InstallFinished(outcome) => {
            state.outcome = Some(outcome);
            state.screen = Screen::Finish;
            Task::none()
        }
        Message::LaunchGuiPressed => {
            if let Some(outcome) = &state.outcome {
                let exe = outcome.install_dir.join(if cfg!(windows) {
                    "ytb_dl_tui_gui.exe"
                } else {
                    "ytb_dl_tui_gui"
                });
                let _ = std::process::Command::new(exe).spawn();
            }
            Task::none()
        }
        Message::FinishPressed => iced::exit(),
    }
}

fn begin_install(state: &State) -> Task<Message> {
    let components = Components {
        tui: state.install_tui,
        gui: state.install_gui,
    };
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = tokio::task::spawn_blocking(move || install_logic::run_install(components, tx));

    let progress = Task::run(UnboundedReceiverStream::new(rx), Message::InstallProgress);
    let finished = Task::perform(handle, |result| {
        Message::InstallFinished(result.expect("install task panicked"))
    });
    Task::batch([progress, finished])
}
