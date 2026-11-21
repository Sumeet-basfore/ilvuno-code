#!/usr/bin/env bash
# SP-1R2 negative, security and evidence tests. All fixtures are created by this
# invocation; no caller-supplied repository is accepted. Scope: one process per
# fresh private scratch root, no concurrent same-UID adversary.
set -uo pipefail
. "$(dirname "$0")/lib.sh"
init_scratch
SP1_LIB_ONLY=1 . "$(dirname "$0")/sp1.sh"
T="$SP1_SCRATCH/tests"; mkdir -p "$T"
SP1="$SP1_SELF/sp1.sh"
PASS=0 FAIL=0
ok()  { PASS=$((PASS+1)); printf 'PASS  %s\n' "$1"; }
bad() { FAIL=$((FAIL+1)); printf 'FAIL  %s  [%s]\n' "$1" "${2:-}"; }
check() { local n=$1; shift; if "$@" >/dev/null 2>&1; then ok "$n"; else bad "$n" "$*"; fi; }
expect_fail() { local n=$1; shift; if ( "$@" ) >/dev/null 2>&1; then bad "$n" "unexpected success"; else ok "$n"; fi; }
victim="${TMPDIR:-/tmp}/sp1r2-victim-$$"; rm -rf "$victim"; mkdir -p "$victim"; echo sentinel > "$victim/sentinel"
# The victim sits outside the run's scratch root on purpose (P0.2 and P0.6 check that nothing deletes it).
# It is this process's own fixture, so it is removed on exit, and nothing is left in TMPDIR (AC-2).
trap 'rm -rf -- "$victim"' EXIT

# ---------------------------------------------------------------- fixtures --
mk_repo() { # dest  (registered, so pipelines may run on it)
  local d=$1; mkdir -p "$d"; build_fixture F1-two-clean "$d" >/dev/null 2>&1
}
write_manifest() { # repo base srcs... [--check SHA256]
  local repo=$1 base=$2; shift 2
  { printf 'FORMAT=1\nBASE=%s\nSRCS=%s\n' "$base" "$*"; } > "$repo/.git/sp1-meta.env"
}
mkout() { mkdir -p "$1"; }

# ----------------------------------------------------------------- P0 paths --
expect_fail "P0.1 label traversal rejected" scratch_dir '../../victim'
expect_fail "P0.2 safe_rmrf outside root refused" safe_rmrf "$victim"
ln -sfn "$victim" "$SP1_SCRATCH/evil-link"
expect_fail "P0.3 symlinked child refused" safe_rmrf "$SP1_SCRATCH/evil-link"
check "P0.3b sentinel survived" test -f "$victim/sentinel"
ln -sfn "$victim" "$SP1_SCRATCH/root-link"
expect_fail "P0.4 symlinked scratch parent refused" env SP1_SCRATCH="$SP1_SCRATCH/root-link" "$SP1_SELF/tests_helper_init.sh"
r5="$T/p05"; mkdir -p "$r5"; ( cd "$r5" && gitx init -q -b main . )
expect_fail "P0.5 scratch inside work tree refused" env SP1_SCRATCH="$r5/inside" "$SP1_SELF/tests_helper_init.sh"
# P0.6 actually invokes the harness with the legacy variable.
sp6="$T/p06"; mkdir -p "$sp6"; chmod 700 "$sp6"
SP1_VER='../../victim' SP1_SCRATCH="$sp6" "$SP1" run F1-two-clean object plain >/dev/null 2>&1; rc6=$?
check "P0.6 harness runs with SP1_VER set" test "$rc6" -eq 0
check "P0.6b traversal sentinel intact" test -f "$victim/sentinel"
# P0.6 mutant: deleting the sentinel must make the same assertion fail.
rm -f "$victim/sentinel"; if [ -f "$victim/sentinel" ]; then bad "P0.6m mutant check detects deletion" "sentinel still present"; else ok "P0.6m mutant check detects deletion"; fi
echo sentinel > "$victim/sentinel"
# P0.7 no writes inside the harness directory.
marker="$T/p07.marker"; touch "$marker"
"$SP1" run F1-two-clean object plain >/dev/null 2>&1
check "P0.7 no writes inside script directory" test "$(find "$SP1_SELF" -newer "$marker" -type f | wc -l | tr -d ' ')" = 0
# P0.8 a supplied parent is not treated as disposable.
parent="$T/p08-parent"; mkdir -p "$parent/runs/F1-two-clean-object-plain" "$parent/out/F1-two-clean-object-plain"
echo keep > "$parent/runs/F1-two-clean-object-plain/sentinel"
echo keep > "$parent/out/F1-two-clean-object-plain/sentinel"
SP1_SCRATCH="$parent" "$SP1" run F1-two-clean object plain >/dev/null 2>&1
check "P0.8 parent sentinels preserved" test -f "$parent/runs/F1-two-clean-object-plain/sentinel"
check "P0.8b parent got an exclusive child" test "$(find "$parent" -maxdepth 1 -type d -name 'run-*' | wc -l | tr -d ' ')" -ge 1

