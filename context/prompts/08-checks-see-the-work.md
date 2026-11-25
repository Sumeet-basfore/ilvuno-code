> Paste everything between the two markers as the goal. Nothing else.

---8<--- PROMPT STARTS ---8<---

**Goal: a round's checks test the tree the gate would merge, so a passing check is evidence about what the
agents actually wrote.**

Read `/home/sumeet/personal-repo/projects/ilvuno-second/context/prompts/README.md` and follow the house
rules in it. This is roadmap stage 6, "Checks that see the work" (`docs/roadmap.md`).

## Today, and the test that proves it

Branch `feat/checks-see-the-work` already exists with one commit, `f2162f2`. It holds a test that is **red on
purpose**. Run it:

```bash
cd /home/sumeet/personal-repo/projects/ilvuno-second
RUSTUP_TOOLCHAIN=1.100.0 cargo test -p runner --test loop_fake a_command_check_runs
```

It fails with:

```
the check has to see the writer's file. readiness: {"checks":[{"detail":"exit 1: no output",
"name":"command_exit_zero test -f brief.md","passed":false}],"failed_tasks":[],
"failing":["command_exit_zero test -f brief.md"]}
```

The scripted agent wrote `brief.md` in its own worktree and the turn committed it. The check looked somewhere
else. `crates/runner/src/checks.rs` line 69 runs a command with `current_dir(at.goal_dir)`, and
`crates/runner/src/driver.rs` line 133 passes `goal_dir: plan.base()`, which is the project folder.

Do not "fix" the test. Make it pass by fixing the runner.

That commit also fixed a bug the test found: a relative `"base"` in a goal was resolved against the process
folder instead of the goal file's folder. That is done. Do not touch it again.

## What to build

### 1 · A throwaway integration tree, in `crates/runner/src/git.rs`

Add one function and its result type, next to `merge` and `remove_worktree`.

```rust
pub struct IntegrationTree { pub path: PathBuf, pub branch: String, root: PathBuf }

pub enum TreeBuild {
    /// The base plus every thread branch that had commits, merged. `path` is a real checkout.
    Ready(IntegrationTree),
    /// Two threads could not merge. The tree was removed and the branch deleted before returning.
    Conflict { files: Vec<String> },
}

pub fn integration_tree(root: &Path, path: &Path, branch: &str, base: &str, thread_branches: &[(String, String)]) -> Result<TreeBuild>
```

`thread_branches` is `(thread name, branch name)`, already sorted by name by the caller.

Steps, in this order:

1. `remove_worktree(root, path)` and `delete_branch(root, branch)` first, so a stale tree from a killed run
   cannot poison this one.
2. `add_worktree(root, path, branch, base)`.
3. For each `(thread, branch_name)` in order: skip it when it has no commits past the base. Ask git, do not
   guess: `git -C <root> rev-list --count <base>..<branch_name>` returns `0`. Add
   `pub fn commits_ahead(root: &Path, base: &str, branch: &str) -> Result<u64>` if that reads better than
   inlining the command.
4. Otherwise `merge(path, branch_name, &format!("round check: {thread}"))`. `merge` already refuses a dirty
   tree, aborts on conflict, and returns `MergeOutcome::Conflict(files)` with the file names. The fresh
   worktree is clean, so the dirty check is not in your way.
5. On `MergeOutcome::Conflict(files)`: `remove_worktree(root, path)`, `delete_branch(root, branch)`, return
   `Ok(TreeBuild::Conflict { files })`.
6. On any other `Err`, clean up the same way and return the error.
7. Otherwise return `Ok(TreeBuild::Ready(tree))`.

Add `impl IntegrationTree { pub fn remove(self) -> Result<()> }` that calls `remove_worktree(&self.root,
&self.path)` and `delete_branch(&self.root, &self.branch)`. Best effort, no panic.

Rules for this function:

- No new dependency. Shell out through the existing `git()` / `git_ok()` helpers.
- Never create the tree inside the project folder. It lives under the home: `ILVUNO_HOME/worktrees/<run8>/`.
- Nothing in it is ever pushed or kept after the round.

### 2 · Wire it into `close_round`, in `crates/runner/src/driver.rs`

`close_round` is the only place checks run. Today it is:

```rust
let artifacts = self.home.artifacts_dir(run_id);
let checks: Vec<CheckResult> = if failed.is_empty() {
    checks::evaluate(plan.checks(), &Locations { artifacts: &artifacts, goal_dir: plan.base() })
} else {
    Vec::new()
};
```

Change it to:

1. Read `git_root, git_base` from `runs` and `name, branch` from `threads WHERE run_id = ?1 AND branch IS
   NOT NULL ORDER BY name`. Both columns are nullable; when either is missing the run is in folder mode and
   nothing changes.
