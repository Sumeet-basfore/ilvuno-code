# 05 — Owner Decision Register

Status: **Mixed.** D-01…D-06, D-16…D-18 and D-19 are **DECIDED 2026-11-20** (record: `docs/decisions/open-questions.md` §0). D-16 (M0 lint), D-17 (canonical repository and module path), D-18 (Apache-2.0 text) and D-19 (M0 delivery shape: two PRs, T-0001/T-0005 scope amendments, deferrals and exclusions) were approved directly and have no request row here. D-07…D-15 remain **OPEN**. This file is the request register. It does not record decisions, and its §8 sign-off block is unsigned — *(superseded 2026-11-20 by the signed M0 authorization in `docs/readiness/06-m0-readiness-checklist.md` §7; the blank template is kept as the historical request form)*.

This register exists because the review's second blocker (BLK-02) was not a documentation bug: it
was that recommendations had been written in a tone that read like decisions. This file keeps them
separate. A recommendation is an input to the owner's choice; it is not the choice.

---

## 1. Rules

1. **One authority.** `docs/decisions/open-questions.md` is the only place where decision status is
   recorded. Any other document that asserts a status (roadmap, milestones, AGENTS) must *link* to
   it, not restate it.
2. **No silent defaults.** If a decision is not made, the dependent work stays blocked. The "if
   undecided" column says what happens — always "blocked", never "proceed with the recommendation".
3. **Recording a decision** means appending to `open-questions.md` §1 in this form:

   ```
   | D-03 | A-3 language | DECIDED 2026-11-20 | Go | owner: <name> |
     rationale: single static binary; no provider SDK in scope (ADR-0010)
     docs updated: ADR-0001, tasks/T-0001.md |
   ```

   A decision without a rationale and a list of documents to update is not recorded.
4. **Reversing a decision** is a new entry with `supersedes: D-xx`. Nothing is edited away.

---

## 2. Decisions that block M0 (answer before any code)

| ID | Decision | Options | Recommendation | Impact if wrong | If undecided |
|---|---|---|---|---|---|
| **D-01** | **What authorizes a milestone?** | (a) owner signs each milestone in `06-m0-readiness-checklist.md`; (b) roadmap's gate sign-off is sufficient; (c) tasks may start when written | **(a)** — one signature, one checklist, one log | Contributors act on contradictory statements; exactly the current failure | **Blocked.** No M0. |
| **D-02** | **Product shape** (`A-1`) | CLI only / CLI + plugin later / desktop now / web now | **CLI only for V1** (product `05` §2) | Building a shell before the claim is tested; 6× cost | **Blocked.** `T-0001`, `T-0605a` undefined. |
| **D-03** | **License** (`A-2`) | Apache-2.0 / AGPL-3.0 / MIT | **Apache-2.0** | Enterprise adoption blocked (AGPL) or no patent grant (MIT); changing later splits contributors | **Blocked.** `T-0001` cannot create `LICENSE`. |
| **D-04** | **Implementation language** (`A-3`) | Go / Rust / TypeScript | **Go** (ADR-0001) | A rewrite after M2 is 3–4 weeks; before M0 it is free | **Blocked.** `T-0001` cannot create `go.mod`. |
| **D-05** | **Platforms and Git floor** (`A-4` + new) | macOS+Linux, Git ≥2.38 / macOS only / +Windows / raise floor | **macOS + Linux; floor = the lowest version verified by X-09** (2.38 preferred) | Wrong floor = parsing breaks on users' machines; Windows = 3–4 weeks | **Blocked.** CI matrix and `doctor` probe undefined. |
| **D-06** | **Verification config location** (`A-9`) | committed `ilvuno.toml` / user-level / both | **Committed** (reviewable, shareable) | User-level config makes verification unreviewable — defeats Marco's use case | **Blocked.** `T-0103`, `T-0305`. |

## 3. Decisions that block M2 design (answer before candidate work)

| ID | Decision | Options | Recommendation | Impact if wrong | If undecided |
|---|---|---|---|---|---|
| **D-07** | **Candidate ref model** | (a) detached worktree + `refs/ilvuno/candidates/<id>`; (b) branch `ilvuno/<id>` under `refs/heads/`; (c) no worktree, objects only | **(a)** — verified (X-04b); keeps the namespace invariant; header line in `git worktree list` is `detached` | (b) pollutes the user's branch list and breaks the only-our-refs invariant; (c) cannot run checks | **Blocked.** M2. |
| **D-08** | **Custom merge-driver policy** | refuse by default + consent / always allow + record / always refuse (no escape) | **Refuse + explicit consent** showing the command (X-05 proves drivers execute code) | Always-allow = silent arbitrary code during a "read" phase; always-refuse breaks repos with a legitimate lockfile driver | **Blocked.** M2 security tests. |
| **D-09** | **Write-scope model** | four effect classes (A/B/C/D) / keep one absolute claim / audit class C too | **Four classes**, with class C declared not-contained (`03` §2) | Keeping the absolute claim guarantees a false test result; auditing class C requires a sandbox we do not have | **Blocked.** `SEC-101`, `INV-1`, BG-3. |
| **D-10** | **Determinism guarantee scope** | full “same inputs” / scoped to config+Git+OS fingerprint / per-machine only | **Scoped to the fingerprint** (`02` §8) | Overclaiming produces bug reports the tool cannot honour; underclaiming loses the cache and approval-binding benefits | **Blocked.** ADR-0007, cache key, approval binding. |

## 4. Decisions that block the plan, not the first commit

