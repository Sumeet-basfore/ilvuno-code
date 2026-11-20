#!/usr/bin/env python3
"""SP-1R2 expectation builder/checker.

Expected values come from a hand-reviewed spec (spec.json) and an independent
Git-native oracle (oracle.json). They are NEVER derived from the pipeline under
test.

  expected.py --update all.jsonl oracle.json spec.json expected.json
      Validates the observed run strictly (28 rows, valid JSON, required fields,
      unique keys, no extras), builds expectations from spec+oracle, and refuses
      to write unless the observed run already agrees with them.

  expected.py --check all.jsonl expected.json
      Fails on missing/extra rows, malformed JSON, missing fields, duplicates,
      unknown verdicts, and any field mismatch (including git version profile,
      OS, merge policy and commit metadata).
"""
import json, sys

REQUIRED = {"fixture", "approach", "variant", "verdict", "exit", "conflict_step",
            "conflict_paths", "check_rc", "cleanup", "candidate", "tree", "content",
            "base", "sources", "git_version", "os", "merge_policy", "metadata",
            "user_state_unchanged", "refs_left", "worktrees_left", "drivers_ran"}
COMPARED = ["verdict", "exit", "conflict_step", "conflict_paths", "check_rc", "cleanup",
            "candidate", "tree", "content", "base", "sources", "merge_policy",
            "metadata", "user_state_unchanged", "refs_left", "worktrees_left", "drivers_ran"]
INT_FIELDS = {"exit", "conflict_step", "check_rc", "refs_left", "worktrees_left"}
KNOWN = {"ok", "check_failed", "conflict", "unrelated_histories", "harness_refused",
         "stopped", "operational_error", "worktree_add_failed"}

def key(r):
    return f"{r['fixture']}|{r['approach']}|{r['variant']}"

def load(path):
    rows, errors = {}, []
    for n, line in enumerate(open(path, "rb"), 1):
        line = line.rstrip(b"\n")
        if not line:
            continue
        try:
            r = json.loads(line.decode("utf-8", "surrogateescape"))
        except Exception as e:
            errors.append(f"line {n}: malformed JSON: {e}"); continue
        missing = REQUIRED - set(r)
        if missing:
            errors.append(f"line {n}: missing fields {sorted(missing)}"); continue
        if r["verdict"] not in KNOWN:
            errors.append(f"line {n}: unknown verdict {r['verdict']!r}")
        for f in INT_FIELDS:
            if not isinstance(r.get(f), int):
                errors.append(f"line {n}: {f} not an integer")
        k = key(r)
        if k in rows:
            errors.append(f"line {n}: duplicate case {k}")
        rows[k] = r
    return rows, errors

def load_run_env(path):
    re_ = json.load(open(path, encoding="utf-8"))
    if not re_.get("git_version") or not re_.get("os"):
        raise SystemExit("run-env: missing git_version or os")
    return re_

def build(spec, oracle, run_env):
    env = spec["environment"]
    if len(spec["cases"]) != spec["declared_case_count"]:
        raise SystemExit(f"spec: declared {spec['declared_case_count']} cases, found {len(spec['cases'])}")
    out = {}
    for k, c in sorted(spec["cases"].items()):
        fixture, approach, variant = k.split("|")
        fx = oracle["fixtures"].get(fixture)
        if fx is None:
            raise SystemExit(f"spec: unknown fixture in case {k}")
        if approach not in ("object", "worktree"):
            raise SystemExit(f"spec: bad approach in {k}")
        cases = fx["candidates"]
        src = c["candidate_from"]
        if src == "none":
            cand = {"sha": "", "tree": "", "content": []}
        elif src == "stop1":
            cand = cases[1] if len(cases) > 1 else {"sha": "", "tree": "", "content": []}
        else:
            cand = cases[-1]
        paths = []
        if c["conflict_paths_from"] == "oracle" and fx.get("conflict"):
            paths = fx["conflict"]["paths"]
        step = fx["conflict"]["step"] if (fx.get("conflict") and c["verdict"] in ("conflict", "unrelated_histories")) else 0
        out[k] = {
            "verdict": c["verdict"], "exit": c["exit"], "conflict_step": step,
            "conflict_paths": paths, "check_rc": c["check_rc"], "cleanup": c["cleanup"],
            "candidate": cand["sha"], "tree": cand["tree"],
            "content": [x for x in cand["content"]],
            "base": fx["base"], "sources": " ".join(fx["sources"]),
            "os": env["os"][0], "merge_policy": env["merge_policy"], "metadata": env["metadata"],
            "user_state_unchanged": "yes", "refs_left": 0,
            "worktrees_left": c["worktrees_left"], "drivers_ran": "no",
        }
    return out, env

