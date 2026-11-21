#!/usr/bin/env bash
# SP-1R2 harness: object-only (A) vs worktree (B) sequential integration.
# Usage:
#   ./sp1.sh fixtures | matrix [--update-expected] | probes | oracle | run <fixture> <approach> [variant]
# Repositories are created by this invocation and registered; there is no command
# that accepts a caller-supplied repository path.
set -uo pipefail
. "$(dirname "$0")/lib.sh"
if [ "${SP1_LIB_ONLY:-0}" != "1" ]; then
  init_scratch
  mkdir -p "$SP1_SCRATCH"/{runs,out}
fi

FIXTURES="F1-two-clean F2-three-clean F3-direct-conflict F4-conflict-step2 F5-already-integrated F6-unrelated F7-combined-build-failure F8-driver F9-cleanup F10-newline-path"
VARIANTS="plain fail-cleanup stop-after1 index-lock keep-conflict"

# ---------------------------------------------------------------- fixtures --
build_fixture() { # name dest
  local name=$1 dest=$2
  chmod -R u+w "$dest" 2>/dev/null || true
  rm_tree "$dest"
  mkdir -p "$dest"
  ( cd "$dest" || exit 1
    gitx init -q -b main .
    printf '.ilvuno/\n' > .gitignore
    seq 1 12 | sed 's/^/line/' > shared.txt; printf 'base\n' > stable.txt
    gitx add -A; gitx commit -qm base; local BASE; BASE=$(gitx rev-parse HEAD)
    br() { gitx checkout -q main; gitx checkout -qB "$1" "$BASE"; }
    cm() { gitx add -A; gitx commit -qm "$1"; gitx rev-parse HEAD; }
    case "$name" in
      F1-two-clean)   br s1; sed_inplace '2s/.*/s1-line2/' shared.txt; echo one > s1.txt; S1=$(cm s1)
                      br s2; sed_inplace '10s/.*/s2-line10/' shared.txt; echo two > s2.txt; S2=$(cm s2); SRCS="$S1 $S2" ;;
      F2-three-clean) br s1; sed_inplace '2s/.*/s1-line2/' shared.txt; echo one > s1.txt; S1=$(cm s1)
                      br s2; sed_inplace '10s/.*/s2-line10/' shared.txt; echo two > s2.txt; S2=$(cm s2)
                      br s3; sed_inplace '6s/.*/s3-line6/' shared.txt; echo three > s3.txt; S3=$(cm s3)
                      SRCS="$S1 $S2 $S3" ;;
      F3-direct-conflict) br s1; sed_inplace '2s/.*/s1-wins/' shared.txt; S1=$(cm s1)
                      br s2; sed_inplace '2s/.*/s2-wins/' shared.txt; S2=$(cm s2); SRCS="$S1 $S2" ;;
      F4-conflict-step2) br s1; sed_inplace '2s/.*/s1-wins/' shared.txt; S1=$(cm s1)
                      br s2; sed_inplace '2s/.*/s2-wins/' shared.txt; S2=$(cm s2)
                      br s3; sed_inplace '11s/.*/s3-line11/' shared.txt; S3=$(cm s3); SRCS="$S1 $S2 $S3" ;;
      F5-already-integrated) br s1; sed_inplace '2s/.*/s1-line2/' shared.txt; S1=$(cm s1)
                      gitx checkout -q main; gitx merge -q --no-ff -m "merge s1" "$S1"; BASE=$(gitx rev-parse HEAD)
                      br s2; sed_inplace '10s/.*/s2-line10/' shared.txt; S2=$(cm s2); SRCS="$S1 $S2" ;;
      F6-unrelated)   br s1; sed_inplace '2s/.*/s1-line2/' shared.txt; S1=$(cm s1)
                      gitx checkout -q --orphan orphan; gitx rm -rqf .; echo unrelated > u.txt
                      gitx add -A; gitx commit -qm orphan; S2=$(gitx rev-parse HEAD); gitx checkout -q main
                      SRCS="$S1 $S2" ;;
      F7-combined-build-failure)
                      mkdir -p config; touch config/.keep
                      printf '#!/bin/sh\nn=$(cat config/*.txt 2>/dev/null | grep -c .)\n[ "$n" -eq 1 ] || { echo "expected 1 region, found $n"; exit 1; }\nexit 0\n' > check.sh
                      chmod +x check.sh; gitx add -A; gitx commit -qm base; BASE=$(gitx rev-parse HEAD)
                      br s1; echo eu > config/eu.txt; S1=$(cm s1)
                      br s2; echo us > config/us.txt; S2=$(cm s2); SRCS="$S1 $S2" ;;
      F8-driver)      printf 'driverfile.txt merge=upper\n' > .gitattributes
                      printf 'hello world\n' > driverfile.txt
                      gitx add -A; gitx commit -qm base; BASE=$(gitx rev-parse HEAD)
                      gitx config merge.upper.driver "sh -c 'echo ran >> \"${DRIVER_MARKER:-/nonexistent}\"; tr a-z A-Z < %A > %A.tmp && mv %A.tmp %A'"
                      br s1; printf 'hello from s1\n' > driverfile.txt; S1=$(cm s1)
                      br s2; printf 'hello from s2\n' > driverfile.txt; S2=$(cm s2); SRCS="$S1 $S2" ;;
      F9-cleanup)     br s1; sed_inplace '2s/.*/s1-line2/' shared.txt; S1=$(cm s1)
                      br s2; sed_inplace '10s/.*/s2-line10/' shared.txt; S2=$(cm s2); SRCS="$S1 $S2" ;;
      F10-newline-path) br s1; printf 'v1\n' > "line
