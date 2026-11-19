# ADR-0011 — Repair is an external command with a bounded budget

Status: proposed. Date: Phase 3. Related: `INT-5xx`, product `A-6`.

## Context

When verification fails on a combination, something must attempt a fix. Options: bundle an agent,
call a provider API, delegate to a human only, or invoke a developer-configured command.

The product claim is bounded repair with a record — not autonomous fixing. Research `05` §2.10
shows every surveyed system's retry limit guards dispatch or a single PR, and none keeps a record
of failed attempts.

## Options

| Option | Autonomy | Provider coupling | Record quality |
|---|---|---|---|
| **Developer-configured command, brief on stdin, budget N** | the user's choice of tool | none | every attempt is evidence |
| Bundled agent (SDK) | fixed to us | high | good, but couples the product |
| Provider API call | fixed | high + credential custody (ADR-0010) | good |
| No repair (human only) | none | none | n/a — loses the bounded-loop value |
| Unbounded loop until green | high | — | rejected: never terminates on an unfixable failure |

## Decision

1. `repair` invokes `config.repair.command` — any executable — with a **deterministic brief** on
   stdin, `cwd` = a fresh worktree off the candidate.
2. Budget default 2, hard max 5. Exhaustion is a typed outcome with an escalation block.
3. Every attempt gets its own branch and worktree; every attempt's diff is archived even on
   failure. Sources are never modified.
4. A repair that produces no new commit counts as a failed attempt.
5. The brief contains no model-generated text: failing command, exit code, redacted output tail,
   the per-source "passed alone" comparison, and the constraint not to modify sources.

## Consequences

Positive: agent-neutral; no dependency; the user's own subscription/tooling is used; the record of
failed attempts is the deliverable, and it is what makes the loop honest.

Negative: the tool cannot show what the repair agent "thought" (no structured events), and quality
depends entirely on the user's command. Both are documented rather than papered over.

## Risks

| Risk | Mitigation |
|---|---|
| The command is a no-op and burns budget invisibly | no-commit attempts are recorded as failures with the exit code |
| The command rewrites source branches | sources are pinned SHAs; nothing we do can move them, and a moved ref is reported |
| The command makes things worse | each attempt is a separate branch; the candidate is untouched; re-verification decides |
| Budget exhaustion feels like a dead end | escalation block names the likely cause (interface mismatch vs flake) and a next action |
| Brief injection (agent-authored text in the brief) | brief fields are structured; the failing output is fenced and labelled untrusted; commands come from config (`09` §4.2) |

## Revisit conditions

1. Users report repair rarely succeeds → measure success-per-attempt; consider removing the
   feature rather than adding autonomy.
2. A structured adapter exists (ADR-0010) → allow `repair.adapter = "claude"` as a *configuration*
   of the same feature, not a new pipeline.
3. Multi-attempt loops with state carry-over are requested → still bounded, and the record grows.
