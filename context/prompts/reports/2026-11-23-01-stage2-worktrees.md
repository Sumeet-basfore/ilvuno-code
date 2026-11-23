# 01 · Stage 2 worktrees — 2026-11-23

## What changed

`ff669cc` — a thread now works on a copy of the project's code, not in a scratch folder.

- Migration `0006_git.sql`: `runs.git_root` and `runs.git_base`, both nullable. `threads.branch` and
  `threads.worktree_path` are now used for real.
- `git::project_git` decides once, at run creation: a git repository, git ≥ 2.38.1, a clean tree, and a
  HEAD to branch from. Eligible runs record the root and base commit and journal `run.git`; everything
  else journals `git.fallback` with the reason and records no columns.
- `Home::worktrees_dir`: `<ILVUNO_HOME>/worktrees/<run8>/<thread>`, outside the project.
- `dispatch::ensure_checkout`: the worktree is created on the thread's first task and reused after that.
  A path that cannot be made falls back for that thread and journals `git.fallback`; the task still runs.
- The agent's `SpawnSpec.cwd` is the worktree. After the session the runner commits what changed on the
  thread's branch (`git::commit_all`, hooks skipped, message `<thread>: task <task8>`); nothing to commit
  means no commit. `git.worktree`, `git.commit` and `git.commit_failed` are journaled.
- `worker-preamble/5` adds one line when the thread has a checkout: it is a real checkout on its branch,
  do not switch branches, leave every change you want to keep in the working tree. Folder mode keeps the
  same prompt byte for byte.
- Export: each thread gains `branch` and `has_checkout`.
- Docs updated: `docs/data-model.md` (columns, invariant 6, export fields), `docs/provider-contract.md`
  (preamble 5), `docs/roadmap.md`, `context/features.md`, `context/risks.md`, `context/decisions.md` (D21).

The merge and gate code was not touched. Landing is goal 02.

## What I verified

- **Suite:** `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` → **147 passed, 0 failed, 3 ignored**.
  The migration-count test was updated from 5 to 6 because the project added a migration; that is the
  only test whose promise changed.
- **Four new tests** in `crates/api/tests/worktrees.rs`, all passing on the scripted provider:
  1. `a_scripted_agent_commits_on_its_branch_and_the_project_does_not_move` — one commit on
     `run/<run8>/reader-1`, project HEAD unchanged, `notes.md` absent from the project, the provider's
     cwd is the worktree, export says `branch` and `has_checkout: true`.
  2. `a_second_task_reuses_the_checkout_and_adds_a_commit` — the second task sees the first task's
     `notes.md`, two commits since the base, project HEAD unchanged.
  3. `a_non_git_base_stays_in_folder_mode` — branch null, `has_checkout` false, the file is in the thread
     folder, `git_root`/`git_base` null, `git.fallback` journaled. The older
     `crates/api/tests/actions.rs::a_scripted_run_reaches_the_gate_and_is_accepted_through_the_api`
     also still passes in folder mode.
  4. `a_dirty_project_falls_back_to_folder_mode_and_the_journal_says_why` — an untracked file keeps the
     project in folder mode; the journal payload contains "uncommitted"; the project HEAD and the dirty
     file are untouched.
- **A real run** on `/tmp/ilvuno-git-repo`, a throwaway repository:
  - base commit `a6f7687`; run `000e3613-b821-4e1c-8863-e871967234d4`; one `writer` thread, one task.
  - reached the gate; branch `run/000e3613/writer`; worktree
    `/tmp/ilvuno-git-home/worktrees/000e3613/writer`.
  - `git -C <worktree> log --oneline` → `d79b90d writer: task 70d2fac6` above `a6f7687 start`;
    `hello.txt` contains `hello`; the worktree is clean.
  - `git -C /tmp/ilvuno-git-repo log --oneline` → only `a6f7687 start`; `git status` clean; no
    `hello.txt` in the project.
  - journal: `run.git` (root + base), `git.worktree` (branch, path, base), `git.commit` (branch, task,
    short sha). `ilvuno replay` → `projection: MATCH (1 rounds, 0 refusals)`; export says
    `"branch": "run/000e3613/writer", "has_checkout": true`.
  - the run used 75,675 tokens on a 30,000 budget: a turn in flight is never killed, and the budget is
    checked before a task. That is existing behaviour, not new.
- **The window still reads a folder-mode run:** `tools/shot.sh /tmp/folder-mode.png 1440 900` against the
  `.demo-home` demo run (created before this change, so `git_root` is null). The board, gate and
  conversation render; `/tmp/desktop.log` has only the GTK font-key and Mesa DRI3 warnings.

## Could not verify

- **A worktree that fails to be created.** The per-thread fallback path is code-inspected, not simulated.
  The dirty-project test covers the run-level fallback, not the `git worktree add` failure.
- **A git-mode run through the window.** The window has no git fields; it ignores `branch` and
  `has_checkout`. The regression check used a folder-mode run, as the goal asked.
- **A goal whose base is a subfolder of a repository.** The worktree is the repository root, so the
  agent's cwd is the root, not the goal folder. The real run and the tests use the repository root.
  This is a design consequence, not a bug, but it has not been exercised with a subfolder base.
- **Old git.** The 2.38.1 floor is a unit test from `research/14-git-floor.md`; this machine has 2.55.0,
  so the refusal on an old git was not run.
- **A real orchestrator run.** The API tests use the scripted orchestrator; the real run was one worker
  task. Real Claude made the commit, but did not plan through an orchestrator.
- **Merge and landing.** Deliberately untouched; goal 02.

## Still open

- Goal 02: the gate shows the diff, accepting merges, a conflict stops the round.
- Worktrees are never deleted after a run; cleanup needs a rule (goal 02 or later).
- The window shows no branch or checkout state; only the export carries it.
- Two runs on one project still are not serialised. They now make separate branches and worktrees, but
  goal 02's merge will need that rule.
- A base inside a subfolder of a repo: decide whether the worktree should be the repo root or the base
  subtree before users hit it.
