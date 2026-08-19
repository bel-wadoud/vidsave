//! Everything that talks to yt-dlp (and, transitively, `ffmpeg`): presence
//! checks, playlist/video metadata resolution, download argument
//! construction, and progress-line parsing.
//!
//! Reimplementing YouTube extraction natively would mean re-solving cipher
//! and throttling changes YouTube ships regularly; yt-dlp already does this
//! well and is updated constantly, so we run it instead -- but rather than
//! depending on a separately-installed `yt-dlp` binary, we ship our own
//! pinned copy of its Python source (see `../vendor/`) plus a bundled
//! Python interpreter, and run `python -m yt_dlp` ourselves. See `YtDlp`
//! below and the installer's `python_runtime.rs` / `main.rs`.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use tokio::process::Command;

use crate::config::Settings;
use crate::models::{DownloadProgress, PlaylistInfo, Video};

/// Marker prefixes used in our custom `--progress-template` so we can pick
/// our structured lines out of yt-dlp's otherwise free-form stdout.
pub const DL_MARKER: &str = "@YDT-DL@";
pub const PP_MARKER: &str = "@YDT-PP@";

fn dl_template() -> String {
    format!(
        "{DL_MARKER}%(progress.status)s\t%(progress.downloaded_bytes)s\t%(progress.total_bytes)s\t%(progress.total_bytes_estimate)s\t%(progress.speed)s\t%(progress.eta)s"
    )
}

fn pp_template() -> String {
    format!("{PP_MARKER}%(progress.status)s")
}

/// A JavaScript engine yt-dlp can shell out to in order to solve YouTube's
/// per-video signature/challenge puzzles. `deno` is the only one yt-dlp will
/// use automatically; anything else (including an already-installed `node`)
/// has to be pointed out explicitly, which is what `to_arg` is for.
#[derive(Debug, Clone)]
pub struct JsRuntime {
    pub name: &'static str,
    pub path: PathBuf,
}

impl JsRuntime {
    fn to_arg(&self) -> String {
        format!("{}:{}", self.name, self.path.display())
    }
}

/// Our bundled `yt-dlp`: a specific pinned Python interpreter paired with
/// our vendored copy of yt-dlp's source (not a separately-installed
/// `yt-dlp` binary -- see the module docs above). Both pieces are installed
/// together, so unlike `ffmpeg`/the JS runtime this deliberately does *not*
/// fall back to searching `PATH`: a random system Python has no guarantee
/// of matching the version this was built and tested against.
#[derive(Debug, Clone)]
pub struct YtDlp {
    pub python_path: PathBuf,
    pub src_dir: PathBuf,
}

impl YtDlp {
    /// A `python -m yt_dlp` command, ready for args to be appended.
    pub fn command(&self) -> Command {
        let mut cmd = Command::new(&self.python_path);
        cmd.env("PYTHONPATH", &self.src_dir);
        cmd.arg("-m").arg("yt_dlp");
        cmd
    }
}

#[derive(Debug, Clone, Default)]
pub struct BinaryStatus {
    pub ytdlp: Option<YtDlp>,
    pub ytdlp_version: Option<String>,
    pub ffmpeg_path: Option<PathBuf>,
    pub js_runtime: Option<JsRuntime>,
}

impl BinaryStatus {
    pub fn ready(&self) -> bool {
        self.ytdlp.is_some()
    }
}

