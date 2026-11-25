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

## Current state update · 2026-11-24 · packaging review

The Rust, UI and comment reviews found no actionable issues. PR #5 is mergeable. Its only check is an
optional CodeRabbit job that skipped automated review for this repository. The `.deb` has not been
installed through a Debian package manager on this machine.

## Current state update · 2026-11-24 · packaging merged

PR [#5](https://github.com/Sumeet-basfore/ilvuno-code/pull/5) merged into `main` as `6af44e8`. The
AppImage and extracted `.deb` checks passed on the actual X11 display. A full `.deb` install through a
Debian package manager remains unverified.

## Current state update · 2026-11-24 · the window's type and colour pass

The owner asked for one pass over every screen of the window, with no new features: make it clearer, and
stop it looking like every other agent tool. The pass used heydamir's `unslop` skill, which is installed at
`~/.agents/skills/unslop`; the project's own `.agents/skills/unslop` is a different skill (pstack's prose
one) and stayed as it was.

What changed, and why it was owed:

- Fourteen type sizes became six. Fifty-five rules were set below 10px, so the smallest label sat at 8.5px.
  The floor is 11px now, body is 13, and anything a person reads for a while is 14.
- The caps went. Twenty-eight rules set `text-transform:uppercase` with wide tracking, so `DONE`, `BOARD`,
  `RUN`, `ASK FIRST` and `FINDING` all shouted at once. Everything is sentence case. The small monospace
  in the margin carries the tone instead.
- One accent, and green means one thing. The tag colours are gone: a kind is a word with a 6px dot in the
  margin, and `INTENT` keeps the accent because it is the mark a person made. Green is back to "this
  worked", so it no longer sits on a `finding`, which is only a note.
- Numbers stopped mixing formats: `1 of 2 rounds`, `197k of 300k tokens`, sizes in kB, and the run id is
  eight characters in both places that show it. `compact()` no longer prints `5,440` next to `600k`.
- A run that cannot take a message no longer offers a dead input box. The bar says why and carries the one
  move that works: Resume, or New run. Two screenshots of a merged run had shown an input nobody could use.
- The board's entries no longer repeat their round inside a round group, the rail stops repeating the whole
  board while the board is the centre view, and the reading measure is 64 characters on both.
- Settings: the agent rows showed `permissions` four times and `opencode opencode v2.3.2`. The chip is gone
  and the version keeps only its number. The Defaults note spoke in `PUT /settings`; it speaks in plain
  words now. Opening New run put the cursor in the JSON box, so a typed path became JSON; it lands in the
  file path now.

Evidence: `crates/desktop/shots/` was retaken from the window against run `64239160` by `tools/shots.sh`,
which now warns when two shots come out byte-identical. That check caught three broken shot steps and seven
missed clicks while this pass was being made. `cargo test --workspace` passed.

Not done, and known: `shots/gate.png`, `shots/stopped.png`, `shots/agent-picker.png` and `shots/states/*`
still show the earlier type, because each needs a run in that state (reviewed, stopped, paused, budget) and
those screens were not shot again. They are evidence of earlier milestones, not of today's window.

## Current state update · 2026-11-24 · three packages, not two

The builder made `.deb` and `.AppImage`. The owner asked for `.rpm` as well, and `rpmbuild` is already on
this machine, so `tools/package-linux.sh` now writes all three from one command into
`target/release/bundle/`.

Four things were wrong in the packages and are fixed:

- The workspace said `license = "UNLICENSED"` while the repo ships Apache-2.0, so the rpm metadata read
  `License: UNLICENSED`. It says Apache-2.0 now.
- The window installed itself as `/usr/bin/desktop`, with icons named `desktop.png`, because the Cargo
  binary is called `desktop`. `mainBinaryName` in `tauri.packaging.conf.json` names it `ilvuno-window`.
  That setting is in the packaging config only, so the dev build is still `target/debug/desktop` and the
  scripts did not change.
- Both packages described themselves as `(none)`, with no maintainer and no home page. They now carry a
  description from `crates/desktop/Cargo.toml`, the publisher, the repository URL and the long description.
