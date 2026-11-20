# SP-1 · Experiment A — Object-Only Sequential Pipeline

**Question:** can a candidate be built by performing the sequential merges entirely in the Git
object database — no working tree, no index — and does the result equal what `git merge` produces?

**Answer: yes on 8 clean-integration cases across two Git versions; identical 40-character candidate SHAs.**
Two defects were found in the control logic, one of them a fail-open driver check on the declared
floor (§6).

Experiment date: 2026-11-19. Script: `docs/experiments/scripts/sp1.sh` (`run_object`).
Fixtures are built per run in `/tmp` and never touch the Ilvuno repository.

| Environment | Git | OS |
|---|---|---|
| Host | **2.55.0** | CachyOS Linux, kernel 7.1.8, x86_64 |
| Container | **2.38.1** (built from source on Alpine 3.17 with `NO_REGEX=YesPlease`) | Alpine Linux 3.17, x86_64 |

2.38.1 is the declared floor (ADR-0004) and is now **verified to run the pipeline**, which closes
the open question X-09 for this code path.

> **SP-1R update (2026-11-19).** This report describes the original SP-1 run. The harness has since been
> remediated: the matrix is an enforcing test (`expected.json`, 28 cases), output goes to a validated
> private scratch root, Git runs under a controlled environment, driver refusal is config-based, and
> classification uses repository state rather than stderr. Numbers below that refer to "8 clean
> cases" remain the original corpus result; current enforced results are in
> `sp1r-regression-matrix.md`. See `sp1r-remediation-report.md` for the finding-by-finding status.

---

## 1. Algorithm as implemented

```
P0  pin        \. "$repo/.git/sp1-meta.env"     # BASE, SRCS populated by the fixture builder
               base_sha, src_sha[i]             # already commits; a real CLI runs rev-parse here
P2  gate       git config --get-regexp '^merge\..*\.driver$'
               git check-attr --source=<sha> merge -- <path>     (>=2.40)
               fallback: scan .gitattributes blobs in the tree   (2.38)
P3  loop       for src in order:
                 git merge-base --is-ancestor "$src" "$acc"  -> skip
                 git merge-tree --write-tree --name-only -z --no-messages "$acc" "$src"
                 # exit 0: stdout "<tree>\0"                 -> commit
                 # exit 1: stdout "<tree>\0<path>\0..."      -> conflict, stop
                 # exit 128: unrelated histories             -> fatal
P3b commit     GIT_AUTHOR_*=ilvuno GIT_COMMITTER_*=ilvuno
               GIT_*_DATE=@<base commit time> +0000
                 git commit-tree "$tree" -p "$acc" -p "$src" \
                     -m "ilvuno: integrate ${src:0:12} (step i/n)"
P4  publish    git update-ref refs/ilvuno/candidates/r "$acc"
               git update-ref refs/ilvuno/meta/r "$(git hash-object -w meta.txt)"
               git worktree add --detach <repo>/.ilvuno/worktrees/candidate "$acc"
P5  verify     ( cd <worktree> && sh ./check.sh )      # fixture-local check only
P6  cleanup    archive patch if dirty -> git worktree remove --force -> update-ref -d x2 -> prune
```

The conflict tree is **never committed**. It is recorded by OID for inspection.

---

## 2. Results — clean integrations

`A` = this experiment. Every row also ran the worktree pipeline (`B`); the `A=B` column is the
equivalence result.

| Fixture | Sources | Exit | Objects added | A=B SHA | A=B tree | Notes |
|---|---|---|---|---|---|---|
| F1 two independent clean | 2 | 0 | 4 | **YES** | **YES** | `b051e4bf28bc…` both versions |
| F2 three independent clean | 3 | 0 | 7 | **YES** | **YES** | — |
| F5 one source already integrated | 2 | 0 | 1 | **YES** | **YES** | step 1 skipped (`merge-base --is-ancestor`) |
| F7 compatible merges, combined check fails | 2 | 1 | 4 | **YES** | **YES** | check exit 1 on both; each source alone passes (P1) |
| F9 cleanup fixture | 2 | 0 | 4 | **YES** | **YES** | — |
| F8 driver, refusal enabled | 2 | 4 | 0 | n/a | n/a | refused before any object write |
| F8 driver, `--ignore-drivers` | 2 | 0 | 4 | **YES** | **YES** | driver executed in both |
| F1 with `index.lock` present | 2 | 0 | 4 | **YES** | **YES** | pipeline never touches the index |

Candidate SHAs are identical between **Git 2.55.0 and 2.38.1** for every clean fixture, and
identical to the worktree pipeline within each version. Two consecutive runs of F2 produced
`e56498262728…` both times; F1 produced `b051e4bf28bc…` on both Git versions.

This is 8 clean-integration cases × 2 Git versions × 2 approaches. It is evidence of equivalence on this
corpus; it is **not** a proof of universal equivalence (§7).

