# The story, in eight turns

Everything below happened on **2026-11-23**, between 01:36 and 21:00 IST: 57 commits in one working day.
The project had a false start before that — a Go attempt — and the day began by studying three shipped apps.

---

## 1. The question, and three apps

The start was not code. It was: *what would make a desktop app where several agents work on one project
better than opening three terminals?*

Three open-source apps were cloned and read — `pingdotgg/t3code`, `stablyai/orca`, `davidondrej/cloudroom`
(MIT, MIT, Apache-2.0). Four documents came out of it:

- ~80% architectural convergence: each app is a shell around vendor CLIs, a transcript, and a worktree.
- The real cost is not the UI. It is the adapters: orca carries 45 hand-tuned PTY records, t3 has 11 native
  drivers, ACP has 17 dialects.
- **Nobody solved ambient shared context.** All three keep a transcript of what was said; none keeps a
  written state that survives a handoff.

That last gap became the product.

## 2. Rust, and a real window

The user rejected a browser UI outright: *"nobody codes in a browser"*. Verification followed: codex is
Rust, ACP has an official Rust SDK (5.5.1). A spike proved both provider paths before committing to either:

- Claude over `stream-json` works with no SDK, if you know three things: `--session-id` must be a UUID, the
  `control_request{"subtype":"initialize"}` handshake is mandatory (skip it and every tool is silently
  auto-denied), and `--permission-prompt-tool stdio` is what makes approvals reach you.
- ACP from Rust worked on the first try (37 s compile, `codex-acp` 2.9.0).
- `codex app-server generate-json-schema` emits 39 files — do not depend on the published protocol crate.

Decisions: engine in Rust as its own process; Tauri 2 shell with a plain HTML page (no bundler — Rust has
nothing like `components/chat`, which is 67k lines in t3 and 98k in orca); providers by ACP first, native
Claude second, **no PTY**.

## 3. The engine, and the thing that makes it different

The engine is a journal. Every command writes a journal row, a projection update and a receipt in **one
SQLite transaction**, so the state can always be rebuilt and a replayed run matches the original. Five
invariants are checked and refused by name.

On top of that: **threads** (an agent with a standing job), **tasks** (one prompt and a scope), **rounds**
(every open task, then the checks), **the board** (what each thread wrote down), and **the gate** (a person
accepts or sends back).

The board is the answer to the gap the three apps left. It is not a transcript; it is a written state that
the next agent reads.

## 4. Built with an agent, and the first real defect

v0 was built by a delegated agent (Claude Haiku 5.5, high effort) in milestones, with 47 tests. Reviewing it
found a real hole: the worker preamble told agents to write to the board, but the only command allowed was
`ilvuno report`. The fix was a second command, `ilvuno note`, and preamble v2.

Four PRD §7 checks passed on live runs; the fourth — refusing a fourth round — was covered by tests.

## 5. Agents are choices, not roles — and no modes

The first design had fixed roles: "Claude codes, Codex reviews". The user rejected it: *if I have only one
subscription, it must still work.* So a goal names agents, and any thread can use any of them.

Then the design had **modes** — a research mode, a feature mode. Rejected again: *the app gives abilities,
the user decides with prompts.* The PRD was rewritten around that sentence. `research/11-product-thesis.md`
recorded the three places the agent disagreed, and the user settled two of them.

The first all-Codex run then failed twice, for two different reasons, both worth remembering: Codex could
not find `ilvuno` on `PATH` (fixed with an absolute binary path in the preamble), and `codex-acp`'s default
mode self-approves risky actions, so the permission policy never ran (fixed with `workspace-write`).

## 6. The window, rejected twice

The first window was rated **5/10**. The second was *"nothing like the aim"*. That was the most useful
feedback of the day, because it forced the question of what the window is *for*.

Cursor, Codex and Vibe Kanban were opened and photographed (`research/ui-refs/`), and the anatomy written
down (`research/13-ui-anatomy.md`): left is the list of conversations, centre is the selected one, right is
the work product — the branch, the diff, the PR.

Ilvuno's window had it backwards: it navigated by run, and put a log in the centre. The third attempt is the
one that shipped: **threads on the left** (Needs you / Working / Done), **the conversation in the middle**,
**the run's shared work on the right** (board, checks, gate, artifacts), with the board keeping a full view
of its own.

## 7. A checklist, and the defects it found

The UI work then ran as a numbered checklist (`docs/ui-checklist.md`), each milestone verified against the
real window: M0 shell, M1 run facts, M2 stop, M3 goal from a file, M4 agent per thread, M6 reachability,
M7 states, M8 settings. 136 tests by the end.

Verifying instead of assuming found things reading never would:

- **Resume never worked.** Nothing set a stopped run back to `working`, so the driver read the state it was
  being resumed from and quit. Every Resume button had been a no-op.
- **A CSS-only edit rebuilt nothing.** The window's files are compiled into the binary, and the build script
  did not watch `ui/`. Two identical screenshots gave it away.
- Long file paths were cut mid-word, because `text-overflow` does not apply to a flex container.
- The agent menu was hidden under the button's own native tooltip.
- A run with nothing running called itself "working".

## 8. GitHub, the cleanup, and the first step of git

The old Go repository became this one: its history was replaced, its branches deleted, and the licence it
had approved (Apache-2.0, with a recorded checksum) carried over.

Then the repository itself was tidied: 41 generated HTML files, three design mocks, a PDF and a stray export
were untracked and ignored. **210 files / 13 MB → 162 files / 7.2 MB.** The screenshots stayed, because they
are the only record of runs that no longer exist.

Last, the finding that set the next stage: **an agent's folder is `run/threads/<name>/` — a scratch folder.**
`base` only resolves reads. No run can change a line under `crates/`. So the north star — Ilvuno building
Ilvuno — is blocked on git, not on polish.

Stage 2 began with its dangerous half: `crates/runner/src/git.rs`, 300 lines and 5 tests on throwaway
repositories — worktree per thread, a commit per turn, a diff the gate can show, and a merge that refuses a
dirty project and aborts a conflict rather than leaving one behind.

---

## Where it stands

| | |
|---|---|
| Code | 3,662 lines of Rust engine, 2,477 lines of window JS, 523 of CSS |
| Tests | 141 |
| Providers | 2 drivers: `claude`, `acp` (which covers Codex, Pi, Cursor, opencode, Antigravity) |
| Real runs | a 35-minute two-reader experiment (362k tokens); a stopped run; a blocked run; a budget-exhausted run |
| Not there | git wiring, tool activity in the conversation, packaging, installs |
| North star | not yet — the next stage is what makes it possible |
