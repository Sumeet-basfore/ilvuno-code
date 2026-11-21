#!/usr/bin/env bash
# SP-1R2 harness library. Throwaway experiment code, not production.
#
# Threat model (M0): one harness process per fresh private scratch root; no
# concurrent same-UID adversary. Path validation is NOT TOCTOU-safe; see
# docs/experiments/sp1r2-closure-evidence.md. Production hostile-filesystem
# cleanup remains a later security gate.
set -uo pipefail
# Never write .pyc files, so a direct run cannot add bytecode to this (tracked) directory.
export PYTHONDONTWRITEBYTECODE=1

SP1_SELF=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SP1_LABEL_RE='^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$'
SP1_REGISTERED_REPOS=()

die() { printf 'sp1r: %s\n' "$*" >&2; exit 2; }

# ------------------------------------------------- portability (GNU and BSD userland) --
# The harness runs on Linux (GNU userland) and macOS (BSD userland, bash 3.2). Each helper picks
# the form that both accept. None of them changes what the oracle records.
sha256_cmd() { # prints the SHA-256 command that works here: GNU sha256sum, or BSD shasum -a 256
  if printf '' | sha256sum >/dev/null 2>&1; then echo sha256sum
  elif printf '' | shasum -a 256 >/dev/null 2>&1; then echo "shasum -a 256"
  else return 1; fi
}
sha256_of() { # sha256_of FILE: SHA-256 hex digest of FILE
  local c; c=$(sha256_cmd) || die "no SHA-256 tool (sha256sum or shasum)"
  $c "$1" | cut -d' ' -f1
}
sha256_stdin() { # SHA-256 hex digest of standard input
  local c; c=$(sha256_cmd) || die "no SHA-256 tool (sha256sum or shasum)"
  $c | cut -d' ' -f1
}
sed_inplace() { # sed_inplace SCRIPT FILE: in-place edit without a backup suffix (BSD sed needs one)
  local s=$1 f=$2
  sed -e "$s" "$f" > "$f.sed.$$" || { rm -f -- "$f.sed.$$"; return 1; }
  cat "$f.sed.$$" > "$f" && rm -f -- "$f.sed.$$"
}
file_uid() { stat -c %u "$1" 2>/dev/null || stat -f %u "$1" 2>/dev/null; }   # GNU, then BSD
canon_path() { # canon_path P: canonical absolute path of P; the parent of P must exist (no realpath -m)
  local p=$1 d b
  case "$p" in */*) ;; *) return 1 ;; esac
  d=${p%/*}; b=${p##*/}
  [ -n "$b" ] || return 1
  [ -n "$d" ] || d=/
  d=$(cd -P -- "$d" 2>/dev/null && pwd -P) || return 1
  case "$d" in /) printf '/%s\n' "$b" ;; *) printf '%s/%s\n' "$d" "$b" ;; esac
}
rm_tree() { # rm_tree PATH: recursive delete; GNU rm also stays on one filesystem (BSD rm has no such option)
  if rm --version >/dev/null 2>&1; then rm -rf --one-file-system -- "$1"; else rm -rf -- "$1"; fi
}

# ------------------------------------------------------------------ scratch --
# A unique, private root per invocation. A caller-supplied path is treated as a
# PARENT: we create a new exclusive child and never delete or modify the parent
# or its existing contents.
_new_id() { od -An -tx1 -N16 /dev/urandom | tr -d ' \n'; }   # 32 hex chars

