#!/usr/bin/env bash
# Open the Ilvuno window (dev run). Starts the runner on 127.0.0.1:4737 if it is not already answering,
# then runs the window. A runner this script started is stopped when the window closes; one that was
# already running is left alone.
#
#   tools/window.sh
#
# State goes where ilvunod and ilvuno always put it (ILVUNO_HOME, or ~/.local/share/ilvuno).
set -euo pipefail
cd "$(dirname "$0")/.."

export RUSTUP_TOOLCHAIN=1.100.0
API="http://127.0.0.1:4737"
DAEMON_PID=""

answers() {
  curl -fsS --max-time 2 "$API/health" >/dev/null 2>&1
}

cleanup() {
  if [ -n "$DAEMON_PID" ]; then
    kill "$DAEMON_PID" 2>/dev/null || true
    wait "$DAEMON_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

if answers; then
  echo "→ runner already answering on 127.0.0.1:4737; leaving it running"
else
  echo "→ starting the runner on 127.0.0.1:4737"
  cargo build -q -p api --bin ilvunod
  ./target/debug/ilvunod &
  DAEMON_PID=$!
  for _ in $(seq 1 50); do
    if answers; then break; fi
    if ! kill -0 "$DAEMON_PID" 2>/dev/null; then
      echo "the runner exited before it answered; see its output above" >&2
      exit 1
    fi
    sleep 0.2
  done
  if ! answers; then
    echo "the runner did not answer on $API within 10 seconds" >&2
    exit 1
  fi
fi

echo "→ opening the window"
cargo run -p desktop
