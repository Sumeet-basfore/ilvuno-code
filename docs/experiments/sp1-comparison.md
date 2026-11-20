# SP-1 · Comparison

Both approaches ran the same 9 fixtures, 5 variants, 4 probes, on two Git versions, with identical
pins, message format, and committer envelope. Numbers below come from
`docs/experiments/scripts/sp1.sh`; raw output is in `/tmp/sp1/out/<version>/all.jsonl` during a run.

| | A — object-only | B — worktree |
|---|---|---|
| Mechanism | `merge-tree --write-tree` + `commit-tree` per step | `git merge --no-ff` in a detached worktree |
| Checkout needed | only after success | before the first merge |
| Git floor | requires `merge-tree --write-tree` (≥ 2.38) | any Git with worktrees (≥ 2.5) |
| Harness size | 65 lines (`run_object`) | 68 lines (`run_worktree`) |
| Git calls, median (max) | 23 (30) | **21 (25)** |
| Median wall time, 2.55 / 2.38 | 761 / 168 ms | 786 / 201 ms |

> **SP-1R update (2026-11-19).** This report describes the original SP-1 run. The harness has since been
> remediated: the matrix is an enforcing test (`expected.json`, 28 cases), output goes to a validated
> private scratch root, Git runs under a controlled environment, driver refusal is config-based, and
> classification uses repository state rather than stderr. Numbers below that refer to "8 clean
> cases" remain the original corpus result; current enforced results are in
> `sp1r-regression-matrix.md`. See `sp1r-remediation-report.md` for the finding-by-finding status.

## 1. Equivalence

**8 clean-integration cases (5 fixtures + 3 variants) × 2 Git versions:** candidate SHAs identical between A and B, and identical
across versions. Trees identical. Conflict step and paths identical. Conflict-tree OIDs identical
across versions (`c2a33c2dc44f…`, `c83cbd3b4866…`).

This is the strongest result of SP-1 and the answer to "is the object pipeline a different
product?" — **no, on this corpus it is the same candidate.**

It does not establish universal equivalence. See §7 of `sp1-object-pipeline.md`.

## 2. Dimension-by-dimension

Legend: **A** / **B** / **=** — better, with the evidence that decided it.

| Dimension | Verdict | Evidence |
|---|---|---|
| **Git correctness** | **=** | Same verdict, step, paths, exit on all 9 fixtures. Same SHA on 8 clean cases. |
| **Fatal classification** | **A** | Unrelated histories: A exit 2 at step 0 with exit 128; B exit 2 at step 2, only after an stderr string match. |
| **Tree equivalence** | **=** | 8/8 clean cases, both versions. |
| **Intermediate side effects** | **A (slightly)** | Identical object writes (median +3.5 objects), but A writes nothing but objects until success; B creates a worktree, an index, and logs before the first merge. |
| **Required complexity** | **=** | 65 vs 68 lines; A needs `-z` tree parsing + `commit-tree` + envelope; B needs lifecycle, abort, and stderr classification. A makes 2 more Git calls on average; B checks out a tree. |
| **Reliability (success path)** | **=** | Identical outcomes and cleanup on every clean-integration case. |
| **Reliability (failure path)** | **A** | On conflict/crash: A leaves 0 worktrees and 0 refs; B leaves 1 worktree. A's only residue is an unreferenced commit; B's is a directory plus admin metadata. |
| **Crash behaviour** | **A** | Stop-after-1: A has nothing to repair and re-runs deterministically; B must remove or reuse the existing path before re-running. |
| **Worktree cleanliness** | **=** | With `.ilvuno/` gitignored, `git status` is clean in both (P2). Without it, both pollute (`?? .ilvuno/`). `index.lock` in the main repo blocks neither. |
| **Debuggability (conflict)** | **B, if kept** / **A, if aborted** | B before abort has markers, stages, `MERGE_HEAD`; after abort it has nothing. A always has the conflicted tree object (`git show <tree>:path`) but no stages. |
| **Debuggability (post-mortem)** | **A** | The conflict tree OID is in the report; B's evidence is a directory that may be gone. |
| **Minimum Git compatibility** | **B** | `merge` is universal; `merge-tree --write-tree` requires 2.38. Both ran on 2.38.1, but the floor is a hard dependency for A and not for B. |
| **Security boundaries** | **=** | Neither is a sandbox. Both execute a configured merge driver identically (§`sp1-security-observations.md`). Both write only Git objects + their own refs + worktree metadata. |
| **Filter/eol fidelity** | **B** | B merges in a checkout, so filters participate exactly as they will for the user; A merges blobs and materializes afterwards. Not exercised in these fixtures. |
| **Cost at scale (100k files)** | **A** on failure, **=** on success | A avoids a checkout when the integration fails; B pays it always. Untested at scale. |

