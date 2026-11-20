# SP-1R — Security Results

Scope: the experiment harness only. This document records what was tested, what the harness now
refuses or neutralises, and what it explicitly does **not** contain.

Tested on Git **2.55.0** (CachyOS Linux) and **2.38.1** (pinned Alpine container), non-root in both.
Every payload wrote only inside the scratch root; sentinels prove nothing outside was touched.

---

## 1. Execution controls in the harness

| Control | Mechanism | Evidence |
|---|---|---|
| Environment | `env -i` with an allowlist; `GIT_CONFIG_*` cannot be inherited | SG-2, SG-2b |
| Global/system config | `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_SYSTEM=/dev/null`, `GIT_CONFIG_NOSYSTEM=1` | all runs |
| Hooks | `-c core.hooksPath=<empty dir>` on **every** invocation | SG-1 |
| Attributes file | `-c core.attributesFile=/dev/null` | all runs |
| Signing | `-c commit.gpgsign=false -c tag.gpgsign=false` | all runs |
| Submodules | `-c submodule.recurse=false`; status asserted | SG-10 |
| Merge policy | `--no-ff --no-edit`, fixed message, fixed envelope; recorded per row | matrix `merge_policy` |
| Executable config | refused before any write and before each step | SG-3…SG-7 |

`check_exec_features` refuses (exit 4, verdict `harness_refused`) when the repository configures:

```
filter.*.smudge | filter.*.clean | filter.*.process
merge.<name>.driver (non-builtin) | merge.default
diff.*.command | diff.*.textconv
core.fsmonitor (enabled)
```

Detection is **config-based**, not attribute-based. That is deliberate: SG-5 and SG-6 showed that
attribute resolution is version-dependent and path-dependent (`check-attr --source` absent on 2.38;
`info/attributes` lives in the common dir for linked worktrees). A configured executable feature is
refused regardless of which attribute file selects it, which closes those bypasses without
depending on `check-attr` at all.

---

## 2. Security-gate results

| Gate | Scenario | Expected | Observed (2.55.0 / 2.38.1) |
|---|---|---|---|
| **SG-1** | `post-checkout`, `pre-merge-commit`, `post-merge` marker hooks; `core.hooksPath` set | hooks do not run | PASS / PASS — marker absent, run completes |
| **SG-2** | `GIT_CONFIG_COUNT/KEY/VALUE` re-point `core.hooksPath` at marker hooks | injection stripped | PASS / PASS — marker absent |
| **SG-2b** | child environment inspection | no `GIT_CONFIG_KEY_*`, `GIT_CONFIG_COUNT=0` | PASS / PASS |
| **SG-3** | `merge.default=spy` + driver, no attribute | refuse; driver not run | PASS / PASS — exit 4 |
| **SG-4** | driver configured; first source adds `.gitattributes` | refuse before/at the step; no driver | PASS / PASS |
| **SG-5** | `core.attributesFile` selects the driver | refuse; driver not run | PASS / PASS |
| **SG-6** | linked-worktree `info/attributes` selects the driver | refuse; driver not run | PASS / PASS |
| **SG-7** | `filter.spy.smudge` + `filter=spy` attribute | refuse before any checkout | PASS / PASS — driver not run |
| **SG-8** | `.ilvuno` symlink to an external directory | nothing written outside; harness does not use `.ilvuno` | PASS / PASS — sentinel intact |
| **SG-9** | `check.sh` modified after the digest was recorded (candidate content) | refuse; tampered script not executed | PASS / PASS — exit 4, marker absent |
| **SG-10** | superproject with a gitlink | submodule not initialised | PASS / PASS — `-<sha> deps/mod` |
| **SG-11** | cleanup fails after admin removal | orphan directory detected and reported | PASS / PASS — `orphaned_dir_removed`, exit 5 |

---

## 3. Driver override matrix (probe P4)

Characterization only — run in a throwaway fixture, **not** in the pipeline. It exists to justify the
refusal policy, not to offer an alternative.

| Override | `merge-tree` | `git merge` | Driver ran | Content | Safe? |
|---|---|---|---|---|---|
| control (driver active) | exit 0 | exit 0 | **yes** | uppercased | no |
| `driver=2` | exit 1 | exit 1, `UU` | no | ours | no — spawns `2`; on 2.38 `merge-tree` returned **0** |
| `driver=true` | exit 0 | exit 0 | no | **ours** | **no — silent loss of the other side** |
| `driver=text` | exit 1 | exit 1, `UU` | no | ours | no — semantics differ by command/version |

On Git 2.38.1 the `driver=text` case returned **exit 0** from `merge-tree` while a real merge
conflicted. That is why the harness refuses instead of trying to neutralize.

The earlier SP-1 claim that `-c merge.<name>.driver=…` could be used to disable a driver is
**withdrawn**: it is either an error, silent data loss, or version-dependent.

---

## 4. Destructive-path results

| Test | Injection | Required behaviour | Result |
|---|---|---|---|
| P0.1 | label `../../victim` | reject | PASS |
| P0.2 | `safe_rmrf` on a path outside the scratch root | reject | PASS |
| P0.3 | scratch child is a symlink to an external directory | reject | PASS |
| P0.3b | sentinel inside the symlink target | untouched | PASS |
| P0.4 | scratch root itself is a symlink | reject | PASS |
| P0.5 | scratch root inside a Git work tree | reject | PASS |
| P0.6 | legacy `SP1_VER=../../victim` | no effect on paths | PASS |
| P0.7 | any pipeline run | nothing written in the harness directory | PASS |

`safe_rmrf` is the only recursive delete. It refuses `/`, `.`, `..`, empty paths, paths outside the
scratch root (after `realpath`), symlinked components, and paths not owned by the current user.

The cleanup path additionally verifies both the directory and the Git admin entry:

```
removed | orphaned_dir_removed | orphaned_dir_left | failed
```

and deletes `refs/ilvuno/*` only when the ref still points at the SHA this run wrote.

---

## 5. What the harness still does not contain

- **Repository-provided commands.** The harness executes exactly one script — its own `check.sh`,
  verified by digest from the candidate tree. It never runs a project's test suite. If a repository
  replaced that script, the digest check refuses (SG-9).
- **Process effects.** A command that does run (in production, or in probe P4's driver) can write
  anywhere the user can. This is not a sandbox; `env -i` and `-c` control *Git's* interpretation,
  not arbitrary programs.
- **The user's global configuration.** Deliberately not modified; it is excluded from every child
  instead.
- **Filesystem isolation.** The candidate worktree lives in the scratch root, but nothing prevents
  an executed program from escaping it.

Production policy for hooks, filters, drivers, attributes, submodules, and arbitrary verification
commands remains an owner decision (D-07…D-10 plus the new hook/filter policy decision). This work
does not pre-empt it.