init_scratch() {
  local parent="${SP1_SCRATCH:-}" child id
  id=$(_new_id)
  if [ -z "$parent" ]; then
    parent=$(mktemp -d "${TMPDIR:-/tmp}/sp1r.XXXXXXXX") || die "cannot create scratch parent"
    parent=$(cd "$parent" && pwd -P)
    child="$parent"                                  # we own the whole mktemp dir
    SP1_SCRATCH_IS_PARENT=0
  else
    local real
    [ -d "$parent" ] || die "SP1_SCRATCH parent is not a directory: $parent"
    case "$parent" in /*) ;; *) die "SP1_SCRATCH must be absolute" ;; esac
    [ -L "$parent" ] && die "SP1_SCRATCH parent must not be a symlink"
    [ "$(file_uid "$parent")" = "$(id -u)" ] || die "SP1_SCRATCH parent not owned by current user"
    real=$(cd "$parent" && pwd -P)
    SP1_SCRATCH_IS_PARENT=1
    child="$real/run-$id"
    mkdir -m700 "$child" 2>/dev/null || die "cannot create exclusive scratch child under $real"
  fi
  # Marker: never overwrite an existing one.
  local marker="$child/.sp1r-owner"
  ( set -C; printf 'format=ilvuno-sp1r-scratch/1\nuid=%s\nid=%s\nparent=%s\n' \
      "$(id -u)" "$id" "$parent" > "$marker" ) 2>/dev/null \
    || die "scratch marker exists or cannot be created: $marker"
  SP1_SCRATCH=$(cd "$child" && pwd -P)
  SP1_RUN_ID=$id
  case "$SP1_SCRATCH" in "$parent"/*|"$parent") ;; *) die "scratch child escaped its parent" ;; esac
  local repo_root; repo_root=$(git -C "$SP1_SELF" rev-parse --show-toplevel 2>/dev/null) || repo_root=""
  if [ -n "$repo_root" ] && [ "${SP1_SCRATCH#"$repo_root"/}" != "$SP1_SCRATCH" ]; then
    die "refusing to use a scratch root inside a git work tree ($repo_root)"
  fi
  case "$SP1_SCRATCH" in "$SP1_SELF"|"$SP1_SELF"/*) die "refusing to write output inside the harness directory" ;; esac
  export SP1_SCRATCH SP1_RUN_ID
  mkdir -p "$SP1_SCRATCH"/{hooks,sandbox-home,sandbox-tmp}
  chmod 700 "$SP1_SCRATCH"
  export SP1_EMPTY_HOOKS="$SP1_SCRATCH/hooks"
  export SP1_SANDBOX_HOME="$SP1_SCRATCH/sandbox-home"
  export SP1_SANDBOX_TMP="$SP1_SCRATCH/sandbox-tmp"

  # Optional cleanup: only ever the run-owned child.
  if [ "${SP1_CLEAN:-0}" = "1" ]; then
    trap 'cleanup_scratch || exit 2' EXIT
  fi
}

# Remove exactly the run-owned root. Verifies the ownership marker and that the
# root is the one this invocation created; never touches the parent's contents.
remove_run_root() {
  local root="${SP1_SCRATCH:-}" marker id parent rec_uid
  [ -n "$root" ] && [ -d "$root" ] || { printf 'cleanup: no run root\n' >&2; return 1; }
  [ -L "$root" ] && { printf 'cleanup: root is a symlink\n' >&2; return 1; }
  marker="$root/.sp1r-owner"
  [ -f "$marker" ] && [ ! -L "$marker" ] || { printf 'cleanup: missing owner marker\n' >&2; return 1; }
  grep -qx 'format=ilvuno-sp1r-scratch/1' "$marker" || { printf 'cleanup: bad marker format\n' >&2; return 1; }
  rec_uid=$(sed -n 's/^uid=//p' "$marker"); [ "$rec_uid" = "$(id -u)" ] || { printf 'cleanup: marker uid mismatch\n' >&2; return 1; }
  id=$(sed -n 's/^id=//p' "$marker"); [ "$id" = "${SP1_RUN_ID:-}" ] || { printf 'cleanup: marker id mismatch\n' >&2; return 1; }
  parent=$(sed -n 's/^parent=//p' "$marker")
  case "$root" in
    "$parent"/run-"$id"|"$parent") ;;
    *) printf 'cleanup: root does not match marker parent/id\n' >&2; return 1 ;;
  esac
  # no symlink components below the root's parent
  local comp=$parent part rel
  rel=${root#"$parent"/}
  if [ -n "$rel" ] && [ "$rel" != "$root" ]; then
    local IFS=/
    for part in $rel; do
      comp="$comp/$part"
      [ -L "$comp" ] && { printf 'cleanup: symlink component %s\n' "$comp" >&2; return 1; }
    done
    unset IFS
  fi
  find -P "$root" -type d -exec chmod u+w {} + 2>/dev/null || true
  if rm_tree "$root"; then
    printf 'cleanup: removed %s\n' "$root" >&2
    return 0
  fi
  printf 'cleanup: FAILED to remove %s\n' "$root" >&2
  return 1
}

cleanup_scratch() { remove_run_root; }

validate_label() {
  local l=$1
  [ -n "$l" ] || { printf 'sp1r: empty label\n' >&2; return 1; }
  case "$l" in .|..) printf 'sp1r: unsafe label: %s\n' "$l" >&2; return 1 ;; esac
  printf '%s' "$l" | grep -Eq "$SP1_LABEL_RE" || { printf 'sp1r: unsafe label: %s\n' "$l" >&2; return 1; }
  printf '%s' "$l"
}

scratch_dir() {
  local l; l=$(validate_label "$1") || return 1
  local p="$SP1_SCRATCH/$l"
  case "$p" in "$SP1_SCRATCH"/*) ;; *) die "path escapes scratch root" ;; esac
  printf '%s' "$p"
}

# ------------------------------------------------------------------- delete --
_owner_ok() { [ "$(file_uid "$1")" = "$(id -u)" ]; }

# The only recursive delete. Refuses anything not beneath the run-owned root.
# NOTE: check/use window exists; see the threat-model note at the top.
safe_rmrf() {
  local p=$1
  [ -n "$p" ] || die "safe_rmrf: empty path"
  case "$p" in /|.|..) die "safe_rmrf: refusing $p" ;; esac
  case "$p" in "$SP1_SCRATCH"/*) ;; *) die "safe_rmrf: outside run-owned root: $p" ;; esac
  [ -L "$p" ] && die "safe_rmrf: refusing symlink: $p"
  local real; real=$(canon_path "$p") || die "safe_rmrf: canonicalisation failed"
  case "$real" in "$SP1_SCRATCH"/*) ;; *) die "safe_rmrf: escapes after resolve" ;; esac
  local comp=$SP1_SCRATCH part
  local rel=${real#"$SP1_SCRATCH"/}
  local IFS=/
  for part in $rel; do
    comp="$comp/$part"
    [ -L "$comp" ] && die "safe_rmrf: symlink component: $comp"
    [ -e "$comp" ] && { _owner_ok "$comp" || die "safe_rmrf: not owned: $comp"; }
  done
  unset IFS
  [ -e "$real" ] || return 0
  find -P "$real" -type d -exec chmod u+w {} + 2>/dev/null || true
  rm_tree "$real"
}

# ------------------------------------------------------------ repo registry --
register_repo() { # path
  local p; p=$(cd "$1" && pwd -P) || die "cannot register repo"
  SP1_REGISTERED_REPOS+=("$p")
}
is_registered_repo() {
  local p; p=$(cd "$1" 2>/dev/null && pwd -P) || return 1
  local r
  for r in "${SP1_REGISTERED_REPOS[@]:-}"; do [ "$r" = "$p" ] && return 0; done
  return 1
}

# ----------------------------------------------------------------- manifest --
# Strict, non-executable fixture manifest: KEY=VALUE lines, no shell evaluation.
#   FORMAT=1
#   BASE=<full oid>
#   SRCS=<full oid>[ <full oid>[ <full oid>]]
#   CHECK_SHA256=<64 hex>        (optional)
read_manifest() { # path -> sets M_FORMAT M_BASE M_SRCS[] M_CHECK
  local f=$1 line key val
  M_FORMAT="" M_BASE="" M_CHECK="" M_SRCS=()
  [ -e "$f" ] || { printf 'manifest: missing\n' >&2; return 1; }
  [ -L "$f" ] && { printf 'manifest: symlink refused\n' >&2; return 1; }
  local out
  out=$(python3 "$SP1_SELF/schema.py" --shell "$f" 2>/dev/null) || { printf 'manifest: invalid\n' >&2; return 1; }
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    key=${line%%=*}; val=${line#*=}
    case "$key" in
      FORMAT) M_FORMAT=$val ;;
      BASE) M_BASE=$val ;;
      SRCS) read -r -a M_SRCS <<< "$val" ;;
      CHECK_SHA256) M_CHECK=$val ;;
    esac
  done <<< "$out"
  [ "$M_FORMAT" = "1" ] && [ -n "$M_BASE" ] && [ ${#M_SRCS[@]} -ge 2 ] && return 0
  printf 'manifest: incomplete\n' >&2; return 1
}

# ------------------------------------------------------------- git wrapper ---
sp1_env() {
  env -i \
    PATH=/usr/local/bin:/usr/bin:/bin \
    HOME="$SP1_SANDBOX_HOME" TMPDIR="$SP1_SANDBOX_TMP" XDG_CONFIG_HOME="$SP1_SANDBOX_HOME/.config" \
    LANG=C LC_ALL=C TZ=UTC \
    GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null \
    GIT_ATTR_NOSYSTEM=1 GIT_TERMINAL_PROMPT=0 GIT_ASKPASS=/bin/false \
    GIT_PAGER=cat PAGER=cat GIT_EDITOR=true GIT_SEQUENCE_EDITOR=true GIT_MERGE_AUTOEDIT=no \
    GIT_AUTHOR_NAME="${SP1_NAME:-fixture}" GIT_AUTHOR_EMAIL="${SP1_EMAIL:-fixture@example.com}" \
    GIT_COMMITTER_NAME="${SP1_NAME:-fixture}" GIT_COMMITTER_EMAIL="${SP1_EMAIL:-fixture@example.com}" \
    GIT_AUTHOR_DATE="${SP1_DATE:-2020-01-01T00:00:00Z}" GIT_COMMITTER_DATE="${SP1_DATE:-2020-01-01T00:00:00Z}" \
    GIT_CONFIG_COUNT=0 \
    "$@"
}

gitx() {
  local gitbin="${SP1_GIT_BIN:-git}"
  if [ "$gitbin" != git ]; then
    case "$gitbin" in /*) ;; *) die "SP1_GIT_BIN must be absolute" ;; esac
    [ -x "$gitbin" ] || die "SP1_GIT_BIN not executable: $gitbin"
  fi
  sp1_env "$gitbin" \
    -c core.hooksPath="$SP1_EMPTY_HOOKS" \
    -c core.attributesFile=/dev/null \
    -c core.fsmonitor=false \
    -c core.pager=cat \
    -c core.quotepath=false \
    -c core.autocrlf=false \
    -c core.eol=lf \
    -c gc.auto=0 \
    -c commit.gpgsign=false \
    -c tag.gpgsign=false \
    -c submodule.recurse=false \
    -c advice.detachedHead=false \
    "$@"
}

check_exec_features() { # repo out
  local repo=$1 out=$2 reasons=() v
  local cfg; cfg=$(gitx -C "$repo" config --local --list 2>/dev/null)
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    v=${line#*=}
    case "$line" in
      filter.*.smudge=*|filter.*.clean=*|filter.*.process=*) reasons+=("filter: $line") ;;
      merge.default=*) reasons+=("merge.default: $line") ;;
      merge.*.driver=*)
        case "$v" in true|:|binary|text|union) ;; *) reasons+=("merge driver: $line") ;; esac ;;
      diff.*.command=*|diff.*.textconv=*) reasons+=("diff driver: $line") ;;
      core.fsmonitor=*) case "$v" in false|"") ;; *) reasons+=("fsmonitor: $line") ;; esac ;;
    esac
  done <<< "$cfg"
  local leaked; leaked=$(env | grep -E '^GIT_CONFIG_(COUNT|KEY_|VALUE_)' || true)
  [ -n "$leaked" ] && reasons+=("runtime config injection present")
  if [ ${#reasons[@]} -gt 0 ]; then printf '%s\n' "${reasons[@]}" > "$out/refusal.txt"; return 4; fi
  return 0
}

# ------------------------------------------------------------ measurements --
objcount() { gitx -C "$1" cat-file --batch-all-objects --batch-check='%(objectname)' 2>/dev/null | wc -l | tr -d ' '; }
refs_snapshot() { gitx -C "$1" for-each-ref --format='%(refname) %(objectname)' | sort; }
refs_without() { # repo excluded_ref...
  local repo=$1; shift
  local out; out=$(refs_snapshot "$repo")
  local r line
  for r in "$@"; do
    out=$(printf '%s\n' "$out" | grep -v -F -x "$r $(gitx -C "$repo" rev-parse "$r" 2>/dev/null)" || true)
    out=$(printf '%s\n' "$out" | grep -v -F "$r " || true)
  done
  printf '%s' "$out"
}
ours_wt() { gitx -C "$1" worktree list --porcelain | awk -v m="$1" '/^worktree /{p=$2; if (p!=m) o++} END{print o+0}'; }

user_fingerprint() { # repo [excluded ref...]
  local repo=$1; shift
  { gitx -C "$repo" rev-parse HEAD 2>/dev/null || echo none
    gitx -C "$repo" status --porcelain=v1 2>/dev/null
    gitx -C "$repo" ls-files -s 2>/dev/null | sha256_stdin
    gitx -C "$repo" stash list 2>/dev/null
    refs_without "$repo" "$@"
    gitx -C "$repo" config --local --list 2>/dev/null | sha256_stdin
    gitx -C "$repo" worktree list --porcelain 2>/dev/null \
      | awk -v p="$SP1_SCRATCH" '/^worktree /{skip=(index($0,"worktree " p)==1)} !skip' || true
    ls -1 "$repo/.git/hooks" 2>/dev/null || true
  } | sha256_stdin
}

unmerged_paths() { # src.nul dest.nul
  python3 - "$1" "$2" <<'PY'
import sys
recs=open(sys.argv[1],'rb').read().split(b'\0')
seen=[]
for r in recs:
    if not r: continue
    p=r.split(b'\t',1)[1] if b'\t' in r else r
    if p not in seen: seen.append(p)
with open(sys.argv[2],'wb') as f:
    for p in seen: f.write(p+b'\0')
PY
}

# Byte-exact content manifest: `git ls-tree -rz` + hash of the blob object bytes.
# Any missing object or non-regular entry fails (never an empty-content fallback).
content_manifest() { # repo sha out.nul
  local gitbin="${SP1_GIT_BIN:-git}"
  [ "$gitbin" = git ] && gitbin=$(command -v git)
  python3 - "$gitbin" "$1" "$2" "$3" <<'PY'
import hashlib, subprocess, sys, os
gitbin, repo, sha, out = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
if gitbin == "git":
    gitbin = "/usr/bin/git" if os.path.exists("/usr/bin/git") else "git"
env = dict(os.environ)
env.update({"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null","GIT_CONFIG_SYSTEM":"/dev/null",
            "GIT_TERMINAL_PROMPT":"0"})
def git(*args, **kw):
    return subprocess.run([gitbin, "-C", repo, *args], check=True, capture_output=True, env=env, **kw)
ls = git("ls-tree","-r","-z","--full-tree",sha).stdout
entries=[]
for rec in ls.split(b"\0"):
    if not rec: continue
    meta, _, path = rec.partition(b"\t")
    if not _: raise SystemExit("ls-tree record without path")
    mode, typ, oid = meta.split(b" ")
    entries.append((mode.decode(), typ.decode(), oid.decode(), path))
with open(out,"wb") as f:
    for mode, typ, oid, path in entries:
        if typ == "blob":
            blob = git("cat-file","blob",oid).stdout
            h = hashlib.sha256(blob).hexdigest()
            rec = path + b"=" + h.encode()
        else:
            rec = path + b"=object:" + oid.encode()
        f.write(rec + b"\0")
PY
}

# Machine-state classification for a failed merge in a worktree.
classify_merge_failure() { # worktree paths_nul_file -> conflict|operational_merge_state|operational_no_state
  local wt=$1 out=$2
  gitx -C "$wt" ls-files -u -z > "$out" 2>/dev/null
  if [ -s "$out" ]; then echo conflict; return 0; fi
  local admin; admin=$(gitx -C "$wt" rev-parse --git-path MERGE_HEAD 2>/dev/null)
  if [ -n "$admin" ] && [ -f "$admin" ]; then echo operational_merge_state; return 0; fi
  echo operational_no_state
}

# ------------------------------------------------------------- json output ---
emit_json() {
  local out=$1; shift
  : > "$out/result.kv"
  while [ $# -gt 0 ]; do
    printf '%s\t%s\n' "$1" "$(printf '%s' "$2" | base64 -w0)" >> "$out/result.kv"
    shift 2
  done
  python3 "$SP1_SELF/emit.py" "$out" | tee "$out/result.json"
}

emit_array() {
  local out=$1 key=$2; shift 2
  : > "$out/$key.nul"
  local e
  for e in "$@"; do printf '%s\0' "$e" >> "$out/$key.nul"; done
}

# read_nul FILE NAME: fill array NAME with the NUL-terminated records of FILE. Written without mapfile -d,
# which bash 3.2 (the macOS default) lacks. A missing file is an error, not an empty list.
read_nul() {
  local f=$1 name=$2 item
  [ -f "$f" ] || die "read_nul: missing $f"
  eval "$name=()"
  while IFS= read -r -d '' item || [ -n "$item" ]; do eval "$name+=(\"\$item\")"; done < "$f"
}
