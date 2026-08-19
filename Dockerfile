# syntax=docker/dockerfile:1

# ---- tools: fetch standalone ffmpeg/deno builds ----------------------------
# Same reasoning as the app's own binary resolution and the setup exe:
# standalone builds need no system packages and stay current, rather than
# whatever (possibly stale) version the distro's package repos carry.
# (yt-dlp is not fetched here -- see the runtime stage: we ship our own
# vendored copy of its source, run via the runtime image's system python3,
# rather than downloading yt-dlp's own release binary.)
FROM debian:bookworm-slim AS tools
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl xz-utils unzip \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /tools
# --retry/--max-time/--connect-timeout: a curl with none of these can hang
# forever on a stalled (not merely slow) connection, which turns a flaky
# network blip into a Docker build that never finishes or fails.
ARG CURL="curl -fL --connect-timeout 30 --max-time 600 --retry 5 --retry-delay 5 --retry-connrefused"
RUN $CURL -o ffmpeg.tar.xz \
        https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz \
    && tar -xf ffmpeg.tar.xz \
    && mv ffmpeg-*-amd64-static/ffmpeg ffmpeg-*-amd64-static/ffprobe . \
    && chmod +x ffmpeg ffprobe \
    && rm -rf ffmpeg.tar.xz ffmpeg-*-amd64-static
RUN $CURL -o deno.zip \
        https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip \
    && unzip -q deno.zip \
    && chmod +x deno \
    && rm deno.zip

# ---- builder: compile ytb_dl_tui from source -------------------------------
# The whole repo, not just tui/ + core/: this is a Cargo workspace, so Cargo
# needs every member's Cargo.toml (gui/, installer/) present to resolve the
# workspace graph even though -p only actually builds and compiles the one
# package we ask for (the GUI's much heavier dependency tree is never
# touched -- this image is the terminal UI only, see the runtime stage).
FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY . .
# Hard cap so a stalled crates.io fetch fails loudly instead of hanging the
# build indefinitely.
RUN timeout 1800 cargo build --release --locked -p ytb_dl_tui

# ---- runtime: minimal image with just what's needed to run it -------------
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates python3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 --shell /bin/bash ytbdl

COPY --from=tools /tools/ffmpeg /tools/ffprobe /tools/deno /usr/local/bin/
COPY --from=builder /app/target/release/ytb_dl_tui /usr/local/bin/ytb_dl_tui
# Our vendored copy of yt-dlp's source (see vendor/update.sh) -- run via the
# system python3 above instead of downloading yt-dlp's own release binary.
COPY vendor/yt_dlp /usr/local/lib/ytb-dl-tui/yt_dlp_src/yt_dlp

RUN mkdir -p /downloads && chown ytbdl:ytbdl /downloads

USER ytbdl
WORKDIR /downloads
ENV TERM=xterm-256color
ENV YTB_DL_TUI_PYTHON=/usr/bin/python3
ENV YTB_DL_TUI_YTDLP_SRC=/usr/local/lib/ytb-dl-tui/yt_dlp_src

# `--output-dir /downloads` is always applied first; passing your own
# `--output-dir` (or a URL, or any other flag) after the image name still
# works normally -- clap takes the last occurrence of a repeated option.
ENTRYPOINT ["ytb_dl_tui", "--output-dir", "/downloads"]