break.txt"; S1=$(cm s1)
                      br s2; printf 'v2\n' > "line
break.txt"; S2=$(cm s2); SRCS="$S1 $S2" ;;
      *) printf 'unknown fixture %s\n' "$name" >&2; exit 3 ;;
    esac
    gitx checkout -q main
    {
      printf 'FORMAT=1\nBASE=%s\n' "$BASE"
      printf 'SRCS=%s\n' "$SRCS"
      if [ -f check.sh ]; then printf 'CHECK_SHA256=%s\n' "$(sha256_of check.sh)"; fi
    } > .git/sp1-meta.env
  )
  register_repo "$dest"
}

# --------------------------------------------------------------- pipelines --
run_object() { # repo out variant
  local repo=$1 out=$2 variant=${3:-plain}
  is_registered_repo "$repo" || die "unregistered repository"
  [ -d "$out" ] || die "output directory missing (the caller must create it): $out"
  read_manifest "$repo/.git/sp1-meta.env" || { printf 'bad manifest\n' >&2; emit_json "$out" approach object fixture "${SP1_FIXTURE:-?}" variant "$variant" verdict harness_refused exit 4 conflict_step 0 check_rc -1 cleanup ok user_state_unchanged no refs_left 0 worktrees_left 0 objects_before 0 objects_after 0 drivers_ran no git_version "$(gitx --version | awk '{print $3}')" os "$(uname -s)" base "" sources "" merge_policy "" metadata "" candidate "" tree ""; return 4; }
  local acc=$M_BASE i=0 n steps="" cstep=0 verdict=ok exitc=0 check_rc=-1 cleanup=ok
  local paths_file="$out/conflict_paths.nul"; : > "$paths_file"
  local CREF="refs/ilvuno/runs/$SP1_RUN_ID/candidate" MREF="refs/ilvuno/runs/$SP1_RUN_ID/meta"
  local gitver osname fp_B fp_A ob_B ob_A op_detail=""
  gitver=$(gitx --version | awk '{print $3}'); osname=$(uname -s)
  fp_B=$(user_fingerprint "$repo" "$CREF" "$MREF"); ob_B=$(objcount "$repo")
  n=${#M_SRCS[@]}
  : > "$out/hooks.marker"; : > "$out/filters.marker"; : > "$out/drivers.marker"
  export DRIVER_MARKER="$out/drivers.marker"

  if ! check_exec_features "$repo" "$out"; then verdict=harness_refused; exitc=4; acc=""; fi
  if [ "$exitc" -eq 0 ]; then
    for src in "${M_SRCS[@]}"; do
      i=$((i+1))
      if ! check_exec_features "$repo" "$out"; then verdict=harness_refused; exitc=4; break; fi
      if gitx -C "$repo" merge-base --is-ancestor "$src" "$acc"; then steps+="$i:skip "; continue; fi
      if ! gitx -C "$repo" merge-base "$acc" "$src" >/dev/null 2>&1; then
        steps+="$i:unrelated "; verdict=unrelated_histories; exitc=3; cstep=$i; break; fi
      gitx -C "$repo" merge-tree --write-tree --name-only -z --no-messages "$acc" "$src" > "$out/mt.bin" 2>"$out/mt.err"; local rc=$?
      local tree; tree=$(head -c 200 "$out/mt.bin" | tr '\0' '\n' | head -1)
      : > "$paths_file"
      if [ "$rc" -eq 1 ]; then
        python3 - "$out/mt.bin" "$paths_file" <<'PY'
import sys
d=open(sys.argv[1],'rb').read().split(b'\0')
with open(sys.argv[2],'wb') as f:
    for p in d[1:]:
        if p: f.write(p+b'\0')
PY
        verdict=conflict; exitc=3; cstep=$i; steps+="$i:conflict "; break
      elif [ "$rc" -ne 0 ]; then
        verdict=operational_error; exitc=5; cstep=$i; steps+="$i:operational-$rc "
        op_detail=$(head -c 200 "$out/mt.err" | tr '\n' ' '); break
      fi
      local ts; ts=$(gitx -C "$repo" show -s --format=%ct "$acc")
      local msg="ilvuno: integrate ${src:0:12} (step $i/$n)"
      acc=$(SP1_NAME=ilvuno SP1_EMAIL=ilvuno@localhost SP1_DATE="@$ts +0000" \
            gitx -C "$repo" commit-tree "$tree" -p "$acc" -p "$src" -m "$msg") || {
        verdict=operational_error; exitc=5; op_detail="commit-tree failed"; break; }
      steps+="$i:clean "
      if [ "$variant" = stop-after1 ] && [ $i -eq 1 ]; then verdict=stopped; exitc=9; break; fi
    done
  fi
  ob_A=$(objcount "$repo")
  if [ "$verdict" = ok ]; then
    # create-only refs under a unique run namespace
    gitx -C "$repo" rev-parse --verify --quiet "$CREF" >/dev/null && { verdict=operational_error; exitc=5; op_detail="ref collision"; }
    [ "$exitc" -eq 0 ] && { gitx -C "$repo" update-ref "$CREF" "$acc" "" || { verdict=operational_error; exitc=5; op_detail="ref create failed"; }; }
    if [ "$exitc" -eq 0 ]; then
      printf 'spike\ngit=%s\n' "$gitver" > "$out/meta.txt"
      local blob; blob=$(gitx -C "$repo" hash-object -w "$out/meta.txt")
      gitx -C "$repo" update-ref "$MREF" "$blob" "" || true
      mkdir -p "$out/candidate"
      if gitx -C "$repo" worktree add -q --detach "$out/candidate" "$acc" 2>"$out/wt.err"; then
        if [ -f "$out/candidate/check.sh" ]; then
          local got; got=$(sha256_of "$out/candidate/check.sh")
          if [ "${M_CHECK:-}" != "$got" ]; then verdict=harness_refused; exitc=4
            printf 'check.sh digest mismatch\n' >> "$out/refusal.txt"
          else ( cd "$out/candidate" && sh ./check.sh ) > "$out/check.log" 2>&1; check_rc=$?; fi
        fi
        if [ "$check_rc" -gt 0 ]; then verdict=check_failed; exitc=1; fi
        [ "$variant" = fail-cleanup ] && chmod a-w "$out/candidate"
        if ! gitx -C "$repo" worktree remove --force "$out/candidate" 2>"$out/rm.err"; then cleanup=failed; fi
        if [ "$cleanup" = ok ]; then cleanup=removed
        elif [ -d "$out/candidate" ]; then
          if _owner_ok "$out/candidate"; then safe_rmrf "$out/candidate" && cleanup=orphaned_dir_removed || cleanup=orphaned_dir_left
          else cleanup=orphaned_dir_left; fi
        fi
        if [ "$(gitx -C "$repo" rev-parse "$CREF" 2>/dev/null)" = "$acc" ]; then
          gitx -C "$repo" update-ref -d "$CREF" "$acc" || true
          gitx -C "$repo" update-ref -d "$MREF" "$blob" || true
        fi
        gitx -C "$repo" worktree prune
      else verdict=worktree_add_failed; exitc=5; fi
    fi
  fi
  case "$cleanup" in failed|orphaned_dir_removed|orphaned_dir_left) [ "$exitc" -eq 0 ] && exitc=5 ;; esac
  fp_A=$(user_fingerprint "$repo" "$CREF" "$MREF")
  if [ -n "${acc:-}" ]; then content_manifest "$repo" "$acc" "$out/content.nul"; else : > "$out/content.nul"; fi
  read_nul "$out/content.nul" CONTENT; read_nul "$paths_file" PATHS
  local steps_arr=(); [ -n "$steps" ] && read -r -a steps_arr <<< "$steps"
  emit_array "$out" content "${CONTENT[@]:-}"
  emit_array "$out" conflict_paths "${PATHS[@]:-}"
  emit_array "$out" steps "${steps_arr[@]:-}"
  emit_json "$out" approach object fixture "${SP1_FIXTURE:-$(basename "$repo")}" variant "$variant" \
    verdict "$verdict" exit "$exitc" conflict_step "$cstep" check_rc "$check_rc" cleanup "$cleanup" \
    candidate "${acc:-}" tree "$([ -n "${acc:-}" ] && gitx -C "$repo" rev-parse "$acc^{tree}" 2>/dev/null || echo "")" \
    user_state_unchanged "$([ "$fp_B" = "$fp_A" ] && echo yes || echo no)" \
    refs_left "$(gitx -C "$repo" for-each-ref --format='%(refname)' | grep -c "^refs/ilvuno/runs/$SP1_RUN_ID/" || true)" \
    worktrees_left "$(ours_wt "$repo")" objects_before "$ob_B" objects_after "$ob_A" \
    drivers_ran "$([ -s "$out/drivers.marker" ] && echo yes || echo no)" \
    git_version "$gitver" os "$osname" base "$M_BASE" sources "${M_SRCS[*]}" \
    merge_policy "--no-ff --no-edit; hooks/attrs/signing/fsmonitor/submodules neutralised via -c" \
    metadata "ilvuno <ilvuno@localhost> @<base commit time>" operational_detail "$op_detail"
  return $exitc
}

