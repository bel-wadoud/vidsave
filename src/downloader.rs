//! Concurrent download queue: spawns one `yt-dlp` child process per video,
//! bounded to a configurable number of simultaneous downloads, and streams
//! structured progress back to the UI over a channel.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

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
    Cancelled(usize),
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

pub struct DownloadManager {
    pub events: mpsc::UnboundedReceiver<DownloadEvent>,
    item_tokens: Vec<CancellationToken>,
    global_token: CancellationToken,
}

impl DownloadManager {
    /// Kick off downloads for every video, respecting `settings.concurrency`.
    pub fn start(videos: Vec<Video>, settings: Settings, binaries: BinaryPaths) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let global_token = CancellationToken::new();
        let semaphore = Arc::new(Semaphore::new(settings.concurrency.max(1)));
        let settings = Arc::new(settings);

        let mut item_tokens = Vec::with_capacity(videos.len());
        for (index, video) in videos.into_iter().enumerate() {
            let token = global_token.child_token();
            item_tokens.push(token.clone());
            let tx = tx.clone();
            let semaphore = Arc::clone(&semaphore);
            let settings = Arc::clone(&settings);
            let binaries = binaries.clone();
            tokio::spawn(async move {
                run_one(index, video, settings, semaphore, token, tx, binaries).await;
            });
        }

        Self {
            events: rx,
            item_tokens,
            global_token,
        }
    }

    /// Cancel one queued/in-flight download by index.
    pub fn cancel_item(&self, index: usize) {
        if let Some(token) = self.item_tokens.get(index) {
            token.cancel();
        }
    }

    /// Cancel every download (e.g. user aborts the whole batch).
    pub fn cancel_all(&self) {
        self.global_token.cancel();
    }
}

async fn run_one(
    index: usize,
    video: Video,
    settings: Arc<Settings>,
    semaphore: Arc<Semaphore>,
    token: CancellationToken,
    tx: mpsc::UnboundedSender<DownloadEvent>,
    binaries: BinaryPaths,
) {
    let permit = tokio::select! {
        biased;
        _ = token.cancelled() => None,
        permit = semaphore.acquire_owned() => permit.ok(),
    };
    let Some(_permit) = permit else {
        let _ = tx.send(DownloadEvent::Cancelled(index));
        return;
    };

    let _ = tx.send(DownloadEvent::Started(index));

    let args = build_download_args(
        &video,
        &settings,
        binaries.ffmpeg.as_deref(),
        binaries.js_runtime.as_ref(),
    );
    let mut cmd = binaries.ytdlp.command();
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
        Outcome::Cancelled => DownloadEvent::Cancelled(index),
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