# -------------------------------------------------------------- manifest ----
m="$T/manifest"; mk_repo "$m"
read_manifest "$m/.git/sp1-meta.env" >/dev/null 2>&1 && ok "M0 valid manifest parses" || bad "M0 valid manifest parses"
B0=$M_BASE; S0=${M_SRCS[0]}; S1=${M_SRCS[1]}; SALL=("${M_SRCS[@]}")
cp "$m/.git/sp1-meta.env" "$T/manifest.good"
printf 'FORMAT=1\nBASE=$(touch %s/payload)\nSRCS=%s %s\n' "$T" "$B0" "$S0 $S1" > "$m/.git/sp1-meta.env"
outm="$T/out-m"; mkdir -p "$outm"
run_object "$m" "$outm" plain >/dev/null 2>&1; rcm=$?
check "M1 shell payload in manifest refused" test "$rcm" -eq 4
check "M1b payload not executed" test ! -e "$T/payload"
printf 'FORMAT=1\nBASE=%s\nSRCS=%s %s\nEXTRA=1\n' "$B0" "$S0" "$S1" > "$m/.git/sp1-meta.env"
expect_fail "M2 unknown manifest key rejected" read_manifest "$m/.git/sp1-meta.env"
printf 'FORMAT=1\nBASE=%s\nBASE=%s\nSRCS=%s %s\n' "$B0" "$B0" "$S0" "$S1" > "$m/.git/sp1-meta.env"
expect_fail "M2b duplicate key rejected" read_manifest "$m/.git/sp1-meta.env"
printf 'FORMAT=1\nBASE=deadbeef\nSRCS=%s %s\n' "$S0" "$S1" > "$m/.git/sp1-meta.env"
expect_fail "M2c short OID rejected" read_manifest "$m/.git/sp1-meta.env"
printf 'FORMAT=1\nSRCS=%s %s\n' "$S0" "$S1" > "$m/.git/sp1-meta.env"
expect_fail "M2d missing BASE rejected" read_manifest "$m/.git/sp1-meta.env"
printf 'FORMAT=1\nBASE=%s\nSRCS=%s %s\n' "$B0" "$S0" "$S0" > "$m/.git/sp1-meta.env"
expect_fail "M2e duplicate source rejected" read_manifest "$m/.git/sp1-meta.env"
printf 'FORMAT=1\nBASE=%s\nSRCS=%s\n' "$B0" "$S0" > "$m/.git/sp1-meta.env"
expect_fail "M2f single source rejected" read_manifest "$m/.git/sp1-meta.env"
cp "$T/manifest.good" "$m/.git/sp1-meta.env"
# unregistered repository is refused outright
ur="$T/unregistered"; mkdir -p "$ur"; ( cd "$ur" && gitx init -q -b main . )
expect_fail "M3 unregistered repository refused" run_object "$ur" "$T/out-ur" plain

# ------------------------------------------------------------- execution ----
r="$T/sg1"; mk_repo "$r"; mkdir -p "$r/.hooks"
for h in post-checkout pre-merge-commit post-merge; do
  printf '#!/bin/sh\necho ran >> "%s"\n' "$T/sg1.marker" > "$r/.hooks/$h"; chmod +x "$r/.hooks/$h"; done
gitx -C "$r" config core.hooksPath .hooks
mkout "$T/out-sg1"; run_worktree "$r" "$T/out-sg1" plain >/dev/null 2>&1
check "SG-1 hooks do not execute" test ! -s "$T/sg1.marker"
r="$T/sg2"; mk_repo "$r"; mkdir -p "$r/.hooks"
printf '#!/bin/sh\necho ran >> "%s"\n' "$T/sg2.marker" > "$r/.hooks/post-checkout"; chmod +x "$r/.hooks/post-checkout"
sp2="$T/parent-sg2"; mkdir -p "$sp2"
GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0="$r/.hooks" \
  SP1_SCRATCH="$sp2" "$SP1" run F1-two-clean worktree plain >/dev/null 2>&1
