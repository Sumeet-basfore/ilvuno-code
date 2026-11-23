#!/usr/bin/env bash
# Screenshot the real window, for checking the shell without a desktop session.
#
#   tools/shot.sh out.png [width] [height]
#
# The window's HTML/CSS/JS is compiled into the binary (`frontendDist` in tauri.conf.json), so this
# rebuilds the window first. It then restarts it on a virtual display at the size you asked for and
# grabs the whole screen. The runner (ilvunod) must already be answering on 127.0.0.1:4737.
set -euo pipefail
cd "$(dirname "$0")/.."

export RUSTUP_TOOLCHAIN=1.100.0
OUT=${1:?usage: tools/shot.sh <out.png> [width] [height]}
W=${2:-1440}
H=${3:-900}
DISP=${ILVUNO_DISPLAY:-:99}

if ! curl -fsS --max-time 2 http://127.0.0.1:4737/health >/dev/null 2>&1; then
  echo "the runner is not answering on 127.0.0.1:4737; start it with:" >&2
  echo "  ILVUNO_HOME=${ILVUNO_HOME:-~/.local/share/ilvuno} ./target/debug/ilvunod &" >&2
  exit 1
fi

# The screen size is fixed when the display starts, so a different size needs a fresh display.
if xdpyinfo -display "$DISP" >/dev/null 2>&1; then
  have=$(xdpyinfo -display "$DISP" | awk '/dimensions:/ {print $2}')
  if [ "$have" != "${W}x${H}" ]; then
    pkill -f "Xvfb $DISP " 2>/dev/null || true
    sleep 0.5
  fi
fi
if ! xdpyinfo -display "$DISP" >/dev/null 2>&1; then
  Xvfb "$DISP" -screen 0 "${W}x${H}x24" >/tmp/xvfb.log 2>&1 &
  sleep 1.2
fi

# Restart the window so the shot shows the files on disk, not the last build's.
for p in $(pgrep -fx "./target/debug/desktop" || true); do kill "$p" 2>/dev/null || true; done
cargo build -q -p desktop

# The window opens at 1440x900 from tauri.conf.json unless we say otherwise; resizing the X window
# afterwards does not resize the page without a window manager.
ILVUNO_WINDOW_SIZE="${W}x${H}" DISPLAY="$DISP" ./target/debug/desktop >/tmp/desktop.log 2>&1 &
sleep "${ILVUNO_SETTLE:-6}"

DISPLAY="$DISP" import -window root "$OUT"
identify "$OUT"
