#!/usr/bin/env bash
# Build the MediaRemote adapter that powers media controls on macOS.
#
# Usage: scripts/build-mediaremote-adapter.sh
#
# Clones a pinned commit of ungive/mediaremote-adapter (BSD-3-Clause), builds
# the universal helper framework with cmake and stages it under
# conbind-launcher/src-tauri/resources/mediaremote-adapter, which tauri.conf.json
# bundles as a resource. The framework is loaded into /usr/bin/perl at runtime,
# never linked, so macOS 15.4+ still lets it read now-playing state.
#
# Skips on non-macOS hosts and when the staged copy already matches the pin.
# Bump REF to update the adapter, then re-run.
set -euo pipefail

REPO="https://github.com/ungive/mediaremote-adapter.git"
REF="0e82bf1c44ec39554fbfc402ec88a48c6ae2a0fd"

cd "$(dirname "$0")/.."
out="conbind-launcher/src-tauri/resources/mediaremote-adapter"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "MediaRemote adapter is macOS-only; skipping."
  exit 0
fi

if [[ -f "$out/.ref" && "$(cat "$out/.ref")" == "$REF" && -d "$out/MediaRemoteAdapter.framework" ]]; then
  echo "MediaRemote adapter already built ($REF)."
  exit 0
fi

if ! command -v cmake >/dev/null 2>&1; then
  echo "cmake is required to build the MediaRemote adapter. Install it with: brew install cmake" >&2
  exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

git clone --quiet --filter=blob:none "$REPO" "$tmp/src"
git -C "$tmp/src" checkout --quiet "$REF"
cmake -S "$tmp/src" -B "$tmp/build" >/dev/null
cmake --build "$tmp/build" >/dev/null

rm -rf "$out"
mkdir -p "$out"
cp -R "$tmp/build/MediaRemoteAdapter.framework" "$out/"
cp "$tmp/build/MediaRemoteAdapterTestClient" "$out/"
cp "$tmp/src/bin/mediaremote-adapter.pl" "$out/"
cp "$tmp/src/LICENSE" "$out/LICENSE"
echo "$REF" >"$out/.ref"

echo "MediaRemote adapter built into $out ($REF)."
