# ytb-dl-tui

[![CI](https://github.com/bel-wadoud/ytb-dl-tui/actions/workflows/ci.yml/badge.svg)](https://github.com/bel-wadoud/ytb-dl-tui/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/bel-wadoud/ytb-dl-tui?label=release)](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest)
[![License: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/license-PolyForm%20Noncommercial%201.0.0-blue)](LICENSE)

A terminal UI for downloading YouTube playlists and videos.

## Download

[![Download for Windows](https://img.shields.io/badge/Download-Windows-0078D6?style=for-the-badge&logo=windows11&logoColor=white)](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb-dl-tui-install-windows-x86_64.exe)
[![Download for Linux](https://img.shields.io/badge/Download-Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb-dl-tui-install-linux-x86_64)

Run the downloaded file (double-click on Windows, or `chmod +x` then run it
on Linux). It installs the app plus everything else it needs, and adds it
to `PATH` -- no admin/root required. Open a **new** terminal afterward and
run:

```sh
ytb_dl_tui
```

Prefer no installer? Grab the portable bundle instead -- unzip it anywhere
and run the app binary inside directly, nothing added to `PATH`:
[Windows](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb-dl-tui-portable-windows-x86_64.zip) /
[Linux](https://github.com/bel-wadoud/ytb-dl-tui/releases/latest/download/ytb-dl-tui-portable-linux-x86_64.zip).
A [Docker image](#docker) is also available.

## Usage

```sh
# start at the blank URL prompt
ytb_dl_tui

# jump straight into a playlist
ytb_dl_tui 'https://www.youtube.com/playlist?list=...'
```

Paste a playlist, channel, or video URL, pick which videos you want,
adjust quality/format/subtitles/etc. on the Settings screen (`F2`), then
hit `Enter` to download.

### Keybindings

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

## Building from source

Requires a recent stable [Rust toolchain](https://rustup.rs).

```sh
cargo build --release
```

## License

[PolyForm Noncommercial License 1.0.0](LICENSE) -- free to use, modify, and
share for any noncommercial purpose. Commercial use is not permitted.
