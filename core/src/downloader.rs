//! Concurrent download queue: spawns one `yt-dlp` child process per video,
//! bounded to a configurable number of simultaneous downloads, and streams
//! structured progress back to the UI over a channel.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{Semaphore, mpsc};
use tokio_util::sync::CancellationToken;

use crate::config::Settings;
use crate::models::{DownloadProgress, Video};
use crate::ytdlp::{JsRuntime, ProgressLine, YtDlp, build_download_args, parse_progress_line};

/// Events emitted for a single queued video, identified by its index in the
/// original video list.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Started(usize),
    Progress(usize, DownloadProgress),
    PostProcessing(usize),
    Log(usize, String),
    Finished(usize, Result<(), String>),
    /// yt-dlp declined to re-download because the video is already recorded
    /// in the `--download-archive` file.
    Skipped(usize),
    /// The user cancelled this item outright (not resumable) -- see
    /// `Paused` for the "stopped, but can pick back up" counterpart.
    Cancelled(usize),
    /// The user paused this item. `DownloadHandle::resume_item` restarts it,
    /// picking up from yt-dlp's own partial-file continuation.
    Paused(usize),
}

/// The binaries resolved at startup (PATH, or sitting next to our own
/// executable), bundled together so download tasks work whether or not the
/// user has installed anything system-wide.
#[derive(Debug, Clone)]
pub struct BinaryPaths {
    pub ytdlp: YtDlp,
    pub ffmpeg: Option<PathBuf>,
    pub js_runtime: Option<JsRuntime>,
}

/// One item's stop signal, replaced wholesale on resume (see
/// `DownloadHandle::resume_item`) rather than reused, since a
/// `CancellationToken` is one-shot -- there's no "un-cancel". Cloning
/// shares the same signal (both halves point at the same token/flag) --
/// `DownloadHandle` keeps one copy to act on, `run_one` is handed the other.
#[derive(Clone)]
struct ItemControl {
    token: CancellationToken,
    /// Distinguishes *why* `token` was cancelled -- `run_one` reads this
    /// right after detecting cancellation to decide whether to report the
    /// item as `Paused` (resumable) or `Cancelled` (not).
    pause_requested: Arc<AtomicBool>,
}

impl ItemControl {
    fn new(parent: &CancellationToken) -> Self {
        Self {
            token: parent.child_token(),
            pause_requested: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Everything about a batch that every item needs but none of them mutate --
/// grouped together mainly so `run_one` doesn't need a parameter for each.
#[derive(Clone)]
struct BatchContext {
    settings: Arc<Settings>,
    semaphore: Arc<Semaphore>,
    binaries: BinaryPaths,
}

/// Cancellation/pause/resume handle for a batch kicked off by `start`, kept
/// separate from the event receiver (see `start`'s doc comment) so a
/// frontend can hand the receiver off wholesale -- e.g. wrapped in a
/// `Stream` for an async UI framework's own task/subscription system --
/// while still holding onto this to control individual items or the whole
/// batch.
pub struct DownloadHandle {
    items: Mutex<Vec<ItemControl>>,
    global_token: CancellationToken,
    /// Kept around (rather than only living in each spawned task's
    /// closure, as in the initial `start()` call) so `resume_item` can
    /// re-launch a single item on demand, long after the initial batch was
    /// spawned.
    videos: Vec<Video>,
    ctx: BatchContext,
    tx: mpsc::UnboundedSender<DownloadEvent>,
}

impl DownloadHandle {
    /// Cancel one queued/in-flight download by index. Not resumable --
    /// see `pause_item` for that.
    pub fn cancel_item(&self, index: usize) {
        if let Some(item) = self.items.lock().unwrap().get(index) {
            item.token.cancel();
        }
    }

    /// Cancel every download (e.g. user aborts the whole batch). Also not
    /// resumable, and takes priority over any individual item's pause
    /// state -- there's no per-item pause_requested flag to set here since
    /// stopping *everything* is unambiguously a hard cancel, not a "pause
    /// the whole batch to resume later" operation.
    pub fn cancel_all(&self) {
        self.global_token.cancel();
    }

    /// Stop one item, keeping whatever it already downloaded (yt-dlp writes
    /// `.part` files as it goes) so `resume_item` can pick up close to
    /// where it left off instead of starting over.
    pub fn pause_item(&self, index: usize) {
        if let Some(item) = self.items.lock().unwrap().get(index) {
            item.pause_requested.store(true, Ordering::Relaxed);
            item.token.cancel();
        }
    }

    /// Re-launches a paused (or cancelled, or failed -- anything not
    /// currently running) item from scratch as far as this process is
    /// concerned; yt-dlp itself resumes from the `.part` file on disk by
    /// default, so in practice this continues rather than restarts.
    pub fn resume_item(&self, index: usize) {
        let Some(video) = self.videos.get(index).cloned() else {
            return;
        };
        let control = ItemControl::new(&self.global_token);
        {
            let mut items = self.items.lock().unwrap();
            let Some(item) = items.get_mut(index) else {
                return;
            };
            *item = control.clone();
        }

        let ctx = self.ctx.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            run_one(index, video, ctx, control, tx).await;
        });
    }
}

/// Kicks off downloads for every video, respecting `settings.concurrency`.
///
/// Returns the cancellation handle and the event receiver separately rather
/// than bundled in one struct: a `DownloadHandle` is fine to hold in
/// long-lived UI state, while the receiver is very often consumed wholesale
/// by whatever mechanism a given frontend uses to turn an async stream into
/// UI events (a polling loop for the TUI; an owned `Stream` handed to the
/// async runtime for the GUI) -- forcing every caller to go through a
/// shared struct just to get at one `mpsc` field would only get in the way
/// of that.
pub fn start(
    videos: Vec<Video>,
    settings: Settings,
    binaries: BinaryPaths,
) -> (DownloadHandle, mpsc::UnboundedReceiver<DownloadEvent>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let global_token = CancellationToken::new();
    let ctx = BatchContext {
        semaphore: Arc::new(Semaphore::new(settings.concurrency.max(1))),
        settings: Arc::new(settings),
        binaries,
    };

    let mut items = Vec::with_capacity(videos.len());
    for (index, video) in videos.iter().enumerate() {
        let control = ItemControl::new(&global_token);
        items.push(control.clone());
        let video = video.clone();
        let ctx = ctx.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            run_one(index, video, ctx, control, tx).await;
        });
    }

    let handle = DownloadHandle {
        items: Mutex::new(items),
        global_token,
        videos,
        ctx,
        tx,
    };
    (handle, rx)
}

