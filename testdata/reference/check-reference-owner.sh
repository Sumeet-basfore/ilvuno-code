#!/usr/bin/env bash
# check-reference-owner.sh: the AC-13 wrong-owner negative test (docs/tasks/T-0005.md).
#
# The wrapper must refuse a TMPDIR owned by another user before it creates, copies, or deletes anything there.
# The test runs the real wrapper inside the pinned Git 2.38.1 image, as the invoking UID, against a directory owned
# by UID 65534 (nobody). The directory is mode 755, so the invoking UID can enter it and the canonical-path check
# passes. Ownership is the first check that can refuse, so the refusal is the ownership one. A mode-700 directory
# owned by another user fails earlier, at "cannot canonicalise root", which does not exercise the ownership check.
# Two results must hold:
#
#   1. wrong owner: the wrapper exits nonzero with "root is not owned by the invoking user", and the protected
#      directory is unchanged (same listing, same sentinel bytes, same modification times).
#   2. control: a directory owned by the invoking UID passes the same preflight. The injected mktemp failure shows
#      that the preflight was passed, which proves that the refusal in (1) is about ownership.
#
# The script needs Docker, the pinned image, and a container that runs as root (to create the other-owned
# directory). When any of these is missing, it prints "NOT RUN" and exits 2. It never reports a pass in that case.
#
# Usage: bash testdata/reference/check-reference-owner.sh
#   REF_SRC       repository root that contains testdata/reference (default: this repository)
#   OWNER_IMAGE   pinned image (default: ilvuno-git238r:rebuild-2026-11-20)

set -uo pipefail

HERE=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
REF_SRC=${REF_SRC:-$(cd -P -- "$HERE/../.." && pwd -P)}
IMG=${OWNER_IMAGE:-ilvuno-git238r:rebuild-2026-11-20}
OTHER_UID=65534

not_run() { echo "NOT RUN: AC-13 wrong-owner test: $*" >&2; exit 2; }
fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; fail=1; }

command -v docker >/dev/null 2>&1 || not_run "docker is not available on this host"
docker image inspect "$IMG" >/dev/null 2>&1 || not_run "image $IMG is not present (build testdata/reference/Dockerfile.git238)"
docker run --rm -u 0 --entrypoint true "$IMG" >/dev/null 2>&1 || not_run "docker cannot run containers here"

base=$(mktemp -d "${TMPDIR:-/tmp}/ilvuno-owner.XXXXXX") || not_run "cannot create a private base"
chmod 700 "$base" || not_run "cannot restrict the private base"
ME="$(id -u):$(id -g)"

# snapshot DIR OUT: container-root listing and sentinel bytes of DIR, written to OUT.
snapshot() {
  docker run --rm -u 0 -v "$1:/p" --entrypoint sh "$IMG" -c 'ls -lan /p; cat /p/sentinel' > "$2" 2>&1
}

# Wrong owner: a directory owned by UID 65534, mode 700, holding a sentinel that must never change.
wrong="$base/wrong"; mkdir -m 755 "$wrong" || not_run "cannot create $wrong"
docker run --rm -u 0 -v "$wrong:/p" --entrypoint sh "$IMG" \
  -c "printf 'sentinel\n' > /p/sentinel && chown $OTHER_UID:$OTHER_UID /p/sentinel && chown $OTHER_UID:$OTHER_UID /p && chmod 755 /p" \
  >/dev/null 2>&1 || not_run "could not create the other-owned directory"
snapshot "$wrong" "$base/before.txt" || not_run "could not snapshot the protected directory"

docker run --rm -u "$ME" -v "$wrong:/p" -v "$REF_SRC:/src:ro" -w /src \
  -e TMPDIR=/p -e SP1_GIT_BIN=/usr/local/bin/git --entrypoint bash "$IMG" \
  -c 'echo "invoking uid=$(id -u)"; testdata/reference/check-reference.sh matrix; echo "wrapper-exit=$?"' \
  > "$base/refused.txt" 2>&1

exit_line=$(grep -E '^wrapper-exit=' "$base/refused.txt" | tail -n 1)
if [ -n "$exit_line" ] && [ "${exit_line#wrapper-exit=}" != 0 ]; then
  pass "wrong owner: the wrapper exits nonzero ($exit_line)"
else
  bad "wrong owner: the wrapper did not exit nonzero (${exit_line:-no exit status printed})"
fi
if grep -q 'root is not owned by the invoking user' "$base/refused.txt"; then
  pass "wrong owner: refused by the ownership check, not by a permission failure"
else
  bad "wrong owner: the ownership refusal message is missing"
fi

snapshot "$wrong" "$base/after.txt"
if cmp -s "$base/before.txt" "$base/after.txt"; then
  pass "wrong owner: the protected directory is unchanged (listing, mtimes and sentinel bytes)"
else
  bad "wrong owner: the protected directory changed"
  diff -u "$base/before.txt" "$base/after.txt" >&2
fi

# Control: a directory owned by the invoking UID passes the same preflight.
own="$base/own"; mkdir -m 700 "$own" || not_run "cannot create the control directory"
docker run --rm -u "$ME" -v "$own:/p" -v "$REF_SRC:/src:ro" -w /src \
  -e TMPDIR=/p -e CHECK_REF_FAIL=mktemp -e SP1_GIT_BIN=/usr/local/bin/git --entrypoint bash "$IMG" \
  -c 'testdata/reference/check-reference.sh matrix' > "$base/control.txt" 2>&1
if grep -q 'injected failure: mktemp' "$base/control.txt"; then
  pass "control: an owned directory passes the preflight (the injected mktemp failure is reached)"
else
  bad "control: an owned directory did not reach the injected failure"
fi

# Give the directories back to the invoking UID, so this script can delete them without privileges.
docker run --rm -u 0 -v "$base:/b" --entrypoint sh "$IMG" -c "chown -R $ME /b" >/dev/null 2>&1 || bad "could not return the test directories to the invoking UID"
rm -rf -- "$base"

if [ "$fail" -eq 0 ]; then
  echo "PASS: AC-13 wrong-owner test (refusal, no protected-root mutation, control)"
  exit 0
fi
echo "FAIL: AC-13 wrong-owner test" >&2
exit 1
