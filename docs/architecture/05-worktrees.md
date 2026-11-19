# 05 — Worktrees

Status: specification. Every git invocation below is normative (ADR-0004: git subprocess only).

---

## 1. Purpose and scope

Worktrees are how Ilvuno gets an isolated checkout without touching the developer's. They
serve exactly two purposes in the MVP:

| Purpose | Created by | Lifetime |
|---|---|---|
| `candidate` | the integration engine | one run; disposable |
| `repair` | the repair loop | one attempt-set; disposable |
| `verify` | runner, only when a command needs a different `cwd` *and* the candidate must stay pristine | one verification run |

There is no `agent` worktree in the MVP, because Ilvuno does not dispatch.

Non-purposes (explicit): worktrees are not a security boundary, not a dependency-isolation
mechanism, and not a substitute for containers (doc `09` §7).

---

## 2. Discovery

`ilvuno doctor` and `ilvuno init` establish what exists before anything is created.

```
git rev-parse --show-toplevel                  → repo root (must be non-bare)
git rev-parse --is-shallow-repository          → shallow ⇒ merge-base may be wrong
git rev-parse --git-common-dir                 → shared .git location
git worktree list --porcelain                  → all worktrees + their branch/HEAD
git rev-parse --abbrev-ref HEAD                → current branch (informational)
git status --porcelain --untracked-files=no    → dirty tracked files (informational)
```

### 2.1 Refusals at discovery

| Condition | Code | Why |
|---|---|---|
| bare repo | `repo_bare` | no working tree to model |
| shallow (`--is-shallow-repository` = true) and merge-base needed | `repo_shallow` | `merge-base` is unreliable; `--unshallow` is the fix |
| `.git` is a file pointing elsewhere (submodule) | `repo_submodule` | not supported in v0.1 (`T-4`) |
| `core.worktree` set | `repo_unusual_layout` | worktree paths become ambiguous |
| sparse checkout active | `repo_sparse` | a candidate would silently omit files (`T-4`) |
| LFS without `git-lfs` on PATH when LFS files are tracked | `repo_lfs_missing` | checks would run against pointer files |
| repo root is inside another Ilvuno run's candidate worktree | `repo_nested_candidate` | prevents recursive integration |

Refusals are exit 2 and name the command that would fix it. None of them "warn and continue":
a silently wrong merge base produces a confidently wrong report, which is worse than a refusal
(`R-5`).

---

## 3. Ownership model

The rule is one line: **Ilvuno owns a worktree if and only if its ref is under
`refs/ilvuno/`.**

| Observation | Meaning | Allowed actions |
|---|---|---|
| `git worktree list` shows a path with branch `refs/ilvuno/candidates/*` | ours | read, merge into, verify, remove |
| path with branch `refs/ilvuno/repair/*` | ours | same |
| path with a user branch (`refs/heads/*`) | theirs | **read-only**: never merge, never verify-with-side-effects, never remove |
| path detached at a SHA | theirs unless it matches a recorded `candidate` sha | read-only |
| path recorded in our journal but absent on disk | `orphaned` | restore or journal-found cleanup |

