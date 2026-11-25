# Timeline

All dates are **2026-11-23** (IST) unless said otherwise. Times are commit times from `git log`, which is
the only clock this repository keeps. The day is 19 h 20 min long, 57 commits.

## Before this repository

The project had a first attempt: a Go CLI at `projects/ilvuno`, repository
`github.com/Sumeet-basfore/ilvuno-code`, Apache-2.0, with a readiness study, a git-floor experiment and two
open PRs. It reached an M0 harness and 1,632 lines. It is the source of the licence this repo carries and of
`research/14-git-floor.md`.

## The day

| Time | Phase | What landed |
|---|---|---|
| 01:36–01:47 | **Engine** | Specs and research imported. Runner with the state machine, the five invariants and the journal (`16b8362`); Claude adapter with the control protocol and a fake provider (`351883b`); dispatch, replay and gate (`c59b5ea`). |
| 09:30–09:56 | **Corrections from the user** | The board was unreachable for workers (`19726ee`), docs rewritten for a non-native reader (`7249f9c`), *agents are choices, not roles* (`7d11120`), per-slot providers (`e28ae2e`), ACP provider (`03d1d4b`), same-provider warning (`b1c0145`). |
| 10:13–10:37 | **Provider reality** | ACP tool output read from text blocks (`3144fa7`), three live goal variants (`256c688`), absolute binary path plus a Codex mode that asks (`699cc59`), the product thesis with three disagreements (`0916141`). |
| 11:24–12:48 | **Threads, export, API** | The research plan replaced by threads and tasks (`dca1aa7`), board readable and budget checked per task (`8ea0c3f`), export as files (`86f549a`), four board prototypes (`f20a387`, `96f6e1c`), CLI and API share one drive path (`b4f3bee`), `ilvunod` on 4737 (`e2bd1cf`), a `base` for HTTP goals (`c4e25fc`), `tools/demo.sh` and the README (`67d864d`), the roadmap (`26adad1`). |
| 13:05–13:26 | **Stage 0 window** | CORS opened to the window only (`6d932d8`), the window reads a real run (`b435557`), `tools/window.sh` (`230cd43`), a contrast fix (`41352e9`), CORS narrowed and JSON required on writes (`be6126c`), first dark shell (`ff86b04`). |
| 14:36–15:49 | **Two rejections** | v3 mock with the board leading (`dadfc23`), `/agents` and `/settings` so settings can be real (`5991e0e`), the v3 shell ported into `crates/desktop/ui` (`37270e3`), and the UI anatomy of Cursor, Codex and Vibe Kanban (`481d10c`). |
| 16:00–17:12 | **Thread-first** | A message becomes a task with no scope (`53e5d2e`), the thread-first layout (`cf3a083`), the UI completion checklist with what is *not* approved (`ab4e40e`). |
| 17:38–18:20 | **M1, M2, M6** | Run facts in the rail (`4c2adfe`), stop a run and fix resume (`40d1009`), control borders and a build that watches `ui/` (`9541b44`). |
| 18:50–19:24 | **Published** | Apache-2.0 licence and a README that matches the window (`1e81f2e`), git-floor research adopted (`dfe88d2`), agent picker per thread (`d36828b`). |
| 19:48–20:17 | **M7, M3, M8** | Honest stopped states and a waiting state (`d58ec15`), open a goal from a file (`d7b4a22`), a settings error belongs to its edit (`9e99bb8`). |
| 20:44–20:56 | **Repo and stage 2** | Roadmap says stage 1 is done (`417e371`), generated pages untracked, 13 MB → 7.2 MB (`a3ca060`), the git core with 5 tests (`37e6296`). |

## Pushes

Counted from the session record, not from git — git does not clock a push.

1. **The replacement.** `--force-with-lease` over the old Go history, deleting its `main` and its two stale
   branches; the open PRs closed themselves. Licence and README went in first.
2. **After M4** (agent picker), 3. **after the docs for M3/M7/M8**, 4. **after the roadmap correction**,
   5. **after the generated-page cleanup**, 6. **after the git core**.

Six pushes after the first. GitHub's events API only returned the old repository's pre-existing events
(Nov 19–22), so per-push timestamps cannot be recovered from here; the milestone batches above are the
honest record.

## Real runs, which cost tokens and time

| Run | What happened | Cost |
|---|---|---|
| two-reader board experiment | 2 rounds, 13 board entries, reached the gate on the 4th live attempt | ~35 min, 362k of 600k tokens |
| stop test | stopped 8 s into a 12-section writing task; the turn finished after the stop; round stayed open | 183k of 400k tokens |
| blocked test | one-word reply, a check that could not pass; round closed `block`, run left waiting | 63k tokens |
| budget test | two tasks, 1,000-token budget; stopped after one turn | 63,274 of 1,000 tokens |

