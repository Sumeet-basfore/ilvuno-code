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
