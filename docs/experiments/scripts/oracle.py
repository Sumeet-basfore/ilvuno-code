#!/usr/bin/env python3
"""Independent Git-native oracle, with a controlled execution boundary.

  oracle.py <scratch> <out.json> <fixture>...

Inputs must be fixtures created by the same harness invocation:
  * <scratch>/.sp1r-owner records format/uid/id/parent and must match SP1_RUN_ID
  * <scratch>/oracle-repos.tsv lists name<TAB>realpath for each fixture repo
  * fixture names are allowlisted; repo realpaths must match and sit under scratch
Every Git subprocess runs with a sanitised environment and neutralised
hooks/attributes/signing/submodules. A fixture whose repository configures an
executable Git feature is recorded as `refused` WITHOUT running a checkout or
merge (its expected refusal comes from spec.json).

The merge algorithm itself (native `git merge --no-ff` in a detached worktree)
remains independent of the pipeline implementations in sp1.sh.
"""
import hashlib, json, os, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import schema

FIXTURES = ["F1-two-clean", "F2-three-clean", "F3-direct-conflict", "F4-conflict-step2",
            "F5-already-integrated", "F6-unrelated", "F7-combined-build-failure",
            "F8-driver", "F9-cleanup", "F10-newline-path"]
REFUSE_PREFIXES = ("filter.", "merge.default", "diff.")

def die(msg, code=2):
    print(f"oracle: {msg}", file=sys.stderr)
    raise SystemExit(code)

def read_kv(path):
    out = {}
    with open(path, "r", encoding="utf-8") as f:
        for line in f.read().splitlines():
            k, _, v = line.partition("=")
            out[k] = v
    return out

def scratch_env(scratch):
    env = {
        "PATH": "/usr/local/bin:/usr/bin:/bin",
        "HOME": os.path.join(scratch, "sandbox-home"),
        "TMPDIR": os.path.join(scratch, "sandbox-tmp"),
        "LANG": "C", "LC_ALL": "C", "TZ": "UTC",
        "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
        "GIT_CONFIG_SYSTEM": "/dev/null", "GIT_TERMINAL_PROMPT": "0",
        "GIT_CONFIG_COUNT": "0",
    }
    return env

def cfg(scratch):
    return ["-c", f"core.hooksPath={os.path.join(scratch, 'hooks')}",
            "-c", "core.attributesFile=/dev/null",
            "-c", "core.fsmonitor=false",
            "-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false",
            "-c", "submodule.recurse=false", "-c", "gc.auto=0"]

def git(scratch, repo, *args, check=True, extra_env=None):
    env = scratch_env(scratch)
    if extra_env:
        env.update(extra_env)
    p = subprocess.run([os.environ.get("SP1_GIT_BIN", "git"), "-C", repo, *cfg(scratch), *args],
                       capture_output=True, env=env)
    if check and p.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)}: {p.returncode}: {p.stderr.decode('utf-8','replace')[:200]}")
    return p

def configured_features(scratch, repo):
    """Return refusal reasons for executable Git features in local config."""
    p = git(scratch, repo, "config", "--local", "--list", check=False)
    reasons = []
    for line in p.stdout.decode("utf-8", "replace").splitlines():
        key, _, val = line.partition("=")
        if key.startswith("filter.") and key.rsplit(".", 1)[-1] in ("smudge", "clean", "process"):
            reasons.append(f"{key} configured")
        elif key == "merge.default":
            reasons.append("merge.default configured")
        elif key.startswith("merge.") and key.endswith(".driver") and val not in ("true", ":", "binary", "text", "union"):
            reasons.append(f"custom merge driver: {key}")
        elif key.startswith("diff.") and key.rsplit(".", 1)[-1] in ("command", "textconv"):
            reasons.append(f"external diff: {key}")
        elif key == "core.fsmonitor" and val not in ("", "false"):
            reasons.append("core.fsmonitor enabled")
    return reasons

def check_marker(scratch):
    if not os.path.isabs(scratch):
        die("scratch must be absolute")
    if os.path.islink(scratch) or not os.path.isdir(scratch):
        die("scratch is not a plain directory")
    marker = os.path.join(scratch, ".sp1r-owner")
    if os.path.islink(marker) or not os.path.isfile(marker):
        die("missing ownership marker")
    m = read_kv(marker)
    if m.get("format") != "ilvuno-sp1r-scratch/1":
        die("bad ownership marker format")
    if m.get("uid") != str(os.getuid()) or os.stat(scratch).st_uid != os.getuid():
        die("scratch is not owned by this user")
    rid = os.environ.get("SP1_RUN_ID", "")
    if not rid or m.get("id") != rid:
        die("run id does not match the scratch marker")
    parent = m.get("parent", "")
    if scratch not in (parent, os.path.join(parent, "run-" + rid)):
        die("scratch is not the run-owned root for this id")

def load_registry(scratch):
    reg = {}
    path = os.path.join(scratch, "oracle-repos.tsv")
    if not os.path.isfile(path):
        die("missing fixture registry")
    with open(path, "r", encoding="utf-8") as f:
        for line in f.read().splitlines():
            if not line:
                continue
            name, _, real = line.partition("\t")
            reg[name] = real
    return reg

def fixture_repo(scratch, name, reg):
    if name not in FIXTURES:
        die(f"fixture not allowlisted: {name}")
    if name not in reg:
        die(f"fixture not registered by this invocation: {name}")
    repo = os.path.join(scratch, "fixtures-" + name, "repo")
    real = os.path.realpath(repo)
    if real != reg[name]:
        die(f"fixture path does not match the registry: {name}")
    if os.path.commonpath([real, os.path.realpath(scratch)]) != os.path.realpath(scratch):
        die(f"fixture escapes the scratch root: {name}")
    if os.path.islink(repo) or not os.path.isdir(os.path.join(real, ".git")):
        die(f"fixture is not a harness-created repository: {name}")
    return real

