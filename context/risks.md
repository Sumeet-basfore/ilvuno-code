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
- **Two drivers only.** `claude` and `acp` are proven. t3 speaks eleven CLIs through their own protocols; here,
  a CLI works if it speaks ACP, and only `codex-acp` has been run.
- **The window is not packaged.** No installer, no auto-update, no first-run guide.
- **A goal whose threads all name one agent is allowed**, and the rail says so. Nothing stops it.

## Technical risks, in the order they are likely to bite

1. **An agent with a real checkout can destroy its branch.** Recoverable — nothing merges until the gate
   accepts — but the diff must be readable *before* accepting. Phase B is now live (worktrees and per-task
   commits); phase C (the gate's diff) is still missing.
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

## Stage 2-C update (2026-11-23)

The gate diff and merge path is now implemented. The dirty-tree refusal, conflict abort, file-name report,
repeat-accept behavior and folder-mode behavior have scripted coverage. The old risk that agents could
write a branch whose diff could not be reviewed before acceptance is resolved for the current Linux build.
Two runs on one project are still not serialized; that remains a separate risk before concurrent runs are
used against the same project.

## Stage 3 update (2026-11-24)

O1 is resolved. The window reads tool calls, permission decisions and dispatch boundaries from the journal.
The tool input was already capped at 2,000 characters; the window shows only selected names and targets.
The fold shows the newest 50 rows per task. Individual tool result text is not stored in the journal, so
the window cannot show tool output or a tool-level error. It can show the first line of a recorded
`dispatch.failed` reason or summary. No new provider call was needed for the real-window screenshots.

PR review found a separate stage 2-C risk: the gate diff showed committed changes, while acceptance could
merge that commit and close the run even if a thread worktree also held uncommitted files. Commit `54c0004`
closes this gap. Acceptance checks every worktree before merging and again inside the merge path; dirty
files are named and the run stays at the gate. The API and git tests cover this. The full workspace suite
now has 160 passing tests and 3 ignored live-provider tests.

## Stage 5 · packaging check (2026-11-24)

Tauri created both Linux package files, but the AppImage is blocked: launching it from outside the repo
started its local API, while the 1440×900 window capture was blank. The launch log only showed a GTK
settings warning and a Mesa DRI3 warning. Do not call packaging complete until a real AppImage window shows
the run list. The `.deb` layout was inspected, but `dpkg-deb` is absent on this CachyOS machine, so its
desktop entry still needs a launch check on a system with Debian package tools.

## Stage 5 · package display follow-up (2026-11-24)

The blank AppImage capture was caused by Xvfb without DRI3. The AppImage opened on the actual X11 display,
and the extracted `.deb` executable opened the same run. A full `.deb` install on a Debian system remains
untested because this machine has no `dpkg-deb`.
