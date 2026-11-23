# 10 — v0 acceptance

Evidence for the four claims in `docs/prd.md` §7. Run on this machine, after the build agent's M1–M3.
Nothing here is a projection of intent; each line was observed.

---

## What was run

```
ILVUNO_HOME=/tmp/ilvuno-accept ./target/debug/ilvuno run --goal examples/research.json
```

Three dispatches in one round — two researchers splitting `research/09-spike.md` by section, one
synthesizer — against the real `claude` CLI (2.1.329), with approvals answered by the runner's policy.

| Run | What it was | Result |
|---|---|---|
| `c5696041` | first real run | reached gate, 1 round, 212,594 tokens |
| `f7203389` | **SIGKILL 14 s in**, then `run --resume` | reached gate, 1 round, 161,014 tokens |
| `75463783` | after the worker-board fix | reached gate, 1 round, 213,689 tokens |

## The four checks

**1. A cited brief exists — pass.** `artifacts/rounds/r0/synthesizer/brief.html`, 2.5–2.8 KB, four
`<section>` elements. Each carries `<cite data-src="09-spike.md" data-section="N">`. The checker resolves
each citation against the source's real headings; a section with no resolving citation fails the round.

**2. Replay reproduces the run — pass.** `ilvuno replay <run>` prints the full transition list
(`run.created` → `round.planned` → `dispatch.queued` → `thread.spawned` → `prompt.injected{"version":
"worker-preamble/1"}` → `tool.call` → `permission.decided` → `dispatch.reported` → gate). It also checks
the projection against the journal and exits non-zero on mismatch.

**3. A fourth round is refused — pass in tests, not exercised on a real run.** Two tests cover it
(`invariant4_refuses_fourth_round_and_keeps_threads_and_artifacts`,
`fourth_round_is_refused_with_a_reason_and_work_is_kept`). Forcing it on a real run would mean three
failing rounds, which is a deliberate waste of quota. Stated as a known gap rather than implied.

**4. Kill mid-round, resume from the last committed row — pass.** SIGKILL during round 0 left
`researcher-a` in `running`. `run --resume` journaled `dispatch.requeued`, re-dispatched **the same
dispatch id** on a new thread, finished b and the synthesizer, and reached the gate. Reports already
committed were not overwritten.

## What the run taught us that no document did

**The preamble asked for something the policy forbade.** Workers were told to "announce your intent on
the board", but the policy permitted exactly one shell command (`ilvuno report`) and `board.md` lives
outside the worktree. The replay shows a worker trying four times — `echo`, `printf`, `Edit`, `ilvuno
--help` — and being refused each time, then writing the complaint into its own report:

> *"Could not post the intent entry to board.md because the board is outside this worktree and the ilvuno
> CLI only permits the report command."*

Fixed in `19726ee`: `ilvuno note --task --kind --body [--scope]`, scope bounded by the dispatch scope,
deduplicated by request id, allowed by the policy, and the preamble bumped to `worker-preamble/2`. On
the next run, three notes appeared and a worker reasoned about a sibling's board entry before deciding it
would not collide.

**The permission layer does real work.** Refused in one run: shell control syntax, chaining, an `Edit` to
`board.md`, `ilvuno --help`, and any command not addressed to the worker's own task id.

**Token accounting is the budget, and it is heavier than expected.** One three-dispatch run consumed
~213k tokens — mostly re-read context, not output. The example goal budgets 600k for three rounds, which
fits roughly two and a half rounds of this shape. Either raise the default or shrink per-turn context;
do not discover this in a fourth round.

**Skipped checks are reported, not hidden.** Two goal checks (`findings-cited-or-rejected`,
`no-uncorrected-contradictions`) are not implemented in v0. The gate prints them as `[skip]` with the
reason. A run can reach `reviewed` with skips visible; it cannot claim them as passes.

## Still open

| Item | State |
|---|---|
| ACP provider (M5) | not built. Claude-only today. |
| Dollar accounting per run | tokens only; `total_cost_usd` is read from the CLI but not persisted |
| Feature template | not built; research only |
| Fourth-round refusal on a live run | tests only, deliberately |
| UI | not started, by design (PRD §7) |
