#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

export RUSTUP_TOOLCHAIN=1.100.0
cargo build --release -p cli -p api

cd crates/desktop
npx --yes @tauri-apps/cli@2.12.6 build --config tauri.packaging.conf.json --bundles deb,appimage
