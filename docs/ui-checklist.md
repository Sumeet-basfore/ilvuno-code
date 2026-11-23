# Ilvuno — UI completion checklist

This is the working plan for finishing the window. It is not a report: it changes as work lands.

How a milestone moves: **implement the smallest complete thing → run the checks → look at the real
window → fix → commit → tick the boxes here.** A box is ticked only with evidence: a test name, a
measured number, or a screenshot path.

Where the rules come from:

- `docs/prd.md` — what v0 must prove, and the non-goals.
- `research/13-ui-anatomy.md` — the layout, taken from Cursor, Codex and Vibe Kanban screenshots.
- `research/01-t3code.md`, `research/02-orca.md` — how the two reference apps organise the same work.
- `AGENTS.md` — no PTY agents, no scheduler, no cloud, no multi-user in v1.

---

## 1. What is approved, and what is not

The goal was to make the window complete, using T3 Code and Orca as references. A reference is not an
approval: these two apps ship whole subsystems that Ilvuno's own docs leave out of v1. Building them
would break `docs/prd.md` §6 and `AGENTS.md`. So they are listed, not built.

### Approved — in the PRD, the data model or the roadmap

| Area | What | Where it is already promised |
|---|---|---|
| Left | Threads as agent sessions, grouped by state, per run | `docs/prd.md` §4 |
| Left | Run switching; project shown from the goal base | `docs/data-model.md` |
| Left | Search across runs, threads, actions | palette |
| Centre | The conversation: prompt, what the agent said, its report, files | `docs/prd.md` §4 |
| Centre | A composer that sends a message task, with no file scope | `data-model.md`, migration 0005 |
| Centre | Live updates, errors, a stopped run with its reason | `docs/api.md` (SSE) |
| Right | Board, checks, gate, artifacts, thread list | `docs/prd.md` §5 |
| Right | Accept or send back at the gate, with a note | `docs/prd.md` §5 |
| Settings | Theme, agent detection with safety, budget defaults, message policy, human gate | `docs/prd.md` §5 |

### Not approved — do not build without the user's word

| Asked for in the goal | Why it is not approved | What would make it approvable |
|---|---|---|
| Git worktrees, branches, commits, diffs, PR creation, merge controls | v1 has one branch and a runner-executed merge at the gate. `AGENTS.md`: "Let an agent's shell touch main" is a *do not*; per-thread worktrees are roadmap stage 2 | Roadmap stage 2 written into the PRD, with conflict rules |
| An in-app terminal / PTY | `AGENTS.md`: no PTY agents in v1 (orca carries 45 hand-tuned records for it) | A decision reversing D2 in `research/07-decision.md` |
| File explorer and file viewer | Not in the PRD. Artifacts are listed today; reading them is a different feature | A PRD line and a security note (reading arbitrary project files) |
| Attachments (files, images) in the composer | Providers take text today; attachments need a per-provider path | Provider contract change |
| Per-message model and reasoning-effort pickers | The agent is chosen per thread at goal time; per-message model choice is not in the PRD | A PRD line. Until then the CLI's own default applies |
| Changing a thread's agent mid-run | The thread's agent is fixed when the run starts | Treated as M4 below — small, and it is the product's point |
| Multiple projects side by side | One run belongs to one project; the rail switches runs, not projects | A PRD line |

Nothing above is silently dropped: each is either a milestone below or a question in §5.

---

## 2. Milestones

Status: **done** · **next** · **blocked** (needs a decision) · **later** (approved, low value now)

### M0 — The thread-first shell · done

- [x] Threads grouped Needs you / Working / Done, from real run state
- [x] Selected thread's conversation in the centre, newest turn at the bottom
- [x] Composer sends `kind:"message"`; the board does not move (test: `a_message_needs_no_scope_claims_no_board_entry`)
- [x] Board keeps a full view (`#board`)
- [x] Settings: General, Agents, Defaults, About
- [x] 11 screenshots at 1440×900 in `crates/desktop/shots/`, from a real run
- Evidence: commit `cf3a083`, `cargo test --workspace` → 130 passed
- Known: `gate.png` comes from an older run with a gate open; the demo run is accepted

### M1 — The run's facts, in the rail · done

The v3 shell showed project, run id, duration, rounds, tokens and a budget bar. v4 dropped them. All of
these are real fields on `GET /runs/:id`; none need new data.

- [x] A **Run** section above Board in the right rail: folder, run id, how long it has run, rounds used
      against the cap, tokens used against the cap, and a bar
- [x] Every number comes from the API. No spend in dollars — the engine records no cost (v3's `$0.12`
      was invented)
- [x] A missing value renders `—`, never `0`
- [x] The folder keeps its leading slash on short paths (`/tmp/stop2`, not `tmp/stop2`)
- [x] Fits at 1440; at 1280 the rail folds and opening it shows the same block, with no sideways scroll
- Evidence: commit `4c2adfe`; `shots/threads.png` shows the block against run `6ba693b9`, whose API
  answers `362211` tokens of `600000` in `2` of `3` rounds — the same numbers on screen

### M2 — Stop a working run · done

A run could only be stopped by waiting for it to stop itself. A person watching a bad round had no move.

- [x] `POST /runs/{id}/stop`: the turn in flight finishes, no new task or round starts, status
      `human_paused`, reason `stopped by you`
