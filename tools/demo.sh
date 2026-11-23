#!/usr/bin/env bash
# Run one real goal, then show you the board, the status, the replay and an export.
#
#   tools/demo.sh                          # the board experiment (about 12 cents of Claude)
#   tools/demo.sh path/to/goal.json        # any goal file
#
# State goes in ./.demo-home so your real ~/.local/share/ilvuno is untouched.
# Delete that folder to start clean.
set -euo pipefail
cd "$(dirname "$0")/.."

export RUSTUP_TOOLCHAIN=1.100.0
export ILVUNO_HOME="${ILVUNO_HOME:-$PWD/.demo-home}"
GOAL="${1:-examples/board-experiment.json}"

echo "→ building"
cargo build -q -p cli

echo "→ running $GOAL"
OUT="$(./target/debug/ilvuno run --goal "$GOAL" | tee /dev/stderr)"
RUN="$(printf '%s\n' "$OUT" | sed -n 's/^run \([0-9a-f-]\{36\}\) created.*/\1/p' | head -1)"
if [ -z "$RUN" ]; then
  echo "could not find a run id in the output above" >&2
  exit 1
fi

echo
echo "→ status"
STATUS_OUT="$(./target/debug/ilvuno status "$RUN")"
printf '%s\n' "$STATUS_OUT"
if printf '%s' "$OUT" | grep -q "waiting for tasks"; then
  cat <<HINT

This goal has no thread named "orchestrator", so it planned nothing and started nothing.
That is by design: an orchestrator plans, or you do. Add a task and continue:

  RUSTUP_TOOLCHAIN=1.100.0 ILVUNO_HOME=$ILVUNO_HOME ./target/debug/ilvuno task \
    --thread <thread-name> --prompt "<what to do>" --scope "file.md"
  RUSTUP_TOOLCHAIN=1.100.0 ILVUNO_HOME=$ILVUNO_HOME ./target/debug/ilvuno run --resume $RUN

Or run the default goal, which includes an orchestrator:  tools/demo.sh
HINT
  exit 0
fi

echo
echo "→ replay (the journal, checked against the database)"
./target/debug/ilvuno replay "$RUN" | tail -4

echo
echo "→ export"
rm -rf .demo-export
./target/debug/ilvuno export "$RUN" --out .demo-export >/dev/null
echo "  .demo-export/run.json, board.md, journal.jsonl, artifacts/"

cat <<EOF

────────────
open these:

  board      $ILVUNO_HOME/runs/$RUN/board.md
  export     .demo-export/board.md
  prototypes open .demo-export/../proto/4-mix.html in a browser
             (they read fixtures/board-experiment, not this run)

want the app's door instead of the CLI?

  ILVUNO_HOME=$ILVUNO_HOME ./target/debug/ilvunod &
  curl -s localhost:4737/runs | head -c 400; echo
  curl -s localhost:4737/runs/$RUN | head -c 400; echo
  kill %1

run id: $RUN
EOF