## The shape of the day

The morning was the engine and the user's corrections. The afternoon was the window, rejected twice. The
evening was verification — and every one of the serious defects (resume, the stale build, the hidden menu,
"working" with nothing running) was found by looking at the real thing, not by reading the code.

## Stage 2 · worktrees

| Time | Phase | What landed |
|---|---|---|
| 23:09 | **Stage 2-A/B** | A clean git project gives every thread its own worktree and branch, with a commit after each task; folder mode is the fallback and journals why (`ff669cc`). |

## Stage 2 · gate diff and merge

| Time | Phase | What landed |
|---|---|---|
| 23:32 | **Stage 2-C** | The API returns a bounded diff per thread; the gate shows each branch, stat and patch. Accept checks for a clean project, merges checked-out branches in task order, and records each landing. Conflicts abort and block with file names. The scripted suite and a real throwaway-project run passed (`e430433`). |

## Stage 3 · tool activity (2026-11-24)

| Phase | What landed |
|---|---|
| **M5** | The API reads filtered journal history. The SSE stream sends bounded tool rows and dispatch IDs. The conversation shows a closed-by-default activity fold under each task, with keyboard access, safe targets, permissions and failures. Tests passed and the real window screenshots were read (`6648207`). Report: [`03-tool-activity`](prompts/reports/2026-11-24-03-tool-activity.md). |
| **PR review** | Review found that gate acceptance could miss uncommitted thread-worktree files. Acceptance now refuses dirty worktrees and reports their paths (`54c0004`); the workspace suite passed with 160 tests and 3 ignored. The README now says “coding agents” (`db7594b`). |

## Stage 5 · packaging prerequisite (2026-11-24)

| Phase | What happened |
|---|---|
| **Prompt 06 held** | Packaging was not started because prompt 04 is still at the human gate. The reviewed dogfood run has not been accepted or landed. See [`06-packaging`](prompts/reports/2026-11-24-06-packaging.md). |

## Stage 4 · first dogfood change landed (2026-11-24)

| Phase | What landed |
|---|---|
| **First dogfood run** | Human accepted run `64239160-0caa-456a-b3cf-3011a0f64bb1`. The runner merged writer commit `82486ce` to local `main` as `d03a026`; it adds one test for `branch_name` with spaces, slashes and Unicode. The post-accept workspace suite passed: 161 passed, 3 ignored, 0 failed. Report: [`04-first-dogfood-change`](prompts/reports/2026-11-24-04-first-dogfood-change.md). |

## Stage 5 · packaging attempt (2026-11-24)

| Phase | What happened |
|---|---|
| **Prompt 06 resumed** | The human accepted the prompt 04 run and the runner landed it as `d03a026`. Work resumed on `feat/linux-packaging`. |
| **Linux build** | Enabled `.deb` and AppImage bundling, added gem icons, a first-run guide and a packaging script. The window starts bundled `ilvunod` when the local health route is not healthy. Decision D24 records this choice. |
| **Package build** | `RUSTUP_TOOLCHAIN=1.100.0 cargo build --release -p desktop` passed. Tauri CLI 2.12.6 produced `Ilvuno_0.0.1_amd64.deb` and `Ilvuno_0.0.1_amd64.AppImage`. The `.deb` metadata and file layout were inspected. |
| **AppImage check stopped** | The AppImage was copied to `/tmp` and launched there. Its bundled runner returned the real run `64239160-0caa-456a-b3cf-3011a0f64bb1`; the 1440×900 window capture was blank. Prompt 06 requires stopping here. `dpkg-deb` is not installed on this CachyOS machine, so the `.deb` desktop entry was not launched. Report: [`06-packaging`](prompts/reports/2026-11-24-06-packaging.md). |

## Stage 5 · prompt 07 land-and-package (2026-11-24)

| Phase | What happened |
|---|---|
| **Packaging commit** | Commit `d4895f6` records the `.deb` and AppImage setup, bundled runner, first-run guide and prompt 06 evidence on `feat/linux-packaging`. |
| **Stop condition** | The previously captured AppImage window was blank while its bundled API returned the real run. Prompt 07 says to stop here. No branch was pushed during this task and no PR was opened. See [`07-land-and-package`](prompts/reports/2026-11-24-07-land-and-package.md). |

## Stage 5 · package display follow-up (2026-11-24)

