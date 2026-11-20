# 04 — MVP Scope Reconciliation

Status: proposal. No approved document modified. Requirement IDs: `docs/product/02-prd.md`.
Task IDs: `docs/tasks/`.

---

## 1. The contradiction, stated exactly

Three documents assign three different scopes, and two requirements import obligations from deferred
milestones.

| Source | Says | Conflicts with |
|---|---|---|
| `planning/02-milestones.md:3` | "M0–M6 are the approved MVP scope" | `AGENTS.md:14` ("only M0"), roadmap ("awaiting M0 approval") |
| `product/05-mvp-scope.md` §4.1 | Full requirement set in scope, including manifests and the passport | `04.2` defers mobile, hosting, dispatch; `05 §4.3` says S1–S3 is the smallest release |
| `product/02-prd.md` §10.7 | `ORQ-102/103` are **P1** | §16 lists them as binding the MVP |
| `product/02-prd.md` §10.11 | `XDC-102` is **P0**, AC requires "three transports, one view" | Doc 10 defers every transport; product 21.2 rung 1 is "git only" |
| `tasks/T-0605.md` AC-1 | "all 12 PRD §26 criteria pass" | Several criteria depend on V2 features |

Consequences: estimates are unstable, M6 can be "complete" with P0 items missing, and a reviewer
cannot trace a requirement to a gate.

---

## 2. Resolution rule

One requirement, one priority, one milestone, one acceptance test. Where a requirement describes a
*property of a format* (portability) rather than a *transport implementation*, keep the property and
move the transport.

---

## 3. Proposed requirement → milestone matrix

### 3.1 V1 and V2

| Milestone set | Name | Question it answers |
|---|---|---|
| **V1** (M0–M4 + safety floor) | vertical slice | "Does combined verification find anything, with evidence I trust?" |
| **V2** (M5–M6) | continuity | "Can I run this weekly without losing state or trust?" |

`V1` is shippable as `v0.1.0-alpha` to design partners. `V2` is the public `v0.1.0`.

### 3.2 Disputed requirements, resolved

| Req | Current | Proposed | Milestone | Why |
|---|---|---|---|---|
| `ORQ-102` manifest input | P1, "binds MVP" | **P2** | v0.2 | Convenience for batches; a CLI with N `--source` flags tests the hypothesis |
| `ORQ-103` emit manifest | P1, "binds MVP" | **P2** | v0.2 | Only useful once manifests are read |
| `ORQ-101` no shared working tree | P0 (seam) | **P0** | M2 | Safety, not convenience |
| `ORQ-106` stable machine contract | P1 | **P0** | V1 (`T-0605a`) | Needed for CI use, gates, and any future host |
| `ORQ-107` plugin contract | P2 | **P2** | post-V2 | Unchanged |
| `XDC-101` offline core | P0 | **P0** | V1 | One egress test; cheap and load-bearing |
| `XDC-102` transport-agnostic view | P0, "three transports" | **P1**, AC = "same passport/evidence renders identically from a repo copy on a second machine (no transport)" | V2 | The *property* is testable today (`TSK-110`); the transports are deferred |
| `XDC-103` static-renderable | P0 | **P1**, AC = a static renderer produces the task view from files alone | V2 | Depends on `TSK-*` |
| `XDC-104` services optional | P0 | **P0** (trivially true while none exist) | V1 | Keep as a regression guard |
| `XDC-105` E2EE sync | P2 | **P2** | not scheduled | Unchanged |
| `INT-260` concurrent-change brief | P1 | **P2** | v0.2 | Requires either dispatch or manual injection to matter; measured later |
| `INT-207` interpretation non-blocking | P0 | **P0** (structure + tests; **no model call in V1**) | V1 | Auditability is free; the model is not needed |
| `TSK-101…113` passport | P0 | **P1** | V2 | Continuity, not the core claim. `TSK-112` (no transcripts) survives as a documentation assertion |
| `INT-501…506` repair | P0 | **P1** | V2 | Bounded repair is valuable and not needed to test the premise |
| `INT-601…605` approval | P0 | **split**: 601 (combined diff) and 604 (no push) in V1; 602/603/605 (gate, signed binding, stability) in V2 | V1/V2 | Reviewers need the diff immediately; the signed gate is team machinery |
| `EVD-106` portable bundle | P0 | **P0** | V1 | This *is* the evidence artifact |
| `EVD-108` cross-store timeline merge | P2 | **P2** | v0.3 | Unchanged |
| `REC-101…107` recovery | P0 | **P0**, but the *obligation* narrows (below) | V1/V2 | See §3.3 |
| `VER-104` flakiness double-run | P1 | **P1** | V1 | Cheap and prevents a known false-confidence path |
| `INT-306`/`INT-210` combination-only | P1 (P0 for the decision) | **P0** | V1 | The measurement instrument; without it V1 cannot answer its question |

### 3.3 Recovery obligations narrowed by the new Git design

Because candidate construction is now a pure function of pinned SHAs
(`02-git-integration-correction.md` §3), the `merge_outcome_unknown` uncertainty class disappears.
What remains:

| Req | V1 implementation | V2 |
|---|---|---|
| `REC-101` journaled state machine | yes | full matrix |
| `REC-102` no uncertain retry | yes (bundle/cleanup only) | — |
| `REC-103` `resume` | replaced by **recompute** (`integrate` re-runs the pure pipeline; same SHA) | `resume` with `--decide` for bundle/cleanup |
| `REC-104` `abort` | yes | — |
| `REC-105` idempotency | yes (`T-0502`, 1 d) | — |
| `REC-106` typed errors | yes | — |
| `REC-107` never delete user work | yes | — |

