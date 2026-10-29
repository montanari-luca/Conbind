#!/usr/bin/env bash
# Build Conbind for Linux (AppImage + .deb for the host architecture).
#
# Usage: scripts/build-linux.sh [--release]
#   (default)   local build: dev bundle ID, no updater signing key needed
#   --release   full release build (needs TAURI_SIGNING_PRIVATE_KEY)
#
# System packages (Debian/Ubuntu):
#   sudo apt-get install -y build-essential pkg-config libssl-dev \
#     libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
#     patchelf libxdo-dev xdg-utils
set -euo pipefail

cd "$(dirname "$0")/.."

if [ "$(uname -s)" != "Linux" ]; then
  echo "This script builds the Linux bundles and must run on Linux." >&2
  exit 1
fi

for tool in pnpm cargo bun pkg-config; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required but was not found on PATH." >&2
    [ "$tool" = "bun" ] && echo "bun builds the ext-builder sidecar. Install it from https://bun.sh" >&2
    exit 1
  fi
done

if ! pkg-config --exists webkit2gtk-4.1; then
  echo "webkit2gtk-4.1 not found. Install libwebkit2gtk-4.1-dev (see the header of this script)." >&2
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

# linuxdeploy and appimagetool are themselves AppImages; self-extract instead of
# needing FUSE (Ubuntu 24.04 blocks the default mount path).
export APPIMAGE_EXTRACT_AND_RUN=1
# libwayland-client belongs to the host graphics ABI. Bundling the build machine's
# copy breaks WebKitGTK's EGL setup on hosts with a newer Mesa.
export LINUXDEPLOY_EXCLUDED_LIBRARIES='libwayland-client.so*'

pnpm install --frozen-lockfile
pnpm build ${build_args[@]+"${build_args[@]}"}

bundle_dir="conbind-launcher/src-tauri/target/release/bundle"
echo
echo "Build output:"
find "$bundle_dir" -maxdepth 2 \( -name '*.AppImage' -o -name '*.deb' \) -print
