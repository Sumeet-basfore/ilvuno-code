#!/usr/bin/env bash
# X-05 probe (benign): does a configured merge driver run under merge-tree --write-tree on this Git?
set -u
G="${GIT_BIN:-git}"
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null LANG=C LC_ALL=C TZ=UTC
export GIT_AUTHOR_NAME=ilvuno GIT_AUTHOR_EMAIL=ilvuno@localhost GIT_COMMITTER_NAME=ilvuno GIT_COMMITTER_EMAIL=ilvuno@localhost
export GIT_AUTHOR_DATE='2026-01-01T00:00:00Z' GIT_COMMITTER_DATE='2026-01-01T00:00:00Z'
W=$(mktemp -d "${TMPDIR:-/tmp}/x05.XXXXXX") || exit 2
trap 'rm -rf "$W"' EXIT
export X05_MARK="$W/driver-ran.marker"
cd "$W" && $G init -q -b main r && cd r || exit 2
printf 'a\nb\nc\n' > data.txt && echo 'data.txt merge=upper' > .gitattributes && $G add . && $G commit -q -m base
B=$($G rev-parse HEAD)
$G checkout -q -b s1 "$B" && sed -i '1s/.*/A1/' data.txt && $G commit -qam s1 && S1=$($G rev-parse HEAD)
$G checkout -q -b s2 "$B" && sed -i '3s/.*/C2/' data.txt && $G commit -qam s2 && S2=$($G rev-parse HEAD)
$G config merge.upper.driver 'echo ran >> "$X05_MARK"; exit 0'
$G merge-tree --write-tree "$S1" "$S2" > "$W/o.txt" 2>&1; echo "merge-tree rc=$?"
if [ -s "$X05_MARK" ]; then echo "driver executed under merge-tree: YES (marker outside repo at $W)"; else echo "driver executed under merge-tree: NO"; fi
