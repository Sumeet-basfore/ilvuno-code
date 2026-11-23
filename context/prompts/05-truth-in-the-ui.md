> Paste everything between the two markers as the goal. Nothing else.

---8<--- PROMPT STARTS ---8<---

**Goal: everything the window shows about a run is true, including after a crash.**

Read `/home/sumeet/personal-repo/projects/ilvuno-second/context/prompts/README.md` and follow the house
rules in it. This is roadmap stage 3, minus the tool-activity panel (that is goal 03).

## Today

Three places where the window is not quite honest:

1. **"Ran for" keeps counting** while a run is paused or at the gate, because a paused run has no end time.
   Leave it overnight and the window says it ran for fourteen hours.
2. **The budget is a percentage with no depth**: one bar for the whole run, no per-round token counts, no
   warning before a round that cannot afford its tasks.
3. **A kill is invisible.** If the runner dies mid-run, the restart shows a run still claiming `working`,
   with a turn that will never finish. Nothing marks it.

## What to build

**1 · A stopped run gets an end time.**
- When a run goes `human_paused`, set `ended_at`. `reopen_if_stopped` already clears it on resume across all
  four stopped states — check that it does, and that "Ran for" is frozen while paused and correct after a
  resume.

**2 · A budget a person can read.**
- The rail's Run block keeps its bar, and gains per-round tokens from the rounds already in the API
  (`rounds[].tokens`).
- Before a round would exceed the budget, the run says so in the journal and the window shows it — the round
  is not attempted in silence.

**3 · A kill is recorded and shown.**
- When the runner starts, any run still marked `working` with **no drive holding it** is marked
  `human_paused` with the reason `runner stopped; the turn was lost`, journaled as `run.transition`.
  Do this once at startup, before serving requests.
- The window shows that reason in the existing stop banner. No new state, no new banner shape.

**4 · An honest first paint.**
- On open, the window must not show `working` for a run whose status has changed since the page loaded. It
  already polls; prove the ordering with a test rather than a comment.

## Where

- `crates/runner/src/run_status.rs` (`set_run_status`, the paused transition),
- `crates/cli/src/ops.rs` (`reopen_if_stopped`),
- `crates/api/src/main.rs` or `lib.rs` — the startup reconciliation, before the listener starts,
- `crates/runner/src/export.rs` — per-round tokens are already exported; confirm rather than duplicate,
- `crates/desktop/ui/app.js` and `style.css` — the Run block and the banner.

## Rules for this goal

- No new run status. `human_paused` with a reason covers a lost turn; do not invent `crashed`.
- The reconciliation must be idempotent: run it twice, get the same result, one journal row.
- Never mark a run paused while a drive is actually in flight — check the claim, not the clock.
- Do not touch the gate, the merge, or the worktree code.

## Tests you must add

1. a run in `working` with no driver is reconciled to `human_paused` with that reason, journaled once;
2. running the reconciliation twice adds no second journal row;
3. a paused run's `ended_at` is set, and after a resume it is empty again;
4. a run whose budget cannot afford the next task records the refusal instead of attempting it.

## Done when

- those tests pass and `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` is green;
- a real run: start it, `kill -9` the runner mid-turn, start the runner again, open the window — it says the
  run is paused because the runner stopped, not `working`;
- screenshots of the paused-with-reason banner and the per-round budget block, and you have read them;
- `context/timeline.md` and `context/decisions.md` are updated.

## Stop and report instead, if

- the reconciliation needs a lock or a table that does not exist — then report the design and stop;
- proving the ordering would require a browser test framework (there is none; use the API test instead).

---8<--- PROMPT ENDS ---8<---

## For you, before you paste

- **Cost:** test-and-build time, plus a couple of tiny real runs for the kill test.
- **Check when it reports:** the `kill -9` case is the one worth reproducing yourself — it is the most
  common real failure, and the window was lying about it.
