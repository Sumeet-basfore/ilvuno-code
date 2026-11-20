# SP-1 · Experiment B — Disposable Worktree Pipeline

**Question:** does the simple design the red-team review recommends — detached worktree, sequential
`git merge --no-ff`, check after success, remove the worktree — meet the same requirements as the
object-only pipeline, and what does it cost at the edges?

**Answer: it produces identical candidates, is available on any Git version, is the more faithful
mechanism, and leaves partial state on every failure path.** The leftovers are bounded and
mechanically removable, but they must be designed for, not discovered.

Script: `docs/experiments/scripts/sp1.sh` (`run_worktree`). Same fixtures, same envelope, same
checks as Experiment A.

| Environment | Git | OS |
|---|---|---|
| Host | **2.55.0** | CachyOS Linux, x86_64 |
| Container | **2.38.1** | Alpine 3.17, x86_64 |

> **SP-1R update (2026-11-19).** This report describes the original SP-1 run. The harness has since been
> remediated: the matrix is an enforcing test (`expected.json`, 28 cases), output goes to a validated
> private scratch root, Git runs under a controlled environment, driver refusal is config-based, and
> classification uses repository state rather than stderr. Numbers below that refer to "8 clean
> cases" remain the original corpus result; current enforced results are in
> `sp1r-regression-matrix.md`. See `sp1r-remediation-report.md` for the finding-by-finding status.

---

## 1. Algorithm as implemented

```
P0  pin        base_sha, src_sha[i]                                (same as A)
P2  gate       driver refusal                                      (same as A)
P3  worktree   git worktree add --detach <repo>/.ilvuno/worktrees/candidate <base_sha>
               # no -B: a branch name would land in refs/heads/
P4  loop       for src in order:
                 git merge-base --is-ancestor "$src" HEAD          -> skip
                 GIT_AUTHOR_*=ilvuno GIT_COMMITTER_*=ilvuno \
                   git merge --no-ff --no-edit -m "ilvuno: integrate <sha> (step i/n)" "$src"
                 # exit 0  -> continue
                 # exit != 0 -> inspect:
                 #   "refusing to merge unrelated histories" -> fatal
                 #   otherwise -> conflict; collect `diff --diff-filter=U`
                 git merge --abort                                  # stop on first conflict
P5  publish    git update-ref refs/ilvuno/candidates/r <HEAD>
P6  verify     ( cd <worktree> && sh ./check.sh )
P7  cleanup    git worktree remove --force -> update-ref -d x2 -> git worktree prune
```

The `--detach` flag is not a detail: `git worktree add -B refs/ilvuno/candidates/<id>` stores a
branch at **`refs/heads/refs/ilvuno/candidates/<id>`** (verified, and in the review). The ref
namespace promise only holds with `--detach` plus an explicit `update-ref`.

---

## 2. Results — clean integrations

| Fixture | Sources | Exit | Objects added | Same SHA as A | Notes |
|---|---|---|---|---|---|
| F1 two independent clean | 2 | 0 | 4 | **YES** | `b051e4bf28bc…` on both versions |
| F2 three independent clean | 3 | 0 | 7 | **YES** | `e56498262728…` on both versions |
| F5 one source already integrated | 2 | 0 | 1 | **YES** | skip works on `HEAD` |
| F7 combined check failure | 2 | 1 | 4 | **YES** | check runs in the worktree, exit 1 |
| F9 cleanup fixture | 2 | 0 | 4 | **YES** | — |
| F8 driver refused | 2 | 4 | 0 | n/a | worktree not created |
| F8 driver allowed | 2 | 0 | 4 | **YES** | driver executes during `git merge` |
| F1 with main `index.lock` | 2 | 0 | 4 | **YES** | the worktree has its own index |

Cross-version: identical candidate SHAs on 2.38.1 and 2.55.0 for every clean fixture.

Because the merge happens in a real checkout, the trees that checks see are exactly the trees Git
produced — there is no second materialization step that could apply different filters.

## 3. Results — conflicts

