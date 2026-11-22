# 08 — The round machine

One page. This is the spec that the runner enforces. It replaces "an orchestrator that figures it out".

Status: draft, awaiting three choices at the bottom. Sources: `07-decision.md` D4–D5, `00-synthesis.md` §5, `06-wedge.md`.

---

## What it is

A **goal run**: a user states a goal and a done spec; one orchestrator agent plans and delegates; workers
run in their own branch; every handoff is a recorded round; the runner decides when to stop.

Two job templates share this engine and differ only in their done spec and their default roles:
**research** (gather → compare → synthesize → report) and **feature** (build → wire → review → merge).

---

## Five objects

| Object | What it holds | Lives for |
|---|---|---|
| **Goal** | text, done spec, budget, owner | the whole run |
| **Run** | one attempt at a goal: journal, rounds, status | until accepted or blocked |
| **Round** | index, plan, dispatches, results, verdict, spend | one turn of the loop |
| **Thread** | one agent session: provider instance, role, branch, worktree | may span many rounds |
| **Ledger entry** | `kind` (intent · finding · decision · constraint · question), `scope` (paths/urls), `thread`, `round`, text, evidence pointer | forever, append-only |

The ledger is the answer to "everyone knows what everyone is doing". Not a group chat: a board every
thread can read, plus messages that only *wake* someone up. Chat is lossy and ordered; a board is
queryable and survives a context wipe.

**Message policy is a run setting, not a hard rule.** The runner supports three values and the user
picks one per goal:

| Value | What it means | Cost |
|---|---|---|
| `anytime` | any thread may message any other thread, any time | highest: every message is a paid turn and can derail work in progress |
| `round` *(default)* | a worker may message the orchestrator, and a peer only inside a round they share | low, predictable |
| `never` | no agent-to-agent messages; the board is the only channel | lowest, quietest, deepest round boundaries |

The board is readable by all threads at all times in every mode, so `never` loses nothing except
immediacy. The setting is stored on the goal, shown at creation, changeable between rounds. Widening or
narrowing it is a config change; the loop does not change.

---

## State machine (enforced by the runner, not by the prompt)

```
planned → dispatched → working → reported → reviewed → accepted → merged
                    ↘ blocked(reason)   ↘ budget_exhausted
                    ↘ no_progress       ↘ human_paused
```

Rules:

- A round cannot start until the previous one has a verdict.
- A worker cannot be dispatched without a ledger `intent` naming the files it will touch.
- A review cannot be `pass` while the diff touches files outside the round's declared scope.
- `accepted` requires both a machine check (done spec) and the human gate, unless auto-accept is on.
- `merged` is its own state: acceptance is not permission to land.
- Every transition is a journal row. A crash resumes at the last committed row; nothing is inferred.

---

## Rights are asymmetric, deliberately

| Role | Read | Write | Message | Branch |
|---|---|---|---|---|
| Orchestrator | whole repo, ledger | none | everyone | none |
| Worker | committed tree + ledger | own worktree only | orchestrator + peers, rate-limited | own |
| Reviewer | whole repo, any diff | comments only | orchestrator | none |
| Merger | whole repo | run branch | orchestrator | run branch |

Why: five agents with equal rights produce n² messages and two agents editing one file. The evidence is
blunt — orca ships no locks and no ownership signal, t3's merge-back moves context and not code,
cloudroom says "share work through Git". Asymmetry is the cheapest correction: workers can *read*
everything committed and *see* every intent, but can only write their own branch.

---

## Branch model

- One branch per thread: `run/<run-id>/<slug>`. One worktree per thread. Commits only, never push.
- The run branch is assembled by the **merger step**, and only from threads whose round verdict is
  `accept`.
- `main` is touched only by the user, or by a merge step after acceptance. Never by a worker.
- Merge conflicts are **not** auto-resolved. The round goes `blocked(conflict)` and the user sees both
  diffs. The research found no product that solves this, and a plausible auto-resolver is worse than an
  honest stop.

Known cost, stated in the references: an isolated worker cannot see another's uncommitted work. We
compensate with the ledger (`intent` rows) and published diffs — not with shared folders.

## Merge is a decision, not a step

The sequence at the end of a run:

