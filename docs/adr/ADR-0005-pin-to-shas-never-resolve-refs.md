# ADR-0005 — Pin every input to a commit SHA; resolve a ref exactly once.

Status: proposed. Date: Phase 3.

## Context

A ref is a mutable pointer. Any design that re-reads `main` mid-run can produce a report about a
repository state that never existed — the classic time-of-check/time-of-use bug, except the
artifact is a *verification claim*.

Prior art shows both failure modes: orca's drift probe fails open when the probe errors
(`ORC-039`, `ORC-040`); cloudroom bases a worktree on a stale local copy when the remote ref is
missing (`CR-061`).

## Decision

1. `Pin()` is the **only** function that resolves a ref, and it returns SHAs plus a
   `pinned_at` timestamp.
2. Every downstream stage takes `sha` values. Function signatures do not accept refs
   (`IntegrateSpec.BaseRef` is the sole exception, and it is pinned on entry, line 1).
3. Merge operations use `merge <sha>`, never `merge <branch>`.
4. `merge-tree`, `diff`, `rev-list`, `cat-file` all take SHAs.
5. The report records, per source, both the ref that was named and the SHA that was used, so a
   reader can see the difference if the ref has since moved.
6. `--strict-refs` (opt-in) re-resolves refs at the end and aborts with `source_moved` if any
   changed. Default off: a moved ref does not invalidate work already done on immutable objects,
   and failing late wastes the run.

## Consequences

Positive: full determinism (ADR-0007 builds on this), safe resume-by-rebuild, cache keys that
cannot be poisoned by a moving ref, and reproducible bug reports.

Negative: a user who expected "test my branch" may be surprised that a commit made mid-run is not
included. The report states the pinned SHA prominently, and `--strict-refs` exists for the
anxious.

## Risks

| Risk | Mitigation |
|---|---|
| A pinned SHA becomes unreachable mid-run | `cat-file -e` before each merge; `source_object_missing` with the steps already completed |
| SHA is correct but not what the user meant (wrong branch) | Report shows ref → SHA mapping; approval requires typing the short SHA |
| Cache serves a stale verdict for a "same-looking" input | Cache key includes all SHAs + config digest + ilvuno/git versions (`08` §8); report states the original timestamp |

## Revisit conditions

1. If users routinely need "test whatever my branch is now" semantics, add an explicit
   `--follow-ref` mode that re-pins and restarts, never one that mixes.
