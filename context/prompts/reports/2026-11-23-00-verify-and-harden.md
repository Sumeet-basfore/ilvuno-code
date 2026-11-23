# 00 · Verify and harden — 2026-11-23

## What changed

`499c4b6` — the fixes this audit found:

- The gate button and palette said **Accept and merge**. Accepting records `accepted`; no merge exists
  yet. Both now say **Accept**.
- A run at the gate said *"This run is reviewed. Start a new run to continue."* It now says the gate is
  open and to accept or send back.
- A spent token budget offered **Resume** in the composer and the palette. D14 says a new run with a
  larger budget is the way forward; the copy now says that, and the palette no longer offers Resume.
- Shortened run paths lost their slash: `.demo-homerun/artifacts/brief.html`. Now `.demo-home/run/...`.
- The run picker and the segmented control (theme, message policy) used the decorative `--line` border
  (1.2:1). They now use `--control-line` (3.2:1 or better); the picker's hover no longer lowers the border.
- `tools/shots.sh`: the board step clicked the top bar, so `board.png` was the thread view; the
  conversation step re-shot the same view; the agent list was captured while still loading (the first
  `/agents` probe took **5.36 s** on this machine). It now opens the board from **View all**, returns with
  Escape, selects reader-1 for the conversation and returns to orchestrator, warms the agent cache and
  waits 7 s for the list, and blurs the last clicked button before the settings and offline shots. It
  also refuses to start when the runner is not answering.
- `README.md`: a goal file is supported; the sentence saying it was not is gone.
- Nine generated pages under `context/prompts/` are no longer tracked; `context/**/*.html` is ignored.
  `gate.png` was re-shot from the current run, and `offline.png` is now tracked.

## What I verified

- **Suite:** `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` → **141 passed, 0 failed, 3 ignored**
  (the live provider tests). Exit 0.
- **Screenshots:** all 20 images in `crates/desktop/shots/` and `crates/desktop/shots/states/` were read.
  The regenerated set was read again after the fixes. `threads.png` and `conversation.png` now differ
  (orchestrator and reader-1); `board.png` is the board; `settings-agents.png` lists the detected agents;
  `palette.png` says Accept; `gate.png` shows the current gate with Accept.
- **Widths:** 1440, 1280, 1060 and 960 at 900 high. No clipped text and no horizontal scrollbar. The run
  title and the rail select ellipsize. At 1280 the work rail hides and the panel toggle appears; at 1060
  and 960 the "Gate open · decide" button wraps to two lines but stays complete.
- **Contrast** (WCAG, computed from the tokens in `style.css`; threshold 4.5:1 for text, 3:1 for controls):

  | Pair | Dark | Light |
  |---|---|---|
  | `--ink` on `--bg` | 15.56:1 | 15.11:1 |
  | `--ink-2` on `--bg` | 7.48:1 | 7.41:1 |
  | `--ink-3` on `--raised` (worst text) | 4.61:1 | 6.23:1 |
  | `--on-accent` on `--accent` | 5.63:1 | 6.09:1 |
  | `--green` / `--green-weak` | 5.85:1 | 5.82:1 |
  | `--red` / `--red-weak` | 5.27:1 | 4.93:1 |
  | `--amber` / `--amber-weak` | 6.93:1 | 6.27:1 |
  | `--accent` / `--accent-weak` | 4.82:1 | 5.06:1 |
  | `--control-line` on `--pane` (worst control) | 3.47:1 | 3.23:1 |
  | `--accent` on `--bg` (focus / primary button) | 5.55:1 | 5.49:1 |

  `--line-2` on the pane is 1.48–1.52:1. It draws decorative chip and card edges, not a control.
- **Engine:** `ilvuno replay fad10b99-…` → `projection: MATCH (2 rounds, 1 refusals)`.
  `ilvuno export fad10b99-… --out /tmp/audit-export` wrote `run.json`, `board.md`, `journal.jsonl` and
  `artifacts/` (three files).
- **Documents:** 18 relative links in `context/`, `docs/` and `research/*.md`; **0 broken**.
  `git ls-files | grep '\.html$'` now shows only the app page, the two mock templates and the two fixture
  artifacts. `context/timeline.md` gained the three commits that landed after its first table.
- **Window log:** a fresh 1440×900 session, used for about a minute (threads, palette, settings, agents,
  board, both themes), wrote only the GTK font-key warning and the Mesa DRI3 warnings. No `ReferenceError`,
  `TypeError`, `Uncaught` or `SyntaxError` in `/tmp/desktop.log`.

## Could not verify

- **SIGKILL honesty is still broken.** Kill `ilvunod` mid-run, restart it, and the run still says
  `working`. Test run `6ae1b4cc-…` in `/tmp/audit-kill-home2` was killed 6 s after creation and still had
  `status=working` in the database; a restart then answered `run working`, `tasks []`, `rounds []`. There is
  no startup sweep or re-attach in `crates/api/src/drive.rs` or `main.rs`. The window would show this as
  *waiting* (`runIsWaiting`), but sending a message writes a task and starts no drive. Goal 05 has to own
  the fix. The test goal had no orchestrator, so no dispatch was running when the kill landed; a kill in
  the middle of a provider turn was not exercised.
- **JavaScript console errors are not forwarded to `/tmp/desktop.log`.** The log only carries GTK and Mesa
  warnings. The absence of errors in the log is true but weak evidence; a console-only error would be
  invisible.
- The 3 ignored tests are live Claude and ACP tests; they were not run (they spend quota).
- `agent-picker.png`, `stopped.png`, `states/budget.png`, `states/paused.png` and `states/waiting.png` are
  historical (18:50–19:48) and their runs no longer exist. `agent-picker.png` shows **WORKING** with no
  tasks; the current window maps that state to **WAITING**. They were not regenerated.
- `states/offline.png` and `offline.png` are two shots of the same state at different times; the root one
  is current.
- A real new agent run was not made for this audit. Everything engine-side used the accepted demo run
  (`fad10b99-…`) or the two crash-test runs, which never dispatched.

## Findings not fixed

- `Cargo.toml` says `license = "UNLICENSED"` while `LICENSE` is Apache-2.0 and D20 chose Apache-2.0.
  Metadata only; needs the owner's word, so it was left alone.
- `docs/ui-checklist.md` still says M5 (tool activity) needs a provider-contract change. R4 records that
  this is wrong: `tool.call` and `permission.decided` are already journaled. The same file says "130 tests
  today"; the count is 141.
- A thread header says "2 tasks in this run", while the task drawer says "#4". The header counts tasks on
  that thread; the drawer numbers the run-wide list. Both are true; the wording is confusing.
- "Ran for" keeps counting at the gate and while paused (9 h 37 m on the demo run). Known limit in
  `context/risks.md`.
- `states/budget.png` still shows "Resume it to continue"; the current window says a new run is the way
  forward. The screenshot is historical.
- Decorative `--line-2` borders remain at 1.5:1 (chips, cards). Not control boundaries.

## Still open

Goal 05 (crash truth), O1 (tool activity), O2 (stage 2 through accept/merge), O3 (history rewrite). This
audit made no decision on any of them.
