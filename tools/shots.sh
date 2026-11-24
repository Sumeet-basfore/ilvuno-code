#!/usr/bin/env bash
# Screenshot every screen of the real window into crates/desktop/shots/.
#
#   tools/shots.sh
#
# The runner (ilvunod) must be answering on 127.0.0.1:4737 with a run in it. The window is built once
# and driven with clicks and keys at 1440x900, so the coordinates below assume that size and the
# default rail widths. A click that misses leaves a stale screenshot, so read the images after a run.
set -euo pipefail
cd "$(dirname "$0")/.."

export RUSTUP_TOOLCHAIN=1.100.0
DISP=${ILVUNO_DISPLAY:-:99}
OUT=crates/desktop/shots
W=1440
H=900
step() { echo "→ $1"; }

for p in $(pgrep -fx "./target/debug/desktop" || true); do kill "$p" 2>/dev/null || true; done
cargo build -q -p desktop

# The screen size is fixed when the display starts; another size means a fresh display.
if xdpyinfo -display "$DISP" >/dev/null 2>&1; then
  have=$(xdpyinfo -display "$DISP" | awk '/dimensions:/ {print $2}')
  if [ "$have" != "${W}x${H}" ]; then
    for p in $(pgrep -x Xvfb); do tr '\0' ' ' < "/proc/$p/cmdline" | grep -q "$DISP" && kill "$p"; done
    sleep 0.8
  fi
fi
if ! xdpyinfo -display "$DISP" >/dev/null 2>&1; then
  Xvfb "$DISP" -screen 0 "${W}x${H}x24" >/tmp/xvfb.log 2>&1 &
  sleep 1.2
fi

seen=/tmp/ilvuno-shots-hashes
: > "$seen"
shot() { DISPLAY="$DISP" xdotool mousemove 720 640; sleep 0.3; DISPLAY="$DISP" import -window root "$OUT/$1"; echo "   $OUT/$1"
  # Three shots came out byte-identical once: a click missed and nothing said so. A repeat is a bug.
  now=$(md5sum "$OUT/$1" | cut -d' ' -f1)
  if grep -qx "$now" "$seen"; then echo "   !! byte-identical to an earlier shot: a click missed" >&2; fi
  echo "$now" >> "$seen"; }
click() { DISPLAY="$DISP" xdotool mousemove "$1" "$2"; sleep 0.15; DISPLAY="$DISP" xdotool click 1; sleep "${3:-0.7}"; }
key() { DISPLAY="$DISP" xdotool key "$1"; sleep "${2:-0.7}"; }

ILVUNO_WINDOW_SIZE="${W}x${H}" DISPLAY="$DISP" ./target/debug/desktop >/tmp/desktop.log 2>&1 &
sleep "${ILVUNO_SETTLE:-6}"

step "thread view: the newest turn"
shot threads.png
step "the other thread"
click 120 200 1.4
shot conversation.png

step "command palette"
key "ctrl+k"
shot palette.png
key Escape

step "settings"
click 26 879 1.2
shot settings.png
step "settings: agents"
click 413 265 3.2
shot settings-agents.png
step "settings: defaults"
click 434 297 1.2
shot settings-defaults.png
key Escape
sleep 0.6

step "board, as a full view"
click 1385 242 1.2
shot board.png

step "a task in the drawer: back to the conversation, then click the quoted task"
click 345 68 1.2
click 600 168 1.2
shot drawer.png
key Escape

step "light theme"
key Escape
click 223 879 1.2
shot light.png
click 223 879 1.2

step "offline: the runner is not answering"
pkill -x ilvunod 2>/dev/null || true
# The window checks the runner every 5s (LIST_EVERY_MS); wait for two checks to land.
sleep 12
shot offline.png

echo "done — read the images before you trust them"
