//! Download history: a persistent record of past download batches, written
//! once a batch finishes (see each frontend's `push_history_entry` call at
//! the point `batch_done` becomes true) and read back on the URL input
//! screen so a normal user can see what they downloaded before -- and, for
//! a playlist/channel batch, drill into which of its videos actually
//! finished vs. failed -- without digging through logs or the filesystem.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::models::{DownloadItem, DownloadState, PlaylistInfo};

/// What ultimately happened to one video in a finished batch. A flattened,
/// serializable counterpart to `DownloadState` -- history only ever records
/// *terminal* outcomes (a batch isn't recorded until every item reaches
/// one), so there's no `Queued`/`Downloading`/`Paused` case to represent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HistoryOutcome {
    Done,
    Skipped,
    Cancelled,
    Failed(String),
}

impl HistoryOutcome {
    pub fn label(&self) -> &'static str {
        match self {
            HistoryOutcome::Done => "Done",
            HistoryOutcome::Skipped => "Skipped",
            HistoryOutcome::Cancelled => "Cancelled",
            HistoryOutcome::Failed(_) => "Failed",
        }
    }

    /// `None` for a state history never records (still in progress) --
    /// only reachable if something upstream calls this before the batch
    /// actually finished.
    fn from_state(state: &DownloadState) -> Option<Self> {
        match state {
            DownloadState::Done => Some(HistoryOutcome::Done),
            DownloadState::Skipped => Some(HistoryOutcome::Skipped),
            DownloadState::Cancelled => Some(HistoryOutcome::Cancelled),
            DownloadState::Failed(msg) => Some(HistoryOutcome::Failed(msg.clone())),
            DownloadState::Queued
            | DownloadState::Starting
            | DownloadState::Downloading(_)
            | DownloadState::PostProcessing
            | DownloadState::Paused => None,
        }
    }
}

/// One video's recorded result within a `HistoryEntry`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryVideoEntry {
    pub title: String,
    pub uploader: Option<String>,
    pub duration_secs: Option<u64>,
    pub filesize_bytes: Option<u64>,
    pub url: String,
    pub outcome: HistoryOutcome,
}

impl HistoryVideoEntry {
    pub fn duration_label(&self) -> String {
        match self.duration_secs {
            Some(s) => crate::models::format_duration(s),
            None => "--:--".to_string(),
        }
    }

    pub fn size_label(&self) -> String {
        match self.filesize_bytes {
            Some(b) => crate::ytdlp::human_bytes(b),
            None => "--".to_string(),
        }
    }
}

/// One finished download batch: a playlist/channel (its videos kept
/// individually, so a click can drill into any one of them) or a lone video
/// (recorded the same way, just with a single entry in `videos` -- see
/// `is_single_video`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Unique within one history file -- unix-time-plus-nanos, not a UUID,
    /// since this never needs to be unique outside this one user's own
    /// history.json.
    pub id: String,
    pub title: String,
    pub source_url: String,
    pub uploader: Option<String>,
    pub is_playlist: bool,
    /// Unix timestamp (seconds) of when the batch finished.
    pub finished_at: u64,
    pub videos: Vec<HistoryVideoEntry>,
}

impl HistoryEntry {
    /// Builds a history entry from a just-finished batch. `None` if `items`
    /// is empty or nothing in it has reached a terminal state yet -- never
    /// call this before `batch_done`.
    pub fn from_batch(playlist: &PlaylistInfo, items: &[DownloadItem]) -> Option<Self> {
        if items.is_empty() {
            return None;
        }
        let videos: Vec<HistoryVideoEntry> = items
            .iter()
            .filter_map(|item| {
                let outcome = HistoryOutcome::from_state(&item.state)?;
                Some(HistoryVideoEntry {
                    title: item.video.title.clone(),
                    uploader: item.video.uploader.clone(),
                    duration_secs: item.video.duration_secs,
                    filesize_bytes: item.video.filesize_bytes,
                    url: item.video.url.clone(),
                    outcome,
                })
            })
            .collect();
        if videos.is_empty() {
            return None;
        }
        Some(HistoryEntry {
            id: unique_id(),
            title: playlist.title.clone(),
            source_url: playlist.source_url.clone(),
            uploader: playlist.uploader.clone(),
            is_playlist: playlist.is_playlist,
            finished_at: unix_now(),
            videos,
        })
    }

    /// A lone-video batch is recorded the same way as a playlist (one entry
    /// either way), but the UI shows it differently: no point drilling into
    /// a "list" of exactly one video when `is_playlist` is already false --
    /// see each frontend's history screen.
    pub fn is_single_video(&self) -> bool {
        !self.is_playlist && self.videos.len() == 1
    }

