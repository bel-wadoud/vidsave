#!/usr/bin/env bash
# Builds the TUI and GUI apps for a target, then builds the installer that
# embeds both. The installer's build.rs refuses to compile without matching
# binaries already sitting in installer/embed/, specifically so this
# multi-step dependency can never be silently skipped.
#
# Usage:
#   ./build-installer.sh                          # native (Linux) build
#   ./build-installer.sh x86_64-pc-windows-gnu     # cross build via `cross`
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

TARGET="${1:-}"
# Respect a user-configured CARGO_TARGET_DIR (a common Rust dev setup, e.g.
# to move build output off a slow filesystem) instead of assuming the
# default ./target -- cargo itself already honors this, we just need to
# know where to find what it produced.
TARGET_DIR="${CARGO_TARGET_DIR:-target}"

if [[ -z "$TARGET" ]]; then
    echo "==> Building vidsave-tui + vidsave (native)"
    cargo build --release -p vidsave-tui -p vidsave
    OUT_DIR="$TARGET_DIR/release"
    EXE_SUFFIX=""
    BUILD_INSTALLER_CMD=(cargo build --release -p vidsave-install)
else
    echo "==> Building vidsave-tui + vidsave (cross target: $TARGET)"
    cross build --release --target "$TARGET" -p vidsave-tui -p vidsave
    OUT_DIR="$TARGET_DIR/$TARGET/release"
    if [[ "$TARGET" == *windows* ]]; then
        EXE_SUFFIX=".exe"
    else
        EXE_SUFFIX=""
    fi
    BUILD_INSTALLER_CMD=(cross build --release --target "$TARGET" -p vidsave-install)
fi

TUI_BIN="$OUT_DIR/vidsave-tui$EXE_SUFFIX"
GUI_BIN="$OUT_DIR/vidsave$EXE_SUFFIX"
INSTALLER_BIN="$OUT_DIR/vidsave-install$EXE_SUFFIX"

for bin in "$TUI_BIN" "$GUI_BIN"; do
    if [[ ! -f "$bin" ]]; then
        echo "error: expected $bin, but it's not there" >&2
        exit 1
    fi
done

mkdir -p installer/embed
cp "$TUI_BIN" "installer/embed/vidsave-tui$EXE_SUFFIX"
cp "$GUI_BIN" "installer/embed/vidsave$EXE_SUFFIX"
echo "==> Embedded $TUI_BIN and $GUI_BIN into installer/embed/"

echo "==> Building installer"
"${BUILD_INSTALLER_CMD[@]}"

echo "==> Done: $INSTALLER_BIN"