/// Where the installer places the bundled interpreter and vendored source,
/// relative to our own executable -- keep in sync with `runtime_dir` /
/// `ytdlp_src_dir` in the installer's `src/main.rs`. Overridable via env
/// vars for non-installer setups (e.g. the Docker image, which uses the
/// system `python3` and a source copy laid out differently -- see
/// `Dockerfile`).
fn resolve_ytdlp() -> Option<YtDlp> {
    let python_path = std::env::var_os("YTB_DL_TUI_PYTHON")
        .map(PathBuf::from)
        .or_else(bundled_python_path)?;
    let src_dir = std::env::var_os("YTB_DL_TUI_YTDLP_SRC")
        .map(PathBuf::from)
        .or_else(bundled_ytdlp_src_dir)?;

    (python_path.is_file() && src_dir.is_dir()).then_some(YtDlp {
        python_path,
        src_dir,
    })
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

fn bundled_python_path() -> Option<PathBuf> {
    let runtime_dir = exe_dir()?.join("python-runtime");
    Some(if cfg!(windows) {
        runtime_dir.join("python.exe")
    } else {
        runtime_dir.join("bin").join("python3")
    })
}

fn bundled_ytdlp_src_dir() -> Option<PathBuf> {
    Some(exe_dir()?.join("yt_dlp_src"))
}

/// Looks for `name` (e.g. `"ffmpeg"`, `"deno"`) on `PATH` first, then falls
/// back to a file of that name sitting next to our own executable. This lets
/// the whole toolchain ship as one folder -- app binary plus the standalone
/// ffmpeg/JS-runtime builds -- with no PATH setup or installation step at
/// all.
fn resolve_binary(name: &str) -> Option<PathBuf> {
    if let Ok(path) = which::which(name) {
        return Some(path);
    }

    let exe_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let candidate = exe_dir.join(&exe_name);
    candidate.is_file().then_some(candidate)
}

/// Without a JS runtime, yt-dlp falls back to player clients (e.g.
/// `android_vr`) that increasingly report perfectly-available videos as
/// UNPLAYABLE -- this is the actual cause behind "some videos in a playlist
/// just won't download" reports, not a bug in the download queue itself.
/// Prefer `deno` (yt-dlp's own default choice when present) but happily use
/// `node` or `bun` if that's what's already on the machine.
fn resolve_js_runtime() -> Option<JsRuntime> {
    for name in ["deno", "node", "bun"] {
        if let Some(path) = resolve_binary(name) {
            return Some(JsRuntime { name, path });
        }
    }
    None
}

/// Probe for our bundled yt-dlp, `ffmpeg`, and a JS runtime. Missing
/// `ffmpeg`/JS runtime are non-fatal (respectively: merging/embedding, and
/// some videos failing to extract) but missing yt-dlp is fatal.
pub async fn check_binaries() -> BinaryStatus {
    let ytdlp = resolve_ytdlp();
    let ffmpeg_path = resolve_binary("ffmpeg");
    let js_runtime = resolve_js_runtime();

    let ytdlp_version = match &ytdlp {
        Some(ytdlp) => ytdlp
            .command()
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()),
        None => None,
    };
    // If we found it but couldn't actually run it, treat yt-dlp as absent.
    let ytdlp = ytdlp.filter(|_| ytdlp_version.is_some());

    BinaryStatus {
        ytdlp,
        ytdlp_version,
        ffmpeg_path,
        js_runtime,
    }
}

/// Resolve a playlist (or a single video, which yt-dlp reports without an
/// `entries` array) into our internal model.
pub async fn fetch_playlist(
    url: &str,
    settings: &Settings,
    ytdlp: &YtDlp,
    js_runtime: Option<&JsRuntime>,
) -> Result<PlaylistInfo> {
    let mut cmd = ytdlp.command();
    cmd.arg("--flat-playlist")
        .arg("--dump-single-json")
        .arg("--no-warnings")
        .arg("--ignore-no-formats-error");

    if !settings.proxy.is_empty() {
        cmd.arg("--proxy").arg(&settings.proxy);
    }
    if !settings.cookies_file.is_empty() {
        cmd.arg("--cookies").arg(&settings.cookies_file);
    }
    if let Some(js) = js_runtime {
        cmd.arg("--js-runtimes").arg(js.to_arg());
    }
    cmd.arg(url);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = cmd
        .output()
        .await
        .context("failed to launch yt-dlp (bundled Python runtime missing or broken?)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("yt-dlp failed to resolve the URL:\n{}", stderr.trim());
    }

    let root: Value = serde_json::from_slice(&output.stdout)
        .context("yt-dlp returned output that could not be parsed as JSON")?;

    parse_playlist_json(&root, settings)
}

