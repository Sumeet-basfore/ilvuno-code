# Roadmap

Final plan, ordered. Each stage has one job, a test that says it is done, and a kill rule.
A stage changes only by editing this file with a reason. Effort figures are guesses, not promises.

North star: **build Ilvuno with Ilvuno.** The first run of that must be small, real, and land a commit.

The one metric: **runs that reach `accepted` without the human sending a message between agents.**
Below half, the loop is decoration.

---

## Where we are

Done: engine (threads, tasks, rounds, gate, budget), board, replay, export, two agent types
(Claude native, ACP/Codex), local API with live events, four board layouts, the window (threads on the
left, conversation in the middle, the run's work on the right), starting a run from a goal file, stopping
and resuming it, choosing a thread's agent, honest states, tests. A clean git project now gives each
thread its own worktree and branch, with a commit after every task; folder mode is the fallback.

Not done: the gate's diff and accept-merge (goal 02), so nothing can land yet. No packaging.

---

## The stages

| # | Stage | Job | Done when | Effort |
|---|---|---|---|---|
| 0 | ~~**Window reads**~~ **done** | a Tauri app showing a real run | open app → run list → run page (mix) with live data; accept a gate from the app — **verified on 2026-11-23** | done |
| 1 | ~~**Window starts**~~ **done** | create and drive a run without the CLI | start a goal from the app (paste or pick a file), watch rounds appear, stop it, resume it, change a thread's agent — **verified on 2026-11-23** | done |
| 2 | **Git** | threads work in real branches | each thread has its own branch and worktree; the runner commits, diffs and merges; `main` untouched until accept | 2–3 w |
| 3 | **Truth in the UI** | safety and cost are visible | each thread shows its safety level; permission log; budget bar; kill the app mid-run and reopen to an honest state | 1–2 w |
| 4 | **Dogfood** | Ilvuno builds Ilvuno | one small change in this repo is produced by a run, reviewed by another agent, merged by the runner, and the commit names the run | 1 w |
| 5 | **Installable** | someone else can use it | a signed Linux build with auto-update, crash log, and a first-run guide | 1–2 w |
| 6 | **Beta** | five to ten real users | the one metric measured, and two of the three known sharp edges fixed | 2–4 w |

Total to beta: roughly 2.5–4 months of solo work with agents. The range is honest; the variance is
mostly stage 2.

---

## Stage notes

### 0 · Window reads
Tauri 2, one route. Reads `ilvunod`: `/runs`, `/runs/:id`, `/runs/:id/events`. No writes except the gate.
The chosen layout (`proto/4-mix.html`) becomes the run page; the other three are deleted.

**Trap:** the Linux webview is 2.52 against Tauri's tested range. If it misbehaves, say so before working
around it.
**Kill rule:** if the board cannot be read in the window without a transcript, the layout is wrong, not
the app.

### 1 · Window starts
A form: project folder, goal, what done looks like, checks, agents, threads, budget. It defaults to one
orchestrator thread, which the user can delete.

**Trap:** detecting which agent CLIs exist and are logged in. A wrong guess shows a run that fails on
spawn. Detect, then let the user correct.
**Kill rule:** if starting a run needs a paragraph of instructions, the form is wrong.

### 2 · Git
Phase A/B is done: a thread on a clean git project works in `ILVUNO_HOME/worktrees/<run8>/<thread>` on
`run/<run8>/<thread>`, and the runner commits after each task. A non-repo, a dirty tree or old git falls
back to folder mode and journals why. Phase C (the gate's diff, accept-merge) is goal 02.

- One branch and one worktree per thread: `run/<run>/<thread>`.
- Workers commit; the runner never pushes.
- Diff per thread, shown in the run page.
- Merge is a runner action, only after `accepted`, journaled with who decided and who executed.
- Conflicts stop the round. No auto-resolver.

**Trap:** worktrees under a temp home instead of next to the repo break the user's editor and their
`git status`. Choose the location once, document it.
**Kill rule:** if a merge ever needs a manual `git` command to recover, stop and simplify.

### 3 · Truth in the UI
The app already knows what each agent is allowed to do; it must show it.

- Safety label per thread: "asks before every read and write" vs "works inside its folder, asks when it
  leaves". No pretending they are equal.
- Permission log, so a blocked action is visible, not mysterious.
- Budget bar, and what the run spent in tokens and dollars.
- Stop, pause, resume, and an honest `unknown` when we cannot tell.

**Kill rule:** if we cannot state a run's true state after a crash, the UI is lying and the stage is not
done.

### 4 · Dogfood
One real change in this repo, chosen small: a bug fix or a small feature with a test.

Claude writes, Codex reviews, the runner merges. The commit message includes the run id. If the run is
worse than working by hand, write down why before changing anything.

**Kill rule:** if two dogfood runs in a row need manual repair, the engine is not ready for stage 5.

### 5 · Installable
A Linux build a stranger can install. Auto-update, a crash log we can read, and a first-run screen that
detects their agents. macOS and Windows after, not during.

### 6 · Beta
Five to ten users, the metric measured weekly. The known sharp edges to fix, in order: no cost numbers
for ACP agents, threads that can read each other's working files, and the board's noise on long runs.

---

## Not on this roadmap

Cloud execution, teleport, remote runners, mobile, embedded browser, computer use, org sharing,
marketplace, roles or modes, a scheduler, PTY-only agents, merge conflict resolution, multi-user.

Each would make the app decide something the user asked to decide, or cost more than the product is
worth today.

---

## Parallel, not stages

- **Terms of service before charging anyone.** Which subscriptions may be driven inside a product is a
  legal question, not an engineering one. It does not block building; it blocks selling.
- **Docs stay true.** A change that contradicts a decision updates the decision in the same commit.

## How the plan changes

Only here, with a reason and a date. If a stage's kill rule fires, we stop and rewrite the stage — we do
not add a new one to work around it.
