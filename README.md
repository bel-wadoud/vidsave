# ytb-dl-tui

[![CI](https://github.com/bel-wadoud/ytb-dl-tui/actions/workflows/ci.yml/badge.svg)](https://github.com/bel-wadoud/ytb-dl-tui/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/bel-wadoud/ytb-dl-tui?label=release)](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest)
[![License: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/license-PolyForm%20Noncommercial%201.0.0-blue)](LICENSE)

A terminal UI for downloading YouTube playlists (and single videos) built
with [ratatui](https://ratatui.rs). It drives [`yt-dlp`](https://github.com/yt-dlp/yt-dlp)
under the hood for the actual extraction/downloading, and `ffmpeg` for
merging/embedding, so it stays correct as YouTube changes instead of
re-implementing extraction logic that breaks every few weeks.

## Download

The **installer** is the recommended way to get started -- it sets up
`ytb_dl_tui` itself plus every tool it needs (`yt-dlp`, `ffmpeg`, `deno`) in
one step, with no admin/root required. See [Installing](#installing) below
for exactly what it does before you run it.

[![Download for Windows](https://img.shields.io/badge/Download-Windows_Installer-0078D6?style=for-the-badge&logo=windows11&logoColor=white)](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb-dl-tui-install-windows-x86_64.exe)
[![Download for Linux](https://img.shields.io/badge/Download-Linux_Installer-FCC624?style=for-the-badge&logo=linux&logoColor=black)](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb-dl-tui-install-linux-x86_64)

Prefer a plain binary with no installer (you'll need `yt-dlp`, `ffmpeg`, and
`deno` on `PATH` or next to it yourself -- see [Manual / portable
setup](#manual--portable-setup)):

[![Download app for Windows](https://img.shields.io/badge/App_binary-Windows-lightgrey?style=flat-square&logo=windows11)](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb_dl_tui-windows-x86_64.exe)
[![Download app for Linux](https://img.shields.io/badge/App_binary-Linux-lightgrey?style=flat-square&logo=linux)](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb_dl_tui-linux-x86_64)

All four files are built and published automatically by [the release
workflow](.github/workflows/release.yml) from tagged source -- see
[Releases](https://github.com/bel-wadoud/ytb-dl-tui/releases) for every
version and its changelog. Prefer Docker, or building from source yourself?
Jump to [Docker](#docker) or [Building](#building).

## Features

- Paste a playlist, channel, or single-video URL and browse the resolved
  video list before downloading anything
- Downloading an actual playlist or channel automatically creates a
  subfolder named after it (sanitized for the filesystem) and puts every
  file from that batch inside -- a lone video URL downloads straight into
  the output directory with no extra nesting
- Every listed playlist entry is downloaded as given, including ones that
  happen to share the same underlying video (some playlists genuinely
  repeat an entry) -- see "download archive" below if you'd rather dedup
- Per-video selection with select all / none / invert and a live
  title/uploader filter
- Video or audio-only downloads, with:
  - Quality cap (4K down to 360p, or best/worst)
  - Container choice for video (MP4 / MKV / WebM)
  - Audio format choice (MP3 / M4A / Opus / FLAC / WAV / source codec)
  - Thumbnail, metadata, and chapter embedding
  - Subtitle download / embed, including auto-generated subtitles, with a
    configurable language list
  - SponsorBlock segment stripping
  - Rate limiting, retry count, proxy, and cookies-file support
  - Playlist start/end index range and reverse order
  - An optional "download archive" (off by default) to skip videos you've
    already downloaded in a previous run -- lives in the app's own data
    directory, independent of whichever output folder a given batch uses
  - Free-form extra `yt-dlp` arguments for anything not exposed directly
  - Configurable output directory and filename template
  - Configurable concurrent-download count
- A live download queue: overall progress gauge, per-item progress with
  speed/ETA, a scrollable log tail per item, and the ability to cancel a
  single item or the whole batch mid-flight
- Settings persist to disk (a TOML file in your platform config directory)
  so they carry over between runs
- Everything is keyboard-driven, with a context-sensitive help overlay
  (`F1`)

## Installing

Download the installer for your platform above (`ytb-dl-tui-install.exe` on
Windows, `ytb-dl-tui-install` on Linux) and run it -- double-click on
Windows, `chmod +x ./ytb-dl-tui-install && ./ytb-dl-tui-install` on Linux.
One file, no other downloads needed. It:

1. Installs `ytb_dl_tui` itself (embedded in the installer) plus
   [yt-dlp](https://github.com/yt-dlp/yt-dlp), [ffmpeg](https://ffmpeg.org/),
   and [deno](https://github.com/denoland/deno) (a JS runtime -- see below
   for why it matters) into one dedicated folder:
   - Windows: `%LOCALAPPDATA%\Programs\ytb-dl-tui`
   - Linux: `~/.local/share/ytb-dl-tui`

   No admin/root needed -- it's a per-user install, the same way VS Code or
   rustup install themselves.
2. Registers that folder on `PATH` (Windows: `HKEY_CURRENT_USER\Environment`
   in the registry, broadcasting the change so you don't have to reboot;
   Linux: appends a clearly marked, idempotent block to whichever of
   `~/.bashrc` / `~/.zshrc` / `~/.profile` already exist, or creates
   `~/.profile` if none do), so `ytb_dl_tui` runs from **any terminal, any
   directory** afterward -- **open a new terminal window** first, since an
   already-open one won't see the change.
3. Downloads yt-dlp/ffmpeg/deno freshly only if they're missing or broken;
   anything already on `PATH` and working, or already installed from a
   previous run, is left alone. Safe to re-run any time.

yt-dlp/ffmpeg/deno are fetched as **standalone binaries** straight from
their own releases -- yt-dlp's PyInstaller build bundles Python internally,
so nothing beyond the installer itself is ever required.

Why deno specifically: yt-dlp needs a JS runtime to solve YouTube's
per-video signature/challenge puzzles. **Without one, a meaningful
fraction of otherwise-perfectly-available videos fail to extract**
(reported as "not available" / `UNPLAYABLE`) -- this is the most common
cause of "some videos in a playlist just won't download" reports, and
isn't a bug in the download queue itself.

Once it's done, open a **new** terminal and run:

```sh
ytb_dl_tui
```

### Manual / portable setup

The app itself checks `PATH` and then its own folder for `yt-dlp`,
`ffmpeg`, and `deno` at every startup (and shows the resolved path, or a
warning, on the URL screen). So instead of the installer, you can grab the
[portable app binary](#download) and put it in a folder alongside your own
copies of `yt-dlp`, `ffmpeg`, and `deno` -- no installer, no PATH changes,
no registry edits.

### Building the installer yourself

It embeds a real `ytb_dl_tui` binary at compile time, so build that first,
for the same target:

```sh
./build-installer.sh                          # native (Linux)
./build-installer.sh x86_64-pc-windows-gnu     # Windows, via https://github.com/cross-rs/cross
```

## Building

Requires a recent stable [Rust toolchain](https://rustup.rs) (edition 2024).

```sh
cargo build --release
```

The binary is at `target/release/ytb_dl_tui`.

## Usage

```sh
# start at the blank URL prompt
ytb_dl_tui

# jump straight into a playlist
ytb_dl_tui 'https://www.youtube.com/playlist?list=...'

# override the configured output directory for this run only
ytb_dl_tui --output-dir ~/Downloads/videos 'https://www.youtube.com/watch?v=...'
```

### Keybindings

Global: `F1` toggles a help overlay with keys for the current screen;
`Ctrl+C` quits from anywhere.

| Screen | Keys |
|---|---|
| URL input | `Enter` fetch, `F2` settings |
| Video list | `Up`/`Down`/`j`/`k` move, `Space` toggle, `a`/`n`/`i` select all/none/invert, `/` filter, `s` or `F2` settings, `Enter` start download, `Esc` back, `q` quit |
| Settings | `Up`/`Down` move, `Left`/`Right` cycle a value, `Enter`/`Space` toggle/cycle/edit, `Shift+S` save to disk, `Esc`/`F2` back |
| Downloading | `Up`/`Down` move, `c` cancel selected item, `C` cancel everything, `Esc` back to the list (downloads keep running in the background), `q` quit |

### Configuration

Settings are saved (via `Shift+S` on the Settings screen) to:

- Linux: `~/.config/ytb-dl-tui/config.toml`
- macOS: `~/Library/Application Support/dev.ytb-dl-tui.ytb-dl-tui/config.toml`
- Windows: `%APPDATA%\ytb-dl-tui\ytb-dl-tui\config\config.toml`

Anything not covered by a dedicated setting can be passed through via the
"Extra yt-dlp arguments" field, which is appended verbatim to every
download invocation.

The "Filename template" field supports one token of our own, `{index}`,
alongside yt-dlp's usual `%(title)s`/`%(ext)s`/etc: it expands to `"N - "`
for a playlist entry (where N is that entry's real position) or nothing for
a lone video. Use `{index}`, not yt-dlp's own `%(playlist_index)s` -- since
we invoke yt-dlp once per video by its direct URL rather than through the
playlist, that field is always empty and can't be relied on to keep
same-video repeats from colliding on one filename.

## Docker

The image bundles the app plus yt-dlp, ffmpeg, and deno -- nothing to
install, nothing to configure.

```sh
docker build -t ytb-dl-tui .

# interactive: -it is required, this is a TUI
docker run --rm -it -v "$(pwd)/downloads:/downloads" ytb-dl-tui

# or jump straight into a playlist
docker run --rm -it -v "$(pwd)/downloads:/downloads" ytb-dl-tui 'https://www.youtube.com/playlist?list=...'
```

Downloads always land in `/downloads` inside the container by default (the
entrypoint passes `--output-dir /downloads`), so mounting a host directory
there with `-v` is the one thing you need to do to get files back out.
Settings saved from within the container (`Shift+S`) and the download
archive live under the container's own `/home/ytbdl`, which is not
persisted unless you mount a volume for it too.

## Project layout

```
src/                   the ytb_dl_tui app
  main.rs               terminal setup, the async event loop
  app.rs                application state machine
  input_handling.rs     keyboard -> state-change dispatch, one fn per screen
  settings_fields.rs    declarative metadata for every Settings screen row
  models.rs             Video/Playlist/DownloadState and friends
  config.rs             Settings struct + TOML persistence
  ytdlp.rs              yt-dlp process invocation, JSON parsing, arg building
  downloader.rs         concurrent download queue (bounded worker pool)
  ui/                   one ratatui render module per screen

installer/              standalone ytb-dl-tui-install crate (see "Installing")
  build.rs               requires embed/ytb_dl_tui(.exe) to exist before building
  embed/                 gitignored; build-installer.sh populates this
  src/install_location.rs  resolves the per-OS per-user install directory
  src/path_env.rs        registers that directory on PATH (registry / shell rc)
  src/tools.rs           per-tool metadata: URLs, archive format, version check
  src/download.rs        blocking HTTP GET with a progress bar
  src/extract.rs         zip/tar.xz handling
  src/main.rs            orchestration: install app, install tools, register PATH

.github/workflows/     CI (fmt/clippy/test on every push) and the tagged
                        release build that publishes the files under
                        "Download" above
build-installer.sh      builds the app then the installer that embeds it
Dockerfile              multi-stage build (see "Docker")
```

## Versioning & releases

This project follows [Semantic Versioning](https://semver.org/): given a
version `MAJOR.MINOR.PATCH`, `MAJOR` marks breaking changes (e.g. to the
config file format or CLI flags), `MINOR` adds functionality in a
backwards-compatible way, and `PATCH` is backwards-compatible fixes. See
[CHANGELOG.md](CHANGELOG.md) for what changed in each release, and
[Releases](https://github.com/bel-wadoud/ytb-dl-tui/releases) for the
built executables.

## License

Licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE): free to
use, modify, and share for any noncommercial purpose (personal use,
learning, research, etc.). Commercial use is not permitted under this
license -- see [LICENSE](LICENSE) for the full, exact terms.
