# Implementation Task Specifications

One file per task, `T-NNNN.md`. Every spec has the same nine fields so reviews are mechanical.
Canonical template: §1 below. Tasks are grouped by milestone; no task spans milestones.

## 1. Template

```
# T-NNNN — <title>

Milestone: Mx · Estimate: n d · Owner: A|B|either · Provisional: yes/no

## Goal
One paragraph. What is true after this task that was not true before.

## Dependencies
Task IDs, ADRs, requirement IDs. Anything that must exist first.

## Allowed file scope
Paths this task may create/modify. Everything else is out of scope and must not appear in the diff.

## Implementation outline
Ordered steps, 5–9 of them. Enough to start, not a script.

## Acceptance criteria
Table of AC-n → falsifiable statement.

## Test commands
Exact commands a reviewer runs.

## Evidence requirements
What must be attached to the PR / gate packet.

## Review checklist
Task-specific review points, on top of `docs/planning/07-definition-of-done.md` §11.
```

## 2. Index

| Task | Milestone | Title | Est. |
|---|---|---|---|
| T-0001 | M0 | Repo skeleton, toolchain, CI, import-rule harness | 1 d |
| T-0002 | M0 | Deterministic fixture forge | 2 d |
| T-0003 | M0 | Failure-class fixture corpus (ported SP-1R3 F1–F10; 9-class scope pending S-1) | 1 d |
| T-0004 | M0 | Safety fingerprint helper | 0.5 d |
| T-0005 | M0 | Reference-behaviour conformance (replaces E-1 merge-tree spike) | 0.5 d |
| T-0101 | M1 | gitx exec wrapper + version probe | 1.5 d |
| T-0102 | M1 | Repo discovery and refusals | 1 d |
| T-0103 | M1 | Config load/validate | 1 d |
| T-0104 | M1 | Pinning: refs → SHAs, merge-base, drift | 1.5 d |
| T-0105 | M1 | doctor, init, worktree discovery | 1.5 d |
| T-0201 | M2 | Journal v1 | 2 d |
| T-0202 | M2 | Policy gate + worktree lifecycle | 2 d |
| T-0203 | M2 | Determinism envelope | 1 d |
| T-0204 | M2 | merge-tree preflight (pairs + tuple) | 1.5 d |
| T-0205 | M2 | Candidate construction + `integrate --dry-run` | 2 d |
| T-0301 | M3 | Model + findings schema | 1.5 d |
| T-0302 | M3 | Overlap and proximity analysis | 1 d |
| T-0303 | M3 | Dependency and interface analysis | 1.5 d |
| T-0304 | M3 | Evidence store + redaction + bundle verify | 2 d |
| T-0305 | M3 | Report renderer, banned claims, bench harness | 2 d |
| T-0401 | M4 | Verification runner, supervision, env allowlist | 2 d |
| T-0402 | M4 | Five-state outcome, command evidence, exit codes | 1.5 d |
| T-0403 | M4 | Combination-only isolation | 1.5 d |
| T-0404 | M4 | `verify` command, tree integrity, `--runs` | 1 d |
| T-0501 | M5 | Uncertainty model, `resume`, `abort` | 2 d |
| T-0502 | M5 | Project lock, idempotency, integrate cache | 1 d |
| T-0503 | M5 | Crash harness and signal matrix | 2 d |
| T-0504 | M5 | Corruption handling, cleanup, orphan repair | 1.5 d |
| T-0601 | M6 | Passport v1 | 1.5 d |
| T-0602 | M6 | Approval gate + key init | 1.5 d |
| T-0603 | M6 | Repair loop (external command) | 1.5 d |
| T-0604 | M6 | explain, stats, brief | 1.5 d |
| T-0605 | M6 | CLI completion, JSON contract, docs, v0.1.0 | 2 d |
| T-7001 | M7 | Adapter interface + conformance harness (prov.) | sketch |
| T-7002 | M7 | Claude adapter (prov.) | sketch |

MVP total: 33 tasks, ~46 engineering days (≈ 9.2 weeks of focus, 13 weeks calendar with gates and
buffer). Estimates are effort, not duration; two agents halve effort only where waves allow
(`planning/03-dependency-graph.md` §4).

## 3. Conventions

- "Owner A/B" refers to the two-agent split in `planning/03-dependency-graph.md` §5.
- Every write-touching task needs a `T-SAFE-*` test.
- Every new error code needs a catalogue entry (`planning/07-definition-of-done.md` §4).
- A task that cannot meet its AC updates its own spec and says so; it does not silently shrink.