run_worktree() { # repo out variant
  local repo=$1 out=$2 variant=${3:-plain}
  is_registered_repo "$repo" || die "unregistered repository"
  [ -d "$out" ] || die "output directory missing (the caller must create it): $out"
  read_manifest "$repo/.git/sp1-meta.env" || { emit_json "$out" approach worktree fixture "${SP1_FIXTURE:-?}" variant "$variant" verdict harness_refused exit 4 conflict_step 0 check_rc -1 cleanup ok user_state_unchanged no refs_left 0 worktrees_left 0 objects_before 0 objects_after 0 drivers_ran no git_version "$(gitx --version | awk '{print $3}')" os "$(uname -s)" base "" sources "" merge_policy "" metadata "" candidate "" tree ""; return 4; }
  local wt="$out/candidate" i=0 n=${#M_SRCS[@]} steps="" cstep=0 verdict=ok exitc=0 check_rc=-1 cleanup=ok
  local paths_file="$out/conflict_paths.nul"; : > "$paths_file"
  local CREF="refs/ilvuno/runs/$SP1_RUN_ID/candidate" MREF="refs/ilvuno/runs/$SP1_RUN_ID/meta"
  local gitver osname fp_B fp_A ob_B ob_A op_detail="" acc=""
  gitver=$(gitx --version | awk '{print $3}'); osname=$(uname -s)
  fp_B=$(user_fingerprint "$repo" "$CREF" "$MREF"); ob_B=$(objcount "$repo")
  : > "$out/hooks.marker"; : > "$out/filters.marker"; : > "$out/drivers.marker"
  export DRIVER_MARKER="$out/drivers.marker"

  if ! check_exec_features "$repo" "$out"; then verdict=harness_refused; exitc=4; acc=""; fi
  if [ "$exitc" -eq 0 ]; then
    mkdir -p "$(dirname "$wt")"
    if ! gitx -C "$repo" worktree add -q --detach "$wt" "$M_BASE" 2>"$out/wt.err"; then
      verdict=worktree_add_failed; exitc=5; fi
  fi
  if [ "$exitc" -eq 0 ]; then
    for src in "${M_SRCS[@]}"; do
      i=$((i+1))
      if ! check_exec_features "$repo" "$out"; then verdict=harness_refused; exitc=4; break; fi
      if gitx -C "$wt" merge-base --is-ancestor "$src" HEAD; then steps+="$i:skip "; continue; fi
      if ! gitx -C "$wt" merge-base HEAD "$src" >/dev/null 2>&1; then
        steps+="$i:unrelated "; verdict=unrelated_histories; exitc=3; cstep=$i; break; fi
      local msg="ilvuno: integrate ${src:0:12} (step $i/$n)"
      if SP1_NAME=ilvuno SP1_EMAIL=ilvuno@localhost \
         gitx -C "$wt" merge --no-ff --no-edit -m "$msg" "$src" >"$out/merge.log" 2>&1; then
        acc=$(gitx -C "$wt" rev-parse HEAD)
        steps+="$i:clean "
        if [ "$variant" = stop-after1 ] && [ $i -eq 1 ]; then verdict=stopped; exitc=9; break; fi
      else
        cstep=$i
        local cls; cls=$(classify_merge_failure "$wt" "$out/unmerged.nul")
        case "$cls" in
          conflict) unmerged_paths "$out/unmerged.nul" "$paths_file"; verdict=conflict; exitc=3; steps+="$i:conflict "
                    [ "$variant" != keep-conflict ] && gitx -C "$wt" merge --abort >/dev/null 2>&1 ;;
          operational_merge_state) verdict=operational_error; exitc=5; steps+="$i:operational "
                    op_detail="merge failed with MERGE_HEAD and no unmerged paths"
                    gitx -C "$wt" merge --abort >/dev/null 2>&1 ;;
          *) verdict=operational_error; exitc=5; steps+="$i:operational "
             op_detail="merge failed with no merge state and no unmerged paths" ;;
        esac
        break
      fi
    done
  fi
  ob_A=$(objcount "$repo")
  if [ "$verdict" = ok ]; then
    acc=$(gitx -C "$wt" rev-parse HEAD)
    gitx -C "$repo" rev-parse --verify --quiet "$CREF" >/dev/null && { verdict=operational_error; exitc=5; op_detail="ref collision"; }
    [ "$exitc" -eq 0 ] && { gitx -C "$repo" update-ref "$CREF" "$acc" "" || { verdict=operational_error; exitc=5; op_detail="ref create failed"; }; }
    if [ "$exitc" -eq 0 ]; then
      printf 'spike\ngit=%s\n' "$gitver" > "$out/meta.txt"
      local blob; blob=$(gitx -C "$repo" hash-object -w "$out/meta.txt")
      gitx -C "$repo" update-ref "$MREF" "$blob" "" || true
      if [ -f "$wt/check.sh" ]; then
        local got; got=$(sha256_of "$wt/check.sh")
        if [ "${M_CHECK:-}" != "$got" ]; then verdict=harness_refused; exitc=4
          printf 'check.sh digest mismatch\n' >> "$out/refusal.txt"
        else ( cd "$wt" && sh ./check.sh ) > "$out/check.log" 2>&1; check_rc=$?; fi
      fi
      if [ "$check_rc" -gt 0 ]; then verdict=check_failed; exitc=1; fi
      [ "$variant" = fail-cleanup ] && chmod a-w "$wt"
      if ! gitx -C "$repo" worktree remove --force "$wt" 2>"$out/rm.err"; then cleanup=failed; fi
      if [ "$cleanup" = ok ]; then cleanup=removed
      elif [ -d "$wt" ]; then
        if _owner_ok "$wt"; then safe_rmrf "$wt" && cleanup=orphaned_dir_removed || cleanup=orphaned_dir_left
        else cleanup=orphaned_dir_left; fi
      fi
      if [ "$(gitx -C "$repo" rev-parse "$CREF" 2>/dev/null)" = "$acc" ]; then
        gitx -C "$repo" update-ref -d "$CREF" "$acc" || true
        gitx -C "$repo" update-ref -d "$MREF" "$blob" || true
      fi
      gitx -C "$repo" worktree prune
    fi
  fi
  case "$cleanup" in failed|orphaned_dir_removed|orphaned_dir_left) [ "$exitc" -eq 0 ] && exitc=5 ;; esac
  fp_A=$(user_fingerprint "$repo" "$CREF" "$MREF")
  if [ -n "${acc:-}" ]; then content_manifest "$repo" "$acc" "$out/content.nul"; else : > "$out/content.nul"; fi
  read_nul "$out/content.nul" CONTENT; read_nul "$paths_file" PATHS
  local steps_arr=(); [ -n "$steps" ] && read -r -a steps_arr <<< "$steps"
  emit_array "$out" content "${CONTENT[@]:-}"
  emit_array "$out" conflict_paths "${PATHS[@]:-}"
  emit_array "$out" steps "${steps_arr[@]:-}"
  emit_json "$out" approach worktree fixture "${SP1_FIXTURE:-$(basename "$repo")}" variant "$variant" \
    verdict "$verdict" exit "$exitc" conflict_step "$cstep" check_rc "$check_rc" cleanup "$cleanup" \
    candidate "${acc:-}" tree "$([ -n "${acc:-}" ] && gitx -C "$repo" rev-parse "$acc^{tree}" 2>/dev/null || echo "")" \
    user_state_unchanged "$([ "$fp_B" = "$fp_A" ] && echo yes || echo no)" \
    refs_left "$(gitx -C "$repo" for-each-ref --format='%(refname)' | grep -c "^refs/ilvuno/runs/$SP1_RUN_ID/" || true)" \
    worktrees_left "$(ours_wt "$repo")" objects_before "$ob_B" objects_after "$ob_A" \
    drivers_ran "$([ -s "$out/drivers.marker" ] && echo yes || echo no)" \
    git_version "$gitver" os "$osname" base "$M_BASE" sources "${M_SRCS[*]}" \
    merge_policy "--no-ff --no-edit; hooks/attrs/signing/fsmonitor/submodules neutralised via -c" \
    metadata "ilvuno <ilvuno@localhost> @<base commit time>" operational_detail "$op_detail"
  return $exitc
}