fn parse_playlist_json(root: &Value, settings: &Settings) -> Result<PlaylistInfo> {
    let playlist_title = root
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Untitled")
        .to_string();
    let uploader = root
        .get("uploader")
        .or_else(|| root.get("channel"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut videos: Vec<Video> = Vec::new();
    let is_playlist;

    if let Some(entries) = root.get("entries").and_then(Value::as_array) {
        is_playlist = true;
        for (i, entry) in entries.iter().enumerate() {
            if entry.is_null() {
                continue; // private/deleted/unavailable entries come back as null
            }
            if let Some(v) = video_from_json(entry, Some(i as u64 + 1)) {
                videos.push(v);
            }
        }
    } else {
        is_playlist = false;
        // A single video URL was given rather than a playlist -- no
        // playlist_index, so it won't get a spurious "1 - " filename prefix.
        if let Some(v) = video_from_json(root, None) {
            videos.push(v);
        }
    }

    if videos.is_empty() {
        bail!("no downloadable videos were found at that URL");
    }

    apply_playlist_range(&mut videos, settings.playlist_start, settings.playlist_end);

    if settings.playlist_reverse {
        videos.reverse();
    }

    Ok(PlaylistInfo {
        title: playlist_title,
        uploader,
        videos,
        is_playlist,
    })
}

fn apply_playlist_range(videos: &mut Vec<Video>, start: Option<u32>, end: Option<u32>) {
    videos.retain(|v| {
        let idx = v.playlist_index.unwrap_or(1) as u32;
        start.is_none_or(|s| idx >= s) && end.is_none_or(|e| idx <= e)
    });
}

fn video_from_json(entry: &Value, fallback_index: Option<u64>) -> Option<Video> {
    let id = entry.get("id").and_then(Value::as_str)?.to_string();
    let title = entry
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or(&id)
        .to_string();
    let uploader = entry
        .get("uploader")
        .or_else(|| entry.get("channel"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let duration_secs = entry
        .get("duration")
        .and_then(Value::as_f64)
        .map(|d| d.round() as u64);
    let playlist_index = entry
        .get("playlist_index")
        .and_then(Value::as_u64)
        .or(fallback_index);

    let url = entry
        .get("webpage_url")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            entry
                .get("url")
                .and_then(Value::as_str)
                .filter(|u| u.starts_with("http"))
                .map(str::to_string)
        })
        .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={id}"));

    Some(Video {
        id,
        title,
        uploader,
        duration_secs,
        playlist_index,
        url,
    })
}

/// Substitutes our own `{index}` token in a filename template with a
/// zero-cost "N - " prefix derived from `Video::playlist_index`.
///
/// yt-dlp's own `%(playlist_index)s` field is *not* usable here: we invoke
/// yt-dlp once per video, by that video's direct URL, so it never has
/// playlist context and the field is always empty. That silently collapses
/// every playlist entry down to just its title -- harmless normally, but
/// when a playlist repeats the same video under more than one entry (which
/// happens more often than you'd expect), every repeat renders to the
/// *identical* filename. Whichever one finishes first wins; yt-dlp sees the
/// destination already exists for the rest and reports success without
/// writing anything, so the queue shows them all as done despite only one
/// file existing. Since we already know each entry's true position from the
/// fetch step, we render the prefix ourselves instead of trusting yt-dlp's
/// (in this context, always-empty) template field.
fn render_filename_template(template: &str, video: &Video) -> String {
    let prefix = match video.playlist_index {
        Some(index) => format!("{index} - "),
        None => String::new(),
    };
    template.replace("{index}", &prefix)
}

/// Build the full `yt-dlp` argument list for downloading a single video
/// according to the current settings.
pub fn build_download_args(
    video: &Video,
    settings: &Settings,
    ffmpeg_path: Option<&Path>,
    js_runtime: Option<&JsRuntime>,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    let rendered_template = render_filename_template(&settings.filename_template, video);
    let output_path = settings.output_dir.join(rendered_template);
    args.push("-o".into());
    args.push(output_path.to_string_lossy().to_string());

    if let Some(js) = js_runtime {
        // Without this, yt-dlp can't solve YouTube's per-video signature
        // challenges and falls back to player clients that report many
        // otherwise-fine videos as UNPLAYABLE.
        args.push("--js-runtimes".into());
        args.push(js.to_arg());
    }

    if let Some(ffmpeg) = ffmpeg_path {
        // Tell yt-dlp exactly where ffmpeg lives so a copy sitting next to
        // our own executable (rather than on PATH) is still found.
        args.push("--ffmpeg-location".into());
        args.push(ffmpeg.to_string_lossy().to_string());
    }

    match settings.media_mode {
        crate::models::MediaMode::AudioOnly => {
            args.push("-x".into());
            if let Some(fmt) = settings.audio_format.ytdlp_name() {
                args.push("--audio-format".into());
                args.push(fmt.into());
            }
            args.push("--audio-quality".into());
            args.push("0".into());
        }
        crate::models::MediaMode::VideoAudio => {
            let selector = match settings.video_quality.height_cap() {
                Some(h) => format!("bestvideo[height<={h}]+bestaudio/best[height<={h}]/best"),
                None => "bestvideo+bestaudio/best".to_string(),
            };
            args.push("-f".into());
            args.push(selector);
            args.push("--merge-output-format".into());
            args.push(settings.video_container.ytdlp_name().into());
            if settings.embed_chapters {
                args.push("--embed-chapters".into());
            }
        }
    }

    if settings.embed_thumbnail {
        args.push("--embed-thumbnail".into());
    }
    if settings.embed_metadata {
        args.push("--embed-metadata".into());
    }
    if settings.write_subtitles {
        args.push("--write-subs".into());
        args.push("--sub-langs".into());
        args.push(settings.subtitle_langs.clone());
    }
    if settings.write_auto_subs {
        args.push("--write-auto-subs".into());
    }
    if settings.embed_subtitles {
        args.push("--embed-subs".into());
    }
    if settings.use_download_archive {
        args.push("--download-archive".into());
        args.push(settings.archive_path().to_string_lossy().to_string());
    }
    if !settings.rate_limit.is_empty() {
        args.push("--limit-rate".into());
        args.push(settings.rate_limit.clone());
    }
    args.push("--retries".into());
    args.push(settings.retries.to_string());
    if !settings.proxy.is_empty() {
        args.push("--proxy".into());
        args.push(settings.proxy.clone());
    }
    if !settings.cookies_file.is_empty() {
        args.push("--cookies".into());
        args.push(settings.cookies_file.clone());
    }
    if settings.sponsorblock_remove {
        args.push("--sponsorblock-remove".into());
        args.push("all".into());
    }

    args.extend(settings.extra_args_list());

    args.push("--newline".into());
    args.push("--no-color".into());
    args.push("--no-warnings".into());
    args.push("--progress-template".into());
    args.push(format!("download:{}", dl_template()));
    args.push("--progress-template".into());
    args.push(format!("postprocess:{}", pp_template()));

    args.push(video.url.clone());
    args
}

/// A structured event recovered from one line of yt-dlp stdout.
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressLine {
    Downloading(DownloadProgress),
    Finished,
    PostProcessing,
    Other,
}

/// Parses a numeric progress field. yt-dlp renders these as plain integers
/// for byte counts but as floats for speed/eta, so go through `f64` and
/// round rather than assuming an integer literal.
fn parse_bytes(s: &str) -> Option<u64> {
    if s.is_empty() || s == "NA" || s == "None" {
        None
    } else {
        s.parse::<f64>().ok().map(|v| v.round() as u64)
    }
}

/// Parse one line of yt-dlp stdout, recognizing our custom progress markers
/// and falling back to `Other` for plain log lines (still worth showing to
/// the user, just without structured progress).
pub fn parse_progress_line(line: &str) -> ProgressLine {
    if let Some(rest) = line.strip_prefix(DL_MARKER) {
        let fields: Vec<&str> = rest.split('\t').collect();
        if fields.len() >= 6 {
            let status = fields[0];
            let downloaded = parse_bytes(fields[1]);
            let total = parse_bytes(fields[2]).or_else(|| parse_bytes(fields[3]));
            let speed_bytes = parse_bytes(fields[4]);
            let eta_secs = parse_bytes(fields[5]);

            if status == "finished" {
                return ProgressLine::Finished;
            }

            let percent = match (downloaded, total) {
                (Some(d), Some(t)) if t > 0 => (d as f32 / t as f32) * 100.0,
                _ => 0.0,
            };
            let speed = speed_bytes.map(|b| format!("{}/s", human_bytes(b)));
            let eta = eta_secs.map(human_duration);

            return ProgressLine::Downloading(DownloadProgress {
                percent,
                speed,
                eta,
                downloaded_bytes: downloaded,
                total_bytes: total,
            });
        }
        return ProgressLine::Other;
    }

    if line.starts_with(PP_MARKER) {
        return ProgressLine::PostProcessing;
    }

    ProgressLine::Other
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value:.0}{}", UNITS[unit])
    } else {
        format!("{value:.1}{}", UNITS[unit])
    }
}

