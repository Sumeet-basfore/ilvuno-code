# Ilvuno — Build Readiness Assessment

**Decision:** **REJECT for M0 start as currently documented**

**Readiness score:** **4.0 / 10**

This assessment answers whether the current documentation is a sufficient, approved basis to start M0. It does not assess implementation quality; the repository has no implementation yet.

## Transparent scoring rubric

Each dimension receives 0–2 points. Scores reward evidence and internal consistency, not document volume.

| Dimension | Score | Rationale |
|---|---:|---|
| Problem definition and evidence | 1.0 / 2 | The core problem is specific and the research records important evidence limits. No user interviews, actual usage, or willingness-to-pay data exist; real-world failure-rate evidence is sparse and conflicting. |
| Differentiation and scope | 1.5 / 2 | A refs-in, combined-candidate-and-checks CLI is distinct from a broad agent workspace, and dispatch/UI are deferred. The planned MVP still includes more than needed to test the value claim, and some P0/deferred boundaries conflict. |
| Technical feasibility of the stated design | 0.5 / 2 | The Git design has a concrete unsupported N-way command and a candidate ref/worktree mismatch. The remaining architecture is plausible but unimplemented. |
| Safety, evidence, and recovery | 0.5 / 2 | The threat model is unusually candid and recovery is well specified. The enforced write boundary does not match the specified Git metadata, custom drivers, and child effects; evidence hashes do not authenticate execution. |
| Approval and gate readiness | 0.5 / 2 | Milestone gates name artifacts and criteria, but M0 approval status conflicts across policy/planning, required A decisions are not recorded, and G0's E-1 task is not executable as specified. |
| **Total** | **4.0 / 10** | **Not ready to begin M0 under the current documented approvals and task contract.** |

## Gate-by-gate readiness

| Milestone/gate | Assessment | Reason |
|---|---|---|
| M0 / G0 | **Blocked** | A-1/A-2/A-3/A-4/A-9 are unrecorded; M0 status is inconsistent; E-1/T-0005 uses an unsupported command shape. Local Git 2.55.0 is above the stated 2.38 floor but does not verify the floor or CI matrix. |
| M1 / G1 | **Conditionally specifiable** | Read-only discovery and pinning have measurable acceptance criteria. They depend on a settled config and Git capability contract. |
| M2 / G2 | **Blocked by design corrections** | Candidate ref naming and tuple-preflight semantics do not match the declared namespace/API. Write accounting must also cover Git metadata/object writes. |
| M3 / G3 | **Mostly verifiable after upstream decisions** | Schema, report, bundle, and redaction tests are specific. Default path handling and the precise evidence claim need definition. |
| M4 / G4 | **Conditionally specifiable** | Process supervision, environment allowlists, and outcomes are well described, and no sandbox claim is explicit. Consent, side-effect language, and the real-world sample gate need alignment. |
| M5 / G5 | **Well specified, not proven** | The uncertainty model and crash matrix are detailed and testable. It depends on correct M2 refs and candidate rebuild semantics. |
| M6 / G6 | **Over-scoped for an unvalidated product** | Passport, approval, repair, stats, brief, docs, and release are grouped into a two-week milestone after a long build. The product plan itself identifies S1–S3 as the smallest useful release. |
| M7 / PG1 | **Deferred appropriately** | Provider adapters are not part of MVP. Claude protocol/auth/terms need verification before a future approval. |

## Readiness positives

- The MVP's core boundary is legible: Git refs and configured checks are the input/output contract; agents are not dispatched.
- The review corpus distinguishes verified source observations from inferred absence and admits that demand and pricing are unknown.
- Agent completion is explicitly separate from integration completion.
- Crash recovery uses durable intent, typed uncertainty, explicit operator decisions, and a broad planned fault matrix.
- Evidence export is designed for offline integrity checks and reports important unsigned/local trust limits.
- Mobile, hosting, and provider work are mostly deferred behind later gates.
- The task specs include scoped files, falsifiable acceptance criteria, evidence artifacts, and review checklists.

## Required human decisions and blockers

The maintainer must record A-1 through A-10 as decided or explicitly deferred with dates. A-1, A-2, A-3, A-4, and A-9 are direct T-0001 dependencies. The maintainer must also reconcile whether M0 is approved, pending approval, or authorized only after G0 entry criteria.

Engineering must replace the invalid tuple-preflight experiment and settle the candidate ref model before M0/G0 can credibly validate the foundation. The allowed-write contract and custom-driver policy must be resolved before candidate-building/security claims advance to M2. The product owner must supply a named owner and date for Q-1/M-1 before the roadmap invests beyond the falsification work.

The full decision register is in [03-required-changes.md](03-required-changes.md); issue-level evidence and verification steps are in [02-critical-risks.md](02-critical-risks.md).

## Verification record

- `git status --short --branch` at audit start: `## main...origin/main` with no listed changes.
- `git --version`: `git version 2.55.0`.
- No build or test command was run; no implementation/build system exists in this repository state.
- External checks used official Git documentation, Anthropic's official Agent SDK overview, and pinned source files from T3 Code and Orca. Links and evidence limits are recorded in the independent review and critical risks.
- No existing architecture, product, ADR, planning, policy, or task document was modified. Only the four requested review files were created.
