#!/usr/bin/env bash
# Builds ytb_dl_tui for a target, then builds the installer that embeds it.
# The installer's build.rs refuses to compile without a matching binary
# already sitting in installer/embed/, specifically so this two-step
# dependency can never be silently skipped.
#
# Usage:
#   ./build-installer.sh                          # native (Linux) build
#   ./build-installer.sh x86_64-pc-windows-gnu     # cross build via `cross`
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

TARGET="${1:-}"

if [[ -z "$TARGET" ]]; then
    echo "==> Building ytb_dl_tui (native)"
    cargo build --release
    APP_BIN="target/release/ytb_dl_tui"
    EMBED_NAME="ytb_dl_tui"
    BUILD_CMD=(cargo build --release)
    INSTALLER_BIN="installer/target/release/ytb-dl-tui-install"
else
    echo "==> Building ytb_dl_tui (cross target: $TARGET)"
    cross build --release --target "$TARGET"
    if [[ "$TARGET" == *windows* ]]; then
        APP_BIN="target/$TARGET/release/ytb_dl_tui.exe"
        EMBED_NAME="ytb_dl_tui.exe"
        INSTALLER_BIN="installer/target/$TARGET/release/ytb-dl-tui-install.exe"
    else
        APP_BIN="target/$TARGET/release/ytb_dl_tui"
        EMBED_NAME="ytb_dl_tui"
        INSTALLER_BIN="installer/target/$TARGET/release/ytb-dl-tui-install"
    fi
    BUILD_CMD=(cross build --release --target "$TARGET")
fi

if [[ ! -f "$APP_BIN" ]]; then
    echo "error: expected $APP_BIN after building ytb_dl_tui, but it's not there" >&2
    exit 1
fi

mkdir -p installer/embed
cp "$APP_BIN" "installer/embed/$EMBED_NAME"
echo "==> Embedded $APP_BIN -> installer/embed/$EMBED_NAME"

echo "==> Building installer"
(cd installer && "${BUILD_CMD[@]}")

echo "==> Done: $INSTALLER_BIN"