| ID | Decision | Options | Recommendation | Impact if wrong | If undecided |
|---|---|---|---|---|---|
| **D-11** | **V1/V2 split** | one MVP / V1 = candidate+checks+evidence, V2 = continuity | **V1/V2** (`04` §4) | ~13 days spent before the premise is measured | Blocked work continues on the old 46-day plan; measurement slips 5 weeks |
| **D-12** | **Requirement re-prioritisation** | keep all P0 / adopt `04` §3.2 | **Adopt §3.2** (manifest→P2, passport→V2, XDC narrowed) | Gates cannot be satisfied as written; M6 "complete" with P0 missing | Traceability check (`T-0605`) cannot pass |
| **D-13** | **Measurement owner and plan** | named owner + `09-measurement-plan.md` / leave as "U+PM" | **Named person, dated plan, MG-1 gate** | G4 cannot advance honestly; the product decision defaults to opinion | G4 blocked; V1 can still ship to design partners |
| **D-14** | **Claims policy** | three assurance levels / current wording | **Three levels** (`03` §6) | Team sales imply provenance we do not deliver; trust damage is permanent | Alpha docs cannot be written safely |
| **D-15** | **Defer M7 (adapters) entirely** | defer until after M-1 reading / keep provisional | **Defer**; PG1 requires the provider-terms packet | Scope creep back into dispatch before the premise is tested | No effect on M0, but T-7001/7002 stay "specified" and invite work |

---

## 5. Recommendations from Phase 2, restated as OPEN

The review correctly flagged these as having been presented as settled. They are listed here with
their current status so there is no ambiguity.

| ID | Question | Phase 2 recommendation | Status |
|---|---|---|---|
| `A-1` | Product shape | Standalone CLI | **OPEN** → D-02 |
| `A-2` | License | Apache-2.0 | **OPEN** → D-03 |
| `A-3` | Language | Go | **OPEN** → D-04 |
| `A-4` | Windows in MVP | No, v0.2 | **OPEN** → D-05 |
| `A-5` | Passport location | Repo-committed, `--local-only` optional | **OPEN**, needed before V2 |
| `A-6` | Repair executor | External command only | **OPEN**, needed before V2 |
| `A-7` | AI interpretation | Opt-in, non-blocking | **OPEN**; V1 ships the *structure* only (no model call), so it does not block |
| `A-8` | Telemetry | None by default, counts-only opt-in | **OPEN**, needed before any metrics export |
| `A-9` | Verification config location | Committed `ilvuno.toml` | **OPEN** → D-06 |
| `A-10` | Monetization gate | No paid feature before 20 weekly actives and M-1 ≥ 5% | **OPEN**, not needed before V2 |

New decisions introduced by this remediation: **D-01** (authority), **D-05** (Git floor, folded into
`A-4`), **D-07** (ref model), **D-08** (drivers), **D-09** (write classes), **D-10** (determinism
scope), **D-11** (scope split), **D-12** (requirements), **D-13** (measurement), **D-14** (claims),
**D-15** (defer adapters).

---

## 6. Suggested decision order (one sitting, ~45 minutes)

```
1. D-01  authority            ← unblocks the rest being meaningful
2. D-02, D-03, D-04, D-05, D-06   ← five M0 decisions
3. D-11, D-12                 ← scope; needed for a re-estimate
4. D-07, D-08, D-09, D-10     ← design; needed before M2, not before M0
5. D-13, D-14, D-15           ← later gates; can be deferred with a date
```

If only 15 minutes are available, decide **D-01…D-06**. That is the minimum that makes M0 a
legitimate milestone; M0's own first task (E-1′) will inform D-07/D-10 afterwards.

---

## 7. What happens after a decision

| Decision | Documents to update (in the same change) |
|---|---|
| D-01 | `AGENTS.md` §1, `CLAUDE.md` §1, roadmap §3/§9.4, milestones §3, `06-m0-readiness-checklist.md` |
| D-02…D-06 | `open-questions.md` §1, `T-0001` dependencies, `ADR-0001` status, roadmap capacity table |
| D-07, D-10 | `ADR-0006`, `ADR-0007`, architecture 05 §4, 06 §5, 02 §4.3 |
| D-08, D-09 | `ADR-0006`, architecture 09 §4/§5/§7, PRD `SEC-101`/`INV-1`, BG-3 test matrix |
| D-11, D-12 | PRD §10.7/§10.11/§16/Appendix B, product 05 §4, planning 02, tasks affected |
| D-13 | product 07 §2/§5, planning 04 BG-5/BG-8, planning 05 G4 → MG-1, new `09-measurement-plan.md` |
| D-14 | architecture 09 §4.5, tasks T-0304/T-0602/T-0605, banned-claims list |
| D-15 | planning 05 PG1, tasks T-7001/T-7002 status |

Deferring a decision is allowed and must be recorded with a date:

```
| D-13 | measurement owner | DEFERRED to 2026-12-15 | owner: <name> | reason: after first design partners |
```

---

## 8. Sign-off block

Fill in and commit this file's companion entry in `open-questions.md`. Do not edit the decision into
this document — this document is the *request*.

```
Owner: ______________________          Date: ____________

D-01  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-02  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-03  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-04  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-05  ☐ approved (floor: _______)  ☐ amended: ____________  ☐ deferred to: ________
D-06  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-07  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-08  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-09  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-10  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-11  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-12  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-13  ☐ approved (owner: _______)  ☐ deferred to: ________
D-14  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
D-15  ☐ approved  ☐ amended: ____________________  ☐ deferred to: ________
```

> **Superseded 2026-11-20.** The owner signed the M0 authorization in
> `docs/readiness/06-m0-readiness-checklist.md` §7. This block is retained as the historical request
> form for D-01's signature convention; it is not the authorization record, and its blank state no
> longer implies that M0 is unauthorized.

**Until this block is filled, M0 is not authorized, and no implementation task may begin.**
