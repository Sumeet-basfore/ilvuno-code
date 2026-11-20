# 01 — Blocker Resolution Plan

Status: remediation proposal. Nothing here authorizes implementation.
**No approved document has been modified.** Proposed edits are stated as text to be applied after
owner approval (`05-owner-decision-register.md`).

Inputs: `docs/reviews/01…04`, `docs/decisions/open-questions.md`, and the affected product,
architecture, ADR, planning, and task files.

---

## 1. What the review rejected, and what this package does about it

The review scored 4.0/10 and rejected M0 start. Its points reduce to five classes:

| Class | Findings | This package |
|---|---|---|
| The Git design cites a command shape that does not exist | BLK-01, HI-01 | Replaces prediction with a verified object-only sequential pipeline (`02`) |
| Governance has no single truth | BLK-02 | One decision register, one status statement, one gate checklist (`05`, `06`) |
| The safety promise overstates what is enforceable | HI-02, HI-03, MED-05 | Write classes, consent model, driver policy (`03`) |
| Requirements and scope disagree with milestones | HI-04, SUG-01 | Requirement → milestone matrix and a scope cut proposal (`04`) |
| Claims outrun evidence | MED-01…MED-04 | Narrowed claims + pre-registered measurement (`04` §6, `03` §6) |

Nothing in this package is an approval. Every correction is either (a) engineering-complete and
ready to apply on approval, or (b) dependent on an experiment that needs a decision to run.

---

## 2. Finding ledger

Each block is one review finding, with the six fields the brief requires. "Status" describes the
*proposal*, not the repository.

### BLK-01 — The N-way `merge-tree` preflight does not exist

| Field | Content |
|---|---|
| **Current problem** | Architecture 06 §4.1, ADR-0006, `T-0005`, and `T-0204` pass 3+ refs to `git merge-tree --write-tree`. The command accepts exactly two. G0 therefore cannot answer its own question, and M2's "tuple prediction" has no implementation. |
| **Evidence** | Verified locally (Git 2.55.0): usage is `git merge-tree [--write-tree] [<options>] <branch1> <branch2>`; `--stdin` is "perform multiple merges, one per line of input", not N-way. Passing three commits exits 129 with the usage text. Full transcript: `02` §2 (X-01, X-02). |
| **Proposed correction** | Delete the prediction/preflight concept. Compute the candidate by **sequential two-way merges in the object database** (`merge-tree --write-tree` → `commit-tree`, N times, in order), then materialize a disposable worktree at the resulting commit. The "prediction" and the "candidate" become the same computation, so the prediction-vs-reality divergence class disappears. |
| **Affected files** | `docs/architecture/06-integration-intelligence.md` §4.1, §5; `docs/adr/ADR-0006`; `docs/architecture/02-domain-model.md` §4.3 (`preflight` state); `docs/planning/02-milestones.md` M2; `docs/tasks/T-0005.md`, `T-0204.md`; `docs/product/02-prd.md` `INT-108`, `INT-206`(note); archived `02-critical-risks.md` references. |
| **Required experiment** | **E-1′ (redefined, 0.5 day):** does the object-only pipeline produce the same tree and commit as real sequential merges, for (a) 3 clean sources, (b) a conflict at step 2, (c) a source that is already an ancestor, (d) a source whose merge-base differs? First three already pass locally — see `02` §5. Repeat on the Git floor. |
| **Approval dependency** | Engineering may apply on owner approval of the *change of guarantee*: preflight no longer exists; conflicts are reported at the merge step that caused them. Also requires D-03 (candidate ref model) and D-05 (determinism scope). |
| **Status** | **Resolved at proposal level**; one floor-version experiment outstanding (`02` §9, X-09). |

### BLK-02 — M0 has no single approved baseline

