# Ilvuno — the project's memory

This folder is the record of the project itself: what was built, when, what was decided, what was refused,
and where the people working on it disagreed. The code says *what*; this says *why*, and what was already
tried.

Read it in this order if you are new:

| File | What it holds |
|---|---|
| [`story.md`](story.md) | The narrative, start to now, in eight turns. Read this first. |
| [`timeline.md`](timeline.md) | Every commit, grouped by the day's phases, with what it changed. |
| [`decisions.md`](decisions.md) | The decision register: what we chose, what we rejected, what would reverse it. |
| [`disagreements.md`](disagreements.md) | Where the user corrected the agent, where the agent pushed back, and how each ended. |
| [`features.md`](features.md) | What exists, what does not, what we refused — with the evidence. |
| [`risks.md`](risks.md) | Open questions, known limits, and what would change the plan. |
| [`prompts/`](prompts/README.md) | One goal per file, ready to run in an agent loop. House rules live there. |

## The short version

Ilvuno is a desktop app where several coding-agent CLIs (Claude, Codex, anything ACP) work on one project in
recorded rounds, sharing a written board, and nothing lands until a person accepts it. Rust engine, local
HTTP API, Tauri window. It runs today; it cannot yet change the project's own code, which is why git is the
next stage.

## How to use this folder

- **Append, do not rewrite.** When something is decided, refused, or corrected, add it. History that gets
  tidied is history that gets lost.
- **A claim needs a name.** Cite a commit, a test, a file, or a number. "It works" is not a record.
- **Say when you were wrong.** `disagreements.md` has a section for the agent's own mistakes, because they
  are the most useful entries in it.

## The rules that shaped it

These came from the user during the work and are not negotiable defaults:

1. Reports are HTML for reading, Markdown for the repository. Never hand-write report HTML.
2. Write plainly: short sentences, everyday words, English is a second language.
3. The app gives abilities — threads, a loop, a board, a gate. It never decides who does what.
4. Agents are choices, not roles. One subscription must be able to fill every thread.
5. Nothing lands without a person. The gate is the product.
6. Trust is earned by checks: run the tests, look at the real window, measure the number, or say you did not.

## Current state update · 2026-11-23

Stage 2-C is in: the gate can show each thread's diff and merge accepted branches into the project's
current branch. It refuses a dirty project and stops on a conflict. The next blocked step for the north
star is no longer gate merging; stage 3 and dogfooding still remain.

## Current state update · 2026-11-24

Stage 3 M5 is done. The conversation shows the tools, targets and permission decisions for each task. It
reads the existing journal and event stream. It does not change a run or write new journal rows. Prompt 04,
the first dogfood change, remains next.

## PR review update · 2026-11-24

PR [#4](https://github.com/Sumeet-basfore/ilvuno-code/pull/4) includes stage 2 and stage 3 because neither
was on `main`. Review found that accepting a gate could omit uncommitted files in a thread worktree.
Commit `54c0004` now refuses acceptance and names dirty files; the API and git tests cover this. The full
workspace suite passed with **160 tests passed and 3 ignored**.

## Packaging status · 2026-11-24

Prompt 06 is waiting. The prompt 04 dogfood run is reviewed and passing, but still needs human acceptance
and landing. The packaging attempt stopped before build changes; see
[`2026-11-24-06-packaging`](prompts/reports/2026-11-24-06-packaging.md).

## Current state update · 2026-11-24 · first dogfood run

Run `64239160-0caa-456a-b3cf-3011a0f64bb1` was accepted by a person and landed by the runner as
`d03a026`. It adds one `branch_name` test. The full workspace suite passed after the landing: **161 passed,
3 ignored, 0 failed**. Prompt 04 is complete; packaging is now underway on `feat/linux-packaging`.

## Current state update · 2026-11-24 · packaging check

The `.deb` and AppImage build artifacts were produced on `feat/linux-packaging`. The AppImage, launched
from `/tmp`, started its bundled API and returned the real accepted run. Its 1440×900 screenshot was blank.
Prompt 06 says to stop on this defect. The package is not ready to ship; the next step is to fix and verify
the blank AppImage window. Details are in [`2026-11-24-06-packaging`](prompts/reports/2026-11-24-06-packaging.md).

## Current state update · 2026-11-24 · prompt 07

The packaging work is committed locally as `d4895f6` on `feat/linux-packaging`. The earlier AppImage check
showed a blank window, so prompt 07 stopped before branch pushes and PR creation. There are no open PRs.
The branch status and recorded test results are in
[`2026-11-24-07-land-and-package`](prompts/reports/2026-11-24-07-land-and-package.md).

## Current state update · 2026-11-24 · package display check

The blank screenshot came from an Xvfb display without DRI3. The AppImage shows the real accepted run on
the actual X11 display. The `.deb` payload was extracted to a fresh `/tmp` root; its CLI and desktop
executable both work there. The workspace suite exited 0. Screenshots and details are in the prompt 07
report.

## Current state update · 2026-11-24 · About version

The AppImage About tab shows `0.0.1`, matching the extracted `.deb` CLI output. Screenshot:
[`linux-about`](../crates/desktop/shots/2026-11-24-linux-about.png).

## Current state update · 2026-11-24 · packaging PR

PR [#5](https://github.com/Sumeet-basfore/ilvuno-code/pull/5) is open from `feat/linux-packaging` to
`main`. It includes the package checks, screenshots and corrected display finding. Review is pending.
