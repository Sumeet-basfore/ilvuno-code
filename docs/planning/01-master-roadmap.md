# 01 — Master Roadmap

Status: proposed plan, awaiting M0 approval. Phase 4 output. No production code exists.
Authorization: none. Decided 2026-11-20: D-01…D-06 (`docs/decisions/open-questions.md` §0); all other decisions OPEN. Milestone authorization →
`docs/readiness/06-m0-readiness-checklist.md` §7 (unsigned). Reconciliation and entry-gate results →
`docs/readiness/07-m0-reconciliation.md`. The merge-tree E-1 spike named in §9 is withdrawn (`07` §3).
Sources: `docs/product/` (scope), `docs/architecture/` (design), `docs/adr/` (decisions).

---

## 1. Guiding principle, applied

> Prove the Integration Intelligence engine before committing to the full multi-agent desktop
> application.

Everything below is ordered by one question: **what is the cheapest artifact that can prove the
integration claim false?**

| Cheapest falsifiers, in order | Cost | If negative |
|---|---|---|
| `merge-tree` cannot see tuple conflicts (E-1) | 2 days | Preflight stops being blocking; the product loses a claim |
| Candidate SHAs are not reproducible across machines (E-2) | 3 days | Determinism becomes same-machine only; cache and approval binding weaken |
| Predictions disagree with real merges (E-3) | 2 days | Findings demote to warnings; the "we told you first" flow dies |
| Recovery cannot be unconditional (E-4) | 2 days | Drop `resume`; require `abort` + re-run |
| Combination-only failure rate is ~0 on real repos (U-3) | weeks, needs users | Stop or reposition (product `07` §5) |

M0–M2 are designed to answer the first four **before** M3–M4 build features on top of them. That
is the whole reason the milestone order differs from the product's S1–S6 slicing: the product
slicing is optimised for demo value, this roadmap is optimised for falsification order.

---

## 2. Shape of the plan

| Phase | Milestones | Weeks | Purpose |
|---|---|---|---|
| **Prove** | M0, M1, M2 | 1 + 1.5 + 2 = 4.5 | Can we build a candidate safely and reproducibly? |
| **Explain** | M3, M4 | 2 + 2 = 4 | Can we say why it failed, with evidence? |
| **Survive** | M5 | 2.5 | Does it survive being interrupted? |
| **Ship** | M6 | 2 | Is it usable by someone who is not us? |
| **Deferred** | M7–M12 | — | Provisional, gated on M6 + commercial signal |

Total MVP: **13 weeks of engineering + 2 weeks buffer = 15 weeks** with one full-time engineer and
two coding agents. Capacity assumption stated openly: this is a *single-maintainer* schedule. Two
humans would not halve it (see §7).

---

## 3. Milestones at a glance

| ID | Milestone | Weeks | Gate | Can ship without? |
|---|---|---|---|---|
| M0 | Repository setup + deterministic fixtures | 1 | G0 | No — everything depends on fixtures |
| M1 | Read-only git and worktree analysis | 1.5 | G1 | No |
| M2 | Disposable integration candidate creation | 2 | G2 | No — this *is* the product |
| M3 | Conflict detection and evidence reporting | 2 | G3 | No — a candidate without a report is unreadable |
| M4 | Combined build/test verification | 2 | G4 | No — the core value |
| M5 | Reliability, cancellation, recovery | 2.5 | G5 | Yes for private alpha; no for public v0.1 |
| M6 | Functional CLI prototype | 2 | G6 | No — the delivery surface |
| M7 | Agent runtime + provider adapters | 3 | PG1 | Provisional |
| M8 | Local web + desktop workspace | 6 | PG2 | Provisional |
| M9 | Multi-agent task orchestration | 4 | PG3 | Provisional |
| M10 | Secure cross-device continuity | 6 | PG4 | Provisional |
| M11 | GitHub PR + CI integration | 2 | PG5 | Provisional |
| M12 | Optional commercial services | 8 | PG6 | Provisional |

M7–M12 are **not approved** and carry no dates. They exist so the MVP's seams are visible and so
nobody builds something that forecloses them (architecture `01` §4).

### 3.1 Milestone order changes from the proposed list

Two deliberate deviations, both dependency-driven:

1. **M1 is read-only only.** The proposed M1 ("read-only Git and worktree analysis") is right, and
   I keep creation out of it — worktree *creation* is the first mutating operation in the product
   and belongs in M2 behind the journal (M2.2), not alongside read-only probes. Mixing them
   produces a milestone whose gate cannot distinguish "read" bugs from "write" bugs.
