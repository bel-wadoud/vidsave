mod app;
mod cli;
mod input_handling;
mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::{mpsc, oneshot};

use crate::app::{App, DownloadSession};
use crate::cli::Cli;
use vidsave_core::config::Settings;
use vidsave_core::downloader::DownloadEvent;
use vidsave_core::models::PlaylistInfo;
use vidsave_core::ytdlp;

type Term = Terminal<CrosstermBackend<io::Stdout>>;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut settings = Settings::load();
    if let Some(dir) = cli.output_dir {
        settings.output_dir = dir;
    }

    let binary_status = ytdlp::check_binaries().await;

    let mut terminal = setup_terminal()?;
    let mut app = App::new(settings, binary_status, cli.url);
    // Silent, informational only -- surfaces on the Updates screen (F3)
    // plus a status line if one's found, never blocks or delays anything.
    app.begin_update_check();
    let result = run(&mut terminal, app).await;
    restore_terminal(&mut terminal)?;

    if let Err(e) = &result {
        eprintln!("error: {e:#}");
    }
    result
}

fn setup_terminal() -> Result<Term> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    // Make sure a panic doesn't leave the user's terminal in raw/alt-screen mode.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    Ok(terminal)
}

fn restore_terminal(terminal: &mut Term) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Reads crossterm input on a dedicated OS thread and forwards events over a
/// channel, so the async event loop never blocks waiting on stdin.
fn spawn_input_thread() -> mpsc::UnboundedReceiver<Event> {
    let (tx, rx) = mpsc::unbounded_channel();
    std::thread::spawn(move || {
        loop {
            match event::poll(Duration::from_millis(100)) {
                Ok(true) => match event::read() {
                    Ok(ev) => {
                        if tx.send(ev).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                },
                Ok(false) => {}
                Err(_) => break,
            }
        }
    });
    rx
}

/// Awaits a one-shot fetch result without busy-looping once it has already
/// resolved. Crucially, this polls the receiver *in place* (by mutable
/// reference) and only clears `*rx` after it actually completes: `select!`
/// drops the future of every branch that doesn't win a given iteration, so
/// taking the receiver out up front (before the `.await`) would silently
/// discard it -- and the fetch result with it -- the first time some other
/// branch (e.g. the tick timer) happened to be ready first.
async fn recv_fetch(
    rx: &mut Option<oneshot::Receiver<Result<PlaylistInfo>>>,
) -> Option<Result<PlaylistInfo>> {
    let result = match rx {
        Some(receiver) => receiver.await,
        None => return std::future::pending().await,
    };
    *rx = None;
    match result {
        Ok(result) => Some(result),
        Err(_) => Some(Err(anyhow::anyhow!("the fetch task ended unexpectedly"))),
    }
}

async fn recv_download_event(session: &mut Option<DownloadSession>) -> Option<DownloadEvent> {
    match session {
        Some(s) => s.events.recv().await,
        None => std::future::pending().await,
    }
}

/// Same "poll in place, only clear once it actually resolves" reasoning as
/// `recv_fetch` -- see its doc comment.
async fn recv_update_check(
    rx: &mut Option<
        oneshot::Receiver<Result<Option<vidsave_core::update_check::UpdateInfo>, String>>,
    >,
) -> Option<Result<Option<vidsave_core::update_check::UpdateInfo>, String>> {
    let result = match rx {
        Some(receiver) => receiver.await,
        None => return std::future::pending().await,
    };
    *rx = None;
    match result {
        Ok(result) => Some(result),
        Err(_) => Some(Err("the update check ended unexpectedly".to_string())),
    }
}

async fn recv_update_install(
    rx: &mut Option<oneshot::Receiver<Result<(), String>>>,
) -> Option<Result<(), String>> {
    let result = match rx {
        Some(receiver) => receiver.await,
        None => return std::future::pending().await,
    };
    *rx = None;
    match result {
        Ok(result) => Some(result),
        Err(_) => Some(Err("the update install task ended unexpectedly".to_string())),
    }
}

async fn run(terminal: &mut Term, mut app: App) -> Result<()> {
    let mut term_events = spawn_input_thread();
    let mut tick = tokio::time::interval(Duration::from_millis(150));

    loop {
        if app.should_quit {
            break;
        }

        terminal.draw(|f| ui::draw(f, &app))?;

        tokio::select! {
            Some(event) = term_events.recv() => {
                if let Event::Key(key) = event {
                    input_handling::handle_key(&mut app, key);
                }
            }
            result = recv_fetch(&mut app.fetch_rx), if app.fetch_rx.is_some() => {
                if let Some(result) = result {
                    app.on_fetch_result(result);
                }
            }
            event = recv_download_event(&mut app.downloader), if app.downloader.is_some() => {
                match event {
                    Some(ev) => app.on_download_event(ev),
                    None => app.on_download_channel_closed(),
                }
            }
            result = recv_update_check(&mut app.update_check_rx), if app.update_check_rx.is_some() => {
                if let Some(result) = result {
                    app.on_update_check_result(result);
                }
            }
            result = recv_update_install(&mut app.update_install_rx), if app.update_install_rx.is_some() => {
                if let Some(result) = result
                    && app.on_update_install_result(result)
                {
                    // The installer's been launched -- it needs this
                    // process' files unlocked to overwrite them.
                    break;
                }
            }
            _ = tick.tick() => {
                app.on_tick();
            }
        }
    }

    Ok(())
}
