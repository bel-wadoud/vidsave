# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-08-18

Initial release.

### Added

- Terminal UI (built with [ratatui](https://ratatui.rs)) for resolving and
  downloading YouTube playlists, channels, and single videos via `yt-dlp`.
- Video list screen with per-video selection (select all / none / invert)
  and a live title/uploader filter.
- Video and audio-only download modes, with:
  - Quality cap (4K down to 360p, or best/worst).
  - Container choice for video (MP4 / MKV / WebM).
  - Audio format choice (MP3 / M4A / Opus / FLAC / WAV / source codec).
  - Thumbnail, metadata, and chapter embedding.
  - Subtitle download / embed, including auto-generated subtitles.
  - SponsorBlock segment stripping.
  - Rate limiting, retry count, proxy, and cookies-file support.
  - Playlist start/end index range and reverse order.
  - Optional download archive to skip already-downloaded videos.
  - Free-form extra `yt-dlp` arguments.
  - Configurable output directory, filename template, and concurrent
    download count.
- Live download queue: overall progress gauge, per-item progress with
  speed/ETA, a scrollable per-item log tail, and single-item or whole-batch
  cancellation.
- Settings persistence to a TOML file in the platform config directory.
- Context-sensitive keyboard help overlay (`F1`).
- Self-contained `ytb-dl-tui-install` installer that installs the app plus
  `yt-dlp`, `ffmpeg`, and `deno`, and registers the install directory on
  `PATH` -- no admin/root required.
- Docker image bundling the app with all of its runtime dependencies.

[1.0.0]: https://github.com/bel-wadoud/ytb-dl-tui/releases/tag/v1.0.0