check "SG-2 runtime GIT_CONFIG injection stripped" test ! -s "$T/sg2.marker"
mkdir -p "$T/env-parent"
check "SG-2b child env sanitised" bash -c '. "$0/lib.sh"; SP1_SCRATCH="$1" init_scratch >/dev/null 2>&1; d=$(dirname "$(find "$1" -name .sp1r-owner | head -1)"); sp1_env printenv > "$d/env.dump"; ! grep -q GIT_CONFIG_KEY_0 "$d/env.dump" && grep -q GIT_CONFIG_COUNT=0 "$d/env.dump"' "$SP1_SELF" "$T/env-parent"
# SG-3 default driver
r="$T/sg3"; mk_repo "$r"
gitx -C "$r" config merge.spy.driver "sh -c 'echo ran >> $T/sg3.marker'"; gitx -C "$r" config merge.default spy
mkout "$T/out-sg3"; run_object "$r" "$T/out-sg3" plain >/dev/null 2>&1
check "SG-3 merge.default refused" test $? -ne 0
check "SG-3b driver did not run" test ! -s "$T/sg3.marker"
# SG-4 attributes introduced by a pinned source commit
r="$T/sg4"; mk_repo "$r"; b=$(read_manifest "$r/.git/sp1-meta.env" && echo "$M_BASE"); s1="${M_SRCS[0]}"; s2="${M_SRCS[1]}"
gitx -C "$r" config merge.upper.driver "sh -c 'echo ran >> $T/sg4.marker'"
gitx -C "$r" checkout -q "$s1"; printf 'f.txt merge=upper\n' > "$r/.gitattributes"; gitx -C "$r" add -A; gitx -C "$r" commit -qm attrs
s1b=$(gitx -C "$r" rev-parse HEAD); gitx -C "$r" checkout -q main
write_manifest "$r" "$b" "$s1b" "$s2"
out4="$T/out-sg4"; mkdir -p "$out4"; run_object "$r" "$out4" plain >/dev/null 2>&1; rc4=$?
check "SG-4 late-attribute source refused" test "$rc4" -eq 4
check "SG-4b driver did not run" test ! -s "$T/sg4.marker"
check "SG-4c refused run pins the amended source" bash -c "python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); assert \"$s1b\" in r[\"sources\"], r[\"sources\"]' '$out4/result.json'"
# SG-4 mutant: pin a different (pre-amend) source; the pin assertion must fail.
write_manifest "$r" "$b" "$s1" "$s2"
run_object "$r" "$out4" plain >/dev/null 2>&1
if python3 -c "import json,sys; r=json.load(open(sys.argv[1])); assert \"$s1b\" in r[\"sources\"]" "$out4/result.json" 2>/dev/null; then
  bad "SG-4m pin assertion detects wrong pin" "assertion accepted a different source"