| Field | Content |
|---|---|
| **Current problem** | Four documents disagree: `AGENTS.md` §1 "only M0 is approved"; roadmap §3 "awaiting M0 approval"; planning 02 §3 "M0–M6 are approved MVP scope"; `open-questions.md` "nothing is decided". `T-0001` depends on A-1…A-4 and A-9, none of which is recorded as decided. A contributor cannot tell whether they may start, or what they are implementing. |
| **Evidence** | Quoted lines: `AGENTS.md:14`, `planning/01-master-roadmap.md:3` and `:226`, `planning/02-milestones.md:3`, `tasks/T-0001.md:10`, `decisions/open-questions.md:9`. |
| **Proposed correction** | Declare **`docs/decisions/open-questions.md` the single authority for decision status**, and `docs/planning/06-m0-readiness-checklist.md` the single authority for milestone authorization. Rewrite the four status sentences to point at those two files instead of asserting status locally. Record A-1…A-10 as decided/deferred with owner and date, or leave them OPEN and keep M0 blocked. |
| **Affected files** | `AGENTS.md` §1; `CLAUDE.md` §1 (derived statement); `docs/planning/01-master-roadmap.md` §3, §9.4; `docs/planning/02-milestones.md` §3; `docs/decisions/open-questions.md` §1; `docs/tasks/T-0001.md` dependencies. |
| **Required experiment** | None. This is a records decision. |
| **Approval dependency** | **Owner only.** D-01 (M0 authority) + D-02…D-06 (the five T-0001 decisions). |
| **Status** | **Blocked on owner** — cannot be resolved by engineering. |

### HI-01 — Worktree creation cannot produce the promised ref

| Field | Content |
|---|---|
| **Current problem** | Architecture 05 §4 runs `git worktree add -B refs/ilvuno/candidates/<id> …` and claims the candidate lands under `refs/ilvuno/`. Git treats `-B` as a *branch name* and stores it under `refs/heads/`. |
| **Evidence** | Verified: the command created `refs/heads/refs/ilvuno/candidates/probe` (a branch literally named `refs/ilvuno/…`), and `git for-each-ref refs/ilvuno` matched nothing. A naive substring check would have "passed" while the namespace invariant was violated. Transcript: `02` §2 (X-04). |
| **Proposed correction** | Use a **detached worktree plus an explicit custom ref**: `git worktree add --detach <path> <candidate_sha>` then `git update-ref refs/ilvuno/candidates/<run_id> <sha>`. Ownership is decided by the worktree **path prefix** (`.ilvuno/worktrees/`) plus the journal, because a detached worktree has no branch line in `git worktree list --porcelain`. Prefix checks must be anchored (`^refs/ilvuno/`), not substring. |
| **Affected files** | `docs/architecture/05-worktrees.md` §3, §4; `ADR-0006`; `docs/architecture/01-overview.md` INV-1; `docs/architecture/02-domain-model.md` §3.7; `docs/tasks/T-0202.md`, `T-0205.md`. |
| **Required experiment** | **X-04 (done)** plus a repeat on the Git floor and a crash-during-cleanup case. |
| **Approval dependency** | D-03 (ref model). Owner must accept that candidate worktrees are detached and recognizable by path, not by branch. |
| **Status** | **Resolved at proposal level** (verified locally). |

### HI-02 — The write-scope promise does not match Git or child processes