def project(r, env):
    out = {"git_version": r.get("git_version"), "os": r.get("os")}
    for f in COMPARED:
        out[f] = r.get(f)
    for f in INT_FIELDS:
        try: out[f] = int(out[f])
        except Exception: pass
    if out.get("git_version") not in env["allowed_git_versions"]:
        out["git_version"] = f"UNSUPPORTED:{out.get('git_version')}"
    return out

def compare(exp, obs, env, run_env, label=""):
    fails = []
    for f in COMPARED:
        if exp.get(f) != obs.get(f):
            fails.append(f"{label}{f}: expected {exp.get(f)!r} got {obs.get(f)!r}")
    # Bind every row to the binary this run actually executed.
    if obs.get("git_version") != run_env["git_version"]:
        fails.append(f"{label}git_version: run executed {run_env['git_version']!r}, row says {obs.get('git_version')!r}")
    if obs.get("os") != run_env["os"]:
        fails.append(f"{label}os: run executed on {run_env['os']!r}, row says {obs.get('os')!r}")
    if str(obs.get("git_version", "")).startswith("UNSUPPORTED:"):
        fails.append(f"{label}git_version: unsupported {obs.get('git_version')!r}")
    return fails

def main():
    mode = sys.argv[1]
    if mode == "--update":
        results, oracle_p, spec_p, out_p, runenv_p = sys.argv[2:7]
        rows, errors = load(results)
        spec = json.load(open(spec_p, encoding="utf-8"))
        oracle = json.load(open(oracle_p, encoding="utf-8"))
        run_env = load_run_env(runenv_p)
        if run_env["git_version"] not in spec["environment"]["allowed_git_versions"]:
            print(f"REFUSING: run Git version {run_env['git_version']} is not supported"); return 1
        if run_env["os"] not in spec["environment"]["os"]:
            print(f"REFUSING: run OS {run_env['os']} is not supported"); return 1
        expected, env = build(spec, oracle, run_env)
        if errors:
            print("REFUSING to write expectations: malformed input")
            for e in errors[:20]: print("  " + e)
            return 1
        if len(rows) != spec["declared_case_count"]:
            print(f"REFUSING: observed {len(rows)} rows, declared {spec['declared_case_count']}")
            return 1
        missing = sorted(set(expected) - set(rows)); extra = sorted(set(rows) - set(expected))
        if missing or extra:
            print("REFUSING: case set differs from spec")
            for k in missing: print("  missing: " + k)
            for k in extra: print("  extra: " + k)
            return 1
        allf = []
        for k in sorted(expected):
            allf += compare(expected[k], project(rows[k], env), env, run_env, k + ": ")
        if allf:
            print("REFUSING: observed run disagrees with the independent oracle")
            for f in allf[:40]: print("  " + f)
            return 1
        json.dump({"schema": "ilvuno.dev/sp1-expected/v3", "environment": env,
                   "cases": expected}, open(out_p, "w", encoding="utf-8"),
                  indent=2, sort_keys=True)
        open(out_p, "a").write("\n")
        print(f"recorded {len(expected)} cases from spec+oracle -> {out_p}")
        return 0

    results, exp_p, runenv_p = sys.argv[2], sys.argv[3], sys.argv[4]
    rows, errors = load(results)
    doc = json.load(open(exp_p, encoding="utf-8"))
    exp, env = doc["cases"], doc["environment"]
    run_env = load_run_env(runenv_p)
    if run_env["git_version"] not in env["allowed_git_versions"]:
        print(f"FAIL: run Git version {run_env['git_version']} is not supported"); return 1
    if run_env["os"] not in env["os"]:
        print(f"FAIL: run OS {run_env['os']} is not supported"); return 1
    fails = list(errors)
    for k in sorted(set(exp) - set(rows)): fails.append("missing row: " + k)
    for k in sorted(set(rows) - set(exp)): fails.append("unexpected row: " + k)
    for k in sorted(set(exp) & set(rows)):
        fails += compare(exp[k], project(rows[k], env), env, run_env, k + ": ")
    if fails:
        print(f"FAIL: {len(fails)} problem(s) across {len(exp)} expected case(s)")
        for f in fails[:80]: print("  " + f)
        if len(fails) > 80: print(f"  ... {len(fails)-80} more")
        return 1
    print(f"PASS: {len(exp)} cases, all fields match expectations")
    return 0

if __name__ == "__main__":
    sys.exit(main())
