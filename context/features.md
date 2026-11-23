# What exists, what does not, what was refused

Evidence, not adjectives. A row without a commit, a test or a file is a row to distrust.

## Built

### Engine (`crates/runner`, 3,662 lines)

| Feature | Proof |
|---|---|
| A journal where every command writes rows, projections and a receipt in one transaction | `crates/runner/src/store.rs`, `tests/invariants.rs` |
| Five invariants, refused by name | `tests/invariants.rs` (14 tests) |
| Threads, tasks, messages (`kind: "message"` = a task with no scope) | `crates/runner/src/tasks.rs` |
| Rounds: every open task, then checks, then a verdict | `crates/runner/src/driver.rs` |
| The board: intents, findings, questions, answers, decisions | `crates/runner/src/ledger.rs` |
| Budgets: rounds, tokens, per-call dollars; checked **per task** | `driver.rs` `budget_exhausted` |
| The gate: accept, or send back with a note | `crates/runner/src/run_status.rs` |
| Stop a run; the turn in flight finishes | `run_status::stop` (`40d1009`) |
| Resume reopens the run before the loop reads it | `cli/src/ops.rs` `reopen_if_stopped` |
| Choose a thread's agent, journaled with from/to | `run_status::set_thread_agent` (`d36828b`) |
| Replay and export (run.json, board.md, journal.jsonl, artifacts) | `crates/runner/src/replay.rs`, `export.rs` |
| Git core: worktree per thread, commit per turn, diff, merge, conflict abort | `crates/runner/src/git.rs`, 5 tests (`37e6296`) |
| A thread works in a worktree of a clean git project, on `run/<run8>/<thread>`; one commit per task; folder mode is the fallback | `crates/api/tests/worktrees.rs` (4 tests), a real run on a throwaway repo |

### Providers

| Feature | Proof |
|---|---|
| Claude over `stream-json` with permissions and interrupt, no SDK | `crates/provider-claude` |
| Any ACP agent behind one trait | `crates/provider-acp` |
| Two drivers: `claude` (its stream-json protocol) and `acp` (proven with `codex-acp`; any other CLI that speaks ACP should work, and has not been run) | `cli/src/providers.rs`, `research/09-spike.md` |
| Safety shown per agent, from the CLI's own answer where it can be read | `/agents`, `safetyOf()` |

### Local API (`ilvunod`, 127.0.0.1:4737)

`GET /health` · `GET/POST /runs` · `GET /runs/:id` · `GET /runs/:id/board` · `GET /runs/:id/events` (SSE) ·
`POST /runs/:id/tasks` · `POST /runs/:id/gate` · `POST /runs/:id/stop` · `POST /runs/:id/resume` ·
`PUT /runs/:id/threads/:name/agent` · `GET /agents` · `GET/PUT /settings`.
CORS: only the window's two origins. Writes must be `application/json`.

### Window (`crates/desktop/ui`, 2,477 JS + 523 CSS)

| Feature | Proof |
|---|---|
| Threads left, grouped Needs you / Working / Done; run picker; search | `cf3a083` |
| The selected thread's conversation, newest turn at the bottom | `cf3a083` |
| Composer → message task; disabled with a stated reason | `53e5d2e` |
| Run facts: folder, id, ran-for, rounds, tokens, bar | `4c2adfe` |
| Stop button; paused banner with Resume | `40d1009` |
| Agent picker per thread, with a one-agent note | `d36828b` |
| New run from a path or the system picker | `d7b4a22` |
| Settings: appearance, agents, defaults, about; an error reverts its own edit | `9e99bb8` |
| Honest states: no runs, waiting, paused, budget spent, offline, loading | `d58ec15` |
| Board as a full view; task drawer; palette; light/dark | `cf3a083` |
| Reachable by keyboard, focus ring at 5.5:1, no clipping at 1440/1280/1060/960 | `9541b44` |

### Tools

`tools/demo.sh` (one real run end to end) · `tools/window.sh` (runner + window) · `tools/shot.sh` (build and
screenshot at a width) · `tools/shots.sh` (the whole screenshot set) · `tools/render-report.py` (Markdown →
HTML, with an index) · `tools/render-prototype.py` (board layouts) · `tools/render-app-v2.py`/`-v3.py` (the
design mocks).

## Not built

| Missing | Blocks | Stage |
|---|---|---|
| **The gate showing a diff, accept merging** | landing anything | 2-C |
| **Tool activity in the conversation** | seeing what the agent ran — the data is already journaled | 3 / M5 |
| Budget bar, permission log, reopen-after-kill honesty | roadmap stage 3 | 3 |
| Packaging, auto-update, first-run guide | anyone else using it | 5 |
| Multiple projects side by side | — | not planned for v1 |

## Refused (see `decisions.md`)

An in-app terminal · PTY agents · a file explorer · attachments · per-message model and reasoning pickers ·
git UI (branches, PRs, merge buttons) · cloud, scheduler, multi-user · copying code from the reference apps ·
an agent's shell touching `main` · inventing money figures.

## How to see it working

```bash
RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace     # 147 tests
tools/demo.sh                                       # one real run, ~12 cents of Claude
tools/window.sh                                     # the runner and the window
```

Screenshots of every screen and every state: `crates/desktop/shots/` and `crates/desktop/shots/states/`.
