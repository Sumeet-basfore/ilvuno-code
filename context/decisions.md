# Decisions

The rules: a decision names what it settles, why, what it rejected, and **what would reverse it**. The six
early ones are argued in full in [`research/07-decision.md`](../research/07-decision.md); this page is the
register of everything, including what came after.

Status: **settled** · **reversed** · **open** (needs the user)

## The six that came before the code

| # | Decision | Why | What would reverse it |
|---|---|---|---|
| **D1** | The engine is Rust, in its own process | one binary, crash isolation, outlives the window. Not for RAM: the measured gap against Electron is 26 MiB | a provider that only speaks a language we cannot reach from Rust |
| **D2** | Providers by tier: ACP first, native Claude and Codex second, **no PTY in v1** | PTY is a permanent tax (orca carries 45 hand-tuned records) | a CLI with no ACP and no stream protocol that people actually need |
| **D3** | Tauri 2 shell; a plain HTML page reading loopback HTTP + SSE | Rust has nothing like `components/chat` (67k lines in t3, 98k in orca) | the webview proving unusable — check before working around it |
| **D4** | The wedge is a goal run: rounds until a definition of done | review is the only proven willingness-to-pay; shared context alone is not a moat | agents getting good enough that a single turn is enough |
| **D5** | The loop has a budget, a stop rule, and a human gate | an agent loop without a stop is a bill | nothing known |
| **D6** | v1 refuses cloud, mobile, scheduler, multi-user, PTY agents, merge arbitration | each is a subsystem, not a setting | a user who needs one to use the app at all |

## Settled while building

| # | Decision | Why | What would reverse it |
|---|---|---|---|
| **D7** | **Agents are choices, not roles** — a goal names agents, any thread may use any of them | *the user's own words*: a person with one subscription must be able to fill every thread | — |
| **D8** | **No modes.** The app gives abilities; the user decides with prompts | modes would make the engine branch on the kind of work, which it must never do | — |
| **D9** | A message is a **task with no scope** — `kind: "message"` | the composer talks to an agent without claiming files; messages write no board intent | a board that needs to record questions asked in passing |
| **D10** | The window is **thread-first**: threads left, conversation centre, the run's work right | Cursor, Codex and Vibe Kanban all do this; our earlier run-first layout hid the conversation | a run with one thread, where a run view is enough |
| **D11** | **Stop finishes the turn in flight**, then starts nothing new; `human_paused` | half a turn leaves nothing worth keeping | agents that checkpoint mid-turn |
| **D12** | **Resume reopens the run** before the loop reads it | without this the loop read the pause it was being resumed from and quit — Resume had never worked | — |
| **D13** | A run is **waiting**, not working, when nothing is running and no round is open | the engine was right; the window was lying | — |
| **D14** | The **gate offers no Resume on a spent budget** | pressing it would do nothing; the honest move is a new run with a larger budget | budgets becoming raisable mid-run |
| **D15** | **No invented money.** No dollar figure in the UI | the engine records tokens, not cost; the v3 mock's `$0.12` was invented | per-call cost actually recorded |
| **D16** | **Files are opened, not pasted**: a path field plus the system picker, `dialog:allow-open`, a 1 MiB cap | people keep goals in files | — |
| **D17** | **Worktrees live under `ILVUNO_HOME/worktrees/<run8>/<thread>`**, not inside the project | inside the project they appear as an untracked folder and can be committed by accident | the editor workflow needing to see them, which is what the gate's diff is for |
| **D18** | The runner's **snapshot commits skip hooks** (`--no-verify`) | a hanging pre-commit hook would freeze the runner; the human reviews the diff at the gate | hooks proving fast and wanted on every turn |
| **D19** | **Generated pages are not committed**: `research/*.html`, `docs/*.html`, `proto/`, `mock/` | GitHub renders Markdown; the HTML only added diff noise. 210 files/13 MB → 162/7.2 MB | — |
| **D20** | The licence is **Apache-2.0**, the text the old repository approved (sha256 `cfc7749b…`) | patent grant; the owner already chose it once | the owner choosing differently |
| **D21** | **A thread works in a git worktree of a clean project** — branch `run/<run8>/<thread>`, checkout under `ILVUNO_HOME/worktrees/…`, a commit after each task. Not a repo, a dirty tree, old git or a worktree that cannot be made: folder mode, journaled once. The project's own working tree is never committed to, checked out or merged | an agent must change the project's code; a scratch folder cannot. The project's HEAD stays still until the gate | a workflow that needs the project tree live, or a repo that cannot use worktrees |

## Reversed or corrected

| # | What was believed | What replaced it | How it was caught |
|---|---|---|---|
| **R1** | Fixed roles: "Claude codes, Codex reviews" | D7 — agents are choices | the user |
| **R2** | Modes for kinds of work | D8 — abilities, not modes | the user |
| **R3** | A browser UI | a Tauri desktop window | the user |
| **R4** | Tool activity needs a provider-contract change (the agent said so) | **wrong**: `tool.call` and `permission.decided` are already journaled, 26 and 24 in one real run; the SSE stream carries them | the agent checking its own claim |
| **R5** | Dogfooding is the next step after the UI | no: an agent's folder is a scratch folder, so no run can touch `crates/` | reading `dispatch.rs` |
| **R6** | Resume worked | it never did (see D12) | a live stop-and-resume |

## Refused, with the reason