2. Build the tree at `self.home.worktree_dir(run_id, &format!("round-{}", round.index))` with branch
   `git::branch_name(run_id, &format!("round-{}", round.index))`. `round.index` is the round's own number.
3. Pass `goal_dir: tree.path()` instead of `plan.base()`.
4. `TreeBuild::Conflict { files }` becomes a failed check result **in front of** the real ones:
   `CheckResult { name: "threads_do_not_merge".into(), passed: false, detail: files.join(", ") }`. The round
   then blocks, the wake prompt names it, and the gate shows it. The project folder stays untouched.
5. Any other failure to build the tree also becomes a failed result, named `round_tree_failed` with the
   message. **Never fall back to `plan.base()` when the tree could not be built.** A check that runs in the
   wrong folder is the bug you are fixing; a missing tree must block the round, not pass it.
6. Remove the tree after `checks::evaluate` returns, including when the checks failed or the round is about
   to stop. Use a small guard so an early `?` cannot leave it behind.

Do not change `git::merge`, `run_status::gate_decide`, or anything on the accept path. Accept still merges
the thread branches into the checked-out branch; the integration tree is a preview, not the merge.

### 3 · Tests, in `crates/runner/tests/loop_fake.rs`

The harness you need is already there: `setup_git(name, goal_json, script)`, `GIT_PROJECT_GOAL`, `drive`,
`status`, `verdicts`, `applied`, and the scripted agent that writes the files in the scope into its own
folder. Extend it. Keep the eight tests that already pass green.

1. **The red test flips green.** `a_command_check_runs_in_the_tree_the_gate_would_merge` passes unchanged,
   and still asserts that `plan.base()/brief.md` does not exist.
2. **Two threads, one check.** Two writer threads, each with its own file, and one check
   `test -f a.md -a -f b.md`. It passes only if both branches are in the tree. A goal with two threads and
   no orchestrator, two `run_task` calls, then `drive`.
3. **A conflict blocks the round and names the file.** Two threads write different content to the same file.
   The round must record a failed check named `threads_do_not_merge` whose detail names that file, the run
   must not reach the gate, and `plan.base()` must still hold its original content.
4. **Broken tree fails, fixed tree passes** (the roadmap's kill rule). Round 0: the agent writes `BAD` and
   the check is `grep -q GOOD brief.md`, so it fails. Round 1: the agent writes `GOOD`, and the same check
   passes. Use the scripted agent's ability to give a different body per write, or a plan with two tasks on
   the same thread.

Add unit tests in `crates/runner/src/git.rs` for `integration_tree` itself, using the `Repo` helper that is
already in that module's `mod tests`: a ready tree with two branches, a skipped branch with no commits, and
a conflict that leaves no worktree behind (`git worktree list` must not name it).

## Constraints

- `RUSTUP_TOOLCHAIN=1.100.0` on every cargo command.
- Work on `feat/checks-see-the-work`. Do not merge to `main`, do not push `main`.
- One `cargo test --workspace` when the change is finished, not after every edit. While working, run the one
  test file.
- No new dependency. No new table, no new state, no new API route, no UI change.
- If you are tempted to add a `check_scope` or a `--tree` flag to a goal: no. The goal format does not
  change.
- The window needs nothing. Do not touch `crates/desktop`.

## Done when

- the four tests above pass and `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` is green;
- the red test's readiness record shows `"passed":true` for `command_exit_zero test -f brief.md`;
- `git status` is clean, the branch is pushed, and the report is written;
- `context/timeline.md` has a line, and `context/decisions.md` records the choice: one integration tree at
  round close, built from the base plus every thread branch with commits, because that is the tree accept
  produces.

## Stop and report instead, if

- `git worktree add` inside this repository behaves differently than `git::add_worktree` assumes, and you
  cannot make a clean tree without changing that function for the accept path too. Then report the exact
  command output and stop.
- A round's checks would need to run before the round closes, or in a different transaction. They do not:
  the tree is built and thrown away around the existing `checks::evaluate` call. If that turns out to be
  false, report it and stop.
- You cannot keep the eight existing tests green. Report which one broke and why. Do not weaken it.

---8<--- PROMPT ENDS ---8<---

## For you, before you paste

- **Cost:** build and test time only. The scripted provider drives the whole loop, so no quota is spent.
  There is no need to run `tools/demo.sh`.
- **Check when it reports:** test 4 is the one worth reading. A check that fails on a broken tree and passes
  after the fix is the whole point of the stage. If that test was weakened to pass, the stage did not land.
- **The red test is the contract.** If the worker changes `a_command_check_runs_in_the_tree_the_gate_would_merge`
  instead of the runner, stop it.