| Field | Content |
|---|---|
| **Current problem** | PRD `SEC-101` says writes are limited to `.ilvuno/`, `refs/ilvuno/*`, and the task directory. In reality the design needs Git object writes, `.git/worktrees/*`, `init` writes to `ilvuno.toml`/`.gitignore`, key writes under `~/.config`, and verification children that may write anywhere and use the network. The invariant is unenforceable as worded, so its test can neither pass honestly nor fail usefully. |
| **Evidence** | Measured write inventories: `merge-tree --write-tree` writes loose objects (object count 30→32 on a conflicting merge); `worktree add` writes `.git/worktrees/<n>/{commondir,gitdir,HEAD,index,logs/HEAD,ORIG_HEAD}`; a merge in the worktree writes objects plus that worktree's `HEAD`/`index`/`logs/HEAD`. Architecture 07 explicitly permits child writes to `$HOME`; `Scope.WriteRoots` in architecture 09 omits all Git-internal paths. Transcripts: `02` §6; analysis: `03` §2. |
| **Proposed correction** | Replace one invariant with **four named effect classes** (A Ilvuno-owned, B Git-managed and declared, C child-process unconstrained, D one-time consent edits), and restate `SEC-101`/`INV-1` as: *"Ilvuno itself writes only classes A and B; class B is enumerated and observable; class C is executed under an explicit consent gate and is not constrained by Ilvuno."* Add a `git-objects` disclosure to the report and a consent step for executable config. |
| **Affected files** | `docs/product/02-prd.md` `SEC-101`, `SEC-106`; `docs/architecture/01-overview.md` INV-1; `docs/architecture/09-security.md` §5, §7; `docs/architecture/08-persistence-and-recovery.md` §6; `docs/planning/04-benchmarks.md` BG-3; `docs/tasks/T-0202.md`, `T-0304.md`, `T-0401.md`, `T-0605.md`. |
| **Required experiment** | **X-06 (partly done):** instrumented write audit across init / merge-tree / worktree add / merge / verify child / crash. Must include a child writing to `$HOME` and one making an outbound request, to prove the declared non-claim. |
| **Approval dependency** | D-07 (write classes). This narrows a published security claim, so it is an owner decision, not an engineering one. |
| **Status** | **Resolved at proposal level**; audit implementation is an M0/M2 test, not a blocker. |

### HI-03 — Custom merge drivers are an execution and determinism boundary

| Field | Content |
|---|---|
| **Current problem** | A repository `.gitattributes` can select `merge=<driver>` whose command comes from Git config. Architecture 05 forces only hooks/signing/conflict style; ADR-0006 says a driver is "recorded". Nothing refuses it, nothing fingerprints it, and the determinism contract ignores it. |
| **Evidence** | Verified: with `merge.upper.driver` configured and `data.txt merge=upper` in `.gitattributes`, **`git merge-tree --write-tree` executed the driver** and it wrote `/tmp/ilv2/DRIVER_RAN` outside the repository. The same driver ran on a real merge. Transcript: `02` §7 (X-05). |
| **Proposed correction** | Refuse by default. Detection is fail-closed and two-part: (a) any non-builtin `merge.*.driver` visible via `git config --get-regexp '^merge\..*\.driver'`, with (b) any matching `merge=<name>` selection in `info/attributes`, `core.attributesFile`, or `.gitattributes` in the merged trees (via `git check-attr --source=<sha>` when the Git version supports it; otherwise conservative refusal). Consent flag `--allow-merge-drivers` records the effective driver commands and a config digest, marks the run `driver_used: true`, and removes it from the reproducible-across-machines claim. |
| **Affected files** | `docs/architecture/05-worktrees.md` §5; `ADR-0006`, `ADR-0007`; `docs/architecture/09-security.md` §4.1, §6; `docs/architecture/06-integration-intelligence.md` §5.1; `docs/tasks/T-0202.md`, `T-0205.md`. |
| **Required experiment** | **X-05 (done)** plus: a driver that fails, a driver desired on one platform only, and a confirming case where refusal prevents the marker file. |
| **Approval dependency** | D-04 (driver policy). Changes what Ilvuno will run on the user's machine — owner decision. |
| **Status** | **Resolved at proposal level** (verified locally). |

### HI-04 — P0 and MVP/deferred assignments disagree

| Field | Content |
|---|---|
| **Current problem** | `ORQ-102/103` are P1 in the requirement tables but "bind the MVP" in PRD §16 and appear in MVP scope. `XDC-102` is P0 but its acceptance requires three transports, while transport is deferred. `T-0605` AC-1 depends on a 12-item traceability check that no document can currently satisfy. |
| **Evidence** | PRD §10.7 rows vs §16 prose vs `product/05-mvp-scope.md` §4.1; `XDC-102`/`XDC-103` acceptance text; planning 04 BG gates assume a different scope again. |
| **Proposed correction** | Publish one generated **requirement → milestone matrix** (`04` §3), with exactly one priority and one acceptance test per requirement. Concretely: `ORQ-102/103` (manifest in/out) → **P2, v0.2**; `ORQ-101/106` stay P0; `XDC-102` → **P1** with a narrowed acceptance ("the same passport renders identically from a repo copy on a second machine", which is `TSK-110`), `XDC-103` → **P1** as a static-render test; `XDC-101` stays P0. PRD §16 stops asserting a different priority set and links to the matrix. |
| **Affected files** | `docs/product/02-prd.md` §10.7, §10.11, §16, Appendix B; `docs/product/05-mvp-scope.md` §4.1; `docs/planning/02-milestones.md` §11 index; `docs/tasks/T-0605.md`; new `docs/planning/08-traceability.md` (generated). |
| **Required experiment** | None; it is a documentation and generator task (`T-0605` gets a `make trace` target). |
| **Approval dependency** | D-08 (scope cut) and D-09 (requirement re-prioritisation). Product decisions. |
| **Status** | **Resolved at proposal level**; applies only after owner approval. |

