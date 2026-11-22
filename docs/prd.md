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

## 3. Jobs to be done

| Job | Today | With Ilvuno |
|---|---|---|
| Summarise a body of sources into something cited | paste sources into one agent, re-paste context, hope | Goal + deliverable stated once; agents split the sources, findings land on a board, one report comes out with citations checked |
| Ship a feature with review | write with one agent, copy the diff into another, paste findings back | One round: build, review, fix, then a readiness report you approve |
| Know what the team did | scroll three transcripts | One journal: every round, every dispatch, every decision |

## 4. The two templates

**Research run.** Roles: orchestrator, 2–4 researchers, one synthesizer, one citation checker.
Loop: `plan → gather → compare → synthesize → verify → report`.
Done: every section cites at least one resolved source; no placeholders; every recorded finding is either
cited or explicitly rejected; contradictions are resolved or recorded as a decision. Then the human gate.

**Feature run.** Roles: orchestrator, builder, reviewer (different provider required), merger.
Loop: `plan → build → review → fix → verify → ready`.
Done: the named test command exits 0; reviewer verdict is not blocking; the diff touches only declared
paths; no new warnings in touched files. Then the human gate, then merge.

Deliverable format is a parameter in both: HTML, PDF, markdown, a diff, a test suite. Format changes the
machine checks, never the loop.

## 5. The flow the UI must support

1. **New run.** Five fields: goal, deliverable, done checks, message policy (`anytime | round | never`),
   budget (rounds and tokens). Templates prefill everything except the goal.
2. **Watch.** A board (ledger) and per-thread transcripts. Board is the default view: intents, findings,
   decisions, questions — with the thread that wrote each.
3. **Gate.** A readiness report: which checks passed, which were skipped and why, diffs per branch, the
   reviewer's verdict. Two buttons: accept, or send back with a note.
4. **Merge.** Only offered after acceptance. The human decides; the runner executes, or the agent does on
   explicit instruction. Both actors recorded.
5. **Recover.** A run can be parked and resumed. A crash resumes at the last journal row.

Screens in v1: run list, run detail (board + transcript tabs), gate review, settings (provider instances,
defaults, budget). Nothing else.

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
