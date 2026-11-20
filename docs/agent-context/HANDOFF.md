# HANDOFF — Ilvuno

Purpose: make the next session start from facts, not chat history. Update this file at the end
of every session; keep it short. Labels: [implemented] [approved] [proposal] [unverified].

## Rules

- Record the **branch, HEAD SHA, and dirty state** exactly.
- List commands run and their results; list what was **not** run.
- Separate facts from claims. Never write "done" without a command/evidence path.
- Do not paste long diffs or logs; link the file or name the command.
- Do not record credentials, tokens, absolute paths, or user data.
- If a decision is needed, name the ID and the owner; do not pick an option.

## Session template

```markdown
### Session <YYYY-MM-DD> — <agent/model> — <task ID or "policy">

- Base: <branch> @ <sha> (origin/main <sha>)
- Working tree: <N modified, N untracked>; staged: none
- Task / scope: <ID, allowed paths>
- Changed: <paths, one line each>
- Verified: <command → result> (repeat)
- Not run / failed: <what and why>
- Decisions needed: <D-ID / question, owner>
- Risks / unknowns: <one line each>
- Next step: <one action>
- Do not touch: <paths>
```

## Current snapshot (2026-11-20)

- Base: `docs/commit-rules`; `998fb1a` plus local governance commits G1–G5; `main` `4d6dd4c`;
  6 commits ahead, not pushed.
- Working tree: 2 modified (`README.md`, `.gitignore`), 82 untracked; no staged files; no stash.
- Governance recorded locally: AGENTS §15 approved plus the recorded M0 two-PR exception; D-19;
  T-0001/T-0005 scope amendments; readiness, review, experiment and context records
  [implemented, local only].
- Last recorded evidence: M0 local checks and reference matrix in `docs/planning/gate-log.md`
  (2026-11-20) [implemented, uncommitted]. Not re-run in this session [unverified].
- Policy: AGENTS §15 Git branching and PR review **approved 2026-11-20** by the project owner;
  rationale recorded in §15 [approved].
- Blocker: G0 not passed (CI, gate runner/reviewer); M1 not authorized.
- Next: owner review of PR-1, then push and CI; PR-2 follows from the post-merge `origin/main`.

## T3 Code migration checklist

Use when moving this checkout into T3 Code or another agent host. The goal is to lose no
uncommitted or untracked work and to keep the repository provider-independent.

### Before moving (read-only)

- [ ] Record: `git rev-parse --abbrev-ref HEAD`, `git rev-parse HEAD`,
      `git status --porcelain=v1`, `git branch -vv`, `git worktree list`, `git stash list`.
- [ ] Save the full diff: `git diff > /tmp/ilvuno-worktree.diff` and
      `git diff --cached > /tmp/ilvuno-staged.diff`.
- [ ] List every untracked file: `git ls-files --others --exclude-standard`.
- [ ] Hash the untracked set (sorted `sha256sum`) and store the list outside the repo.
- [ ] Archive the **entire worktree including `.git`, untracked, and ignored files** with
      `tar`/`rsync` to a location outside the repository. Verify the archive opens and the
      file count matches. Do not rely on `git stash` alone; it omits untracked files by default.
- [ ] Confirm no credentials, `.env`, or local notes are in the tree; do not copy them.

### Transfer / open

- [ ] Open the **existing checkout** in T3 Code if possible. Do not start from a fresh clone:
      a clone contains only committed history and loses all M0 work.
- [ ] If a new host is unavoidable, restore the archive first, then verify against the recorded
      state. Never `git clean`, `reset --hard`, `stash drop`, or delete the original.
- [ ] Keep the current branch. Do not let the host create a replacement branch or worktree that
      hides the dirty tree; check `git worktree list`.
- [ ] T3 Code defaults new threads to full access and can create its own worktrees. Reduce access,
      keep host worktrees outside the repo, and do not confuse T3 worktrees with `.worktrees/`.
- [ ] T3's "merge back" moves conversation context, not code (research T3-048). It is not an
      integration step and must not be treated as one.

### After opening (verify)

- [ ] `git rev-parse HEAD` and `git status --porcelain` match the pre-move record exactly.
- [ ] Untracked file count and hashes match.
- [ ] `go version` is 1.27.0; `git --version` ≥ 2.38.1; Python 3 and Docker available for the
      reference harness.
- [ ] `make test`, `make lint`, `make docs-check` pass (or record the exact failure).
- [ ] No new refs, branches, stashes, or worktrees appeared.
- [ ] Write the result into this HANDOFF before doing any task work.

### Rollback

- [ ] Stop; do not delete anything.
- [ ] Restore the archive to a new directory, verify hashes, and compare `git status`.
- [ ] Report the divergence; do not "fix" it by resetting or cleaning.
