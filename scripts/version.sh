#!/usr/bin/env bash
# View or update the Conbind app version.
#
# Usage: scripts/version.sh -v          print the current version
#        scripts/version.sh 0.1.1       set the version everywhere
#
# Updates conbind-launcher/package.json, src-tauri/Cargo.toml, the `conbind`
# entry in src-tauri/Cargo.lock and the offline fallback in AboutTab.svelte.
# Pre-release suffixes must be numeric (0.1.0-1, never 0.1.0-beta) so the
# version stays valid for the Windows MSI installer.
set -euo pipefail

cd "$(dirname "$0")/.."

launcher="conbind-launcher"
pkg="$launcher/package.json"
cargo="$launcher/src-tauri/Cargo.toml"
lock="$launcher/src-tauri/Cargo.lock"
about="$launcher/src/routes/settings/tabs/AboutTab.svelte"

current_version() {
  sed -n 's/^  "version": "\(.*\)",$/\1/p' "$pkg" | head -n 1
}

usage() {
  echo "Usage: $0 -v | <x.y.z[-n]>" >&2
  exit 64
}

[ $# -eq 1 ] || usage

case "$1" in
  -v | --version)
    current_version
    exit 0
    ;;
  -h | --help) usage ;;
esac

new="$1"
if ! [[ "$new" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9]+(\.[0-9]+)*)?$ ]]; then
  echo "Invalid version: \"$new\". Use x.y.z, optionally with a numeric pre-release (0.1.0-1)." >&2
  exit 64
fi

old="$(current_version)"
if [ "$old" = "$new" ]; then
  echo "Already at $new"
  exit 0
fi

# Replace the first match only; later `version = ` lines belong to dependencies.
perl -0pi -e 's/^  "version": "[^"]*"/  "version": "'"$new"'"/m' "$pkg"
perl -0pi -e 's/^version = "[^"]*"/version = "'"$new"'"/m' "$cargo"
perl -0pi -e 's/(name = "conbind"\nversion = ")[^"]*"/${1}'"$new"'"/' "$lock"
perl -0pi -e "s/appVersion = '[^']*';(\\s*\\}\\);\\s*\\n\\s*let unlisten)/appVersion = '$new';\$1/" "$about"

echo "$old → $new"
git diff --stat -- "$pkg" "$cargo" "$lock" "$about"