2. **Verification (M4) precedes reliability (M5).** The proposed order is the same, and it is
   correct: you cannot test recovery of a pipeline that does not exist yet. But M5 must not be
   treated as polish — it is the milestone that makes the public release defensible, and it is
   budgeted at 2.5 weeks, the second-largest slice.

---

## 4. What each milestone proves, and to whom

| Milestone | Proof | Audience |
|---|---|---|
| M0 | Fixtures can express every failure class reproducibly | Us (and any future contributor) |
| M1 | We can read a repository without changing it | Us |
| M2 | A candidate is a deterministic function of pinned inputs | Us; E-1/E-2/E-3 resolved here |
| M3 | Findings are classified, attributed, and bundled | Us; the report becomes reviewable |
| M4 | Combined verification finds what per-branch verification misses | Priya + Marco (personas) |
| M5 | Interruption never produces a false verdict | Dana (platform) |
| M6 | Someone outside the project can use it in 10 minutes | Public alpha |
| M7+ | (provisional) | — |

---

## 5. Scope discipline

### 5.1 In the MVP (M0–M6)

Everything required by PRD P0 requirements: git refs in, deterministic candidate, deterministic
findings, verification with five outcomes, evidence bundles, passport, approval gate, repair via
external command, resume/abort, exit-code contract, doctor, JSON contract, docs.

### 5.2 Explicitly not scheduled

| Not scheduled | Why | Would be scheduled when |
|---|---|---|
| Any UI (web, desktop, TUI) | ADR-0003 | Gate G4 + paid demand |
| Dispatch / agent launching | ADR-0010 | PG1, after G6 + user request |
| Windows | `A-4`, `WT-5` | v0.2 |
| Submodules, LFS, sparse | `T-4` | v0.2 |
| Hosted anything | ADR-0015, `A-10` | After 20 weekly actives and M-1 ≥ 5% |
| Telemetry | `A-8` | Never by default |
| Secret scanning | `SC-4` | Never (different product) |
| Parallel verification commands | `VR-3` | After the sequential loop is measured |
| Interpretation (AI findings) model | `A-7` | After the deterministic report is trusted |

### 5.3 The one thing that would change the plan

If E-1 through E-3 all pass but U-3 (combination-only rate) later measures < 2% on real repos, the
project stops or repositions (product `07` §5). No amount of M5/M6 quality changes that. The
roadmap therefore front-loads a *usable measurement instrument* in M4 (`T-0404`) rather than
treating measurement as a later research task.

---

## 6. Cadence and working agreements

| Practice | Detail |
|---|---|
| Sprint length | 1 week, aligned to milestones (M1 = 1.5 sprints, etc.) |
| Demo at gate | A recorded terminal session running the gate's benchmark set, not a slide |
| Review | Every task: one human review. Every gate: human sign-off against listed criteria |
| Agent contribution | Bounded tasks, one owner per package, cap of 2 agents (`02-milestones.md` §11) |
| Branch model | `main` protected; work on `feat/<task-id>`; merge by squash with the task ID in the subject |
| Definition of Done | `07-definition-of-done.md`, enforced in review |
| Benchmark regression | Benchmarks run in CI from M3 onward; a regression blocks merge |
| Documentation | ADR amendment or new ADR for any structural deviation; no silent drift |

---

## 7. Why this is not parallelised further

An honest note for whoever reads this expecting "4 agents, 4× speed":

| Concurrent coding agents | Observed cost | Net |
|---|---|---|
| 1 | none | baseline |
| 2 | integration overhead on shared files (`model`, `journal`, `cli`), ~10–15% of each agent's time | ~1.7× |
| 3 | constant rebasing on those files; review becomes the bottleneck (one human) | ~1.6× and worse quality |
| 4+ | agent-authored conflicts that look like agent-authored code and take longer to review than to write | slower |

The limit here is **review bandwidth**, not code generation. One human reviewing two agents' diffs
per day is the real constraint; a third agent adds diffs, not throughput. This is why the plan caps
at 2 agents (`02-milestones.md` §11, `04-benchmarks.md` §7 for the measurable version).

---

## 8. Timeline (single maintainer + 2 agents)

```
week  1     2     3     4     5     6     7     8     9    10    11    12    13    14    15
M0   ██
M1      ████
M2            ████ ████
M3                      ████ ████
M4                                ████ ████
M5                                          ████ ████ ██
M6                                                      ████ ████
buf                                                           ████
gate  G0    G1        G2        G3        G4        G5        G6
```

Buffer (2 weeks) is not optional: it absorbs E-1/E-2 surprises and the first external user's bug
reports. A plan without buffer is a plan that ships M5 unfinished, and M5 is the milestone the
public release depends on.

---