cmd_run() { # fixture approach variant  (fixture is always harness-built)
  local fx=$1 appr=$2 variant=${3:-plain}
  local label; label=$(validate_label "${fx}-${appr}-${variant}") || die "unsafe label"
  local run="$SP1_SCRATCH/runs/$label" out="$SP1_SCRATCH/out/$label"
  safe_rmrf "$run" 2>/dev/null; safe_rmrf "$out" 2>/dev/null
  mkdir -p "$run" "$out"
  export SP1_FIXTURE="$fx"
  build_fixture "$fx" "$run/repo" || { echo "fixture build failed: $fx" >&2; return 2; }
  local rc=0
  if [ "$appr" = object ]; then run_object "$run/repo" "$out" "$variant"; rc=$?; else run_worktree "$run/repo" "$out" "$variant"; rc=$?; fi
  printf '%s %s %s rc=%s\n' "$fx" "$appr" "$variant" "$rc" >> "$SP1_SCRATCH/out/summary.txt"
  return $rc
}

cmd_matrix() {
  local update=0; [ "${2:-}" = "--update-expected" ] && update=1
  local all="$SP1_SCRATCH/out/all.jsonl"; : > "$all"; : > "$SP1_SCRATCH/out/summary.txt"
  local gv os; gv=$(gitx --version | awk '{print $3}'); os=$(uname -s)
  printf '{"git_version":"%s","os":"%s"}\n' "$gv" "$os" > "$SP1_SCRATCH/out/run-env.json"
  local f a v
  for f in $FIXTURES; do for a in object worktree; do
    cmd_run "$f" "$a" plain >/dev/null 2>&1
    cat "$SP1_SCRATCH/out/$f-$a-plain/result.json" >> "$all"
  done; done
  for v in fail-cleanup stop-after1 index-lock keep-conflict; do
    case $v in fail-cleanup) f=F9-cleanup ;; stop-after1) f=F2-three-clean ;; index-lock) f=F1-two-clean ;; keep-conflict) f=F3-direct-conflict ;; esac
    for a in object worktree; do
      cmd_run "$f" "$a" "$v" >/dev/null 2>&1
      cat "$SP1_SCRATCH/out/$f-$a-$v/result.json" >> "$all"
    done
  done
  if [ "$update" -eq 1 ]; then
    python3 "$SP1_SELF/expected.py" --update "$all" "$SP1_SELF/oracle.json" "$SP1_SELF/spec.json" "$SP1_SELF/expected.json" "$SP1_SCRATCH/out/run-env.json"; return $?
  fi
  python3 "$SP1_SELF/expected.py" --check "$all" "$SP1_SELF/expected.json" "$SP1_SCRATCH/out/run-env.json"; return $?
}