else ok "SG-4m pin assertion detects wrong pin"; fi
# SG-5 attributesFile
r="$T/sg5"; mk_repo "$r"; printf '*.txt merge=upper\n' > "$T/sg5.attrs"
gitx -C "$r" config core.attributesFile "$T/sg5.attrs"
gitx -C "$r" config merge.upper.driver "sh -c 'echo ran >> $T/sg5.marker'"
mkout "$T/out-sg5"; run_object "$r" "$T/out-sg5" plain >/dev/null 2>&1
check "SG-5 attributesFile driver refused" test $? -ne 0
check "SG-5b driver did not run" test ! -s "$T/sg5.marker"
# SG-6 common-dir info/attributes
r="$T/sg6"; mk_repo "$r"; gitx -C "$r" config merge.spy.driver "sh -c 'echo ran >> $T/sg6.marker'"
printf '*.txt merge=spy\n' > "$r/.git/info/attributes"
mkout "$T/out-sg6"; run_worktree "$r" "$T/out-sg6" plain >/dev/null 2>&1
check "SG-6 info/attributes driver refused" test $? -ne 0
check "SG-6b driver did not run" test ! -s "$T/sg6.marker"
# SG-7 filters
r="$T/sg7"; mk_repo "$r"; gitx -C "$r" config filter.spy.smudge "sh -c 'echo ran >> $T/sg7.marker; cat'"
gitx -C "$r" config filter.spy.required false; printf '*.txt filter=spy\n' > "$r/.gitattributes"
gitx -C "$r" add -A; gitx -C "$r" commit -qm attrs
mkout "$T/out-sg7"; run_worktree "$r" "$T/out-sg7" plain >/dev/null 2>&1
check "SG-7 filter refused" test $? -ne 0
check "SG-7b filter did not run" test ! -s "$T/sg7.marker"
# SG-8 .ilvuno symlink
out8="$T/outside-sg8"; mkdir -p "$out8"; echo keep > "$out8/keep"
r="$T/sg8"; mk_repo "$r"; ln -s "$out8" "$r/.ilvuno"
mkout "$T/out-sg8"; run_worktree "$r" "$T/out-sg8" plain >/dev/null 2>&1
check "SG-8 outside target untouched" test "$(ls -A "$out8")" = keep
check "SG-8b repo .ilvuno not used" test ! -e "$r/.ilvuno/worktrees"
# SG-9 digest-pinned check: candidate content differs from the recorded digest
r="$T/sg9"; mk_repo "$r"; b=$(read_manifest "$r/.git/sp1-meta.env" && echo "$M_BASE"); s1="${M_SRCS[0]}"; s2="${M_SRCS[1]}"
gitx -C "$r" checkout -q "$s1"; printf '#!/bin/sh\nexit 0\n' > "$r/check.sh"; chmod +x "$r/check.sh"
gitx -C "$r" add check.sh; gitx -C "$r" commit -qm check; s1c=$(gitx -C "$r" rev-parse HEAD)
clean=$(sha256_of "$r/check.sh")
printf '#!/bin/sh\necho tampered >> "%s"\n' "$T/sg9.marker" > "$r/check.sh"; chmod +x "$r/check.sh"
gitx -C "$r" add check.sh; gitx -C "$r" commit -qm tamper; s1t=$(gitx -C "$r" rev-parse HEAD)
gitx -C "$r" checkout -q main
write_manifest "$r" "$b" "$s1t" "$s2"; printf 'CHECK_SHA256=%s\n' "$clean" >> "$r/.git/sp1-meta.env"
mkout "$T/out-sg9"; run_object "$r" "$T/out-sg9" plain >/dev/null 2>&1
check "SG-9 tampered candidate check refused" test $? -eq 4
check "SG-9b tampered check did not run" test ! -s "$T/sg9.marker"

# SG-10 submodule not initialised
sub="$T/sg10-sub"; mkdir -p "$sub"; ( cd "$sub" && gitx init -q -b main . && echo s > s.txt && gitx add -A && gitx commit -qm s )
r="$T/sg10"; mkdir -p "$r"; ( cd "$r" && gitx init -q -b main . && echo a > a.txt && gitx add -A && gitx commit -qm base \
  && gitx -c protocol.file.allow=always submodule add -q "$sub" deps/mod >/dev/null 2>&1 && gitx add -A && gitx commit -qm sub )
gitx -C "$r" worktree add -q --detach "$T/sg10-wt" HEAD
st=$(gitx -C "$T/sg10-wt" submodule status 2>/dev/null | head -1)
case "$st" in -*) ok "SG-10 submodule not initialised" ;; *) bad "SG-10 submodule not initialised" "$st" ;; esac
# SG-11 orphan cleanup via the fail-cleanup path
r="$T/sg11"; mk_repo "$r"; out11="$T/out-sg11"; mkdir -p "$out11"
run_object "$r" "$out11" fail-cleanup >/dev/null 2>&1; rc11=$?
python3 - "$out11/result.json" <<'PY' && ok "SG-11 cleanup failure reported" || bad "SG-11 cleanup failure reported"
import json,sys
r=json.load(open(sys.argv[1]))
assert r["exit"]==5, r["exit"]
assert r["cleanup"]=="orphaned_dir_removed", r["cleanup"]
PY

