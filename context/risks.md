# Risks, limits and open questions

## Needs the owner, not an agent

| # | Question | Why it matters | Default if unanswered |
|---|---|---|---|
| **O1** | Tool activity in the conversation — v1 or later? | the most-wanted missing panel; the journal already has the rows | build it in stage 3 |
| **O2** | Does stage 2 continue through phase C (gate diff, accept merges)? | the north star cannot happen without it | pause after phase B, where nothing lands automatically |
| **O3** | Rewrite history to drop the removed blobs, then force-push? | `.git` is 23 MB; a clone fetches all of it | leave it |

## Known limits, stated plainly

- **"Ran for" keeps counting** while a run is paused or at the gate: the engine has no end time for those
  states. Honest for a finished run, generous for a paused one.
- **No screenshots of `blocked` or `no_progress`.** Both need an orchestrator and two failing rounds. The
  banner they use is verified in the paused and budget states.
- **`shots/gate.png` is an older run.** The demo run behind the other screenshots was deleted from `/tmp`;
  the images remain the only record.
- **The git floor (2.38.1) has not been run on this machine** (it has 2.55.0). The floor logic is tested; the
  behaviour on an old git is only known from the previous project's study (`research/14-git-floor.md`).
- **Two drivers only.** Nine of the eleven CLIs that t3 speaks to natively are reachable here only if they
  speak ACP.
- **The window is not packaged.** No installer, no auto-update, no first-run guide.
- **A goal whose threads all name one agent is allowed**, and the rail says so. Nothing stops it.

## Technical risks, in the order they are likely to bite

1. **An agent with a real checkout can destroy its branch.** Recoverable — nothing merges until the gate
   accepts — but the diff must be readable *before* phase B turns this on for real.
2. **A hanging hook or a custom merge driver.** Hooks are skipped on our snapshot commits (`--no-verify`).
   A custom merge driver runs inside `git merge`; it must be detected and refused before phase C merges.
3. **A conflict mid-merge.** `git.rs` aborts and names the files, and refuses a dirty project before it
   starts. The gate must then say "stopped", not "failed quietly".
4. **Two runs on one project.** Nothing serialises them today. Two runs would create separate worktrees but
   could merge into the same branch. Needs a rule before it happens.
5. **The repo moves or is renamed** while a worktree points into it. `git worktree prune` handles the
   bookkeeping; the worktree path is stale.
6. **Linux-only.** Nothing has run on macOS or Windows, and the webview is 2.52 against Tauri's tested range.

## Product risks

- **The one metric.** `runs that reach accepted without the human sending a message between agents`. If it
  stays below half, the loop is decoration. It has never been measured over a set of runs.
- **The wedge is unproven.** "Review is the only proven willingness to pay" came from reading competitors'
  pricing, not from a user.
- **Surface.** The window has the shape of T3 Code and Orca but roughly a fifth of their surface. People who
  switch will notice the missing terminal and diffs first.
- **Trust in the board.** If agents write vague entries, the shared state is worse than no state. The
  preamble asks for specifics; nothing enforces them.

## Process risks

- **Context loss** — this folder exists because a long session's reasoning is worth more than its commits.
  Keep appending to it.
- **Long turns, long silences.** A single working day produced 57 commits; the record must not depend on
  anyone remembering it.
- **Delegation.** Builds were delegated (Pi harness, `deepseek-v4.1-flash`, thinking max) and reviews to a
  different model (`gpt-6-luna`, reasoning xhigh). Two of the most expensive mistakes of the day — "resume
  works", "the menu is broken" — survived review because they were never *run*. Delegation does not replace
  verification.
- **A claim in a commit message is not a check.** Every defect found came from executing something.

## What would change the plan

- Agents that reliably finish a whole small change in one turn → rounds and the gate matter less; the wedged
  design would need rethinking.
- A provider that only works over PTY → D2 reverses, and the cost of D2 returns.
- A second person using it → packaging, multi-project and permissions move ahead of git polish.
