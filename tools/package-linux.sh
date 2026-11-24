#!/usr/bin/env bash
# Build the Linux packages: .deb, .rpm and .AppImage, each with the CLI and the runner inside.
#
#   tools/package-linux.sh
#
# The bundles land in crates/desktop/target/release/bundle/. A .deb or .rpm installed on a machine puts
# `ilvuno` on the PATH; the window finds `ilvunod` next to itself in the package's resource folder.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

export RUSTUP_TOOLCHAIN=1.100.0
cargo build --release -p cli -p api

cd crates/desktop
npx --yes @tauri-apps/cli@2.12.6 build --config tauri.packaging.conf.json --bundles deb,rpm,appimage

echo
echo "→ packages"
find ../../target/release/bundle -maxdepth 2 -type f \
  \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \) -exec du -h {} +
