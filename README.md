# ytb-dl-tui

[![CI](https://github.com/bel-wadoud/ytb-dl-tui/actions/workflows/ci.yml/badge.svg)](https://github.com/bel-wadoud/ytb-dl-tui/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/bel-wadoud/ytb-dl-tui?label=release)](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest)
[![License: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/license-PolyForm%20Noncommercial%201.0.0-blue)](LICENSE)

Download YouTube playlists and videos, as a terminal UI or a desktop GUI.

## Download

[![Download for Windows](https://img.shields.io/badge/Download-Windows-0078D6?style=for-the-badge&logo=windows11&logoColor=white)](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb-dl-tui-install-windows-x86_64.exe)
[![Download for Linux](https://img.shields.io/badge/Download-Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb-dl-tui-install-linux-x86_64)

Run the downloaded file (double-click on Windows, or `chmod +x` then run it
on Linux) and a setup window walks you through it: pick the terminal UI,
the desktop GUI, or both, and it installs everything needed -- no admin/root
required. The GUI gets a Start Menu / app-launcher shortcut; the terminal UI
runs as `ytb_dl_tui` from any **new** terminal window.

Prefer no installer? Grab the portable bundle instead -- unzip it anywhere
and run the app binary inside directly, nothing added to `PATH`:
[Windows](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb-dl-tui-portable-windows-x86_64.zip) /
[Linux](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb-dl-tui-portable-linux-x86_64.zip).
A [Docker image](#docker) is also available (terminal UI only).

## Usage

Launch **ytb-dl-tui** from your Start Menu / app launcher for the desktop
GUI, or run the terminal UI from any terminal:

```sh
# start at the blank URL prompt
ytb_dl_tui

# jump straight into a playlist
ytb_dl_tui 'https://www.youtube.com/playlist?list=...'
```

Paste a playlist, channel, or video URL, pick which videos you want,
adjust quality/format/subtitles/etc. in Settings, then start the download.
Both frontends share the same settings file and download logic -- only how
you interact with them differs.

### Terminal UI keybindings

`F1` toggles help for the current screen; `Ctrl+C` quits from anywhere.

| Screen | Keys |
|---|---|
| URL input | `Enter` fetch, `F2` settings |
| Video list | `Up`/`Down` move, `Space` toggle, `a`/`n`/`i` select all/none/invert, `/` filter, `Enter` start download |
| Settings | `Up`/`Down` move, `Left`/`Right`/`Enter` change value, `Shift+S` save |
| Downloading | `c` cancel selected, `C` cancel all, `Esc` back to the list |

Settings are saved to a config file in your platform's usual config
directory (e.g. `~/.config/ytb-dl-tui/config.toml` on Linux).

## Docker

```sh
docker build -t ytb-dl-tui .
docker run --rm -it -v "$(pwd)/downloads:/downloads" ytb-dl-tui
```

The image only runs the terminal UI (a GUI needs a display, which a
container doesn't have).

## Building from source

Requires a recent stable [Rust toolchain](https://rustup.rs).

```sh
cargo build --release -p ytb_dl_tui       # terminal UI
cargo build --release -p ytb-dl-tui-gui   # desktop GUI
```

## License

[PolyForm Noncommercial License 1.0.0](LICENSE) -- free to use, modify, and
share for any noncommercial purpose. Commercial use is not permitted.