### MED-01 — Value and false-block thresholds use incompatible samples

| Field | Content |
|---|---|
| **Current problem** | M-1 wants ≥50 integrations across ≥20 users; G4 accepts 5 repositories and may advance with zero combination-only failures; BG-5 uses 5 repositories / 100 runs. Three different gates answer "is the phenomenon real?" with three different evidentiary bars. |
| **Evidence** | `product/07-success-metrics.md` §2, §5; `planning/05-release-gates.md` G4 criterion 8; `planning/04-benchmarks.md` BG-5. |
| **Proposed correction** | Separations, not reconciliation: **(a) fixture gates** measure correctness (24/24, BG-1…BG-10) and never market value; **(b) a pre-registered measurement plan** (`docs/planning/09-measurement-plan.md`, to be written) owns M-1: eligible-run definition, repo/user diversity, exclusions, denominator, interval, stopping rule, and the action at each outcome; **(c) G4 stops citing a market threshold** and becomes "the instrument works and has produced its first 20 real runs", with the verdict deferred to the measurement plan. |
| **Affected files** | `docs/product/07-success-metrics.md` §2, §5; `docs/planning/05-release-gates.md` G4; `docs/planning/04-benchmarks.md` BG-5, BG-8; `docs/decisions/open-questions.md` Q-1. |
| **Required experiment** | None now (needs users). The plan must exist before G4, not before M0. |
| **Approval dependency** | D-10 (Q-1 owner + date + plan). |
| **Status** | **Resolved at proposal level**; blocked on a named owner. |

### MED-02 — Comparative benchmarks are not independently reproducible

| Field | Content |
|---|---|
| **Current problem** | BL-1 measures "time to know the answer" performed by a person; BG-5's ≤2% false-block gate depends on manual adjudication. Neither has an operator script, start/stop events, or a gold-label protocol, so the 5× claim cannot be reproduced. |
| **Evidence** | `planning/04-benchmarks.md` §1, §3, §5. |
| **Proposed correction** | Add an operator protocol to the benchmark appendix: fixed task set (the 24 fixtures plus 3 real repos), explicit start (`branch list pinned`) and stop (`verdict known and stated`) events, a scripted BL-1 reproduction, a written adjudication rule for false blocks (two reviewers, disagreement → third adjudicator, raw labels retained), and a published anonymised measurement packet. Timing excludes agent runtime (already stated) and includes conflict-reading time (currently ambiguous). |
| **Affected files** | `docs/planning/04-benchmarks.md` §1, §3.1, §5; `docs/planning/05-release-gates.md` G4. |
| **Required experiment** | **X-08:** two operators run BL-1 and the false-block audit on 3 fixtures; compare labels and timing distributions. |
| **Approval dependency** | Engineering may write the protocol; publishing a 5× claim in release notes needs D-11 (claims policy). |
| **Status** | **Resolved at proposal level**; protocol text drafted in `04` §6. |

### MED-03 — Bundle integrity is not provenance