pub fn human_duration(total_secs: u64) -> String {
    crate::models::format_duration(total_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_downloading_progress_line() {
        let line = format!("{DL_MARKER}downloading\t25000\t100000\t100000\t51200\t75");
        match parse_progress_line(&line) {
            ProgressLine::Downloading(p) => {
                assert!((p.percent - 25.0).abs() < 0.01);
                assert_eq!(p.downloaded_bytes, Some(25000));
                assert_eq!(p.total_bytes, Some(100000));
                assert_eq!(p.speed.as_deref(), Some("50.0KiB/s"));
            }
            other => panic!("expected Downloading, got {other:?}"),
        }
    }

    #[test]
    fn falls_back_to_total_bytes_estimate_when_total_bytes_is_na() {
        let line = format!("{DL_MARKER}downloading\t1000\tNA\t4000\tNA\tNA");
        match parse_progress_line(&line) {
            ProgressLine::Downloading(p) => {
                assert_eq!(p.total_bytes, Some(4000));
                assert_eq!(p.speed, None);
                assert_eq!(p.eta, None);
            }
            other => panic!("expected Downloading, got {other:?}"),
        }
    }

    #[test]
    fn recognizes_finished_and_postprocess_markers() {
        let finished = format!("{DL_MARKER}finished\t100\t100\t100\t0\t0");
        assert_eq!(parse_progress_line(&finished), ProgressLine::Finished);

        let pp = format!("{PP_MARKER}started");
        assert_eq!(parse_progress_line(&pp), ProgressLine::PostProcessing);
    }

    #[test]
    fn plain_log_lines_pass_through_as_other() {
        assert_eq!(
            parse_progress_line("[youtube] Extracting URL"),
            ProgressLine::Other
        );
    }

    #[test]
    fn human_bytes_formats_units() {
        assert_eq!(human_bytes(512), "512B");
        assert_eq!(human_bytes(2048), "2.0KiB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.0MiB");
    }

    fn sample_video(id: &str, index: u64) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "title": format!("Video {id}"),
            "uploader": "Some Channel",
            "duration": 100,
            "playlist_index": index,
        })
    }

    #[test]
    fn parses_playlist_with_null_entries_filtered_out() {
        let root = serde_json::json!({
            "title": "My Playlist",
            "uploader": "Some Channel",
            "entries": [sample_video("a", 1), serde_json::Value::Null, sample_video("b", 3)],
        });
        let settings = Settings::default();
        let playlist = parse_playlist_json(&root, &settings).unwrap();
        assert_eq!(playlist.title, "My Playlist");
        assert_eq!(playlist.videos.len(), 2);
        assert_eq!(playlist.videos[0].id, "a");
        assert_eq!(playlist.videos[1].id, "b");
    }

    #[test]
    fn parses_single_video_without_entries_array() {
        let root = sample_video("solo", 1);
        let settings = Settings::default();
        let playlist = parse_playlist_json(&root, &settings).unwrap();
        assert_eq!(playlist.videos.len(), 1);
        assert_eq!(playlist.videos[0].id, "solo");
    }

    #[test]
    fn playlist_start_end_range_filters_by_index() {
        let settings = Settings {
            playlist_start: Some(2),
            playlist_end: Some(3),
            ..Settings::default()
        };
        let root = serde_json::json!({
            "title": "Ranged",
            "entries": [sample_video("a", 1), sample_video("b", 2), sample_video("c", 3), sample_video("d", 4)],
        });
        let playlist = parse_playlist_json(&root, &settings).unwrap();
        let ids: Vec<&str> = playlist.videos.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "c"]);
    }

    #[test]
    fn playlist_reverse_flips_order() {
        let settings = Settings {
            playlist_reverse: true,
            ..Settings::default()
        };
        let root = serde_json::json!({
            "title": "Reversed",
            "entries": [sample_video("a", 1), sample_video("b", 2)],
        });
        let playlist = parse_playlist_json(&root, &settings).unwrap();
        let ids: Vec<&str> = playlist.videos.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "a"]);
    }

    #[test]
    fn build_download_args_selects_audio_extraction_when_audio_only() {
        let settings = Settings {
            media_mode: crate::models::MediaMode::AudioOnly,
            audio_format: crate::models::AudioFormat::Mp3,
            ..Settings::default()
        };
        let video = Video {
            id: "abc".into(),
            title: "T".into(),
            uploader: None,
            duration_secs: None,
            playlist_index: None,
            url: "https://example.com/watch?v=abc".into(),
        };
        let args = build_download_args(&video, &settings, None, None);
        assert!(args.iter().any(|a| a == "-x"));
        assert!(args.windows(2).any(|w| w == ["--audio-format", "mp3"]));
        assert!(!args.iter().any(|a| a == "-f"));
    }

    #[test]
    fn build_download_args_caps_video_height_when_quality_set() {
        let settings = Settings {
            video_quality: crate::models::VideoQuality::P720,
            ..Settings::default()
        };
        let video = Video {
            id: "abc".into(),
            title: "T".into(),
            uploader: None,
            duration_secs: None,
            playlist_index: None,
            url: "https://example.com/watch?v=abc".into(),
        };
        let args = build_download_args(&video, &settings, None, None);
        let format_arg = args
            .windows(2)
            .find(|w| w[0] == "-f")
            .map(|w| w[1].clone())
            .expect("expected a -f argument");
        assert!(format_arg.contains("height<=720"));
    }

    #[test]
    fn build_download_args_passes_ffmpeg_location_when_given() {
        let settings = Settings::default();
        let video = Video {
            id: "abc".into(),
            title: "T".into(),
            uploader: None,
            duration_secs: None,
            playlist_index: None,
            url: "https://example.com/watch?v=abc".into(),
        };
        let ffmpeg = std::path::Path::new("/opt/tools/ffmpeg");
        let args = build_download_args(&video, &settings, Some(ffmpeg), None);
        assert!(
            args.windows(2)
                .any(|w| w[0] == "--ffmpeg-location" && w[1] == "/opt/tools/ffmpeg")
        );
    }

    #[test]
    fn render_filename_template_uses_index_for_playlist_entries() {
        let video = Video {
            id: "a".into(),
            title: "T".into(),
            uploader: None,
            duration_secs: None,
            playlist_index: Some(15),
            url: "u".into(),
        };
        assert_eq!(
            render_filename_template("{index}%(title)s.%(ext)s", &video),
            "15 - %(title)s.%(ext)s"
        );
    }

    #[test]
    fn render_filename_template_omits_index_for_lone_video() {
        let video = Video {
            id: "a".into(),
            title: "T".into(),
            uploader: None,
            duration_secs: None,
            playlist_index: None,
            url: "u".into(),
        };
        assert_eq!(
            render_filename_template("{index}%(title)s.%(ext)s", &video),
            "%(title)s.%(ext)s"
        );
    }

    /// Regression test for the actual reported bug: a playlist that lists
    /// the same underlying video at two different positions must not have
    /// both entries collapse onto one output filename (yt-dlp's own
    /// `%(playlist_index)s` is always empty in our per-video-URL invocation
    /// model, which is exactly what caused this).
    #[test]
    fn duplicate_titled_playlist_entries_get_distinct_output_filenames() {
        let settings = Settings::default();
        let make = |index| Video {
            id: "dup".into(),
            title: "Same Title".into(),
            uploader: None,
            duration_secs: None,
            playlist_index: Some(index),
            url: "u".into(),
        };
        let args_a = build_download_args(&make(3), &settings, None, None);
        let args_b = build_download_args(&make(9), &settings, None, None);
        let output_path = |args: &[String]| {
            args.windows(2)
                .find(|w| w[0] == "-o")
                .map(|w| w[1].clone())
                .expect("expected a -o argument")
        };
        assert_ne!(output_path(&args_a), output_path(&args_b));
    }

    #[test]
    fn single_video_json_without_playlist_index_field_has_none_index() {
        let root = serde_json::json!({
            "id": "solo2",
            "title": "Solo Video",
        });
        let settings = Settings::default();
        let playlist = parse_playlist_json(&root, &settings).unwrap();
        assert_eq!(playlist.videos[0].playlist_index, None);
        assert!(!playlist.is_playlist);
    }
}
