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

## Stage 2-C update · gate diff and merge (2026-11-23)

| Feature | Proof |
|---|---|
| Per-thread diff endpoint with a 128 KiB patch cap and full diffstat | `crates/api/src/runs.rs`, `crates/runner/src/git.rs`; API tests in `crates/api/tests/worktrees.rs` |
| Run detail includes each thread's branch, short commit and diffstat | `crates/runner/src/export.rs`; live API response for throwaway run `ae186839-1120-4401-a1fc-77225af4bb11` |
| Accept refuses a dirty project, merges branches in task order, aborts conflicts and is idempotent | `crates/runner/src/run_status.rs`; five scripted API cases in `worktrees.rs` |
| Gate lists branch, diffstat, folded patch and the files/threads to merge | `crates/desktop/ui/app.js`, `style.css`; screenshot read at `/tmp/ilvuno-stage2-live.HZtImU/gate.png` |
| Adapter stderr is excluded from thread commits so it does not pollute the gate diff | `crates/runner/src/git.rs`; `provider_stderr_capture_is_not_committed_as_thread_work` |
| Real accepted run landed `hello.txt` only after the gate; merge subject includes the run id | Throwaway repo run `ae186839-1120-4401-a1fc-77225af4bb11`; 62,623 tokens |

## Stage 3 · tool activity update (2026-11-24)

M5 is no longer blocked. The API returns bounded, filtered journal pages. The window shows tool names,
short targets, permission decisions and recorded task failures under the matching task. It reads existing
history and SSE rows; it does not change a run or write journal rows. Commit `6648207` adds three API
tests for paging, scripted tool and permission rows, and the stored input cap, plus an SSE boundary test.

The real run showed **7 tools and 2 denied permissions**. The window screenshot is
[`tool-activity.png`](../crates/desktop/shots/tool-activity.png); the open fold is
[`tool-activity-open.png`](../crates/desktop/shots/tool-activity-open.png). The workspace had **158
passing tests and 3 ignored live-provider tests**.

## PR review follow-up · 2026-11-24

Gate acceptance now refuses a thread worktree with uncommitted files and names those paths. Only an
untracked `provider-stderr.log` is ignored; a staged or tracked copy is still dirty. The API and git
coverage passed as part of the workspace suite: **160 tests passed and 3 ignored** (`54c0004`).

## Stage 5 · packaging status (2026-11-24)

Tauri produced a `.deb` and an AppImage. The `.deb` metadata includes `libwebkit2gtk-4.1-0` and `libgtk-3-0`;
its file list includes the desktop app, the `ilvuno` CLI and `ilvunod` under the Tauri resource directory.
The AppImage also contains `ilvuno` and `ilvunod`. Launching the AppImage from outside the repository
started the API and returned the real accepted run, but the captured app window was blank. Packaging is
therefore **not complete**. The `.deb` layout was inspected from its archive; this system has no `dpkg-deb`,
so its desktop entry was not executed.

## Stage 5 · package display follow-up (2026-11-24)

The earlier blank AppImage screenshot came from Xvfb without DRI3. On the actual X11 display, both the
AppImage and the extracted `.deb` opened the window and showed the accepted run. The `.deb` CLI returned
`ilvuno 0.0.1`. The workspace suite exited 0. The packages have not yet been published in a PR.

## Stage 5 · About version follow-up (2026-11-24)

The AppImage About tab displays `0.0.1`, matching the extracted `.deb` CLI output. Screenshot:
[`linux-about`](../crates/desktop/shots/2026-11-24-linux-about.png).

## Stage 5 · packaging PR (2026-11-24)

The earlier note that the packages were not in a PR records the status before review began. PR
[#5](https://github.com/Sumeet-basfore/ilvuno-code/pull/5) now contains the Linux packaging changes and
verification. Review is pending.

## Stage 5 · packaging review (2026-11-24)

The Rust, UI and comment reviews found no actionable issues. PR #5 is mergeable. Its only check is an
optional CodeRabbit job that skipped automated review for this repository.
