# Prompts to run in a loop

Each file here is one **goal** for an agent loop. Paste the block as the goal, let it iterate, read the
report it leaves.

| Order | File | Goal | Status |
|---|---|---|---|
| — | [`00-verify-and-harden.md`](00-verify-and-harden.md) | prove the current build still works; fix regressions, add nothing | run any time |
| 1 | [`01-stage2-worktrees.md`](01-stage2-worktrees.md) | a thread works in a git worktree of the project, on its own branch | ready |
| 2 | [`02-stage2-gate-diff-and-merge.md`](02-stage2-gate-diff-and-merge.md) | the gate shows the diff; accepting merges it | done 2026-11-23 |
| 3 | [`03-tool-activity.md`](03-tool-activity.md) | the conversation shows what the agent actually ran | done 2026-11-24 |
| 4 | [`04-first-dogfood-change.md`](04-first-dogfood-change.md) | Ilvuno changes Ilvuno, once, for real | after 01 + 02 |
| 5 | [`05-truth-in-the-ui.md`](05-truth-in-the-ui.md) | budget bar, permission log, an honest reopen after a kill | after 02 |
| 6 | [`06-packaging.md`](06-packaging.md) | an installable Linux build | last |

Run them **in order**. 01 and 02 together are stage 2, and 04 cannot begin before both.

---

## The house rules — every prompt inherits these

Every prompt below starts with "read this file and follow the house rules". This is that file. Do not
copy these rules into a prompt; change them here.

### The project

`/home/sumeet/personal-repo/projects/ilvuno-second` — Ilvuno. A desktop app where several coding-agent CLIs
work on one project in recorded rounds, share a written board, and nothing lands until a person accepts it.
Rust engine (`crates/runner`), local HTTP API (`crates/api`, 127.0.0.1:4737), Tauri window (`crates/desktop`).

### Read before acting

1. `context/README.md`, then `context/story.md`, `context/decisions.md`, `context/features.md`,
   `context/risks.md` — the project's memory, including what was tried and refused.
2. `AGENTS.md` — the project rules.
3. Whatever the prompt names.

### Always

- `RUSTUP_TOOLCHAIN=1.100.0` on **every** cargo command. Plain `cargo` uses a broken toolchain on this
  machine.
- Work on a **new branch**. Commit after each verified milestone. Push the branch. **Do not merge to main
  and do not push main** — the human merges.
- The smallest change that fully works. No new dependency without asking. Match the surrounding code.
- Plain English in docs and UI copy: short sentences, everyday words. English is the reader's second
  language.
- Update `context/` as part of the work: a line in `timeline.md`, any decision in `decisions.md`, and — if
  you were wrong about something — an entry in `disagreements.md`. **Append, never rewrite.**

### Never

- force-push, `git reset --hard`, `git clean -fdx`, or delete a branch you did not create.
- `pkill` / `killall`. Kill only a PID you started.
- touch `.env`, credentials, or any secret.
- let an agent's shell touch `main`.
- add PTY agents, a scheduler, cloud or multi-user support (`docs/prd.md` §6).
- commit generated pages (`research/*.html`, `docs/*.html`, `context/*.html`, `proto/`, `mock/`).

### Delegation

Sub-agents are allowed **only** on model `gpt-6-luna` with `reasoningEffort: "xhigh"`. No other model, no
exceptions. If you cannot set exactly that, do the work yourself. Never delegate the final verification of a
milestone.

### Verification — a milestone is not done because it compiles

- `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` passes, with new tests for new behaviour.
- Anything visual: build and **look at the real window** (`tools/shot.sh out.png 1440 900`), then read the
  image. Never claim a layout works unseen.
- Anything about a run: a real run, or the scripted provider in the API tests. Never a mock written for the
  occasion.
- Report numbers, not adjectives: test counts, measured contrast, real token counts.

### Loop discipline

- Each iteration ends with a **commit** or a **written blocker**. No silent loops.
- If the goal is already met, stop and say so.
- If you cannot verify something, say exactly what you could not verify and stop. Do not claim it.

### The report you leave

A short file at `context/prompts/reports/<date>-<prompt>.md`: what changed (with commit hashes), what you
verified and how, what you could not verify, and what is still open. Five short sections. No prose.
