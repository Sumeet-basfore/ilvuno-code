> Paste everything between the two markers as the goal. Nothing else.

---8<--- PROMPT STARTS ---8<---

**Goal: an agent works on a copy of the project's code, not in a scratch folder.**

Read `/home/sumeet/personal-repo/projects/ilvuno-second/context/prompts/README.md` and follow the house
rules in it. Then work only on the goal below, until it is met.

## Today

An agent's working folder is `ILVUNO_HOME/runs/<run>/threads/<name>/` — a scratch folder. `base` only
resolves reads. So no run can change a line of the project's code. That is the one thing blocking the north
star. Fix it, and nothing else.

## What to build

When a run's project folder is a git repository with a **clean** working tree, each thread's work happens in
its own **worktree of that repository** on its own branch:

- branch `run/<run8>/<thread>` (see `git::branch_name` in `crates/runner/src/git.rs`)
- worktree at `<ILVUNO_HOME>/worktrees/<run8>/<thread>`
- the agent's `cwd` (`SpawnSpec.cwd`) is that worktree, not the thread folder
- after every turn, whatever the agent changed is **committed on the branch**: `git add -A`, then commit
  with the thread and task in the message. Nothing to commit means no commit.

If the project is not a git repository, or the tree is dirty, or git is older than `git::FLOOR`, the run must
fall back to today's behaviour and say so once in the journal. **A folder-mode run must keep working
exactly as it does now.**

## Where

- `crates/runner/src/git.rs` — already written and tested (worktree, commit, diff, merge, conflict abort).
  Use it; do not reimplement it. Extend it only if something is genuinely missing.
- `crates/runner/migrations/` — add `0006_git.sql`: `runs.git_root TEXT`, `runs.git_base TEXT` (nullable).
  Register it in `crates/runner/src/store.rs`. (The `threads` table already has `branch` and
  `worktree_path` — use them.)
- Wherever a run is created (`cli/src/ops.rs::create_run`) — decide git eligibility once, record root and
  base commit, journal the fact. A run that is not eligible records nothing.
- `crates/runner/src/dispatch.rs` — create the worktree on the thread's first task if it has none; set
  `SpawnSpec.cwd`; after the session ends, commit.
- `crates/runner/src/prompts/mod.rs` — one line in the worker preamble when the thread has a checkout: it is
  a real checkout, its branch is its own, it must not switch branches, and every change it wants to keep
  must be left in the working tree.
- `crates/runner/src/export.rs` — a thread's JSON gains `branch`, and whether it has a checkout.

## Rules for this goal

- Never commit, merge or check out anything in the **project's own** working tree. The runner only creates
  worktrees and commits inside them.
- A worktree is created once per thread, not per task. Reuse it on the next task so the agent keeps its files.
- Do not touch the merge path. Landing is the next goal, not this one.
- If a worktree cannot be created, the run must not die: fall back to folder mode, journal why, carry on.

## Tests you must add

In `crates/api/tests/` (the scripted provider, a temp repository as the goal's `base`):

1. a scripted agent writes a file in its cwd → the file is committed on
   `run/<run8>/<thread>`, and the **project's HEAD has not moved**.
2. a second task on the same thread sees the first task's file, and gets a second commit.
3. a goal whose `base` is not a git repository still runs in folder mode (this may already be covered —
   then say so and point at the test).
4. a dirty project falls back to folder mode and the journal says why.

## Done when

- the four tests above pass, and `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` is green;
- one **real** run on a throwaway repository (create one under `/tmp`) shows: a commit on
  `run/<run8>/<thread>`, `git log` readable, and the project's own branch untouched;
- `tools/shot.sh` still shows the window working against a folder-mode run (nothing regressed);
- `context/timeline.md` and `context/decisions.md` are updated with what you chose and why;
- you leave the report file the house rules ask for.

## Stop and report instead, if

- the worktree approach would need a new dependency;
- a test can only pass by touching the real project;
- you find yourself changing the merge or gate code (that is goal 02).

---8<--- PROMPT ENDS ---8<---

## For you, before you paste

- **Cost:** mostly test-and-build time. The one real run needs a tiny goal (one thread, one short task) —
  a few cents.
- **What to check when it reports:** a branch exists per thread, the project's `git log` has **no** new
  commits, and folder-mode runs still work.
- **Do not merge the branch yet** — 02 depends on this one, and it is easier to review together.
