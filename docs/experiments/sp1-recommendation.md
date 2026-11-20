# SP-1 · Recommendation

Status: engineering recommendation. No production code, no approved document modified, nothing
pushed. Awaiting owner approval.

> **SP-1R update (2026-11-19).** This report describes the original SP-1 run. The harness has since been
> remediated: the matrix is an enforcing test (`expected.json`, 28 cases), output goes to a validated
> private scratch root, Git runs under a controlled environment, driver refusal is config-based, and
> classification uses repository state rather than stderr. Numbers below that refer to "8 clean
> cases" remain the original corpus result; current enforced results are in
> `sp1r-regression-matrix.md`. See `sp1r-remediation-report.md` for the finding-by-finding status.

---

## 1. Recommendation in one paragraph

**Adopt the disposable-worktree pipeline (B) as the merge engine, with the hardening in §3.**
It produced byte-identical candidates to the object-only pipeline on every clean fixture across two
Git versions, it is the same `git merge` a developer would run (so "this is what your merge looks
like" is true by construction rather than by equivalence testing), it does not depend on a Git
feature introduced in 2.38, and on conflict it leaves the artefact a developer actually needs —
markers, stage entries, `MERGE_HEAD` — instead of discarding it. Its one real weakness, partial
state on failure, is bounded to one directory and mechanically removable. **Keep the object-only
pipeline in the codebase as a benchmark comparator and as the documented fallback**, not as the
primary path.

This **reverses the Phase 7 proposal** (`docs/readiness/02-git-integration-correction.md` §3,
decision D-07). The reversal is cheap and evidence-preserving: both pipelines produce the same
candidate SHA on every clean fixture, so evidence and approvals remain valid if the engine is
swapped later.

## 2. Why, given that both pass

SP-1 shows zero capability difference on the required fixtures. The decision therefore rests on
which risk the project prefers to own.

| Risk | A (object-only) | B (worktree) |
|---|---|---|
| **Silent fidelity divergence** (`merge-tree` ≠ `merge` on some repo) | Possible; would undermine the product's central claim, and would be hard to notice | Structurally impossible: it *is* `merge` |
| **Partial state on failure** | None (0 worktrees, 0 refs) | One worktree + admin metadata; visible, removable, journaled |
| **Floor dependency** | `merge-tree --write-tree` (2.38+), verified but a hard requirement | `merge`, available everywhere |
| **Debuggability on conflict** | Conflict tree object only (no stages, no markers on disk) | Full conflict state, if the worktree is kept |
| **Checkout/filter fidelity** | Merge on blobs, materialize afterwards — filters can differ | Merge in the checkout; what is tested is what merged |

A fidelity divergence is **silent and product-destroying**; a leftover directory is **loud and
repairable**. Prefer the loud failure. That is the whole argument.

The counter-argument — that B's leftovers complicate recovery — is real but small, and SP-1
quantified it: one worktree, zero refs, one object. With `--keep-worktree` on conflict it becomes a
feature (inspect), and `abort` becomes the single recovery action.

## 3. Mandatory hardening (both designs, but B especially)

| # | Requirement | Why (evidence) |
|---|---|---|
| **H-1** | `git worktree add --detach <path> <pinned_sha>`; never `-B`; publish `refs/ilvuno/candidates/<run>` with `update-ref` | `-B refs/ilvuno/x` creates `refs/heads/refs/ilvuno/x` (verified twice, both versions) |
| **H-2** | Refuse custom merge drivers *and any other configured executable feature* before the first merge and before every step; judge by repository config, not by attribute resolution | Driver executes in `merge` *and* `merge-tree`; attribute-based detection is version- and path-dependent (SG-3…SG-7) |
| **H-3** | Stop at the first conflict; **keep** the worktree by default, print the path and inspection command, and remove it only on an explicit cleanup | After `merge --abort` the only debugging artefact is gone; the harness now keeps it (`keep-conflict`) |
| **H-4** | Classify unrelated histories by an explicit stderr match; anything unrecognised is `fatal_unknown`, never `conflict` | `git merge` returns 1 for both; a silent reclassification is a wrong verdict |
| **H-5** | Cleanup verifies the **directory** and the admin entries, and journals the outcome (`removed | orphaned_dir | stale_admin`) | `worktree remove` deletes admin metadata before failing on the directory (P3) |
| **H-6** | `init` writes `.ilvuno/` to `.gitignore`; `doctor` fails if it is missing | Without it, every candidate makes the main `git status` dirty (P2) |
| **H-7** | Fixed committer envelope (`ilvuno <ilvuno@localhost>`, date from the base commit, `--no-ff`, fixed message) | Required for identical SHAs across runs and versions; verified |
| **H-8** | Recovery = **re-run from pinned SHAs**, never a resume state machine for merges. A crash is an aborted run | Recompute produced identical SHAs; the pipeline has no durable intermediate state worth resuming |
| **H-9** | One run per project (advisory lock), and refuse if the candidate path already exists unless it is journaled as ours | A leftover worktree from a crash would otherwise collide on re-run |

H-1, H-2, H-5, H-6, H-8 are new or sharpened relative to the readiness documents; H-2's fallback and
H-5's directory check were both discovered in SP-1.

## 4. What changes in the existing proposal

| Document | Change (proposed, not applied) |
|---|---|
| `readiness/02` §3 | Replace the object-pipeline algorithm with B + H-1…H-9. Keep §3.5's conflict-tree note as an aside, not the mechanism |
| `readiness/02` §5 | Keep the detached-worktree ref model exactly (unchanged; verified) |
| `readiness/02` §7 | Strengthen the driver policy: refusal is mandatory; add the "never `-c` neutralise" table from `sp1-security-observations.md` §2 |
| `readiness/02` §8 | Determinism contract unchanged; note that both pipelines gave the same SHAs, so the guarantee does not depend on the mechanism |
| `readiness/05` D-07 | Reopen and resolve as **B + hardening**; D-10 unchanged |
| `planning/02-milestones.md` M2 | Rename "candidate construction" tasks: delete the `merge-tree` parsing deliverable; add driver fallback + cleanup verification |
| `tasks/T-0204` | Stays deleted (no preflight) |
| `tasks/T-0205` | Rewrite around B: worktree lifecycle, merge loop, conflict retention, classification |
| `tasks/T-0501` | Reduce further: no `merge_outcome_unknown`; only bundle/cleanup uncertainty |
| `docs/architecture/05` §4 | Ref-style text unchanged; add the `--detach` rationale and the P3 cleanup finding |

## 5. Remaining uncertainties (do not build on these)

| # | Uncertainty | How to close | Blocks |
|---|---|---|---|
| 1 | `merge-tree` vs `merge` divergence in edge cases (submodules, LFS, filters, `merge.ff`, `-X`, case-only renames) | 20-case edge corpus; compare trees | Only if A is retained as fallback |
| 2 | macOS behaviour (case-insensitive FS, APFS) | run the same matrix on a macOS runner | M0 CI matrix |
| 3 | Cost at 100k files (checkout per failed integration) | scale fixture; measure `worktree add` | M2 budget |
| 4 | Crash **during** `git merge` (not between merges) | kill mid-merge, assert `MERGE_HEAD`/index state and recovery | M5 crash matrix |
| 5 | Concurrent runs against one repo | two-process test; lock behaviour | M5 |
| 6 | Whether `.gitattributes` can evade the fallback (submodule, symlink, `core.attributesFile`) | adversarial attribute fixtures | H-2 hardening |
| 7 | Hooks during `git merge` (post-merge) | fixture with a `post-merge` hook | H-2 completeness |
| 8 | Behaviour with `merge.ff=only` or `-X` in the user's config | config-variation matrix | verdict stability |

## 6. Should M0 adopt this design?

**Yes — with two additions and one deletion.**

M0's purpose is a trustworthy harness plus a proven pipeline. SP-1 already delivered the pipeline
proof that `T-0005` was written to produce, so:

- **Delete** `T-0005` as specified (E-1′ is answered; its evidence is this report plus the
  reproducible script).
- **Add** to `T-0003`/`T-0004`: the driver-refusal fixture F8 and the attribute fallback must work on
  the floor; the `.ilvuno/` ignore assertion belongs to `doctor`.
- **Add** to `T-0002`: a `forge` capability that builds the 9 SP-1 fixtures (they already exist as a
  script; port them, do not re-invent them).
- **Keep** `T-0001` reduced (no package stubs), `T-0002` forge, `T-0003` fixtures, `T-0004` safety.
- **G0** gains two exit criteria: driver refusal fails closed on the floor; candidate SHA is
  identical across the CI matrix for the reference fixture.
- **Do not** put the worktree pipeline into production under M0 — M0 remains harness-only. The
  engine lands in M2 with H-1…H-9 as acceptance criteria.

M0 stays at **5 days**. The pipeline decision (D-07) is now answerable without further experiments.

## 7. What would change this recommendation

| Evidence | Would flip it to A |
|---|---|
| A fidelity corpus shows `merge-tree` == `merge` on ≥ 50 edge cases, including filters and submodules | makes A's fidelity risk negligible |
| Checkout cost at 100k files exceeds ~30 s per failed run | A's "no checkout on failure" becomes decisive |
| Users need to verify without a checkout (e.g. read-only CI mounts) | A is the only option |
| B's leftover management produces post-incident data loss in practice | A's clean failure state wins |

Until then, B is the recommendation: **equal output, more faithful mechanism, wider floor, better
conflict artefact, with a bounded and now-quantified failure mode.**

---

## 8. Reproducing this

```
docs/experiments/scripts/sp1.sh fixtures         # build 9 fixtures
docs/experiments/scripts/sp1.sh matrix           # 28 asserted cases (14 fixture/variant pairs x 2)
docs/experiments/scripts/sp1.sh probes           # P1..P5
docker build -f docs/experiments/scripts/Dockerfile.git238 -t ilvuno-git238r .
docker run --rm -u "$(id -u)":"$(id -g)" -v "$PWD":/sp1r -w /sp1r \
  -e SP1_GIT_BIN=/usr/local/bin/git -e SP1_SCRATCH=/tmp/sp1-c \
  ilvuno-git238r bash -lc 'mkdir -m700 -p "$SP1_SCRATCH"; ./sp1.sh matrix && ./tests.sh'
```

Outputs land in `out/<version>/all.jsonl` (one JSON object per run) and `out/<version>/probes/`.
Nothing is written outside the working directory, and no project verification command is executed —
the only check is the fixture-local `check.sh`.
