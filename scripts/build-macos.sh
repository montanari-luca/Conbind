#!/usr/bin/env bash
# Build Conbind for macOS.
#
# Usage: scripts/build-macos.sh [--release]
#   (default)   local build: dev bundle ID, no updater signing key needed
#   --release   full release build (needs TAURI_SIGNING_PRIVATE_KEY)
#
# Proc-macro stripping is disabled because the current Apple linker emits
# stripped proc-macro dylibs that dyld rejects ("mis-aligned LINKEDIT string
# pool"), which breaks the build with E0463 "can't find crate".
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v bun >/dev/null 2>&1; then
  echo "bun is required for the ext-builder sidecar. Install it with: brew install bun" >&2
  exit 1
fi

case "${1:-}" in
  --release) build_args=() ;;
  "") build_args=(--local) ;;
  *)
    echo "Usage: $0 [--release]" >&2
    exit 64
    ;;
esac

pnpm install
CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_STRIP=none pnpm build ${build_args[@]+"${build_args[@]}"}

bundle_dir="conbind-launcher/src-tauri/target/release/bundle"
echo
echo "Build output:"
find "$bundle_dir" -maxdepth 2 \( -name '*.app' -o -name '*.dmg' \) -print
