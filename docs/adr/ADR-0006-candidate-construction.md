# ADR-0006 — Candidate construction: sequential real merges in a disposable worktree

Status: proposed. Date: Phase 3. Depends on ADR-0004, ADR-0005, ADR-0007.

## Context

An integration candidate must answer: "if these N changes land on base, what does the result
look like?" The answer must be attributable (which source caused which step), faithful (same
mechanism as the developer's own merge), reproducible (same inputs → same SHA), and safe (never
touch the user's checkout).

## Options

| Option | Fidelity | Attribution | Determinism | Verdict |
|---|---|---|---|---|
| **Sequential `git merge --no-ff` per source in a scratch worktree** | exact | per-step merge commit | yes with envelope | **chosen** |
| Octopus merge (`git merge a b c`) | exact-ish | none (single commit) | yes | rejected |
| Rebase each source onto the accumulated branch | different conflict semantics (can conflict where a merge does not, and vice versa); rewrites SHAs the user would actually land | per-step | yes | rejected |
| Cherry-pick each commit | different semantics for merge commits; expensive; loses authorship grouping | per-commit | yes | rejected |
| `merge-tree` only (no worktree, no verification) | exact for conflicts | pairwise | yes | insufficient: cannot run checks |
| Test each branch against base independently | does not answer the question | n/a | yes | rejected (this is what everyone already does) |

## Decision

**Sequential real merges, one per source, in a disposable worktree, in a deterministic order,
with `merge-tree` preflight.**

Specifics:

1. `git merge-tree --write-tree` preflight over all pairs **and over the full tuple** before any
   mutation. Pairs alone miss conflicts that appear only as a set (`06` §4.1, test `T-CONF-2`).
2. Worktree at `.ilvuno/worktrees/<run_id>/candidate` on `refs/ilvuno/candidates/<run_id>`,
   created with `--no-checkout` then `reset --hard <base_sha>`.
3. `--no-ff --no-edit -m "ilvuno: integrate <sha[:12]> (step i/n)"` so every step is a commit and
   attributable in the diff.
4. On conflict: `merge --abort` (never leave a half-merged tree), record paths and both SHAs,
   then stop (`on_conflict: stop`) or skip and continue (`continue`).
5. Candidate ref is deleted on settle after archiving a patch if dirty.

## Why worktrees live inside `.ilvuno/`

| Location | Pro | Con |
|---|---|---|
| Inside the repo (`.ilvuno/worktrees/`) | survives repo moves; one `.gitignore` line; obvious to the user | `git clean -xdf` destroys candidates (acceptable: disposable) |
| Sibling of the repo | not in `git status` scope at all | breaks if the repo moves; clutters the parent dir; needs config |
| Temp dir (`/tmp`) | auto-cleaned | cross-filesystem worktree add can fail; evidence and patch paths become unrelated to the repo |

Chosen: inside. Revisit if agents are observed scanning into it (WT-1).

## Consequences

Positive: per-step attribution in the diff (the review surface depends on it); identical merge
semantics to the developer; deterministic rebuild after a crash is trivially safe (re-run from
SHAs).

Negative: merge commits accumulate (N+1 for N sources), so the candidate graph is chattier than a
rebase-based candidate would be. Disk: a full worktree per run (mitigated by cleanup at settle,
cap of 4 concurrent worktrees).

## Risks

| Risk | Mitigation |
|---|---|
| A user's `.gitattributes` merge driver (e.g. custom lockfile merge) changes behaviour | recorded in the report; the candidate is what the user's own merge would produce, which is the point |
| `merge-tree` unavailable (git < 2.38) | explicit degradation with a report note; refusal if `--require-preflight` |
| Large repos: worktree add is slow (checkout of 100k files) | `--no-checkout` + `reset --hard` still writes the tree; measured in `NFR-102`; large-repo mode (partial checkout) is a possible v0.2 optimisation |
| Disk exhaustion from many runs | worktree cap + cleanup at settle + `gc` |
| A conflict leaves partial state if the process dies mid-`merge --abort` | journaled; `resume --decide=remerge` discards and rebuilds (safe by ADR-0005) |

## Revisit conditions

1. Users report the merge view is unrealistic because their team squashes → offer an opt-in
   rebase-based candidate mode, clearly labelled as a different question.
2. Candidate build exceeds the `NFR-102` budget on the large fixture → partial checkout /
   `sparse` optimisation (which then requires revisiting the v0.1 sparse refusal).
3. Octopus becomes necessary for very large batches (>8 sources) → still reject: attribution is
   worth more than speed.