| Fixture | Exit | Step | Paths | `MERGE_HEAD` before abort | After abort |
|---|---|---|---|---|---|
| F3 direct conflict | 3 | 2 | `shared.txt` | yes | no |
| F4 conflict at step 2 | 3 | 2 | `shared.txt` | yes | no |

The step index and paths match A exactly. The difference is what exists afterwards:

- **before** `merge --abort`: conflict markers in the working tree, stage entries in the index,
  `MERGE_HEAD` — the state a developer expects and can inspect;
- **after** `merge --abort` (the review's lean design stops here): all of that is gone, and the
  worktree sits at the last successful commit.

So the debuggability advantage of B exists **only if the worktree is kept on conflict**. Aborting
by default throws away the most useful artefact the approach can produce.

## 4. Results — unrelated histories

`git merge` returns exit 1 for this and for a normal conflict. The distinction requires an stderr
match:

```
fatal: refusing to merge unrelated histories
```

Recorded as `fatal`, but at **step 2** and only after a merge attempt, versus A's step 0 with exit
128. If the match is omitted, the run is reported as a conflict (this was the first harness
version's behaviour — a misclassification, not a crash).

## 5. Results — interruption and cleanup failure

`--stop-after 1` (exit 9, no cleanup):

| Leftover | B | A |
|---|---|---|
| refs | 0 | 0 |
| **worktrees (excluding main)** | **1** | 0 |
| objects added | 1 | 1 |
| re-run possible without repair | no — path exists | yes |

`--fail-cleanup` (worktree directory made read-only, `worktree remove --force`, exit 5):

```
error: failed to delete '…/.ilvuno/worktrees/candidate': Permission denied
worktree list entries: 1        (main only)
.git/worktrees entries: 0
directory still present: yes
```

`git worktree remove` **deletes the administrative entry before failing on the directory**. The
result is an orphan directory that `git worktree list` and `git worktree prune` no longer know
about. The same failure occurs in A, because A also materializes a worktree for checks — this is a
shared property of any design that checks out a tree, not an A/B difference.

The opposite failure also exists (P5): if the directory is deleted under Git (`git clean -xdf`),
the admin entry becomes stale and `git worktree prune` fixes it.

## 6. Defects and costs observed

| # | Observation | Consequence |
|---|---|---|
| 1 | The worktree is created **before** the first merge | a conflict always leaves a directory to clean; a crash always leaves one |
| 2 | `merge --abort` discards the conflict state | the approach's best debugging artefact is lost unless the worktree is kept |
| 3 | Unrelated histories must be distinguished by stderr text | brittle; a wording change silently reclassifies a fatal error as a conflict |
| 4 | `.ilvuno/` must be gitignored before the first run | otherwise every candidate makes the main `git status` dirty (P2) |
| 5 | `worktree remove --force` can orphan a directory while deleting its admin entry | cleanup needs a directory check, not only `worktree list` |
| 6 | A repo-level `git clean -xdf` destroys candidate worktrees | same for A once it materializes; admin entries then need `prune` |
| 7 | Merging requires a full checkout | at 100k files this is the dominant cost of a failed integration |
| 8 | `git merge` honors `merge.ff`, `-X`, and strategy options | fidelity by construction, but also more inputs that affect the result |

## 7. Why this is the more faithful mechanism

A defines a candidate as "what `merge-tree` computes". B defines it as "what `git merge` does".
Those were identical on the corpus, but they are different claims:

- `merge-tree` operates on blobs; `merge` operates on a checkout, so filters and eol conversion
  participate in the result;
- `merge-tree` accepts no strategy options in this design, while `merge` consults the repository's
  merge settings;
- Git's own documentation describes `merge-tree` as using the same merge machinery, which is
  exactly why the equality holds — until a code path differs by design.

For a product whose promise is "this is what your merge would look like", the mechanism that *is*
the merge is the safer default.

## 8. Limits

Same limit set as Experiment A (§7): 8 fixtures, 2 Git versions, Linux x86_64 only, tiny
repositories, spike-quality code. Additionally, this experiment always removed the worktree on
conflict for comparability; the "keep the worktree" variant was only observed pre-abort, not run
as a separate lifecycle.