Why so strict: two of the three surveyed products damaged user state through cleanup built on
weaker assumptions — orca removed an in-use worktree in a folder workspace (`ORC-095`), t3code
deleted a worktree and lost uncommitted work (`T3-068`, #37769). Ownership by ref namespace is
the cheapest possible rule to audit.

### 3.1 Naming and paths

```
<repo>/.ilvuno/worktrees/<run_id>/candidate       # candidate
<repo>/.ilvuno/worktrees/<run_id>/repair-<n>      # repair attempt n
```

Inside `.ilvuno/`, which is gitignored, so:
- `git status` in the main tree never shows candidate files;
- `git clean -xdf` by the user destroys candidates — acceptable, because evidence lives
  outside and the candidate is disposable (`INV-14` renaming aside: deleting `.ilvuno/` leaves
  the repo readable).
- a candidate worktree nested inside the repo root is safe because git ignores nested
  worktrees listed in `git worktree list` for the parent.

Trade-off, stated: putting worktrees inside the repo means an agent running `rg .` in the main
checkout will *not* see them (good) while an agent running from inside `.ilvuno/worktrees/...`
would. The alternative (outside the repo tree) breaks when the repo moves. We choose inside,
documented (ADR-0006).

### 3.2 Remote/worktree duplication rule

A worktree path may never be reused across runs. `<run_id>` in the path guarantees that
without a lock, even if a stale journal entry exists.

---

## 4. Creation

Normative invocation:

```
git -C <repo> worktree add \
    --no-checkout \
    -B refs/ilvuno/candidates/<run_id> \
    .ilvuno/worktrees/<run_id>/candidate \
    <base_sha>

git -C <worktree> -c core.hooksPath= -c commit.gpgsign=false \
    reset --hard <base_sha>
```

Details that matter:

| Flag / step | Reason |
|---|---|
| `--no-checkout` then `reset --hard <base_sha>` | avoids checking out the wrong tree during creation, and lets us set the identity/date envelope **before** the first checkout-triggered work |
| base given as a **SHA**, never a ref | `INV-2`; the base cannot move under us |
| `-B` with a fully qualified `refs/ilvuno/...` | prevents branch-name collisions with user branches and avoids `check-ref-format` surprises |
| `core.hooksPath=` (empty) on every commit-producing command | `INV-11`, ADR-0013: repo hooks are arbitrary code and would run as the user |
| `commit.gpgsign=false` | signing would change the SHA and could prompt for a passphrase mid-run |
| `--no-track` | no upstream configuration is written for our refs |

Rollback on failure: `git worktree remove --force` then `git update-ref -d refs/ilvuno/...`.
Both are attempted; if either fails, the worktree is journaled `orphaned` (a recorded state,
not an exception), and `abort`/`resume` can retry (doc `02` §4.4).

### 4.1 Detached-HEAD note

We use a branch ref (`-B`), not detached HEAD, so that `git worktree list` names our purpose
and so `abort` has a single ref to delete. A detached candidate is also acceptable; we choose
the branch for observability and record the choice in the journal.

---

## 5. The shared-`.git` reality

All worktrees share one object database, one ref namespace, one config, and one hooks
directory. Consequences, all of which must be handled explicitly:

| Shared thing | Consequence | Handling |
|---|---|---|
| Object store | a candidate's objects are visible repo-wide; `gc` may prune unreachable ones | candidates are referenced by a ref, so they are reachable and safe |
| `refs/` | our refs share a namespace with the user's | strict `refs/ilvuno/` prefix; `abort` deletes only ours |
| `config` | a user's `commit.template`, `merge.*` settings leak in | we pass explicit `-c` overrides for the settings that change semantics (hooks, gpg, `merge.conflictStyle=merge`) |
| hooks | repo hooks would run on our commits | disabled (`core.hooksPath=`) and this divergence is recorded in the report |
| `.git/index.lock` | races between our commands and the user's | our operations are serialized by the project lock; a race with the *user's* git yields `git_locked`, retried ≤ 3 times over ~2 s, then refused |
| `worktrees/<name>/` metadata | a crash can leave half-created metadata | `git worktree prune` is run at `resume`/`abort` only, never during a run |
| `gc.auto` | background gc during our run can block | run our git commands with `-c gc.auto=0` |

`merge.conflictStyle=merge` is forced deliberately: `zdiff3` output is nicer for humans but
changes the conflict markers we parse and record, and predictions must match reality. The
report notes that a user with a different global conflict style may see slightly different
marker text.

---

## 6. Head tracking and drift

Every stage records the observed HEAD and tree:

```
git -C <wt> rev-parse HEAD
git -C <wt> rev-parse HEAD^{tree}
git -C <wt> status --porcelain=v1 --untracked-files=all    (only at verification boundaries)
```

| Observation | Meaning | Action |
|---|---|---|
| HEAD ≠ journaled `post_sha` | something else wrote to our worktree | abort run (`worktree_drift`) |
| tree dirty after a verification command | a command wrote files | record `tree_after` ≠ `tree_before` as a warning; evidence notes it |
| untracked files after verification | same | same, with a path list (capped, redacted) |
| worktree missing at `resume` | deleted externally | `orphaned`; resume rebuilds from the pinned base + source SHAs (safe, fully deterministic) |

Rebuilding from pinned SHAs is possible precisely because nothing in the run depends on the
worktree's history — only on SHAs (`INV-2`). That is the payoff for the strictness.

---

## 7. Cleanup

```
on run settle (verified | failed | blocked | abandoned):
  1. journal cleanup.intent(worktrees[])
  2. if worktree dirty and purpose == candidate: still remove (disposable by contract),
     but archive a patch to .ilvuno/runs/<id>/candidate.patch first, with a digest
  3. git worktree remove --force <path>      (force is justified: disposable, patched)
  4. git update-ref -d refs/ilvuno/candidates/<run_id>
  5. git worktree prune
  6. journal cleanup.result(...)
```

Rules:

| Rule | Reason |
|---|---|
| Never remove a worktree whose ref is not `refs/ilvuno/` | `INV-1` |
| Always archive a patch before removing a dirty candidate | the candidate may contain a repair the user cares about; deleting work silently is the t3code #37769 failure |
| `--force` is allowed only for `refs/ilvuno/*` worktrees | force is the dangerous flag; scope it |
| Keep `repair-*` worktrees until the run settles, then patch + remove | bounded disk |
| Never delete a ref that is not fully qualified under our prefix | a bare `-d <name>` can delete a user branch |
| Record every removal, including failures | `orphaned` must be visible |

Retention: candidate patches are kept with the run's journal until `abort` or `bundle prune`.

---

## 8. Failure scenarios

| Scenario | Detection | Behaviour |
|---|---|---|
| `worktree add` fails (disk full, path exists) | exit ≠ 0 | rollback attempt; `candidate_build_failed`; exit 2 |
| Path exists from a prior crashed run | `os.Stat` + journal check | if ours and journaled: reuse after `prune` + `reset --hard`; if not journaled: refuse |
| User deletes `.ilvuno/` mid-run | worktree gone | `worktree_drift`; run aborts; evidence of what was done is retained |
| User runs `git gc --prune=now` mid-run | rare ref loss | candidate ref deletion would fail; `orphaned`; report says the candidate is gone |
| Two Ilvuno versions on the same repo | lock + journal schema version | unknown schema ⇒ refuse to resume; reading is always allowed |
| Worktree path on a different filesystem | `git worktree add` may be slow/fail | recorded; no special handling |
| Symlink at the target path | `os.Lstat` | refuse (`worktree_path_occupied`) — never follow |
| Windows reserved names / long paths (v0.2) | n/a in MVP | documented as unsupported |
| Repo on a network filesystem | lock/fsync may be unreliable | `doctor` warns; determinism test may fail; documented |

---

## 9. Isolation limits (documented, not claimed away)

| Limit | Consequence | Mitigation |
|---|---|---|
| Worktrees share `.git` | a shell-capable process can read/write other worktrees and the main repo | not a security boundary (doc `09` §7); correct isolation requires a container/VM, out of MVP |
| Worktrees share the OS user | verification commands run as the developer | env allowlist limits secrets, not capability; documented |
| Verification may write outside the worktree | `pnpm`/`cargo` can write to `~/.cache` | allowed and recorded; no attempt to sandbox (that would break most toolchains) |
| Symlinks in the repo can point outside | a command could escape the worktree | allowed (matches normal dev behaviour); `INV-12` checks `cwd`, not file access |
| Nested worktrees inside `.ilvuno/` | agents may or may not see them | documented; `doctor` prints the path |
| `git worktree lock` not used | a stale `prune` by the user can remove metadata | recorded; resume rebuilds |

The honest summary: **worktree isolation is a correctness mechanism, not a containment
mechanism.** Any claim to the contrary would be the same mistake as orca's "tool denial is not
a sandbox" (`T3-061`).

---

## 10. Open questions

| ID | Question | Recommendation |
|---|---|---|
| `WT-1` | Worktrees inside vs outside the repo tree? | Inside (`.ilvuno/worktrees/`); revisit if agents are observed scanning into it |
| `WT-2` | Should we support `--worktree-dir` override? | Yes, one config key, because some CI mounts repos read-only except for a temp dir |
| `WT-3` | Is `-B` (branch) better than detached HEAD for candidates? | Branch: observable in `git worktree list`, single ref to delete |
| `WT-4` | Do we need `git worktree lock` to survive user `prune`? | No; resume can rebuild from SHAs, so lock adds a stale-lock failure mode |
| `WT-5` | Support submodules/LFS/sparse in v0.2? | Refuse in v0.1; a wrong candidate is worse than a refusal (product `T-4`) |
