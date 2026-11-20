#!/usr/bin/env bash
# check-reference.sh: run the SP-1R3 reference harness under operational conditions R3-1 to R3-4
# (docs/tasks/T-0005.md). The tracked files in this directory are never modified by this script.
#
# Usage: check-reference.sh oracle|matrix|tests
#
#   TMPDIR          REQUIRED. An absolute, canonical, existing, private directory (mode 700, owned by
#                   the invoking user) outside the work tree. The Makefile creates it. Every root this
#                   script creates lives under it, and this script never deletes a path it did not create.
#   SP1_GIT_BIN     optional git binary (negative tests use a recording shim).
#   CHECK_REF_FAIL  test seam: comma list of steps forced to fail: mktemp,chmod,cp,rm,oracle,cleanup.
#   CHECK_REF_MUTATE test seam: "oracle" alters the generated oracle copy after a successful run.
#
# Exit status is 0 only when every step succeeded and the comparison matched. Otherwise it is nonzero,
# and no pass line is printed.

set -euo pipefail

usage() { echo "usage: $0 oracle|matrix|tests" >&2; exit 2; }
die() { echo "check-reference: $*" >&2; exit 1; }

mode="${1:-}"
case "$mode" in oracle|matrix|tests) ;; *) usage ;; esac

REF_DIR=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
GIT_BIN="${SP1_GIT_BIN:-git}"
SEAM=",${CHECK_REF_FAIL:-},"
MUTATE="${CHECK_REF_MUTATE:-}"

# inject STEP: true when the test seam forces STEP to fail.
inject() { case "$SEAM" in *",$1,"*) return 0 ;; *) return 1 ;; esac; }

# ---------------------------------------------------------------- roots (R3-1) --
canon() { (cd -P -- "$1" 2>/dev/null && pwd -P); }
stat_uid() { stat -c %u -- "$1" 2>/dev/null || stat -f %u -- "$1"; }
stat_mode() { stat -c %a -- "$1" 2>/dev/null || stat -f %Lp -- "$1"; }

# validate_root PATH: refuse anything that is not an absolute, canonical, private, existing directory
# outside the work tree. Runs before the first repository-writing git command (AC-13).
validate_root() {
  local p=$1 c top prefix comp
  case "$p" in /*) ;; *) die "root is not absolute: $p" ;; esac
  case "$p" in *//*|*/./*|*/../*|*/.|*/..) die "root is not canonical: $p" ;; esac
  [ "$p" != / ] || die "root may not be /"
  prefix=""
  local IFS=/
  for comp in $p; do
    [ -n "$comp" ] || continue
    prefix="$prefix/$comp"
    [ ! -L "$prefix" ] || die "symlink component: $prefix"
  done
  unset IFS
  [ -d "$p" ] || die "root is not an existing directory: $p"
  c=$(canon "$p") || die "cannot canonicalise root: $p"
  [ "$c" = "$p" ] || die "root is not canonical: $p resolves to $c"
  [ "$(stat_uid "$p")" = "$(id -u)" ] || die "root is not owned by the invoking user: $p"
  [ "$(stat_mode "$p")" = 700 ] || die "root mode $(stat_mode "$p") is not 700: $p"
  if top=$("$GIT_BIN" rev-parse --show-toplevel 2>/dev/null); then
    top=$(canon "$top") || die "cannot canonicalise the work tree"
    case "$p/" in "$top/"*) die "root is inside the work tree: $p" ;; esac
  fi
}

# ------------------------------------------------- cleanup trap (R3-1, status kept) --
RECORDED=()
cleanup() {
  local rc=$?
  trap - EXIT
  local cleanup_failed=0 d
  for d in "${RECORDED[@]+"${RECORDED[@]}"}"; do
    if inject cleanup || ! rm -rf -- "$d"; then
      echo "check-reference: cleanup failed: $d" >&2
      cleanup_failed=1
    fi
  done
  if [ "$rc" -eq 0 ] && [ "$cleanup_failed" -eq 1 ]; then rc=1; fi
  exit "$rc"
}
trap cleanup EXIT

# ----------------------------------------------------------------- preflight --
[ -n "${TMPDIR:-}" ] || die "TMPDIR must name a private directory (the Makefile sets it)"
validate_root "$TMPDIR"
BASE="$TMPDIR"

# mk_private_dir: create and record a fresh mode-700 root under BASE. Sets NEWDIR.
mk_private_dir() {
  local d
  if inject mktemp; then die "injected failure: mktemp"; fi
  d=$(mktemp -d "$BASE/ilvuno-ref.XXXXXX") || die "mktemp failed under $BASE"
  RECORDED+=("$d")
  if inject chmod; then die "injected failure: chmod"; fi
  chmod 700 -- "$d" || die "chmod failed: $d"
  validate_root "$d"
  NEWDIR="$d"
}

# ----------------------------------------------------- fresh private copy (R3-1) --
mk_private_dir; WORK="$NEWDIR"
if inject cp; then die "injected failure: cp"; fi
cp -a -- "$REF_DIR" "$WORK/ref" || die "cp failed"
mk_private_dir; PARENT="$NEWDIR"

run_sp1() { # run_sp1 MODE: runs the copy's harness with the private scratch parent.
  SP1_SCRATCH="$PARENT" SP1_GIT_BIN="$GIT_BIN" "$WORK/ref/$1" "${@:2}"
}

case "$mode" in
  oracle)
    # Setup gate (R3-3): the copied oracle is removed before the oracle runs, so no stale output can be read.
    if inject rm; then die "injected failure: rm"; fi
    rm -f -- "$WORK/ref/oracle.json" || die "rm failed"
    if inject oracle; then die "injected failure: oracle"; fi
    echo "check-reference: invoking the oracle in the fresh copy"
    if ! run_sp1 sp1.sh oracle; then die "oracle invocation failed; no comparison was made"; fi
    if [ "$MUTATE" = oracle ]; then printf 'mutated\n' >> "$WORK/ref/oracle.json"; fi
    [ -s "$WORK/ref/oracle.json" ] || die "oracle produced no output; no comparison was made"
    echo "check-reference: comparing the generated oracle with the tracked copy"
    if ! cmp -s -- "$WORK/ref/oracle.json" "$REF_DIR/oracle.json"; then
      diff -u -- "$REF_DIR/oracle.json" "$WORK/ref/oracle.json" >&2 || true
      die "generated oracle differs from the tracked oracle.json (no pass evidence)"
    fi
    echo "PASS: oracle regenerated in a fresh copy and identical to the tracked oracle.json"
    ;;
  matrix)
    if ! run_sp1 sp1.sh matrix; then die "matrix invocation failed"; fi
    # The harness writes run-env.json under its own run child of the scratch parent (lib.sh init_scratch).
    env_file=$(find "$PARENT" -maxdepth 3 -type f -name run-env.json | head -n 1)
    [ -s "$env_file" ] || die "run-env.json missing"
    gv=$(sed -n 's/.*"git_version":"\([^"]*\)".*/\1/p' "$env_file")
    case "$gv" in 2.55.0|2.38.1) ;; *) die "run-env reports git_version '$gv'; only 2.55.0 and 2.38.1 are supported" ;; esac
    echo "PASS: matrix on git $gv (run-env.json: $(cat "$env_file"))"
    ;;
  tests)
    if ! run_sp1 tests.sh; then die "tests.sh failed"; fi
    echo "PASS: tests.sh"
    ;;
esac
