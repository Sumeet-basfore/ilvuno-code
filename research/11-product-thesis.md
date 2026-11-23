# 11 — Product thesis

One page. This is the argument we both sign off before any more code. The spec lives in `docs/prd.md`;
this is the *why*, and the places where I think the current framing is wrong.

---

## The one sentence

**Several AI agents work on one project, and the project remembers what each of them did.**

The app is not an agent. It does not decide the work. It is the room they work in, the page they write
on, and the door you open before anything ships.

## The problem, in the user's words

You run two agents. One writes. One checks. You become the messenger: copy the diff, paste the
constraint, remember what the first one tried. When you change agent mid-project, the new one starts
blind.

That is the pain. Not "I need another chat window". Not "I need more models". **The context dies whenever
work moves.**

## What we sell

Not agents — you already pay for those. We sell the three things agents don't do for each other:

1. **A shared memory of the project** (the board). Every intent, finding, decision and open question, in
   one place, written by the agents, readable by you.
2. **A safe place to run them** (isolation). Each thread in its own copy and branch. Nothing touches
   `main` without you.
3. **A gate** (the record). Nothing lands until you accept, and every step is replayable.

If those three are good, the app is worth using with any agent you like. If they are bad, no amount of
model choice saves it.

## The one opinion we ship — and a disagreement with you

You said: no roles, no modes, the user decides everything, agents just follow the prompt. I agree with
the rule and I think "no opinions at all" is a trap.

With zero shipped text, the first run is mush. A new user types "build my app" into five threads and gets
five monologues. That is not freedom, it is a blank page.

**What I propose instead of modes:** three short texts, visible and editable, never hidden:

| Text | What it says | Can the user edit it |
|---|---|---|
| Worker contract | read the board first; say what you will touch before touching it; report once; ask instead of guessing | yes |
| Orchestrator contract | plan; make tasks with a scope; ask the human; do not write the deliverable yourself | yes |
| Done question | "what does finished look like for this run?" — one free-text answer, plus optional checks | yes |

These are not modes. There is no "research mode". The engine never branches on them. They are defaults a
user can delete, which is the difference between a helpful start and a workflow we impose.

## Second disagreement: agents are not equally safe

"Any agent, any job" is right, but the safety is not equal, and hiding that would be dishonest:

- Claude asks us before every read and write. We approve or deny.
- Codex works freely inside its folder and only asks when it leaves. `/tmp` is not a wall for it.

So the same job is more or less controlled depending on the agent. The app must **show** each thread's
safety level, not pretend they match. That is a label on a thread, not a mode on the app.

## Third point: who moves the loop

Three candidates for "when does the next round start". I propose a rule that does not depend on roles:

1. The orchestrator thread says it is done and asks for the gate.
2. The runner refuses to open the gate if the checks the user wrote are not met.
3. The human accepts, sends back with a note, or stops.

If there is no orchestrator, the user presses "next round". Same rule, no special case.

## The first screen

**Board first, chat second.** The default view of a run is the board — who is doing what, what was found,
what is open. Transcripts are one click away.

That is the differentiator and it is also the risk: people expect chat. My argument for board-first is
that chat is exactly what every other product already gives them, and the board is the thing that makes
one project feel like one project.

## The one experiment that decides the build

Before the UI, run this by hand and read the output:

> Three threads on one small goal (one of them an orchestrator). One real task each.
> Question: **is the resulting board readable and useful to a human, without reading any transcript?**

If yes — the product exists, and everything else is polish.
If no — the board is a log for machines, the app is a nicer terminal, and we should stop and rethink
before writing more code.

## What we will not build

Modes, roles, a fixed plan, a scheduler, cloud execution, multi-user, an agent marketplace, a model of our
own. Each one would make the app decide something the user asked to decide.

## Agreement checklist

Tick these four, and I start building:

1. The one sentence: agents on one project, and the project remembers.
2. Ship the three texts as editable defaults, not as modes.
3. Show each thread's safety level instead of pretending they are equal.
4. Board is the default view of a run.

Plus the experiment: the board must be readable by a human, or we stop.

---

## Experiment result (added after the first run)

Run: `examples/board-experiment.json`, three threads (one orchestrator, two readers), all Claude, one goal.
Result: reached the gate in two rounds. Round 0 wrote the two notes files but no brief, so the checks
failed; the orchestrator was woken with the failing check, made one new task, and round 1 produced
`brief.html`. 13 board entries. About 350k tokens, roughly $0.12.

**Is the board readable without a transcript?** Yes, after three changes that the first version failed:

1. Paths were absolute (`/tmp/claude-1000/…/threads/reader-1/…`). Unreadable. Now relative to the run.
2. There was no header. Now the board opens with the goal, what done looks like, and the run's status.
3. Bodies were unbounded. Now trimmed to 400 characters; the full text stays in the journal.

Still weak: long task prompts dominate the board, and source paths outside the run stay absolute. A person
can follow who did what and why round 0 failed; they would not enjoy reading it. That is enough to build
on, not enough to ship.

**A real bug this experiment found:** the token budget was checked once per round, so one round of three
tasks could exceed it (300k planned, 368k spent). The build agent had raised the budget to 600k to make
the run pass — that is not a fix. The budget is now checked before every task in a round.

**And one mistake of mine:** I overwrote a newer `ledger.rs` with an older copy, losing the author field.
The board test caught it in the same minute. Small, but the reason tests exist.