| Phase | What happened |
|---|---|
| **AppImage** | The old blank capture came from Xvfb, which logged that DRI3 was unavailable. Running the AppImage on the real X11 display showed the window and run `64239160-0caa-456a-b3cf-3011a0f64bb1`. Screenshot: [`linux-appimage`](../crates/desktop/shots/2026-11-24-linux-appimage.png). |
| **`.deb`** | Extracted the package payload into a fresh `/tmp` root. The packaged CLI returned `ilvuno 0.0.1`; the desktop entry's `Exec=desktop` command opened the window and read the same run. Screenshot: [`linux-deb`](../crates/desktop/shots/2026-11-24-linux-deb.png). |
| **Tests** | `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` exited 0. Its output was truncated by the tool, so no aggregate test count is recorded. |

## Stage 5 · About version check (2026-11-24)

The AppImage About tab displayed `0.0.1`, matching the extracted `.deb` CLI. Screenshot:
[`linux-about`](../crates/desktop/shots/2026-11-24-linux-about.png).

## Stage 5 · packaging PR (2026-11-24)

PR [#5](https://github.com/Sumeet-basfore/ilvuno-code/pull/5) is open from `feat/linux-packaging` to
`main`. It contains the packaging code and the later display verification. The review and merge are pending.

## Stage 5 · packaging review (2026-11-24)

The Rust, UI and comment reviews of PR #5 found no actionable issues. The PR is mergeable. CodeRabbit
skipped its optional review, and no required checks are configured.

## Stage 5 · packaging merged (2026-11-24)

PR [#5](https://github.com/Sumeet-basfore/ilvuno-code/pull/5) merged into `main` as `6af44e8`. The
packaging implementation, verification report and screenshots are now in `main`.

## Stage 6 · checks that see the work (2026-11-25)

| Phase | What landed |
|---|---|
| **Round check tree** | Round checks build a temporary worktree from the run base and the checked-out thread branches, in acceptance order. Unstarted planned branches are skipped. Conflicts and tree-build failures block the round without running checks against the project folder. |
| **Coverage** | The scripted loop suite passed 13 tests. The integration-tree unit tests passed 2 tests. The workspace suite is pending. See [`08-checks-see-the-work`](prompts/reports/2026-11-25-08-checks-see-the-work.md). |

The single workspace run passed: **172 passed, 3 ignored, 0 failed**. The ignored tests use live provider
accounts. Code commit: `9a95210`. The branch push is pending.

The verified branch was pushed to `origin/feat/checks-see-the-work`.

## Stage 6 review correction (2026-11-25)

Code review found that the check tree started at `git_base`, while acceptance merges into the project's
current HEAD. Round close now blocks if HEAD moved after run creation. Gate acceptance checks the same base
again, so a project change while a person reviews the diff is refused. Two scripted tests cover these
states. The roadmap now describes the integration-tree behavior and the remaining stage 3 and 5 work.

A second review found a partial-accept retry could also land a later branch on changed project code. Each
merge step now checks the prior commit recorded in the journal. A scripted test covers that retry.

## Harness · T3 Code to Orca (2026-11-25)

The owner moved the work harness from T3 Code to Orca, and may move it again. The reason given is a worry
about how T3 Code handles the data of the agents running in it. That was not verified.

Checked before recording it: `rg -i "t3 ?code"` over the tree returns only research and context prose about
the reference products; no build file, config or code path depends on the harness. The delegation lines in
the project `AGENTS.md` name Orca workers, and the prompt briefs name no harness at all.

Nothing in `crates/`, the release or the open pull requests changed.

## Stage 6 follow-up · merge journal recovery (2026-11-25)

Git and SQLite cannot commit atomically. Acceptance retries now recognize an unjournaled Git merge only when
its parents, message and tree match the expected merge. It records that merge and continues. A different
tree or project movement refuses the remaining branches. Two scripted tests cover recovery and refusal.

## Prompt 09 · Paseo comparison (2026-11-25)

The five Paseo reviews are now summarized in `research/15-paseo.md`, which names the target commit and cites
the upstream files reviewed. The summary says nine native providers, while the pinned provider docs list
ten. The page records that difference and preserves the 25.1k star count as a dated snapshot. Its HTML
reading copy was generated and read, but is not committed.

## Prompt 09 · Claude stream replay (2026-11-25)

Added hand-written stream-json fixtures for a normal tool turn and a permission request. The replay test feeds
each frame through `provider_claude::wire::parse_frame` and checks the events, including an unknown frame as
`Raw`. `RUSTUP_TOOLCHAIN=1.100.0 cargo test -p provider-claude --test replay` passed: 2 passed, 0 failed.

## Prompt 09 · Credential boundary tests (2026-11-25)

Added an ACP test for `env -i`, an API test that binds through the production loopback address helper, and
three plain-English lines in the security model. The first test attempt hit a full `/tmp` filesystem while
compiling dependencies. Retrying with the existing `/home` build target passed: API 56 passed, ACP 19 passed,
1 ignored, 0 failed.
