# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.1.5] - 2026-08-19

### Fixed

- Windows: `yt-dlp`/`ffmpeg` subprocesses no longer flash a console window
  while running -- this was also breaking downloads outright in some setups
  (the flashing console could steal focus/input from the app mid-download).
  The desktop app and installer no longer allocate a console window at all.

### Changed

- Desktop app: the downloading screen has been redesigned. Each video now
  gets its own small progress bar, and the overall playlist progress (a
  single clearly-labeled bar at the top) is separate from it, instead of one
  big unlabeled bar. Raw download output is collapsed by default per video
  ("Details" to expand) so the normal view only ever shows plain progress
  info.
- Desktop app: each video in the download queue now has its own
  pause/resume toggle (icon swaps between the two) and cancel button,
  instead of only a single "cancel selected" control.
- Desktop app: the status bar at the bottom always describes what's
  currently happening (starting up, fetching, downloading, ready, ...)
  instead of just showing the app name when idle.
- Desktop app and installer windows now have a proper icon.

## [2.1.0] - 2026-08-19

### Added

- A desktop GUI (`vidsave`), built with [iced](https://iced.rs),
  covering everything the terminal UI does: URL resolution, per-video
  selection with filtering, the full settings form, and a live download
  queue with per-item progress and cancellation. Shares every bit of
  config/download/yt-dlp logic with the terminal UI via a new shared
  `vidsave-core` library crate -- both frontends run the exact same code,
  not two parallel implementations.
- The installer is now a graphical setup wizard: Welcome -> choose the
  terminal UI, the desktop GUI, or both -> live install progress -> Finish,
  instead of a console script. Adds a Start Menu shortcut (Windows) / XDG
  desktop entry (Linux) for the GUI so it's launchable like any other
  installed app, not just from a terminal.
- `vidsave-install --silent` (with optional `--no-tui` / `--no-gui`)
  installs non-interactively for scripted/unattended setups, printing the
  same progress the wizard would show.
- Project restructured into a Cargo workspace (`core`, `tui`, `gui`,
  `installer`) with one shared lockfile and target directory.

## [2.0.0] - 2026-08-18

### Changed

- **Breaking:** no longer depends on a separately-installed `yt-dlp` binary.
  The app now bundles a pinned Python runtime and runs our own vendored copy
  of yt-dlp's source (see `vendor/`) instead of shelling out to a `yt-dlp`
  executable found on `PATH` or next to the app. If you built a manual
  "portable folder" per the old README by placing a `yt-dlp` binary next to
  `vidsave-tui` yourself, that no longer works -- use the installer or the new
  portable bundle release asset instead, both of which set this up
  automatically. `ffmpeg`/the JS runtime are unaffected: still resolved from
  `PATH` or the app's own folder.
- The installer now also fetches a portable Python interpreter and installs
  our vendored yt-dlp source in place of downloading yt-dlp's own release
  binary.
- The Docker image runs yt-dlp via the image's system `python3` and a copy of
  the vendored source, instead of downloading yt-dlp's release binary.
- Advanced/non-installer setups (e.g. custom Docker-like environments) can
  point at a different interpreter or yt-dlp source via the
  `VIDSAVE_PYTHON` / `VIDSAVE_YTDLP_SRC` environment variables.

### Fixed

- The installer (and the Docker image) now also install `ffprobe` alongside
  `ffmpeg`. yt-dlp looks for `ffprobe` in the same directory as whatever
  `--ffmpeg-location` points at, so without it, thumbnail/metadata embedding
  silently failed for anyone relying on the auto-installed ffmpeg rather
  than a system copy that happened to already have both.

### Added

- `vidsave-portable-{linux,windows}-x86_64.zip` release asset: the app
  plus its bundled Python runtime, vendored yt-dlp, ffmpeg, and deno, ready
  to run from an unzipped folder with no installer and no `PATH` changes.
- `vendor/update.sh` to refresh the vendored yt-dlp source to the latest
  upstream release.

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
- Self-contained `vidsave-install` installer that installs the app plus
  `yt-dlp`, `ffmpeg`, and `deno`, and registers the install directory on
  `PATH` -- no admin/root required.
- Docker image bundling the app with all of its runtime dependencies.

[2.1.5]: https://github.com/bel-wadoud/vidsave/releases/tag/v2.1.5
[2.1.0]: https://github.com/bel-wadoud/vidsave/releases/tag/v2.1.0
[2.0.0]: https://github.com/bel-wadoud/vidsave/releases/tag/v2.0.0
[1.0.0]: https://github.com/bel-wadoud/vidsave/releases/tag/v1.0.0