async fn run_one(
    index: usize,
    video: Video,
    ctx: BatchContext,
    control: ItemControl,
    tx: mpsc::UnboundedSender<DownloadEvent>,
) {
    let ItemControl {
        token,
        pause_requested,
    } = control;
    let stop_event = |pause_requested: &AtomicBool| {
        if pause_requested.load(Ordering::Relaxed) {
            DownloadEvent::Paused(index)
        } else {
            DownloadEvent::Cancelled(index)
        }
    };

    let permit = tokio::select! {
        biased;
        _ = token.cancelled() => None,
        permit = ctx.semaphore.acquire_owned() => permit.ok(),
    };
    let Some(_permit) = permit else {
        let _ = tx.send(stop_event(&pause_requested));
        return;
    };

    let _ = tx.send(DownloadEvent::Started(index));

    let args = build_download_args(
        &video,
        &ctx.settings,
        ctx.binaries.ffmpeg.as_deref(),
        ctx.binaries.js_runtime.as_ref(),
    );
    let mut cmd = ctx.binaries.ytdlp.command();
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(DownloadEvent::Finished(
                index,
                Err(format!("failed to launch yt-dlp: {e}")),
            ));
            return;
        }
    };

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    let already_archived = Arc::new(AtomicBool::new(false));
    let already_archived_writer = Arc::clone(&already_archived);
    let tx_out = tx.clone();
    let out_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            match parse_progress_line(&line) {
                ProgressLine::Downloading(p) => {
                    let _ = tx_out.send(DownloadEvent::Progress(index, p));
                }
                ProgressLine::PostProcessing => {
                    let _ = tx_out.send(DownloadEvent::PostProcessing(index));
                }
                ProgressLine::Finished => {}
                ProgressLine::Other => {
                    if line.contains("has already been recorded in the archive") {
                        already_archived_writer.store(true, Ordering::Relaxed);
                    }
                    let _ = tx_out.send(DownloadEvent::Log(index, line));
                }
            }
        }
    });

    let recent_stderr = Arc::new(Mutex::new(Vec::<String>::new()));
    let recent_stderr_writer = Arc::clone(&recent_stderr);
    let tx_err = tx.clone();
    let err_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            {
                let mut buf = recent_stderr_writer.lock().unwrap();
                buf.push(line.clone());
                const MAX_KEPT: usize = 20;
                if buf.len() > MAX_KEPT {
                    let excess = buf.len() - MAX_KEPT;
                    buf.drain(0..excess);
                }
            }
            let _ = tx_err.send(DownloadEvent::Log(index, line));
        }
    });

    let outcome = tokio::select! {
        status = child.wait() => Outcome::Exited(status),
        _ = token.cancelled() => Outcome::Cancelled,
    };

    if matches!(outcome, Outcome::Cancelled) {
        let _ = child.start_kill();
        let _ = child.wait().await;
    }

    let _ = out_task.await;
    let _ = err_task.await;

    let event = match outcome {
        Outcome::Cancelled => stop_event(&pause_requested),
        Outcome::Exited(Ok(status))
            if status.success() && already_archived.load(Ordering::Relaxed) =>
        {
            DownloadEvent::Skipped(index)
        }
        Outcome::Exited(Ok(status)) if status.success() => DownloadEvent::Finished(index, Ok(())),
        Outcome::Exited(Ok(status)) => {
            let tail = recent_stderr.lock().unwrap();
            let message = if tail.is_empty() {
                format!("yt-dlp exited with status {status}")
            } else {
                tail.iter()
                    .rev()
                    .take(3)
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" | ")
            };
            DownloadEvent::Finished(index, Err(message))
        }
        Outcome::Exited(Err(e)) => {
            DownloadEvent::Finished(index, Err(format!("failed waiting on yt-dlp: {e}")))
        }
    };
    let _ = tx.send(event);
}

enum Outcome {
    Exited(std::io::Result<std::process::ExitStatus>),
    Cancelled,
}
