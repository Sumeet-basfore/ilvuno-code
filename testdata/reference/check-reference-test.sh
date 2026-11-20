#!/usr/bin/env bash
# Negative tests for check-reference.sh (T-0005 AC-9 to AC-13). Every case runs the wrapper inside a
# throwaway sandbox repository under a private temporary root, so no case writes to the real work tree.
#
# Usage: TMPDIR=<private dir> check-reference-test.sh
# Wrong-owner roots (AC-13) need a second user and are not tested here; the owner check is in the wrapper.

set -uo pipefail

usage_fail() { echo "check-reference-test: $*" >&2; exit 2; }
[ -n "${TMPDIR:-}" ] || usage_fail "TMPDIR must name a private directory"

HERE=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
WRAPPER="$HERE/check-reference.sh"
REAL_GIT=$(command -v git) || usage_fail "git not found"
ROOT=$(mktemp -d "$TMPDIR/ref-tests.XXXXXX") || usage_fail "mktemp failed"
ROOT=$(cd -P -- "$ROOT" && pwd -P)
chmod 700 "$ROOT"
trap 'rm -rf "$ROOT"' EXIT

pass=0; fail=0
ok()  { pass=$((pass+1)); printf 'PASS  %s\n' "$1"; }
bad() { fail=$((fail+1)); printf 'FAIL  %s\n' "$1"; }
check() { local name=$1; shift; if "$@"; then ok "$name"; else bad "$name"; fi; }

tracked_sha() { sha256sum "$HERE/oracle.json" | cut -c1-64; }
leftovers() { find "$1" -maxdepth 1 -name 'ilvuno-ref.*' 2>/dev/null | wc -l | tr -d ' '; }

# A sandbox repository, the working directory for every wrapper invocation.
SB="$ROOT/sandbox"
mkdir -p "$SB" && (cd "$SB" && "$REAL_GIT" init -q -b main .) || usage_fail "sandbox init failed"

# A private base for one case, created by this test and removed at the end of the run.
new_base() { local b; b=$(mktemp -d "$ROOT/base.XXXXXX"); chmod 700 "$b"; printf '%s' "$b"; }

# run_wrapper MODE BASE OUT ERR [VAR=value ...]: runs the wrapper in the sandbox and prints its exit status.
run_wrapper() {
  local mode=$1 base=$2 out=$3 err=$4; shift 4
  (cd "$SB" && env -i PATH="$PATH" HOME="$ROOT" TMPDIR="$base" "$@" "$WRAPPER" "$mode") >"$out" 2>"$err"
  echo $?
}

# ------------------------------------------------ AC-9: setup failures stop before the oracle --
before=$(tracked_sha)
for step in mktemp chmod cp rm; do
  b=$(new_base); o="$ROOT/o.$step"; e="$ROOT/e.$step"
  rc=$(run_wrapper oracle "$b" "$o" "$e" CHECK_REF_FAIL="$step")
  check "AC-9 $step failure exits nonzero" [ "$rc" -ne 0 ]
  check "AC-9 $step failure invokes no oracle" bash -c "! grep -q 'invoking the oracle' '$o'"
  check "AC-9 $step failure publishes no comparison" bash -c "! grep -q 'comparing' '$o'"
  check "AC-9 $step failure names the injected step" grep -q "injected failure: $step" "$e"
done
check "AC-9 tracked oracle.json unchanged" [ "$(tracked_sha)" = "$before" ]

# ------------------------------------------------------ AC-10: oracle failure -------------
b=$(new_base); o="$ROOT/o.oracle"; e="$ROOT/e.oracle"
rc=$(run_wrapper oracle "$b" "$o" "$e" CHECK_REF_FAIL=oracle)
check "AC-10 oracle failure exits nonzero" [ "$rc" -ne 0 ]
check "AC-10 oracle failure runs no comparison" bash -c "! grep -q 'comparing' '$o'"
check "AC-10 tracked oracle.json unchanged" [ "$(tracked_sha)" = "$before" ]
check "AC-10 roots created by the run are removed" [ "$(leftovers "$b")" = 0 ]