## 3. Pass/fail matrix (required fixtures)

| Fixture | A | B | Pass criterion met by |
|---|---|---|---|
| F1 two independent clean changes | pass | pass | both |
| F2 three independent clean changes | pass | pass | both |
| F3 direct file conflict | pass | pass | both (step 2, `shared.txt`, exit 3) |
| F4 conflict during the second merge step | pass | pass | both (step 2, exit 3) |
| F5 branch already integrated | pass | pass | both (skip, no empty merge commit) |
| F6 unrelated histories | pass | pass (with stderr match) | A without string parsing; B needs it |
| F7 compatible merges with combined build failure | pass | pass | both (`check_failed`, exit 1, combination-only confirmed by P1) |
| F8 custom merge-driver attempt | pass (refused) | pass (refused) | both, **after** the 2.38 fallback fix |
| F9 worktree cleanup failure | pass (exit 5, refs reported) | pass (exit 5, orphan directory reported) | both detect; neither fully recovers without a directory check |

Variants: `ignore-drivers` (both execute the driver, identical SHA), `stop-after1` (A clean, B
leaves a worktree), `index-lock` (neither affected), `drivers-text` (behaviour differs by version
and approach — see security doc).

## 4. What SP-1 settles, and what it does not

**Settled:**

1. The corrected Phase 7 design (A) is implementable and produces identical candidates to the
   conventional approach (B), on the clean case set and two Git versions.
2. `merge-tree --write-tree` exists and works on 2.38.1; `--name-only -z --no-messages` output is
   parseable on both versions.
3. A custom merge driver runs during `merge-tree` exactly as during `merge`; refusal must be
   implemented in **both** designs.
4. `check-attr --source` is **not** available on 2.38. SP-1R removed the dependency entirely: the
   harness now refuses any configured executable feature regardless of which attribute file selects
   it (SG-3…SG-7), so attribute resolution is not part of the control.
5. Worktrees created inside the repo require the `.ilvuno/` ignore line in both designs.
6. `git worktree remove` can delete admin metadata and leave a directory behind. Cleanup must check
   the directory, not only `worktree list`.
7. Both approaches can leave the main working tree untouched; this is a property of the design
   (ignore line + no writes outside the candidate), not of the mechanism.

**Not settled:**

1. Universal equivalence of `merge-tree` and `merge` (submodules, LFS, filters, `merge.ff`, `-X`,
   huge renames, case-insensitive filesystems).
2. Behaviour on macOS.
3. Cost at 100k files and on network filesystems.
4. Concurrent runs against one repository.
5. Whether a repository's own hooks/attributes/`info/attributes` can defeat the detection fallback
   in ways not covered by F8.
6. Crash *during* `git merge` itself (only crash *between* merges was simulated).

## 5. Headline numbers for the decision

```
candidate equality .......... 8/8 clean-integration cases, 2 Git versions, 2 approaches
conflict agreement .......... 2/2 fixtures: same step, same path, same exit
cross-version SHAs .......... identical (2.38.1 vs 2.55.0) on every clean fixture
median Git calls ............ A 23 / B 21
worktrees left after crash ... A 0 / B 1
refs left after crash ........ A 0 / B 0
driver executions when refused  A 0 / B 0   (after the 2.38 fallback fix)
driver executions when allowed  A 1 / B 1   (identical resulting SHA)
fixtures failing gate ........ 0 / 9 for both
```

Zero failures on the required fixtures means the choice is **not** about capability. It is about
which failure mode and which fidelity risk the project prefers to own — see
`sp1-recommendation.md`.