cmd_oracle() {
  local f d
  : > "$SP1_SCRATCH/oracle-repos.tsv"
  for f in $FIXTURES; do
    d=$(scratch_dir "fixtures-$f") || die "label"
    safe_rmrf "$d" 2>/dev/null; mkdir -p "$d"
    build_fixture "$f" "$d/repo" >/dev/null 2>&1 || die "oracle fixture build failed: $f"
    printf '%s\t%s\n' "$f" "$(cd "$d/repo" && pwd -P)" >> "$SP1_SCRATCH/oracle-repos.tsv"
  done
  python3 "$SP1_SELF/oracle.py" "$SP1_SCRATCH" "$SP1_SELF/oracle.json" $FIXTURES
}

cmd_fixtures() {
  local f d; for f in $FIXTURES; do
    d=$(scratch_dir "fixtures-$f") || die "label"
    safe_rmrf "$d" 2>/dev/null
    mkdir -p "$d"
    build_fixture "$f" "$d/repo" >/dev/null 2>&1 && echo "built $f"
  done
}

cmd_probes() {
  local out="$SP1_SCRATCH/out/probes"; safe_rmrf "$out" 2>/dev/null; mkdir -p "$out"
  local w="$SP1_SCRATCH/runs/probe-f7"; mkdir -p "$w"
  build_fixture F7-combined-build-failure "$w/repo" >/dev/null 2>&1
  read_manifest "$w/repo/.git/sp1-meta.env"
  { echo "P1: per-source check (F7)"; for s in "${M_SRCS[@]}"; do
      gitx -C "$w/repo" worktree add -q --detach "$w/one" "$s" 2>/dev/null
      ( cd "$w/one" && sh ./check.sh ) >/dev/null 2>&1; local rc=$?
      echo "  source ${s:0:8} alone check_rc=$rc"
      gitx -C "$w/repo" worktree remove --force "$w/one" 2>/dev/null; gitx -C "$w/repo" worktree prune
    done; } | tee "$out/p1-per-source.txt"
  for mode in with-ignore without-ignore; do
    local r="$SP1_SCRATCH/runs/probe-ignore-$mode"; mkdir -p "$r"
    build_fixture F9-cleanup "$r/repo" >/dev/null 2>&1
    [ "$mode" = without-ignore ] && sed_inplace '/\.ilvuno/d' "$r/repo/.gitignore" && gitx -C "$r/repo" commit -qam "drop ignore"
    mkdir -p "$r/repo/.ilvuno/worktrees"
    gitx -C "$r/repo" worktree add -q --detach "$r/repo/.ilvuno/worktrees/x" HEAD 2>/dev/null
    local st; st=$(gitx -C "$r/repo" status --porcelain | head -3 | paste -sd'|')
    echo "P2: $mode -> status=[${st:-clean}]" | tee -a "$out/p2-ignore.txt"
    chmod -R u+w "$r/repo/.ilvuno" 2>/dev/null; gitx -C "$r/repo" worktree remove --force "$r/repo/.ilvuno/worktrees/x" 2>/dev/null; gitx -C "$r/repo" worktree prune
  done
  local r3="$SP1_SCRATCH/runs/probe-orphan"; mkdir -p "$r3"
  build_fixture F9-cleanup "$r3/repo" >/dev/null 2>&1
  gitx -C "$r3/repo" worktree add -q --detach "$r3/wt" HEAD 2>/dev/null
  chmod a-w "$r3/wt"
  gitx -C "$r3/repo" worktree remove --force "$r3/wt" 2>"$out/p3.rm.err"; local rc3=$?
  { echo "P3: remove rc=$rc3 err=$(head -1 "$out/p3.rm.err")"
    echo "    worktree_list_entries=$(gitx -C "$r3/repo" worktree list --porcelain | grep -c '^worktree ')"
    echo "    admin_entries=$(ls "$r3/repo/.git/worktrees" 2>/dev/null | wc -l | tr -d ' ')"
    echo "    directory_still_present=$([ -d "$r3/wt" ] && echo yes || echo no)"
  } | tee "$out/p3-orphan.txt"
  chmod u+w "$r3/wt" 2>/dev/null; safe_rmrf "$r3/wt" 2>/dev/null; gitx -C "$r3/repo" worktree prune
  local r4="$SP1_SCRATCH/runs/probe-driver"; mkdir -p "$r4"
  DRIVER_MARKER="$out/drv.marker" build_fixture F8-driver "$r4/repo" >/dev/null 2>&1
  read_manifest "$r4/repo/.git/sp1-meta.env"
  { echo "P4: driver override matrix"
    for v in control 2 true text; do
      local cfg=(); [ "$v" != control ] && cfg=(-c "merge.upper.driver=$v")
      rm -f "$out/drv.marker"
      local o1 rc1 o2 rc2
      o1=$(gitx -C "$r4/repo" "${cfg[@]}" merge-tree --write-tree --no-messages "$(gitx -C "$r4/repo" rev-parse s1)" "$(gitx -C "$r4/repo" rev-parse s2)" 2>&1); rc1=$?
      gitx -C "$r4/repo" worktree add -q --detach "$r4/wt" "$(gitx -C "$r4/repo" rev-parse s1)" 2>/dev/null
      o2=$(gitx -C "$r4/wt" "${cfg[@]}" merge --no-ff --no-edit -m m "$(gitx -C "$r4/repo" rev-parse s2)" 2>&1); rc2=$?
      local st; st=$(gitx -C "$r4/wt" status --porcelain | head -1)
      local mt_content content
      mt_content=$(gitx -C "$r4/repo" show "$(echo "$o1" | head -1):driverfile.txt" 2>/dev/null | tr '\n' '|')
      content=$(cat "$r4/wt/driverfile.txt" 2>/dev/null | tr '\n' '|')
      echo "  driver=$v merge-tree exit=$rc1 content=[$mt_content] | merge exit=$rc2 state=[${st:-clean}] content=[$content] marker=$([ -s "$out/drv.marker" ] && echo yes || echo no)"
      gitx -C "$r4/wt" merge --abort 2>/dev/null; gitx -C "$r4/repo" worktree remove --force "$r4/wt" 2>/dev/null
    done; } | tee "$out/p4-driver.txt"
}

if [ "${SP1_LIB_ONLY:-0}" != "1" ]; then
  case "${1:-}" in
    fixtures) cmd_fixtures ;;
    run) cmd_run "$2" "$3" "${4:-plain}" ;;
    matrix) cmd_matrix "$@" ;;
    probes) cmd_probes ;;
    oracle) cmd_oracle ;;
    *) sed -n '2,8p' "$0"; exit 2 ;;
  esac
fi