- [x] The status change is journaled in one transaction with the projection (`run.stop` + `run.transition`)
- [x] A **Stop** button in the topbar while the run is working; its tooltip says what happens
- [x] The banner says who stopped it and offers Resume
- [x] Resume works — it did not before: `drive_with` now reopens a stopped run before the loop reads it
- Evidence: commit `40d1009`; `cargo test --workspace` → 133 passed, including
  `stopping_a_run_pauses_it_and_keeps_the_reason`, `a_finished_run_cannot_be_stopped`,
  `a_stopped_run_can_be_resumed`. Stopped a real run 8s in (12 shell sections): the turn finished, the
  task finished as `reported`, the round stayed `open`, and `shots/stopped.png` is that screen.
      

### M3 — New run from a goal file · later

The New run dialog takes pasted JSON. People keep goals in files.

- [ ] A file picker (Tauri dialog) that reads a `.json` goal and fills the editor
- [ ] A bad file shows the parse error and does not start a run
- Acceptance: a run created from `examples/board-experiment.json`; a syntax error shown as an error

### M4 — Choose the agent for a thread · later

The product's claim is "agents are choices". Today the choice is made once, in the goal file.

- [ ] The thread header can change the agent for the *next* task, from the detected list
- [ ] A same-provider warning when a run would use one CLI for every thread
- [ ] The journal records who chose, and the coordinator's prompt version, per thread
- Acceptance: change a thread from Codex to Claude, next task runs on Claude, journal shows the choice

### M5 — What the agent actually ran · blocked

The conversation shows the agent's words and its report. It does not show the tools it called. Codex and
Cursor both show this, and it is the first thing people look for when a run fails.

- Not approved yet: the runner records results, not tool events. Streaming them means a provider-contract
  change (Claude `tool_use`, ACP `session/update`).
- Decision needed: is a read-only activity list in v1, or does it wait?

### M6 — Reachability and scaling · next

- [ ] Keyboard-only pass: every action reachable, focus visible, no trap leaks (dialogs already trap)
- [ ] Contrast measured ≥ 4.5:1 for text, ≥ 3:1 for the chip borders, in dark and light
- [ ] No horizontal scroll at 1440, 1280, 1060; the rail folds at 1280 as designed
- [ ] Zoom 200% keeps the conversation readable
- Acceptance: numbers written down per check; screenshots at the three widths

### M7 — Every state says what it is · later

- [ ] No runs, no threads, no tasks, blocked, budget exhausted, no progress, offline, loading
- [ ] Each one states the cause and offers the next action, or says plainly that there is none
- Acceptance: one screenshot per state in `shots/states/`

### M8 — Settings that stick, or say why not · later

- [ ] A failed `PUT /settings` shows the error and keeps the old value on screen
- [ ] Each setting names its effect in one line (budget rounds, tokens, policy, gate)
- Acceptance: stop the runner, change a setting, see the error; restart, value unchanged

---

## 3. How to verify

```bash
RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace          # 130 tests today
ILVUNO_HOME=/tmp/demo-real ./target/debug/ilvunod &      # the runner the window reads
tools/shot.sh /tmp/check.png 1280 900                    # one shot at a width (opens a real 1280 window)
tools/shots.sh                                           # the full set into crates/desktop/shots/
```

The window's HTML, CSS and JS are compiled into the binary (`frontendDist`), so **an edit needs
`cargo build -p desktop` before it can be seen.** `tools/shot.sh` does that for you.

## 4. Defects found and fixed

| Found | Cause | Fixed in |
|---|---|---|
| Long file paths cut with no ellipsis | `.fchip` was `inline-flex`; `text-overflow` does not apply to a flex container (measured: 213px box, 226px of text) | `cf3a083` |
| A "more" link on short board entries | `.led-more{display:block}` beat the `hidden` attribute | `cf3a083` |
| `**bold**` printed literally in messages | agent text was escaped into one `pre-wrap` block | `cf3a083` |
| A thread opened half-way up an old message | `scrollHeight` read before the font swap reflowed the pane | `cf3a083` |
| Palette count read "1 runs" | plural missing | `cf3a083` |
| Shortcuts said ⌘ on Linux | copy written for macOS | `cf3a083` |
| The task drawer printed raw markdown | it escaped `message` instead of using `md()` | `cf3a083` |
| **Resume did nothing on any stopped run** | nothing set a run back to `working`, so the driver read the paused state and returned at once | `40d1009` |
| "1 rounds", and "start a new run" on a paused run | plural and status copy | `40d1009` |
| Short folder paths lost their leading slash | the shortening rule assumed a deep path | `40d1009` |

## 5. Known limits

- "Ran for" keeps counting while a run is paused or at the gate, because the engine has no end time for
  those states. It is honest for a finished run and generous for a paused one.
- `gate.png` is an older run with a gate open; the demo run is accepted.

## 6. Open questions for the user

1. **Tool activity in the conversation (M5).** In or out of v1? It needs a provider-contract change.
2. **Git (worktrees, branches, PR, merge).** Roadmap stage 2, and the biggest single piece of the
   reference apps. Approve it as the next stage, or keep v1 to one branch and the gate?
3. **A terminal.** `AGENTS.md` says no. Reference apps all have one. Confirm it stays out.
4. **Cost.** The engine records tokens, not money. Should it record a per-call cost cap result, so the
   rail can show spend honestly?