- The script's closing list looked for the bundles in the wrong folder after `cd crates/desktop`.

What was checked, and how: the AppImage was started on the X11 display, its bundled
`/tmp/.mount_*/usr/lib/Ilvuno/ilvunod` answered `/health`, and its window drew the real accepted run. Both
the `.deb` and the `.rpm` payloads were unpacked to `/tmp` and their `ilvuno 0.0.1` ran; each package's
`ilvunod` answered `/health` on its own port. Not checked: an actual `dpkg -i` (this host is Arch, no dpkg)
and a real `rpm -i` (needs root, and `rpm --root` needs a chroot). The rpm is unsigned.

## Current state update · 2026-11-24 · v0.0.1 released

The owner asked for the packages to go on GitHub, then asked for it to be done for them. Three pull
requests landed first, in order, each retargeted by hand as its base merged: #7 the README, #8 the UI
pass, #9 the packages. `main` is `e52a327`.

`tools/release.sh v0.0.1` built the three packages from that commit, wrote `target/release/v0.0.1/` with
a `SHA256SUMS`, tagged, and published https://github.com/Sumeet-basfore/ilvuno-code/releases/tag/v0.0.1
as the Latest release. Assets: the `.deb` (11.7 MB), the `.rpm` (11.7 MB), the `.AppImage` (116.9 MB), and
`SHA256SUMS`.

Checked after publishing, from a fresh download of the release itself: all three files match `SHA256SUMS`,
and the downloaded AppImage starts, mounts its bundled `ilvunod` from `/tmp/.mount_*/usr/lib/Ilvuno/`, and
answers `/health`. The tag points at `e52a327`, which is `origin/main`.

Two things held up the release that a person should know about. The working tree had an uncommitted
`## Factory` block in `AGENTS.md` that nobody in this session wrote, so it was stashed and restored
afterwards, still uncommitted. `.agents/skills/skills/` was a stray 724 MB copy of a skills store inside
the repo, untracked; it now sits at `/tmp/ilvuno-stray-skills` and can be deleted.

Known: the rpm is unsigned, so `dnf` warns about the key. The `.deb` has never been installed by a real
package manager, because this host is Arch.

## Current state update · 2026-11-24 · pause point

Stopped here, with a cold-start note at `/tmp/ilvuno-pause-resume.md`. `main` is `c60b4dd` on GitHub, and
one local commit sits on top with this note.

The question that ended the day: is Ilvuno ready to build itself? Not yet. It did once, in run `64239160`,
which added a test to `crates/runner/src/git.rs` and merged it. But a command check runs in the project
folder, not in a thread's worktree (`crates/runner/src/checks.rs`, `current_dir(at.goal_dir)`), so the
runner cannot test what an agent wrote. A person still runs the test.

Next: stage 6, with the design fork settled first. Worktrees are per thread and checks are per round, so
either every thread's worktree is checked, or the runner builds one integration worktree at round close and
checks that. The second matches accept, which merges the threads into one tree, and it is the tree the gate
should show.

## Current state · 2026-11-25 · stage 6

Prompt 08 adds a temporary integration worktree around the existing round checks. It uses the checked-out
thread branches in the same order as gate acceptance. It skips planned branches with no checkout. A
conflict or tree-build error blocks the round. Checks never fall back to the project folder. The workspace
suite is pending. The implementation is on `feat/checks-see-the-work`.

## Stage 6 verification · 2026-11-25

The one `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` run passed: **172 passed, 3 ignored, 0
failed**. The ignored tests require live provider credentials. The code commit is `9a95210`. The branch
push is pending.

The branch push completed to `origin/feat/checks-see-the-work` after verification.

## Stage 6 review correction · 2026-11-25

Review found that the integration tree used the run's starting commit while gate acceptance merges into the
project's current commit. The runner now blocks checks if the project moved before round close. The gate
also refuses acceptance if it moved after the checks passed. The run must start again so checks cover the
new project state. The two movement cases have scripted tests.

A second review found the same risk during a partial merge and retry. Each branch merge now checks the
expected project commit from the run base or the last recorded merge. The runner refuses the remaining
branches if the project moved. A scripted test covers this recovery path.
