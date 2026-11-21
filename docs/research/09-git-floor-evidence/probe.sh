#!/usr/bin/env bash
# X-09 probe. Read-only experiment in a private temp repository. Prints raw results.
# Covers readiness/02 X-01, X-02b, X-04, X-04b, X-06 and X-10. X-03 and X-05 are covered by the SP-1R3 matrix and probes.
set -u
G="${GIT_BIN:-git}"
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
export LANG=C LC_ALL=C TZ=UTC GIT_TERMINAL_PROMPT=0
export GIT_AUTHOR_NAME=ilvuno GIT_AUTHOR_EMAIL=ilvuno@localhost
export GIT_COMMITTER_NAME=ilvuno GIT_COMMITTER_EMAIL=ilvuno@localhost
export GIT_AUTHOR_DATE='2026-01-01T00:00:00Z' GIT_COMMITTER_DATE='2026-01-01T00:00:00Z'

W=$(mktemp -d "${TMPDIR:-/tmp}/x09.XXXXXX") || exit 2
trap 'rm -rf "$W"' EXIT
cd "$W" || exit 2

echo "## version: $($G --version)"
$G init -q -b main r && cd r || exit 2
printf 'l1\nl2\nl3\n' > shared.txt && $G add . && $G commit -q -m base
B=$($G rev-parse HEAD)
$G checkout -q -b s1 "$B" && sed -i '2s/.*/s1-line2/' shared.txt && echo one > s1.txt && $G add . && $G commit -q -m s1 && S1=$($G rev-parse HEAD)
$G checkout -q -b s2 "$B" && echo two > s2.txt && $G add . && $G commit -q -m s2 && S2=$($G rev-parse HEAD)
$G checkout -q -b s3 "$B" && echo three > s3.txt && $G add . && $G commit -q -m s3 && S3=$($G rev-parse HEAD)
$G checkout -q -b cf "$B" && sed -i '2s/.*/cf-line2/' shared.txt && $G commit -q -am cf && CF=$($G rev-parse HEAD)
$G checkout -q --orphan u && echo u > u.txt && $G add . && $G commit -q -m u && U=$($G rev-parse HEAD)

# mt: run merge-tree with the reference flags and print exit code, stdout (NUL shown as |) and stderr head.
mt() {
  $G merge-tree --write-tree --name-only -z --no-messages "$@" > "$W/o.bin" 2> "$W/e.txt"
  local rc=$?
  printf 'rc=%s stdout=[%s] stderr=[%s]\n' "$rc" "$(tr '\0' '|' < "$W/o.bin")" "$(head -c 160 "$W/e.txt" | tr '\n' ' ')"
}
objs() { find .git/objects -type f | wc -l; }

echo "## X-01 usage with three commits (expect exit 129)"
$G merge-tree --write-tree "$S1" "$S2" "$S3" > /dev/null 2> "$W/u.err"; echo "rc=$?"; head -1 "$W/u.err"
echo "## X-01 option support in merge-tree usage"
$G merge-tree -h > "$W/h.txt" 2>&1
for opt in -- --write-tree --name-only --no-messages -z --stdin --merge-base; do
  printf '%-16s %s\n' "$opt" "$(grep -c -- "$opt" "$W/h.txt")"
done
echo "## X-06 loose object delta, conflicting merge-tree (first merge-tree call in this repo)"
before=$(objs); mt "$S1" "$CF" > /dev/null; after=$(objs); echo "objects before=$before after=$after"
echo "## X-02b shape: clean (s1,s2)";     mt "$S1" "$S2"
echo "## X-02b shape: conflict (s1,cf)";  mt "$S1" "$CF"
echo "## X-02b shape: unrelated (s1,u)";  mt "$S1" "$U"

echo "## X-04 worktree add -B refs/ilvuno/... (expect branch under refs/heads)"
$G worktree add -q -B refs/ilvuno/candidates/probe "$W/wt1" "$S1" 2> "$W/e1"; echo "rc=$? $(head -c 120 "$W/e1")"
echo "refs/ilvuno entries: $($G for-each-ref --format='%(refname)' refs/ilvuno | wc -l)"
echo "refs/heads entries containing ilvuno: $($G for-each-ref --format='%(refname)' refs/heads | grep -c ilvuno)"
$G worktree remove --force "$W/wt1" 2>/dev/null; $G update-ref -d refs/heads/refs/ilvuno/candidates/probe 2>/dev/null

echo "## X-04b worktree add --detach + update-ref + cleanup"
$G worktree add -q --detach "$W/wt2" "$S1" && echo "detach rc=0"
echo "worktree HEAD == s1: $([ "$($G -C "$W/wt2" rev-parse HEAD)" = "$S1" ] && echo yes || echo no)"
$G worktree list --porcelain | grep -c '^detached$'
$G update-ref refs/ilvuno/candidates/p2 "$S1"
echo "refs/ilvuno after create: $($G for-each-ref --format='%(refname)' refs/ilvuno)"
$G worktree remove --force "$W/wt2" && $G update-ref -d refs/ilvuno/candidates/p2 && $G worktree prune
echo "after cleanup: worktrees=$($G worktree list --porcelain | grep -c '^worktree ') refs/ilvuno=$($G for-each-ref --format='%(refname)' refs/ilvuno | wc -l)"

echo "## X-10 check-attr --source availability"
$G check-attr --source="$S1" merge -- shared.txt > "$W/ca.txt" 2> "$W/ca.err"; echo "rc=$? stdout=[$(head -c 120 "$W/ca.txt" | tr '\n' ' ')] stderr=[$(head -c 160 "$W/ca.err" | tr '\n' ' ')]"
echo "## done"