# ------------------------------------------ AC-11: cleanup failure, status preserved ------
b=$(new_base); o="$ROOT/o.cleanup"; e="$ROOT/e.cleanup"
rc=$(run_wrapper oracle "$b" "$o" "$e" CHECK_REF_FAIL=cleanup)
check "AC-11 cleanup failure after a successful oracle exits nonzero" [ "$rc" -ne 0 ]
check "AC-11 cleanup failure is reported" grep -q 'cleanup failed' "$e"
rm -rf "$b"/ilvuno-ref.* # the injected failure left these; the test removes only its own roots

b=$(new_base); o="$ROOT/o.both"; e="$ROOT/e.both"
rc=$(run_wrapper oracle "$b" "$o" "$e" CHECK_REF_FAIL=oracle,cleanup)
check "AC-11 original failure status is preserved (exit 1)" [ "$rc" -eq 1 ]
check "AC-11 cleanup failure is also reported" grep -q 'cleanup failed' "$e"
rm -rf "$b"/ilvuno-ref.*

# ------------------------------------ AC-12: mismatch publishes no passing evidence --------
b=$(new_base); o="$ROOT/o.mismatch"; e="$ROOT/e.mismatch"
rc=$(run_wrapper oracle "$b" "$o" "$e" CHECK_REF_MUTATE=oracle)
check "AC-12 mismatch exits nonzero" [ "$rc" -ne 0 ]
check "AC-12 mismatch is reported with a diff" grep -q 'differs' "$e"
check "AC-12 mismatch emits no PASS line" bash -c "! grep -q '^PASS' '$o'"
check "AC-12 scratch roots are removed" [ "$(leftovers "$b")" = 0 ]
check "AC-12 tracked oracle.json unchanged" [ "$(tracked_sha)" = "$before" ]

# ------------------------------------------ AC-13: invalid roots refused before any write --
SHIM_DIR="$ROOT/shim"; mkdir -p "$SHIM_DIR"
SHIM="$SHIM_DIR/git"
cat > "$SHIM" <<SHIM_EOF
#!/bin/sh
printf '%s\n' "\$*" >> "\$SHIM_LOG"
case "\$1" in
  rev-parse|--version) exec "$REAL_GIT" "\$@" ;;
  *) echo "shim: refused repository write: \$*" >&2; exit 97 ;;
esac
SHIM_EOF
chmod 700 "$SHIM"

# only_readonly LOG: every recorded git invocation is a read-only one (rev-parse or --version).
only_readonly() {
  [ -s "$1" ] || return 0
  ! grep -v -E '^(rev-parse|--version)( |$)' "$1" | grep -q .
}

# Control: a valid private root passes preflight (the failure is the injected mktemp, not the root).
b=$(new_base); o="$ROOT/o.control"; e="$ROOT/e.control"; log="$ROOT/log.control"
rc=$(run_wrapper oracle "$b" "$o" "$e" CHECK_REF_FAIL=mktemp SHIM_LOG="$log" SP1_GIT_BIN="$SHIM")
check "AC-13 control: a valid root passes preflight" grep -q 'injected failure: mktemp' "$e"

invalid_bases() { # prints "name|path" for each invalid root, created under $ROOT
  local good real link inside wide
  real="$ROOT/real"; mkdir -p "$real"; chmod 700 "$real"
  link="$ROOT/link"; ln -s "$real" "$link"
  inside="$SB/private"; mkdir -p "$inside"; chmod 700 "$inside"
  wide="$ROOT/wide"; mkdir -p "$wide"; chmod 755 "$wide"
  printf '%s\n' \
    "relative|relative/dir" \
    "noncanonical|$real/./x" \
    "symlink-aliased|$link" \
    "inside-worktree|$inside" \
    "mode-755|$wide" \
    "missing|$ROOT/missing"
}

while IFS='|' read -r name path; do
  for mode in oracle matrix tests; do
    o="$ROOT/o.$name.$mode"; e="$ROOT/e.$name.$mode"; log="$ROOT/log.$name.$mode"
    : > "$log"
    rc=$(run_wrapper "$mode" "$path" "$o" "$e" SHIM_LOG="$log" SP1_GIT_BIN="$SHIM")
    check "AC-13 $name ($mode) refused nonzero" [ "$rc" -ne 0 ]
    check "AC-13 $name ($mode) made only read-only git calls" only_readonly "$log"
    check "AC-13 $name ($mode) published no evidence" bash -c "! grep -q '^PASS' '$o'"
  done
done < <(invalid_bases)

check "AC-13 tracked oracle.json unchanged" [ "$(tracked_sha)" = "$before" ]

echo
echo "check-reference-test: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
