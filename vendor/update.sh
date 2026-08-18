#!/usr/bin/env bash
# Refreshes vendor/yt_dlp to the latest yt-dlp release. yt-dlp reverse-engineers
# YouTube's extraction (signature cipher, InnerTube API, format resolution) and
# ships fixes for YouTube's changes multiple times a month -- run this
# periodically (or when downloads start failing) to pick those up.
#
# Usage: ./vendor/update.sh
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

TAG="$(curl -fsSL https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest \
    | grep -o '"tag_name": *"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/')"
if [[ -z "$TAG" ]]; then
    echo "error: could not determine the latest yt-dlp release tag" >&2
    exit 1
fi

CURRENT="$(cat YT_DLP_VERSION 2>/dev/null || echo "none")"
if [[ "$TAG" == "$CURRENT" ]]; then
    echo "already up to date (yt-dlp $TAG)"
    exit 0
fi

echo "==> Updating vendored yt-dlp: $CURRENT -> $TAG"
SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT

curl -fsSL -o "$SCRATCH/src.tar.gz" \
    "https://github.com/yt-dlp/yt-dlp/archive/refs/tags/${TAG}.tar.gz"
mkdir -p "$SCRATCH/extract"
tar xzf "$SCRATCH/src.tar.gz" -C "$SCRATCH/extract" --strip-components=1

rm -rf yt_dlp
cp -r "$SCRATCH/extract/yt_dlp" yt_dlp
find yt_dlp -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true
find yt_dlp -name "*.pyc" -delete
cp "$SCRATCH/extract/LICENSE" YT_DLP_LICENSE
echo "$TAG" > YT_DLP_VERSION

echo "==> Done. Review the diff, bump the app version/CHANGELOG, and commit."
