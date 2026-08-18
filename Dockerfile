# syntax=docker/dockerfile:1

# ---- tools: fetch standalone yt-dlp/ffmpeg/deno builds --------------------
# Same reasoning as the app's own binary resolution and the setup exe:
# standalone builds need no Python/system packages and stay current, rather
# than whatever (possibly stale) version the distro's package repos carry.
FROM debian:bookworm-slim AS tools
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl xz-utils unzip \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /tools
# --retry/--max-time/--connect-timeout: a curl with none of these can hang
# forever on a stalled (not merely slow) connection, which turns a flaky
# network blip into a Docker build that never finishes or fails.
ARG CURL="curl -fL --connect-timeout 30 --max-time 600 --retry 5 --retry-delay 5 --retry-connrefused"
RUN $CURL -o yt-dlp \
        https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux \
    && chmod +x yt-dlp
RUN $CURL -o ffmpeg.tar.xz \
        https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz \
    && tar -xf ffmpeg.tar.xz \
    && mv ffmpeg-*-amd64-static/ffmpeg . \
    && chmod +x ffmpeg \
    && rm -rf ffmpeg.tar.xz ffmpeg-*-amd64-static
RUN $CURL -o deno.zip \
        https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip \
    && unzip -q deno.zip \
    && chmod +x deno \
    && rm deno.zip

# ---- builder: compile ytb_dl_tui from source -------------------------------
FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# Hard cap so a stalled crates.io fetch fails loudly instead of hanging the
# build indefinitely.
RUN timeout 1800 cargo build --release --locked

# ---- runtime: minimal image with just what's needed to run it -------------
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 --shell /bin/bash ytbdl

COPY --from=tools /tools/yt-dlp /tools/ffmpeg /tools/deno /usr/local/bin/
COPY --from=builder /app/target/release/ytb_dl_tui /usr/local/bin/ytb_dl_tui

RUN mkdir -p /downloads && chown ytbdl:ytbdl /downloads

USER ytbdl
WORKDIR /downloads
ENV TERM=xterm-256color

# `--output-dir /downloads` is always applied first; passing your own
# `--output-dir` (or a URL, or any other flag) after the image name still
# works normally -- clap takes the last occurrence of a repeated option.
ENTRYPOINT ["ytb_dl_tui", "--output-dir", "/downloads"]
