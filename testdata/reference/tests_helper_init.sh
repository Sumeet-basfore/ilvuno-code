#!/usr/bin/env bash
# Minimal launcher: validates the scratch root the same way the harness does.
set -uo pipefail
. "$(dirname "$0")/lib.sh"
init_scratch
echo "init_scratch unexpectedly accepted: $SP1_SCRATCH" >&2
exit 1
