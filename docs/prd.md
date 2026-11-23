# PRD — Ilvuno v1

One page of product, one page of flow. Engine details live in `research/08-round-machine.md`; decisions and
their reversal conditions live in `research/07-decision.md`. This document does not repeat either.

---

## 1. The product in one paragraph

A desktop app where a developer states a goal, assigns a small team of coding agents, and lets them run
in rounds until the work is done. Every handoff between agents is a recorded artifact, so context is not
lost when work moves from one agent to the next. The human approves at the end, or at each round.

**The claim we must earn:** *you stop being the messenger between your agents.*

## 2. Who it is for

**Primary: the developer already running two or more agents on one project.** Evidenced in the research:
they run Claude Code and Codex side by side, hand-carry diffs, and paste constraints back and forth
(`research/06-wedge.md` §1). They already pay $100–400/month for subscriptions. Their pain is not "one
more chat pane" — it is that a handoff loses the reason for the work.

**Secondary: the small team that has to show review evidence.** They pay $24–72 per seat per month for
AI review that never sees the conversation that produced the code.

Not the user in v1: enterprises, non-coders, anyone expecting a hosted service.

## 3. What the app is

**An interface with abilities, not a workflow with roles.**

The app provides six things and decides nothing else:

| Ability | What it means |
|---|---|
| Threads | any number, each on any agent the user has configured |
| A loop | turns run, the user or an orchestrator decides what happens next, a budget stops it |
| A shared board | every thread reads it; intents, findings, questions, decisions |
| Messages | a thread can reach another, within the policy the user picked |
| Isolation | each thread works in its own copy of the workspace, on its own branch |
| A gate | nothing lands until a human accepts, and the whole run replays from a record |

**No roles. No modes.** There is no "research mode" and no "coding mode". There is no
"this agent is for review". A thread is an agent plus a prompt. What the threads do is whatever the user
asks for. The same agent can build on Monday and review on Tuesday, in the same project, without any
setting changing.

**The orchestrator is a thread, not a feature.** If the user starts one, it is an ordinary thread with a
system prompt that helps it plan and ask questions. It may create tasks for other threads. It has no
special authority: the same budget, the same gate, the same record. A user who does not want one simply
does not start one and prompts each thread by hand.

**Research and coding are examples, not product surface.** "Summarise these papers" and "write this
feature and have another agent review it" are prompts a user writes. They are useful as example files;
they are not modes in the engine, and the engine must not branch on them.

## 4. What a task is

A task is the unit of work the loop moves: a prompt, a target thread, and a declared scope (files or
sources it may touch). Tasks come from two places, and both are normal:

1. **The user** writes a task and points it at a thread.
2. **An orchestrator thread** creates tasks as it plans, using the same command the user uses.

The runner enforces only bounds, never content: how many tasks per round, the budget, the scope, and
whether the thread's agent is configured. It does not know or care whether a task is research, code,
review or a haiku.

"What done means" is written by the user for the run: free text plus optional machine checks (a command
that exits 0, a file that exists, a citation that resolves). If only free text is given, the human gate is
the judge.

## 5. The flow the UI must support

1. **Project.** Pick a folder and the agents you have (each an instance of a CLI you already use).
2. **Threads.** Add threads: name, agent, and a prompt. Or add one orchestrator thread and let it plan.
3. **Tasks.** The user or the orchestrator creates them: prompt + thread + scope.
4. **Watch.** The board is the default view — intents, findings, questions, decisions, with who wrote
   each. Per-thread transcripts behind it.
5. **Gate.** Readiness: what ran, what passed, what was skipped and why, diffs per branch. Accept, or
   send back with a note.
6. **Merge.** Only after acceptance. The human decides; the runner executes, or the agent does on
   explicit instruction. Both actors recorded.
7. **Recover.** Park and resume; a crash resumes at the last journal row.

Screens in v1: projects, project (board + threads + transcripts), gate, settings (agents, defaults,
budget). Nothing else.

## 6. Non-goals for v1

Cloud execution, teleport, remote runners, mobile, embedded browser, computer use, org sharing,
marketplace, scheduler/cron, merge arbitration, PTY-only agents, more than five concurrent threads,
multi-user.

## 7. What v0 must prove before any UI exists

Headless, one command:

```
ilvuno run --goal goal.json
```

Acceptance:

1. A real paper is summarised into a page where **every section cites a source that resolves**.
2. The journal replays the run: same rounds, same dispatches, same verdicts.
3. It refuses a fourth round, and says why, without deleting branches.
4. Killing the process mid-round and restarting resumes from the last committed row, not from zero.

If all four hold, the UI is a view. If not, the engine is wrong and no interface will save it.

## 8. How we know it worked

First ten users, one number: **runs that reach `accepted` without the user sending a manual message
between agents.** Below half, the loop is decoration and we go back to the review-only product.

## 9. Open product questions

1. Do terms of service allow driving these subscriptions inside a product? (Pre-launch, see
   `research/06-wedge.md` §7.)
2. Does a research run actually cost cents, or does the orchestration overhead make it dollars? Measure
   on the first real run.
3. Is the board readable by a human, or is it a log that only agents read? Watch the first three testers.