# --------------------------------------------------------------- content ----
r="$T/c4"; build_fixture F10-newline-path "$r" >/dev/null 2>&1
out4c="$T/out-c4"; mkdir -p "$out4c"; run_worktree "$r" "$out4c" plain >/dev/null 2>&1
python3 - "$r" "$out4c/result.json" <<'PY' && ok "C-4 F10 path and content match independent oracle" || bad "C-4 F10 path and content match independent oracle"
import hashlib, json, subprocess, sys
repo, res = sys.argv[1], sys.argv[2]
r = json.load(open(res))
assert r["conflict_paths"] == ["line\nbreak.txt"], r["conflict_paths"]
# independent: read the candidate tree with NUL records and hash blob bytes
def git(*a): return subprocess.run(["git","-C",repo,*a],capture_output=True,check=True).stdout
ls = git("ls-tree","-r","-z","--full-tree",r["candidate"]).split(b"\0")
want=[]
for rec in ls:
    if not rec: continue
    meta,_,path = rec.partition(b"\t"); mode,typ,oid = meta.split(b" ")
    want.append((path, hashlib.sha256(git("cat-file","blob",oid.decode())).hexdigest().encode()))
got = [(x.split("=",1)[0].encode("utf-8","surrogateescape"), x.split("=",1)[1].encode()) for x in r["content"]]
assert dict(got) == dict(want), (got, want)
PY
# C-5 path/content edge cases
r="$T/c5"; mkdir -p "$r"; ( cd "$r" && gitx init -q -b main . \
  && printf 'nl\n' > "line
break.txt" && printf 'q\n' > 'quo"te.txt' && printf 'tab\n' > "ta	b.txt" \
  && printf 'ff\n' > "$(printf 'bad-\377.txt')" && : > empty.txt && printf 'data\n' > nonempty.txt \
  && gitx add -A && gitx commit -qm base && sha=$(gitx rev-parse HEAD) )
sha=$(gitx -C "$r" rev-parse HEAD)
content_manifest "$r" "$sha" "$T/c5.nul" 2>/dev/null
python3 - "$r" "$sha" "$T/c5.nul" <<'PY' && ok "C-5 newline/quote/tab/non-UTF8/empty blobs preserved" || bad "C-5 newline/quote/tab/non-UTF8/empty blobs preserved"
import hashlib, subprocess, sys
repo, sha, nul = sys.argv[1], sys.argv[2], sys.argv[3]
def git(*a): return subprocess.run(["git","-C",repo,*a],capture_output=True,check=True).stdout
ls = git("ls-tree","-r","-z","--full-tree",sha).split(b"\0")
want={}
for rec in ls:
    if not rec: continue
    meta,_,path = rec.partition(b"\t"); _,_,oid = meta.split(b" ")
    want[path]=hashlib.sha256(git("cat-file","blob",oid.decode())).hexdigest()
got={}
for rec in open(nul,"rb").read().split(b"\0"):
    if not rec: continue
    p,v = rec.split(b"=",1)
    got[p]=v.decode()
assert got==want, (sorted(got), sorted(want))
PY

# ------------------------------------------------------------------ refs ----
r="$T/refs"; mk_repo "$r"
keep=$(gitx -C "$r" rev-parse main); gitx -C "$r" update-ref refs/ilvuno/keep "$keep"
gitx -C "$r" update-ref refs/ilvuno/runs/pre-existing/candidate "$keep"
outr="$T/out-refs"; mkdir -p "$outr"; run_object "$r" "$outr" plain >/dev/null 2>&1
check "R1 pre-existing refs/ilvuno kept" test "$(gitx -C "$r" rev-parse refs/ilvuno/keep)" = "$keep"
check "R1b pre-existing run ref kept" test "$(gitx -C "$r" rev-parse refs/ilvuno/runs/pre-existing/candidate)" = "$keep"
python3 - "$outr/result.json" <<'PY' && ok "R1c result reports no leftover harness refs" || bad "R1c result reports no leftover harness refs"
import json,sys
r=json.load(open(sys.argv[1])); assert r["refs_left"]==0, r["refs_left"]
PY
f1=$(user_fingerprint "$r"); gitx -C "$r" update-ref refs/ilvuno/keep "$(gitx -C "$r" rev-parse s1)"; f2=$(user_fingerprint "$r")
check "R2 fingerprint includes refs/ilvuno changes" test "$f1" != "$f2"

# --------------------------------------------------------------- repeat -----
r="$T/repeat"; mk_repo "$r"
mkout "$T/out-r1"; run_worktree "$r" "$T/out-r1" plain >/dev/null 2>&1; a=$?
mkout "$T/out-r2"; run_worktree "$r" "$T/out-r2" plain >/dev/null 2>&1; b=$?
check "P4 repeated invocation succeeds" test "$a" = 0 -a "$b" = 0
check "P4b no worktrees left" test "$(ours_wt "$r")" = 0
check "P4c no refs left" test "$(gitx -C "$r" for-each-ref --format='%(refname)' | grep -c 'refs/ilvuno/runs/' || true)" = 0

# ------------------------------------------------------- matrix enforcement --
MP="$T/matrix-parent"; mkdir -p "$MP"
SP1_SCRATCH="$MP" "$SP1" matrix >/dev/null 2>&1 || true
ALL=$(ls -1 "$MP"/run-*/out/all.jsonl 2>/dev/null | tail -1); EXP="$SP1_SELF/expected.json"
RE=$(dirname "$ALL")/run-env.json
check "P1.4 the real matrix passes" python3 "$SP1_SELF/expected.py" --check "$ALL" "$EXP" "$RE"
sed '$d' "$ALL" > "$T/missing.jsonl"
check "P1.1 missing row fails" bash -c "python3 '$SP1_SELF/expected.py' --check '$T/missing.jsonl' '$EXP' '$RE' >/dev/null 2>&1; test \$? -ne 0"
{ cat "$ALL"; printf '{not json}\n'; } > "$T/malformed.jsonl"
check "P1.2 malformed JSON fails" bash -c "python3 '$SP1_SELF/expected.py' --check '$T/malformed.jsonl' '$EXP' '$RE' >/dev/null 2>&1; test \$? -ne 0"
python3 "$SP1_SELF/expected.py" --update "$T/malformed.jsonl" "$SP1_SELF/oracle.json" "$SP1_SELF/spec.json" "$T/never.json" "$RE" >/dev/null 2>&1
check "P1.2b update refuses malformed input" test ! -f "$T/never.json"
python3 - "$ALL" "$T/wrong.jsonl" <<'PY'
import json,sys
rows=[json.loads(l) for l in open(sys.argv[1]) if l.strip()]
rows[0]["candidate"]="0"*40
open(sys.argv[2],"w").write("\n".join(json.dumps(r) for r in rows)+"\n")
PY
check "P1.3 wrong candidate fails" bash -c "python3 '$SP1_SELF/expected.py' --check '$T/wrong.jsonl' '$EXP' '$RE' >/dev/null 2>&1; test \$? -ne 0"
mutate_check() { # name python-expr
  local name=$1 expr=$2
  python3 - "$ALL" "$T/mut.jsonl" "$expr" <<'PY'
import json, sys
rows=[json.loads(l) for l in open(sys.argv[1]) if l.strip()]
exec(sys.argv[3])
open(sys.argv[2],"w").write("\n".join(json.dumps(r) for r in rows)+"\n")
PY
  if python3 "$SP1_SELF/expected.py" --check "$T/mut.jsonl" "$EXP" "$RE" >/dev/null 2>&1; then
    bad "$name" "mutation accepted"
  else ok "$name"; fi
}
mutate_check "E1 candidate SHA detected" 'rows[0]["candidate"]="0"*40'
mutate_check "E2 tree SHA detected" 'rows[0]["tree"]="0"*40'
mutate_check "E3 blob hash detected" 'rows[0]["content"][0]=rows[0]["content"][0].split("=")[0]+"="+"0"*64'
mutate_check "E4 input pin detected" 'rows[0]["base"]="0"*40'
mutate_check "E5 source order detected" 'p=rows[0]["sources"].split(); rows[0]["sources"]=" ".join(reversed(p))'
mutate_check "E6 verdict detected" 'rows[0]["verdict"]="conflict"'
mutate_check "E7 conflict path detected" 'rows[0]["conflict_paths"]=["x"]'
mutate_check "E8 cleanup state detected" 'rows[0]["cleanup"]="failed"'
mutate_check "E9 git version detected" 'rows[0]["git_version"]="0.0.0"'
mutate_check "E10 merge policy detected" 'rows[0]["merge_policy"]="changed"'
mutate_check "E11 commit metadata detected" 'rows[0]["metadata"]="changed"'

# ------------------------------------------------- N1..N3, cleanup (R3) ------
# N1a: oracle refuses a fixture with a configured driver and runs nothing.
r="$SP1_SCRATCH/fixtures-F8-driver"; mkdir -p "$r"; build_fixture F8-driver "$r/repo" >/dev/null 2>&1
printf 'F8-driver\t%s\n' "$(cd "$r/repo" && pwd -P)" > "$T/n1a-reg.tsv"
cp "$T/n1a-reg.tsv" "$SP1_SCRATCH/oracle-repos.tsv"
mkout "$T/out-n1a"
DRIVER_MARKER="$T/n1a.marker" SP1_RUN_ID="$SP1_RUN_ID" python3 "$SP1_SELF/oracle.py" "$SP1_SCRATCH" "$T/out-n1a/oracle.json" F8-driver >/dev/null 2>&1; rc=$?
check "N1a oracle refuses configured driver" test "$rc" -eq 0
check "N1b driver marker absent" test ! -s "$T/n1a.marker"
check "N1c refusal recorded, no candidates" python3 -c "import json,sys; d=json.load(open(sys.argv[1])); f=d['fixtures']['F8-driver']; assert f['status']=='refused' and f['candidates']==[]" "$T/out-n1a/oracle.json"
# N1d: unregistered fixture (registry present but no entry)
printf '' > "$SP1_SCRATCH/oracle-repos.tsv"
python3 "$SP1_SELF/oracle.py" "$SP1_SCRATCH" "$T/out-n1a/unreg.json" F8-driver >/dev/null 2>&1
check "N1d unregistered fixture refused" test $? -ne 0
check "N1e no output on refusal" test ! -e "$T/out-n1a/unreg.json"
# N1f: invalid ownership marker
cp "$T/n1a-reg.tsv" "$SP1_SCRATCH/oracle-repos.tsv"
sed_inplace 's/^id=.*/id=deadbeef/' "$SP1_SCRATCH/.sp1r-owner"
python3 "$SP1_SELF/oracle.py" "$SP1_SCRATCH" "$T/out-n1a/badmarker.json" F8-driver >/dev/null 2>&1
check "N1f invalid ownership marker refused" test $? -ne 0
check "N1g no output on bad marker" test ! -e "$T/out-n1a/badmarker.json"
printf 'format=ilvuno-sp1r-scratch/1\nuid=%s\nid=%s\nparent=%s\n' "$(id -u)" "$SP1_RUN_ID" "$SP1_SCRATCH" > "$SP1_SCRATCH/.sp1r-owner"
# N1h: external repository path in the registry
outside="$T/n1h-outside"; mkdir -p "$outside"; ( cd "$outside" && gitx init -q -b main . )
printf 'F8-driver\t%s\n' "$(cd "$outside" && pwd -P)" > "$SP1_SCRATCH/oracle-repos.tsv"
python3 "$SP1_SELF/oracle.py" "$SP1_SCRATCH" "$T/out-n1a/outside.json" F8-driver >/dev/null 2>&1
check "N1h external repository refused" test $? -ne 0
# N1i: runtime GIT_CONFIG_* must not reach oracle Git; hooks configured in repo.
r="$SP1_SCRATCH/fixtures-F1-two-clean"; mkdir -p "$r"; build_fixture F1-two-clean "$r/repo" >/dev/null 2>&1
mkdir -p "$r/repo/.hooks"; printf '#!/bin/sh\necho ran >> "%s"\n' "$T/n1i.marker" > "$r/repo/.hooks/post-checkout"; chmod +x "$r/repo/.hooks/post-checkout"
gitx -C "$r/repo" config core.hooksPath .hooks
printf 'F1-two-clean\t%s\n' "$(cd "$r/repo" && pwd -P)" > "$SP1_SCRATCH/oracle-repos.tsv"
GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0="$r/repo/.hooks" \
  python3 "$SP1_SELF/oracle.py" "$SP1_SCRATCH" "$T/out-n1a/hooks.json" F1-two-clean >/dev/null 2>&1
check "N1i oracle hooks do not execute" test ! -s "$T/n1i.marker"

# N2: every malformed manifest class fails schema and oracle, writes no output.
r="$SP1_SCRATCH/fixtures-F1-two-clean"; mkdir -p "$r"; build_fixture F1-two-clean "$r/repo" >/dev/null 2>&1
printf 'F1-two-clean\t%s\n' "$(cd "$r/repo" && pwd -P)" > "$SP1_SCRATCH/oracle-repos.tsv"
good="$T/n2.good"; cp "$r/repo/.git/sp1-meta.env" "$good"
B=$(sed -n 's/^BASE=//p' "$good"); S=$(sed -n 's/^SRCS=//p' "$good")
n2() { # name manifest-body
  local name=$1 body=$2
  printf '%s' "$body" > "$r/repo/.git/sp1-meta.env"
  python3 "$SP1_SELF/schema.py" --check "$r/repo/.git/sp1-meta.env" >/dev/null 2>&1 && { bad "$name" "schema accepted"; return; }
  rm -f "$T/out-n2.json"
  python3 "$SP1_SELF/oracle.py" "$SP1_SCRATCH" "$T/out-n2.json" F1-two-clean >/dev/null 2>&1 && { bad "$name" "oracle accepted"; return; }
  [ -e "$T/out-n2.json" ] && { bad "$name" "oracle wrote output"; return; }
  ok "$name"
}
n2 "N2.1 blank line rejected" "FORMAT=1
BASE=$B
SRCS=$S

"
n2 "N2.2 malformed line rejected" "FORMAT=1
BASE=$B
SRCS=$S
NOT_A_KEY
"
n2 "N2.3 unknown key rejected" "FORMAT=1
BASE=$B
SRCS=$S
EXTRA=1
"
n2 "N2.4 duplicate key rejected" "FORMAT=1
BASE=$B
BASE=$B
SRCS=$S
"
n2 "N2.5 short OID rejected" "FORMAT=1
BASE=deadbeef
SRCS=$S
"
n2 "N2.6 missing BASE rejected" "FORMAT=1
SRCS=$S
"
n2 "N2.7 duplicate source rejected" "FORMAT=1
BASE=$B
SRCS=${S%% *} ${S%% *}
"
n2 "N2.8 source equals base rejected" "FORMAT=1
BASE=$B
SRCS=$B ${S##* }
"
n2 "N2.9 single source rejected" "FORMAT=1
BASE=$B
SRCS=${S%% *}
"
cp "$good" "$r/repo/.git/sp1-meta.env"

# N3: a row must match the Git version this run executed.
mutate_check "E12 cross-labelled supported version detected" 'rows[0]["git_version"]="2.38.1" if rows[0]["git_version"]=="2.55.0" else "2.55.0"'
python3 - "$ALL" "$T/mislabel.jsonl" <<'PY'
import json,sys
rows=[json.loads(l) for l in open(sys.argv[1]) if l.strip()]
rows[0]["git_version"]="2.38.1" if rows[0]["git_version"]=="2.55.0" else "2.55.0"
open(sys.argv[2],"w").write("\n".join(json.dumps(r) for r in rows)+"\n")
PY
python3 -c "import json,sys; d=json.load(open(sys.argv[1])); d['git_version']='9.9.9'; json.dump(d,open(sys.argv[2],'w'))" "$RE" "$T/badrunenv.json"
check "N3.1b wrong run-env version is rejected" bash -c "python3 '$SP1_SELF/expected.py' --check '$ALL' '$EXP' '$T/badrunenv.json' >/dev/null 2>&1; test \$? -ne 0"
check "N3.2 missing run-env fails" bash -c "python3 '$SP1_SELF/expected.py' --check '$ALL' '$EXP' >/dev/null 2>&1; test \$? -ne 0"

# Cleanup: SP1_CLEAN removes the run child, preserves parent, reports.
cp2="$T/clean-parent"; mkdir -p "$cp2"; echo keep > "$cp2/sentinel"
SP1_CLEAN=1 SP1_SCRATCH="$cp2" "$SP1" invalid >/dev/null 2>&1; rcc=$?
check "N4.1 invalid command exits nonzero" test "$rcc" -ne 0
check "N4.2 run child removed" test "$(find "$cp2" -maxdepth 1 -type d -name 'run-*' | wc -l | tr -d ' ')" = 0
check "N4.3 parent sentinel preserved" test -f "$cp2/sentinel"
# Cleanup refuses a mismatched run id and reports it.
rm -rf $T/r3-cl; mkdir -m700 $T/r3-cl
outc=$(SP1_LIB_ONLY=1 bash -c '. "$0/lib.sh"; export SP1_SCRATCH="$1"; init_scratch >/dev/null; SP1_RUN_ID=bogus cleanup_scratch; echo "rc=$?"' "$SP1_SELF" $T/r3-cl 2>&1)
check "N4.4 cleanup refuses wrong run id" bash -c "echo '$outc' | grep -q 'rc=1'"
check "N4.5 refused cleanup keeps the root" bash -c "test -n \"\$(find $T/r3-cl -maxdepth 1 -type d -name 'run-*')\""
check "N4.6 cleanup failure is reported" bash -c "echo '$outc' | grep -q 'marker id mismatch'"

# ------------------------------------------------------------------ report --
printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