## 9. M0 readiness assessment

**Verdict: conditionally ready. Three blockers, all resolvable in under a day.**

### 9.1 Ready now

| Prerequisite | Status |
|---|---|
| Product scope fixed (MVP boundary, non-goals) | ✅ `05-mvp-scope.md` §4 |
| Requirements enumerated with acceptance criteria | ✅ 97 requirements, 69 P0 |
| Architecture specified to implementation depth | ✅ 12 documents, ~5,000 lines |
| Structural decisions recorded with revisit conditions | ✅ 15 ADRs |
| Failure model defined (five outcomes, five uncertainty kinds) | ✅ architecture `02` §4, `07` §2 |
| Test strategy with named required fixtures | ✅ architecture `11` §3 |
| Exit-code and error contract | ✅ architecture `01` §6, `12` §6 |
| Benchmark set specified | ✅ `04-benchmarks.md` (24 scenarios) |

### 9.2 Blockers before the first commit

| # | Blocker | Why it blocks | Resolution | Time |
|---|---|---|---|---|
| **B-1** | `A-1`, `A-2`, `A-3`, `A-4`, `A-9` **decided 2026-11-20** (D-02, D-03, D-04, A-4 excluded from V1, D-06). Resolved | Language (Go), license (Apache-2.0), platform set (macOS+Linux), and config location (committed `ilvuno.toml`) determine the repo skeleton itself. Building on an unanswered decision means rebuilding | Approve the four ADR-linked recommendations, or amend them | < 1 h of decision, no engineering |
| **B-2** | No fixture forge proof-of-concept | The entire test strategy assumes the forge can produce deterministic repos with fixed identity/date and reproduce the nine failure classes. If it cannot, M0's gate is unachievable and the plan is wrong | Run **E-1** as a throwaway spike (2 days, no production code) | 2 days |
| **B-3** | Git version floor untested on the target machines | ADR-0004 needs ≥2.38 for `merge-tree --write-tree`. If the maintainer's machine or CI image is older, the preflight design degrades on day one | `git --version` on dev + CI images; pin an image that has ≥2.38 | 15 min |

### 9.3 Risks that are accepted, not blocking

| Risk | Why we proceed anyway |
|---|---|
| U-3 (combination-only rate) is unknown | It is *the* thing M4 measures; waiting for the answer means never building the instrument |
| No user interviews exist | E-4-style falsification plus M6 alpha covers this; a 15-week plan cannot wait for a research round |
| `merge-tree` fidelity unproven on rename-heavy repos | E-1/E-3 resolve it inside M0–M2 before features depend on it |
| No second maintainer | Schedule is already single-maintainer-scoped; hiring would not compress it (see §7) |

### 9.4 Go/no-go recommendation

Proceed to M0 **after** B-1 is answered and B-3 is checked. Run B-2 (E-1) as M0's first task, not
as a prerequisite — it is the milestone's own proof, and if it fails we want to fail inside M0
with three days spent, not before M0 with a plan to rewrite.

**Status 2026-11-20:** D-01…D-06 and planning items A-4, S-1…S-4, C-5 and C-7 decided. B-1 is resolved. M0 is still not authorized. Entry-gate result: `docs/readiness/07-m0-reconciliation.md` §4 (7 PASS · 3 OPEN · 0 FAIL). The E-1 spike named in this section is withdrawn; the SP-1R3 reference harness replaces it.

**Requested approval:**

1. Approve `A-1` (CLI), `A-2` (Apache-2.0), `A-3` (Go), `A-4` (no Windows in MVP), `A-9` (committed
   `ilvuno.toml`).
2. Approve the milestone order and dates in §3 and §8, including the 2-week buffer.
3. Authorise M0 only (1 week, tasks `T-0001`…`T-0005`) — not M1–M6 as a block.
4. Accept the falsification gates: if E-1/E-2/E-3 fail, M2 is re-scoped rather than pushed through.

---

## 10. Reading order

| Doc | For |
|---|---|
| `02-milestones.md` | Per-milestone scope, tasks, acceptance, security, DoD, rollback |
| `03-dependency-graph.md` | Task DAG, critical path, parallel waves, owner map |
| `04-benchmarks.md` | 24 scenarios, baselines, measurable gates, benchmark harness |
| `05-release-gates.md` | G0–G6 exit criteria, sign-off, artefacts, provisional PG1–PG6 |
| `06-risk-register.md` | 20 risks with triggers, owners, mitigations, review dates |
| `07-definition-of-done.md` | Universal and per-artefact DoD, quality bars, review checklists |
| `docs/tasks/` | One spec per task: scope, outline, AC, test commands, evidence, review checklist |