1. The reviewer reports. Findings are fixed in the worker's branch, or explicitly declined with a
   reason. The loop continues until the reviewer has nothing blocking.
2. The runner assembles a **readiness report** for the round: the machine checks that passed, the checks
   that were skipped and why, the diffs per branch, and the reviewer's verdict.
3. A human reads that report and decides. The decision is recorded with an actor and a timestamp. The
   runner offers merge **only** when the readiness report is complete — a missing check is shown, not
   hidden.
4. The merge itself may be executed by the agent, on the human's instruction, because agents have a
   terminal. The runner wraps that command so the action is journaled either way. Who decided and who
   executed are two separate fields; "the agent merged it" is never an excuse for a merge nobody asked
   for.

So: no autonomous merge, and no requirement that the human click. The default is a gate; the escape
hatch is an explicit instruction.

---

## Done specs

**The deliverable is a parameter, not a template.** The user says what they want in the goal itself —
report, PDF, HTML, slides, a diff, a test suite — and the orchestrator turns that into the `deliverable`
block. Format changes the checks; it never changes the loop.

```json
{ "deliverable": { "format": ["html", "pdf"], "audience": "me",
                   "length": "one page per section", "must_cite": true } }
```

Consequences: a PDF deliverable is checked for existence, page count and that every referenced fixture
resolves, while the *content* checks stay identical. A markdown-only run is the same engine with one
line changed. Anything the machine cannot check falls back to the human gate — which is honest, and
still terminates.

**Research run**

```json
{ "type": "research",
  "checks": [
    "every ## section in the report resolves ≥1 citation",
    "zero 'TODO' / '[citation needed]' / 'TBD' markers",
    "every ledger finding is cited in the report or explicitly rejected",
    "no two findings contradict without a recorded decision"
  ],
  "human_gate": "accept the report",
  "max_rounds": 3 }
```

The last check is the one that makes it a synthesis rather than a pile: contradictions must be *resolved*
or *recorded*, not averaged.

**Feature run**

```json
{ "type": "feature",
  "checks": [
    "named test command exits 0",
    "reviewer verdict ∈ {pass, pass_with_notes}",
    "diff ⊆ declared paths",
    "no new compiler warnings in touched files"
  ],
  "human_gate": "accept the diff",
  "max_rounds": 5 }
```

---

## Stop rules (from D5, now mechanical)

| Trigger | Action |
|---|---|
| rounds > `max_rounds` | `blocked(rounds)`, hand the journal to the human |
| spend > budget | `budget_exhausted`, freeze threads, keep branches |
| identical failing check twice | `no_progress` |
| no diff and no ledger entry in a round | `no_progress` |
| reviewer blocks twice on the same finding | `blocked(stuck)`, escalate |
| human says stop | `human_paused`, resumable |

Every one of these keeps the work. None deletes a branch.

---

## What the runner refuses, by construction

1. **Merging without a verdict.** No path from a worker's branch to `main` that skips a review row.
2. **Unbounded rounds.** `max_rounds` is a number in a column, not advice in a prompt.
3. **Silent context loss.** A handoff is a ledger row with `included` and `dropped` lists. Trimming is
   allowed; hiding the trim is not.
4. **Worker-created worktrees.** Checkout creation belongs to the runner (orca learned this: agents
   making their own checkouts causes the `config.lock` race).
5. **A plan with no scopes.** Every dispatch names the paths or sources it will touch.

---

## Settled in v1

- **Merge:** human decides; agent may execute on instruction; both recorded.
- **Message policy:** a per-run setting — `anytime`, `round` (default), `never`.
- **Deliverable:** user-specified per goal, defaulting per template.

## Still open

1. **Budget per run.** Proposed default until you say otherwise: **3 rounds, 150k tokens, stop on the
   first no-progress round**. At current prices that is cents per run, and it is a column in the goal,
   not advice in a prompt.
2. **The first goal to run for real.** Two candidates below; pick one and we start.
   - *research:* "summarise these three papers into one cited page, as HTML" — exercises the ledger,
     citations and the human gate.
   - *feature:* "add one endpoint with one passing test in a scratch repo" — exercises branches,
     review, readiness report and merge.

## Not in this spec, on purpose

Scheduling, cron, cloud execution, multi-user, more than five threads, and any automatic conflict
resolution.