| Refused | Why | What would make it allowed |
|---|---|---|
| Copying code from t3code, orca or cloudroom | enormous, coupled to toolchains we are not adopting; Apache-2.0 would mean `NOTICE` and marked files | a shape, copied with attribution, never a subsystem |
| An in-app terminal / PTY | D2, D6 | reversing D2 |
| Git worktrees/branches/PR/merge **in the UI** | those were not approved for the window work; stage 2 is the engine's own git, not a git GUI | — |
| File explorer, attachments, per-message model pickers | not in the PRD | a PRD line each |
| An agent's shell touching `main` | the rule that makes the gate meaningful | — |
| History rewrite / force-push to shrink `.git` | destructive; 23 MB is tolerable | the owner saying yes |

## Open

| # | Question | Blocks |
|---|---|---|
| **O1** | Show what the agent actually ran (M5) — in v1 or later? | the conversation's most-wanted missing panel; the data is already there |
| **O2** | Stage 2 approved through phase C (gate shows diff, accept merges)? | the north star |
| **O3** | Rewrite git history to drop the removed blobs, then force-push? | repository size only |

## Stage 2-C · merge rules settled (2026-11-23)

| # | Decision | Why | What would reverse it |
|---|---|---|---|
| **D22** | Accept merges each checked-out thread branch into the project's current branch, in first-task order. The project must be clean before the first merge. Each merge and its journal event share a transaction. A conflict aborts that merge and blocks the run with the file names. Repeated accept is safe. Folder mode records acceptance without merging. | The gate must show the work and make acceptance change the project. A dirty tree or unresolved conflict must never be hidden. | A merge that cannot be aborted cleanly, or a test that shows the project can be left half-merged. |

O2 is resolved: stage 2 continues through phase C. The gate diff and merge path landed in `e430433`.

## Stage 3 · tool activity settled (2026-11-24)

| # | Decision | Why | What would reverse it |
|---|---|---|---|
| **D23** | O1 is resolved: show tool activity in v1 using existing journal rows. Use dispatch boundary rows to place activity under the right task. Show only the tool name, a short target, permission state and the first line of a recorded dispatch failure. | The user approved M5. The journal and SSE already have the needed records, so the window can stay read-only and the provider contract stays unchanged. | If the runner stops recording a reliable dispatch boundary, revisit how a tool row is tied to a task before showing it. |

O1 is resolved. The implementation is `6648207`; run and screenshot evidence is in
[`context/prompts/reports/2026-11-24-03-tool-activity.md`](prompts/reports/2026-11-24-03-tool-activity.md).

## Stage 5 · Linux packaging decision (2026-11-24)

| # | Decision | Why | What would reverse it |
|---|---|---|---|
| **D24** | The packaged window starts its bundled `ilvunod` when `/health` on `127.0.0.1:4737` is not healthy. It uses Tauri's resource directory and a direct executable path. The runner stays alive after the window closes. The child gets only `HOME`, `PATH`, `USER`, locale, terminal/temp settings and `ILVUNO_HOME` from the parent. | This lets a person open the app without starting a service by hand, and closing the window cannot kill an active run. A systemd user unit adds setup work; a manual start defeats the first-run goal. | If the AppImage launch shows that this process model is unreliable, or a user needs a documented stop control before v1. |

The runner startup code and package layout are implemented, but prompt 06 is not complete. The AppImage started `ilvunod` and returned the real accepted run, while the captured window was blank. See the packaging continuation report before treating this decision as shipped.

## Stage 6 · checks that see the work (2026-11-25)

| # | Decision | Why | What would reverse it |
|---|---|---|---|
| **D25** | At round close, checks use one temporary integration worktree built from the run base and the checked-out thread branches, in the same order as gate acceptance. A merge conflict or tree-build error blocks the round. The temporary branch uses a distinct suffix so it cannot collide with a valid thread name. | Per-thread checks miss changes from other threads. Checking the combined tree the gate accepts makes a passing command relevant to the work that will land. A partial tree is not a valid check target. | If acceptance changes its branch selection or order, round checks must use that same logic. |

The tree is removed after checks. On a merge conflict, configured checks are skipped because no valid combined tree exists. The readiness record shows the conflict and its files.

### Stage 6 follow-up · keep the checked base fixed

The project must still be at the run's recorded `git_base` when checks close the round and when a person
accepts the work. If the project moved before checks, the round records `project_head_changed`. If it moved
after checks pass, acceptance is refused. A new run is required because its branches start from the new
project commit. This avoids accepting work that was checked against a different base.

Each merge step also checks the expected project HEAD. A retry uses the latest recorded landed commit as
its expected HEAD. If the project moved during a partial accept, the runner refuses the remaining branches.

## Harness · Orca instead of T3 Code (2026-11-25)

| # | Decision | Why | What would reverse it |
|---|---|---|---|
| **D26** | The work harness is Orca, or whatever the owner chooses next. Ilvuno itself takes no dependency on any harness: it runs agent CLIs, and its engine, API, window and packages are unchanged by this. | The owner stopped trusting T3 Code with the data of the agents inside it. That is the owner's call about their own machine, and it costs the project nothing because nothing in the code ever depended on the harness. | Nothing in Ilvuno. If a harness ever offers an embedded agent runtime that the product needs, that would be a new product decision, not a swap of tools. |

Verified before writing this: a search of the tree for T3 and Orca finds only research prose about the three
reference products. No build file, no config, no code path names either as a dependency.

Recorded honestly: the reason for leaving is the owner's and was not audited here. The durable sentence is
"we stopped using it", not "it was proven to do X".

## Gate recovery · a merge without its journal row (2026-11-25)

| # | Decision | Why | What would reverse it |
|---|---|---|---|
| **D27** | On acceptance retry, record an unjournaled Git merge only when its first parent, second parent, message and tree match the expected merge. Otherwise refuse to merge more branches. | Git and SQLite cannot commit atomically. Retrying safely must preserve a merge Git already made without accepting unrelated or altered project state. | A single transactional store for Git refs and the journal, or evidence that Git can atomically commit both. |