    pub fn done_count(&self) -> usize {
        self.videos
            .iter()
            .filter(|v| v.outcome == HistoryOutcome::Done)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.videos
            .iter()
            .filter(|v| matches!(v.outcome, HistoryOutcome::Failed(_)))
            .count()
    }

    /// `"2026-08-19 14:32 UTC"`. Deliberately UTC rather than the viewer's
    /// local time -- converting correctly needs either a date/time
    /// dependency or platform-specific `unsafe` calls, neither of which is
    /// worth it just for this one label.
    pub fn finished_at_label(&self) -> String {
        format_unix_utc(self.finished_at)
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn unique_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}-{}", now.as_secs(), now.subsec_nanos())
}

/// Newest-first list of every recorded batch, capped at `MAX_ENTRIES` so
/// `history.json` can't grow forever. Missing/unreadable/corrupt file reads
/// back as an empty history rather than an error -- losing history is never
/// worth surfacing as a failure to a normal user.
const MAX_ENTRIES: usize = 200;

pub fn history_path() -> Option<PathBuf> {
    crate::config::Settings::project_dirs().map(|d| d.data_dir().join("history.json"))
}

pub fn load_history() -> Vec<HistoryEntry> {
    let Some(path) = history_path() else {
        return Vec::new();
    };
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

/// Records one finished batch, newest-first, trimming anything past
/// `MAX_ENTRIES`.
pub fn push_history_entry(entry: HistoryEntry) -> anyhow::Result<()> {
    let path =
        history_path().ok_or_else(|| anyhow::anyhow!("could not determine data directory"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut entries = load_history();
    entries.insert(0, entry);
    entries.truncate(MAX_ENTRIES);
    let contents = serde_json::to_string_pretty(&entries)?;
    std::fs::write(&path, contents)?;
    Ok(())
}

/// Formats a unix timestamp as `"YYYY-MM-DD HH:MM UTC"`. Implemented by
/// hand (the well-known "civil_from_days" algorithm) rather than pulling in
/// a date/time crate just for this one display string.
fn format_unix_utc(secs: u64) -> String {
    let secs = secs as i64;
    let days = secs.div_euclid(86400);
    let time_of_day = secs.rem_euclid(86400);
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;

    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02} UTC")
}

/// Howard Hinnant's `civil_from_days`: days-since-unix-epoch -> (year, month, day).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DownloadProgress, Video};

    fn video(title: &str) -> Video {
        Video {
            id: title.to_string(),
            title: title.to_string(),
            uploader: Some("Uploader".to_string()),
            duration_secs: Some(125),
            playlist_index: Some(1),
            url: format!("https://example.com/{title}"),
            filesize_bytes: Some(1024 * 1024),
        }
    }

    fn playlist() -> PlaylistInfo {
        PlaylistInfo {
            title: "My Playlist".to_string(),
            uploader: Some("Uploader".to_string()),
            videos: vec![],
            is_playlist: true,
            source_url: "https://example.com/list".to_string(),
        }
    }

    #[test]
    fn from_batch_maps_terminal_states_to_outcomes() {
        let mut done = DownloadItem::new(video("a"));
        done.set_state(DownloadState::Done);
        let mut failed = DownloadItem::new(video("b"));
        failed.set_state(DownloadState::Failed("boom".to_string()));

        let entry = HistoryEntry::from_batch(&playlist(), &[done, failed]).unwrap();
        assert_eq!(entry.videos.len(), 2);
        assert_eq!(entry.done_count(), 1);
        assert_eq!(entry.failed_count(), 1);
        assert_eq!(
            entry.videos[1].outcome,
            HistoryOutcome::Failed("boom".into())
        );
    }

    #[test]
    fn from_batch_ignores_non_terminal_items() {
        let mut still_going = DownloadItem::new(video("a"));
        still_going.set_state(DownloadState::Downloading(DownloadProgress::default()));
        assert!(HistoryEntry::from_batch(&playlist(), &[still_going]).is_none());
    }

    #[test]
    fn from_batch_returns_none_for_empty_items() {
        assert!(HistoryEntry::from_batch(&playlist(), &[]).is_none());
    }

    #[test]
    fn single_video_batch_is_not_treated_as_playlist_drill_down() {
        let mut item = DownloadItem::new(video("solo"));
        item.set_state(DownloadState::Done);
        let mut info = playlist();
        info.is_playlist = false;
        let entry = HistoryEntry::from_batch(&info, &[item]).unwrap();
        assert!(entry.is_single_video());
    }

    #[test]
    fn civil_from_days_matches_known_date() {
        // 2024-01-01 is 19723 days after the unix epoch.
        assert_eq!(civil_from_days(19723), (2024, 1, 1));
        // The epoch itself.
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }
}