This is a genuine simplification, not a cut: a recomputable pipeline is safer than a resumable one.

### 3.4 Traceability mechanism

`make trace` (part of `T-0605`) generates `docs/planning/08-traceability.md`:

```
| requirement | priority | milestone | task | acceptance test | gate |
```

and fails CI when: a P0 has no in-scope task; a requirement appears with two priorities; a task
cites a requirement it does not cover; or a deferred requirement is referenced by an MVP gate.
Until `T-0605` exists, the matrix above is the manual source of truth and is checked by review.

---

## 4. Proposed milestone content (V1)

| Milestone | Tasks | Est. | Answer |
|---|---|---|---|
| M0 | `T-0001`(revised), `T-0002`, `T-0003`, `T-0004`, `T-0005`(redefined) | 5 d | Can we build deterministic fixtures and prove the pipeline? |
| M1 | `T-0101`…`T-0105` | 6.5 d | Can we read a repo safely and refuse the rest? |
| M2 | `T-0201`, `T-0202`(revised), `T-0203`, `T-0205`(absorbing `T-0204`) | 7 d | Is the candidate correct and reproducible? |
| M3 | `T-0301`…`T-0305` | 8 d | Is the report explainable and the bundle verifiable? |
| M4 | `T-0401`…`T-0404` | 6 d | Does combined verification find what per-branch misses? |
| M4.5 (safety floor) | `T-0502` (lock/idempotency), `T-0504` (cleanup/orphans), reduced crash matrix inside `T-0503a` | 3 d | Does it leave a mess? |
| **V1 release** | `T-0605a` (CLI complete for integrate/verify/show, JSON, docs, alpha packaging) | 2 d | Can a stranger run it? |
| **V1 total** | **25 tasks** | **~37.5 d** | |

V2 (M5 full + M6): `T-0501`, `T-0503`, `T-0601`…`T-0604`, `T-0605b` ≈ 8 tasks / ~13 d.

**Calendar check:** 37.5 engineering days ≈ 7.5 focused weeks; with gates, review, and one agent
assist, 10–11 weeks to `v0.1.0-alpha`, then 4–5 weeks to `v0.1.0`. The previous plan's 15 weeks for
the whole MVP is consistent with this split; what changes is that measurement starts ~5 weeks
earlier.

---

## 5. Gate changes required

| Gate | Current | Proposed |
|---|---|---|
| G0 | E-1 uses an invalid command | E-1′ = pipeline equivalence + Git floor matrix (`02` §9) |
| G2 | "preflight predicts, real merge confirms" | "pipeline output equals a reference `git merge --no-ff` run on the corpus (tree equality); conflict reported at its step" |
| G3 | bundle + report gates | unchanged, plus assurance-level wording (`03` §6) |
| G4 | "5 real repos; continue even if zero combination failures" | **instrument gate**: ≥20 real multi-source runs with recorded `combination_only` outcomes, a stated first reading, and **no market verdict**. The verdict moves to the measurement plan (`§6`) |
| G5 | full crash matrix | full matrix (V2) |
| G6 | 12 PRD criteria | V2 criteria; V1 has its own smaller acceptance list (`06` §5) |

Naming: to avoid two things called "G4" answering different questions, the market gate is renamed
**MG-1** and lives in the measurement plan.

---

## 6. Measurement plan skeleton (MED-01, MED-02)

To be written as `docs/planning/09-measurement-plan.md` once an owner exists (D-10). Required fields,
listed so the owner is not asked for a blank page:

| Field | Proposed value |
|---|---|
| Eligible integration | ≥2 distinct source SHAs, ≥1 configured check executed, bundle sealed, candidate SHA recorded |
| Denominator | eligible integrations in the window (not all runs, not all repos) |
| Unit of analysis | the integration; repo and user reported separately |
| Diversity requirement | ≥5 repositories, ≥3 maintainers before any verdict |
| Minimum sample for a reading | 50 integrations; report a Wilson 95% interval, not a point estimate |
| Exclusions | dry-runs, single-source runs, `unverified` runs, runs on fixture repos, runs after a config change within the same day |
| Stopping rule | 50 integrations **or** 8 weeks, whichever first; then re-register before continuing |
| Decision rule | interval lower bound ≥3% → continue; upper bound <2% → stop/reposition; otherwise collect another window |
| False-block audit | sample 20 blocking findings; two reviewers label independently; disagreement → third; raw labels retained |
| Publication | anonymised table of per-integration outcomes (no repo names, no paths, no SHAs) |

Why an interval rather than the 5% threshold: with 50 runs, a 5% rate has a wide interval; a single
number invites over-reading. The decision rule above uses the interval bounds, which is honest at
this sample size.

---

## 7. What gets cheaper, and what gets more expensive

| Change | Effect |
|---|---|
| Removing N-way preflight | −1.5 d in M2, +0.5 d pipeline tests |
| Recompute instead of resume (V1) | −2 d in V1, +0.5 d in V2 |
| Deferring passport/approval/repair | −13 d before measurement starts |
| Adding the driver gate + consent | +1 d (M2/M3) |
| Adding path redaction class | +0.5 d (M3) |
| Traceability generator | +0.5 d (M6) |
| Net | V1 ≈ 37.5 d vs 46 d for the previous whole-MVP plan, with measurement 5 weeks earlier |

---

## 8. Approval dependencies

| Change | Decision |
|---|---|
| V1/V2 split | **D-08** |
| Requirement re-prioritisation (§3.2) | **D-09** |
| G4 → instrument gate; MG-1 creation | **D-10** |
| Dropping manifest from MVP | covered by D-09 |
| Alpha release before passport/approval/repair | **D-08** |