| Field | Content |
|---|---|
| **Current problem** | Digests prove a bundle did not change after sealing. They do not prove who ran it, on what machine, or that an unsigned approval came from a particular person. Current wording ("verifiable", "evidence") can be read as the stronger claim. |
| **Evidence** | Architecture 09 §4.5 already states the limit; `T-0602` permits unsigned approvals; no test distinguishes assurance levels end-to-end. |
| **Proposed correction** | Three named assurance levels, used verbatim in docs and report footers: **integrity** (digest consistency, offline, unsigned) · **attributed** (Ed25519-signed approval bound to `(candidate_sha, evidence_digest)`) · **provenanced** (reserved; requires a trusted executor, **not offered**). Team/cross-user workflows require *attributed*. Add a test matrix covering unsigned, signed, tampered, recomputed-chain, and missing-evidence. |
| **Affected files** | `docs/architecture/09-security.md` §4.5; `docs/product/02-prd.md` `EVD-*` wording; `docs/tasks/T-0304.md`, `T-0602.md`, `T-0605.md`. |
| **Required experiment** | None (tests, not research). |
| **Approval dependency** | D-11 (claims policy). |
| **Status** | **Resolved at proposal level.** |

### MED-04 — The future Claude adapter is under-verified

| Field | Content |
|---|---|
| **Current problem** | Architecture 03 and `T-7002` assume a stable stream-JSON surface and the user's CLI login. The review found the current vendor docs describe an SDK for Python/TypeScript, `-p --output-format json` for other languages, and constraints on third-party use of consumer login. None of this is verified for our intended flow. |
| **Evidence** | Review 01 §"Verified findings"; review 02 MED-04 (vendor links). Not re-verified here: the adapter is post-MVP and `AUTH-3` already forbids custody. |
| **Proposed correction** | Keep M7 provisional. Replace "the CLI works on this machine" with a **PG1 entry packet**: supported protocol + version band recorded from vendor docs, conformance fixtures against that band, typed refusal for auth/unsupported environments, and a written terms decision for the local-login flow. `T-7002` gains "no credential read/store" as a tested assertion, not a note. |
| **Affected files** | `docs/architecture/03-agent-adapters.md` §2, §3; `ADR-0010`; `docs/tasks/T-7001.md`, `T-7002.md`; `docs/planning/05-release-gates.md` PG1. |
| **Required experiment** | **X-10 (post-MVP):** conformance run against the supported version band; auth-state matrix (`ok`/`needs_login`/`unsupported_env`). |
| **Approval dependency** | D-12 (defer M7 entirely until after M-1). |
| **Status** | **Resolved at proposal level** (deferral + gated packet). |

### MED-05 — Absolute paths in child output are normalized nowhere

| Field | Content |
|---|---|
| **Current problem** | `INV-9` and the observability requirements say no absolute paths in persisted artifacts. `SEC-103` makes path redaction an *export* option. The redactor's default classes cover secrets, not paths. A test command printing `$PWD` writes a machine-specific path into evidence. |
| **Evidence** | `architecture/09` §6 (three classes: pattern, entropy, user); `T-0304` tests secrets only; no fixture prints an absolute path. |
| **Proposed correction** | Add **class `path`** to write-time redaction: absolute paths under the repo root → repo-relative; under `$HOME` → `~/…`; other absolute paths → `«abs-path»` with the basename retained. Keep raw output possible only via `--keep-raw` (already refused for bundles). Add the fixture from the review (a command printing `$PWD` and `$HOME`) to `T-0304` and to BG-3. |
| **Affected files** | `docs/architecture/09-security.md` §6; `docs/architecture/12-observability.md` §10; `docs/product/02-prd.md` `SEC-103`; `docs/tasks/T-0304.md`. |
| **Required experiment** | **X-07:** run the path-printing fixture, then inspect every persisted artifact and an exported bundle for absolute paths. |
| **Approval dependency** | D-05 (determinism/redaction scope) — small, but it changes what evidence looks like. |
| **Status** | **Resolved at proposal level.** |

### SUG-01 — Build the smallest vertical slice first