def content(repo, sha, scratch):
    ls = git(scratch, repo, "ls-tree", "-r", "-z", "--full-tree", sha).stdout
    out = []
    for rec in ls.split(b"\0"):
        if not rec:
            continue
        meta, _, path = rec.partition(b"\t")
        mode, typ, oid = meta.split(b" ")
        if typ.decode() != "blob":
            out.append(path.decode("utf-8", "surrogateescape") + "=object:" + oid.decode())
            continue
        blob = git(scratch, repo, "cat-file", "blob", oid.decode()).stdout
        out.append(path.decode("utf-8", "surrogateescape") + "=" + hashlib.sha256(blob).hexdigest())
    return out

def oracle_one(scratch, name, repo):
    try:
        m = schema.read_manifest_file(os.path.join(repo, ".git", "sp1-meta.env"))
    except (schema.ManifestError, OSError) as e:
        die(f"{name}: invalid manifest: {e}")
    base = m["BASE"]
    sources = m["SRCS"]
    reasons = configured_features(scratch, repo)
    if reasons:
        return {"base": base, "sources": sources, "status": "refused",
                "reasons": reasons, "steps": [], "candidates": [], "conflict": None}
    wt = os.path.join(scratch, "oracle-wt", name)
    os.makedirs(os.path.dirname(wt), exist_ok=True)
    git(scratch, repo, "worktree", "add", "-q", "--detach", wt, base)
    try:
        cands = [{"after": 0, "sha": base,
                  "tree": git(scratch, repo, "rev-parse", f"{base}^{{tree}}").stdout.decode().strip(),
                  "content": content(repo, base, scratch)}]
        steps, conflict, status = [], None, "ok"
        for i, src in enumerate(sources, 1):
            if git(scratch, wt, "merge-base", "--is-ancestor", src, "HEAD", check=False).returncode == 0:
                steps.append({"index": i, "source": src, "status": "already_integrated"}); continue
            if git(scratch, wt, "merge-base", "HEAD", src, check=False).returncode != 0:
                steps.append({"index": i, "source": src, "status": "unrelated"})
                conflict, status = {"step": i, "paths": []}, "unrelated_histories"; break
            again = configured_features(scratch, repo)
            if again:
                return {"base": base, "sources": sources, "status": "refused",
                        "reasons": again, "steps": steps, "candidates": [], "conflict": None}
            ts = git(scratch, repo, "show", "-s", "--format=%ct", base).stdout.decode().strip()
            msg = f"ilvuno: integrate {src[:12]} (step {i}/{len(sources)})"
            env = {"GIT_AUTHOR_NAME": "ilvuno", "GIT_AUTHOR_EMAIL": "ilvuno@localhost",
                   "GIT_COMMITTER_NAME": "ilvuno", "GIT_COMMITTER_EMAIL": "ilvuno@localhost",
                   "GIT_AUTHOR_DATE": f"@{ts} +0000", "GIT_COMMITTER_DATE": f"@{ts} +0000"}
            p = git(scratch, wt, "merge", "--no-ff", "--no-edit", "-m", msg, src,
                    check=False, extra_env=env)
            if p.returncode == 0:
                head = git(scratch, wt, "rev-parse", "HEAD").stdout.decode().strip()
                cands.append({"after": i, "sha": head,
                              "tree": git(scratch, wt, "rev-parse", "HEAD^{tree}").stdout.decode().strip(),
                              "content": content(repo, head, scratch)})
                steps.append({"index": i, "source": src, "status": "clean"})
            else:
                un = git(scratch, wt, "ls-files", "-u", "-z").stdout.split(b"\0")
                paths = []
                for r in un:
                    if not r: continue
                    pth = r.split(b"\t", 1)[1] if b"\t" in r else r
                    s = pth.decode("utf-8", "surrogateescape")
                    if s not in paths: paths.append(s)
                conflict = {"step": i, "paths": paths}
                status = "conflict" if paths else "operational"
                break
        return {"base": base, "sources": sources, "status": status, "steps": steps,
                "candidates": cands, "conflict": conflict}
    finally:
        subprocess.run([os.environ.get("SP1_GIT_BIN", "git"), "-C", repo, "worktree",
                        "remove", "--force", wt], capture_output=True, env=scratch_env(scratch))
        subprocess.run([os.environ.get("SP1_GIT_BIN", "git"), "-C", repo, "worktree", "prune"],
                       capture_output=True, env=scratch_env(scratch))

def main():
    if len(sys.argv) < 4:
        die("usage: oracle.py <scratch> <out.json> <fixture>...")
    scratch, out = sys.argv[1], sys.argv[2]
    names = sys.argv[3:]
    if len(set(names)) != len(names):
        die("duplicate fixture arguments")
    check_marker(scratch)
    reg = load_registry(scratch)
    data = {"schema": "ilvuno.dev/sp1-oracle/v1", "git_version": None, "fixtures": {}}
    for f in names:
        repo = fixture_repo(scratch, f, reg)
        data["fixtures"][f] = oracle_one(scratch, f, repo)
    tmp = out + ".tmp"
    with open(tmp, "w", encoding="utf-8") as fh:
        json.dump(data, fh, indent=2, sort_keys=True, ensure_ascii=True)
        fh.write("\n")
    os.replace(tmp, out)
    print(f"oracle: {len(data['fixtures'])} fixtures -> {out}")

if __name__ == "__main__":
    main()
