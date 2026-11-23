> Paste everything between the two markers as the goal. Nothing else.

---8<--- PROMPT STARTS ---8<---

**Goal: the gate shows what each thread changed, and accepting it merges that change into the project.**

Read `/home/sumeet/personal-repo/projects/ilvuno-second/context/prompts/README.md` and follow the house
rules in it. This goal assumes the previous one is in: threads work in real worktrees and commit per turn.

## Today

A run can reach the gate and a person can accept it, but accepting changes nothing in the project. The
window shows a board, checks and artifacts — not one line of the code the agents wrote.

Fit that. And nothing else.

## What to build

**1 · The diff reaches the API.**
- `GET /runs/:id/threads/:name/diff` returns `{ branch, base, stat, patch, truncated }` for a thread with a
  checkout. `patch` is capped (128 KiB is plenty); when capped, `truncated` is true and `stat` is still
  whole. A thread in folder mode returns `{ branch: null }` — not an error.
- The run detail (`GET /runs/:id`) gains, per thread, `branch`, `commit` (short sha) and `stat` — so the
  window can list them without one request per thread.

**2 · The gate lands the change.**
- On accept (`POST /runs/:id/gate`, `decision: accept`), after the current accept logic, the runner merges
  each thread's branch into the project's **current** branch, in task order.
- Before the first merge: the project's tree must be clean. If it is not, refuse the accept with
  `409` and say what is dirty. Nothing is half-merged.
- A conflict aborts that merge, names the files, sets the run to `blocked` with those files as the reason,
  and stops. No auto-resolution, no forcing.
- Every merge is journaled: `merge.landed { branch, sha, by: "user" }`, or `merge.conflict { branch, files }`.
  One transaction per merge, like every other command.
- Accepting a **folder-mode** run behaves exactly as today (the decision is recorded, nothing is merged).

**3 · The window shows it.**
- The gate panel lists each thread with its branch and diffstat, and the patch behind a fold (a `<details>`
  is enough — do not build a diff viewer).
- The accept button says what it will do — the number of threads and files — not just "Accept".
- When a merge conflicts, the banner names the files and says the project was left as it was.

## Where

- `crates/runner/src/git.rs` — `merge()` and `MergeOutcome` already exist and are tested. Use them.
- `crates/runner/src/run_status.rs` — the accept path (`decide_gate_and_render`). Add the merge step after
  the decision is recorded, and make the whole thing idempotent: accepting twice must not merge twice.
- `crates/api/src/runs.rs` + `lib.rs` — the diff route.
- `crates/runner/src/export.rs` — per-thread branch, commit, stat in the run JSON.
- `crates/desktop/ui/app.js` — the gate panel and the accept button copy.

## Rules for this goal

- A merge happens **only** after a person accepts. Never on a round passing, never on a resume.
- Never `git merge --abort` is your own tool here; `git::merge` already aborts. Do not add a second path.
- Do not let a merge touch a project with uncommitted changes — refuse first, always.
- One thread, one branch, one merge. If two threads touch the same file, that is a conflict, not a queue.
- Cap what you send to the window. A 2 MB patch will freeze the page.

## Tests you must add

In `crates/api/tests/`, with the scripted provider and temp repositories:

1. a scripted run writes a file, reaches the gate, accept → the file is in the **project** working tree, and
   `git log` in the project names the run;
2. a run whose thread branch conflicts with a change committed to the project after the run started →
   accept is refused or conflicts, the project is unchanged and clean, and the run says which files;
3. a dirty project → accept is refused with `409`, and no merge is attempted;
4. accepting twice → one merge, second call is a no-op;
5. folder-mode run → accept works as before.

## Done when

- those five tests pass and `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` is green;
- a **real** run on a throwaway repository: the change written by an agent lands in the project only when
  the gate is accepted, and `git log` shows the run id in the message;
- `tools/shot.sh` shows the gate panel with a branch and a diffstat, and you have read that image;
- `context/timeline.md`, `context/decisions.md` (the merge rules) and the change log are updated;
- the report file exists.

## Stop and report instead, if

- a merge would need a manual git command to recover — the roadmap's kill rule says stop and simplify;
- you would need to change how a run is created or how a worktree is made (that is goal 01);
- the only way to test it is against the real project.

---8<--- PROMPT ENDS ---8<---

## For you, before you paste

- **This is the dangerous one.** After it, accepting can change your repository. The safety is: clean tree or
  refusal, conflict or abort, and one journal row per merge.
- **Check when it reports:** merge a throwaway run yourself, then confirm the second accept does nothing.
- **Cost:** test-and-build time, plus one tiny real run.