| Field | Content |
|---|---|
| **Current problem** | The plan spends 46 engineering days before measuring the premise, including passport, approval signing, repair, and auxiliary commands. The product's own scope document says S1–S3 is the smallest release that can answer "does this work for me". |
| **Evidence** | `product/05-mvp-scope.md` §4.3; `planning/02-milestones.md` M0–M6; review 04 readiness table ("M6 over-scoped for an unvalidated product"). |
| **Proposed correction** | Split the MVP into **V1 (vertical slice, M0–M4)** = refs → candidate → verification → evidence bundle → report; and **V2 (continuity, M5–M6)** = recovery hardening, passport, approval, repair, release. Gate V2 behind "≥5 real users ran ≥2 integrations and M-1 has a first reading". Recovery work that protects safety (uncertainty model, abort, cleanup) moves into V1; the rest defers. Full renumbering in `04` §5. |
| **Affected files** | `docs/planning/01-master-roadmap.md` §3, §8, §9; `docs/planning/02-milestones.md` M3–M6; `docs/tasks/T-0501…T-0605` scope flags; `docs/product/05-mvp-scope.md` §4.3. |
| **Required experiment** | None. Product decision. |
| **Approval dependency** | D-08 (scope cut). |
| **Status** | **Resolved at proposal level**; awaits owner. |

---

## 3. What changes if the proposals are approved

| Area | Before | After |
|---|---|---|
| Preflight | Separate N-way prediction step | Removed; one sequential object pipeline, conflict reported at its step |
| Candidate ref | `refs/ilvuno/*` claimed via `worktree add -B` (impossible) | Detached worktree + `update-ref refs/ilvuno/candidates/<id>` |
| "No writes" | Absolute claim | Four effect classes with an enumerated Git-internal set |
| Merge drivers | Recorded | Refused by default; consent records identity; determinism claim excluded |
| Determinism | "Same inputs → same SHA" | Same guarantees, with the full input set named (config, attributes, drivers, Git and OS versions) |
| Requirements | Three conflicting priority sets | One generated matrix |
| MVP | 33 tasks / 46 days in one block | V1 (≈ 12 tasks) then V2 gated on a real reading |
| M0 authority | Four documents disagree | One register + one checklist |

---

## 4. Sequencing

```
today        owner decisions D-01..D-12        (no code)
   │
   ├─ D-02..D-06 recorded ──► M0 protocol work may start (T-0001 revised, T-0002, T-0004)
   │                            │
   │                            ├─ X-09 (Git floor) ──► confirms/replaces the 2.38 floor
   │                            └─ X-03 (pipeline on floor) ──► closes E-1′
   │
   ├─ D-03 + D-07 ──────────► M2 design may be rewritten (ref + write classes)
   ├─ D-04 ─────────────────► driver policy applied
   ├─ D-08 + D-09 ──────────► scope/requirements rewritten, estimates reissued
   └─ D-10 ─────────────────► measurement owner names a date; G4 text fixed
```

Nothing above authorizes M1–M6. `06-m0-readiness-checklist.md` states what M0 would contain if
approved and what evidence closes it.

---

## 5. Things this package deliberately does not do

| Not done | Why |
|---|---|
| Edit `docs/architecture/**`, `docs/adr/**`, `docs/product/**`, `docs/planning/**` | Owner authorization required; proposals are written to be applied verbatim |
| Decide A-1…A-10 | Recommendations are not approvals; register keeps them OPEN |
| Fix the M0 status sentences in `AGENTS.md` | That is the decision being requested; editing it now would manufacture consent |
| Write the measurement plan | Needs a named owner and a date (D-10); a plan without an owner is a wish |
| Implement anything | Explicitly out of scope for this phase |
| Re-run the review | Not ours to grade |

---

## 6. Residual disagreement with the review

Two places where this package does not simply accept the review:

1. **Tuple conflicts.** The review treats the tuple case as a thing to re-implement correctly. The
   experiments suggest it is not merely unsupported but **semantically near-nonexistent**: 60
   randomized trials (7 pair-clean) and 4 targeted constructions (rename+modify, add/add identical,
   mode change, attributes change) produced **zero** pair-clean/tuple-conflict cases. Because each
   sequential merge uses the original base as merge-base, a step-*k* conflict requires hunk overlap
   that a pairwise check already sees. We therefore remove the requirement rather than repair it
   (`02` §8).
2. **"Reduce M0 scaffolding" severity.** The review rates it High. We agree on the change but treat
   it as low-risk and cheap: deleting stub packages before they have consumers cannot break
   anything that exists. It is bundled into the M0 task rewrite (`06` §3), not a separate gate.