## 3. Results — conflicts

| Fixture | Exit | Failing step | Paths | Conflict tree (recorded, not committed) |
|---|---|---|---|---|
| F3 direct conflict | 3 | 2 | `shared.txt` | `c2a33c2dc44f…` (same OID on 2.38 and 2.55) |
| F4 conflict at the second merge step | 3 | 2 | `shared.txt` | `c83cbd3b4866…` (same OID on both) |

Properties verified:

- the **step index is exact** (2, not 1 and not 3) in both fixtures;
- no `refs/ilvuno/*` ref is written and no worktree is created on the conflict path;
- the conflicted tree exists as an object and can be read with
  `git show <tree>:shared.txt`, so the conflict is inspectable without a checkout;
- the tree is not committed, so a conflict cannot be mistaken for a candidate.

## 4. Results — unrelated histories

```
$ git merge-tree --write-tree --no-messages <src1> <orphan>
fatal: refusing to merge unrelated histories        # stderr, exit 128
```

Recorded as `fatal`, step **0** — the condition is detected before any merge is attempted, with a
distinct exit class. The worktree pipeline reports the same condition through `git merge` exit 1
plus an stderr match, and therefore cannot distinguish it from a conflict by exit code alone.

## 5. Results — interruption

`--stop-after 1` simulates a crash after the first merge commits (exit 9, no cleanup):

| Leftover | A | B (worktree) |
|---|---|---|
| `refs/ilvuno/*` | 0 | 0 |
| worktrees (excluding main) | **0** | **1** |
| loose objects added | 1 | 1 |
| main working tree untouched | yes | yes |

The single unreferenced commit object is garbage-collectable and is re-derivable: re-running the
pipeline from the same pinned SHAs produces the same SHA (verified). This is what makes
"crash = aborted run, re-run" credible for A — there is no state to repair.

## 6. Defects found — both real, both fixed in the harness

### 6.1 Driver refusal failed open on Git 2.38 (severity: high)

`git check-attr --source=<tree>` exists on 2.55 but **not** on 2.38 (`exit 129`). The first
implementation used it unconditionally, so on 2.38 driver detection returned nothing and the
custom driver **executed**:

```
# on Git 2.38.1, before the fallback:
F8-driver object plain   verdict=ok   exit=0   drivers_ran=yes     <-- fail open
# after adding the .gitattributes-tree fallback:
F8-driver object plain   verdict=driver_refused  exit=4  drivers_ran=no
```

This is exactly the class of bug the review warned about, found by running the floor. The fallback
scans `.gitattributes` blobs in the merge inputs and is deliberately conservative (any in-tree
selection of a configured non-builtin driver refuses the run).

### 6.2 Harness artefacts polluted the main tree (severity: low, measurement only)

The first harness wrote `meta.env`/`result.json` inside the fixture repository, so
`main_untouched` reported `NO` for correct runs. Fixed by writing all artefacts outside the work
tree (`out/`), keeping the metadata in `.git/sp1-meta.env`, and counting only non-main worktrees.
After the fix, `main_untouched` is `yes` in every scenario for both approaches, with `.ilvuno/`
ignored.

## 7. Limits of this experiment

1. **No universal equivalence claim.** `merge-tree` approximates `git merge` on 8 fixtures plus
   4 probes. Edge cases not exercised: submodules, LFS filters, `merge.ff` settings, `-X` strategy
   options, rename limits, sparse checkout, case-only renames on case-insensitive filesystems.
2. `merge-tree` works on blobs; `git merge` works on a checkout. Filters (`core.autocrlf`,
   `filter.*.smudge`) apply to the checkout, so a repository with content-transforming filters
   could yield a candidate tree that differs from the tested working tree. Not tested.
3. Fixtures are tiny (≤ 20 files). Timing differences at this scale are noise and are not used as
   evidence.
4. Only two Git versions were tested, both on x86_64 Linux. macOS is untested.
5. The harness is a spike (367 lines including both approaches, fixtures, probes), not production
   code. No structured error types, no journal, no report.

## 8. What this experiment changes in the readiness proposal

| Readiness claim | Status after SP-1 |
|---|---|
| "Object pipeline produces the same candidate as real merges" | **Supported** on 8 clean-integration cases × 2 versions (SHA equality, not just tree equality) |
| "Preflight writes only objects, never refs/index/worktree" | **Confirmed**: on conflict, 0 refs, 0 worktrees; objects written even on conflict |
| "Driver refusal is implementable via `check-attr --source`" | **Corrected**: needs a fallback on 2.38; without it the control fails open |
| "Determinism across Git versions" | **Supported** for the corpus (2.38.1 vs 2.55.0 identical SHAs), still not a universal claim |
| "Recovery is recompute, not resume" | **Supported**: identical SHA on re-run, no partial state |
