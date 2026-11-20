# 07 — M0 Reconciliation (revision 6)

Date: 2026-11-20 · Status: **M0 AUTHORIZED 2026-11-20 by the project owner (`06` §7), scope §3.1 and T-0001…T-0005. Gate G0 is not passed.** M1 and later are not authorized.
Approved: **D-01…D-06, D-16…D-18** and planning items **A-4, S-1…S-4, C-5, C-7** (`docs/decisions/open-questions.md` §0).
Entry gate (`readiness/06` §1): **10 PASS · 0 OPEN · 0 FAIL.** Implementation evidence and G0 status: `readiness/06` §4 and `planning/gate-log.md`.

Revision 6 (2026-11-20): owner authorization of M0 recorded; implementation evidence (T-0001 to T-0005) and G0 status recorded. Revision history: rev 1 (no decisions; NO-GO). rev 2 (D-01…D-06 recorded). rev 3 (planning approvals recorded, E-10 evidence produced, reference image rebuilt, C-5 verified, Apache-2.0 draft prepared, ADR-0003 and related Tauri references corrected). rev 4 (D-16…D-18 recorded, E-3 satisfied, T-0001/T-0005 corrected, independent review requested). rev 5 (this file): E-9 PASS on the independent Codex review; T-0005 adds path preflight and AC-12/AC-13; totals 9 PASS · 1 OPEN.

---

## 0. Files changed in revision 4

Policy files, flagged for human review (AGENTS §14): `AGENTS.md` (§1 row, §4.1 bullet).

| File | Change |
|---|---|
| `docs/decisions/open-questions.md` | §0: planning approvals table (A-4, S-1…S-4, C-5, C-7) with basis and documents updated. **Rev 4:** D-16 (M0 lint), D-17 (repository and module path), D-18 (Apache-2.0 text) rows added with scope and approver; D-03 rationale updated; status line now 9 decisions. |
| `docs/readiness/05-owner-decision-register.md` | **Rev 4:** status line records D-16…D-18 as approved, with no request row (approved directly). |
| `docs/adr/ADR-0003-no-ui-in-mvp-tauri-later.md` | Retitled "No UI in V1. Desktop shell not decided". Added: this ADR approves no framework. Tauri and React are candidates only, and Electron is not a candidate. Evaluation row, decision item 2 and risk row no longer present the shell as approved. |
| `docs/adr/ADR-0001-language-and-runtime.md` | Three passages that treated the Tauri two-language future as "accepted for now" now say "not approved; arises only if a shell is approved". |
| `docs/adr/README.md` | Index statuses: 0001 accepted (D-04); 0003 accepted (D-02), and its description no longer says "Tauri+React later". |
| `docs/planning/02-milestones.md` | M0 deliverable 6 (F1–F8, F10 ported; F9 kept). Dependencies line (A-4 excluded from V1). M8 sketch: shell is a candidate, not approved. |
| `docs/planning/05-release-gates.md` | G0 criterion 3 (S-1). Criterion 5 replaced by sequential-integration reference conformance (S-3). Entry line: Git ≥ 2.38.1. "If E-1 is negative" rule replaced. Sign-off artefact. |
| `docs/planning/01-master-roadmap.md` | B-1 resolved (A-1…A-4 and A-9 decided). §9.4 status line. |
| `docs/readiness/06-m0-readiness-checklist.md` | Header (**9 PASS · 1 OPEN · 0 FAIL**). E-3 and E-9 rows PASS; E-8 records the revised T-0005 and pending independent acceptance. §3.1 fixture row. G0-3 and G0-6 (S-1, S-3). §6 status line. §7: decisions D-16…D-18 and R3-1…R3-4 recorded; sign-off still unsigned. |
| `docs/readiness/LICENSE-draft-Apache-2.0.txt` | Apache-2.0 text, copied from apache.org (`cfc7749b…`, 11,358 bytes). **Rev 4:** approved by D-18; T-0001 copies it to root `LICENSE`. |
| `docs/research/09-git-floor.md` | **New.** X-09 evidence note (E-10). |
| `docs/research/09-git-floor-evidence/` | **New.** Probe scripts, logs, Docker build log, SP-1R3 run logs. |
| `AGENTS.md` | §1: decided-items row (planning items added). §4.1: S-2 bullet (Go production; Python, Bash and Docker for tooling only). **Rev 4:** §4.1 lint bullet now records the approved M0 exception (D-16) and preserves `staticcheck` as the future requirement. **Policy file, needs review.** |
| `docs/tasks/T-0001.md` | Windows excluded from V1 (A-4). CI scope approved (S-4), workflow reviewed before merge. **Rev 4:** module path fixed by D-17 (step 1, AC-7, checklist); lint aligned with D-16; `LICENSE` copied verbatim from the D-18 text. |
| `docs/tasks/T-0003.md` | S-1 approved: ported F1–F8 and F10, F9 fault-injection test (AC-7), other classes deferred. |
| `docs/tasks/T-0005.md` | S-2 and C-5 and C-7 approved, with verified values. Step 2 names the three C-5 line references. Step 6 updated (note written as E-10 evidence). **Rev 4:** AC-5 narrowed to production paths (`cmd`, `internal`, `Makefile`, `.github`), so historical documents and reference fixtures may mention `merge-tree`. **Rev 5:** explicit path preflight (absolute/canonical, ownership, mode 700, symlink components, outside the worktree) and AC-12 (comparison mismatch) / AC-13 (invalid root), with AC-9…AC-11 and the EXIT-trap semantics preserved. |
| `docs/reviews/m0-final-review-request.md` | **New (rev 4).** Focused request for independent review of E-3, E-8, E-9, lint consistency, module path, SP-1R3 conditions and CI ownership. |
| `docs/readiness/07-m0-reconciliation.md` | This file (rev 5). |

**Decisions recorded (rev 4):** D-16 (M0 lint), D-17 (canonical repository and module path), D-18
(Apache-2.0 text). `T-0001` and `T-0005` were corrected against `docs/reviews/m0-final-entry-review.md`
(E-8/E-9). E-3 and E-9 are satisfied; E-8 remains **OPEN** pending independent acceptance of the
revised `T-0005` (request: `docs/reviews/m0-final-review-request.md`).

**Not changed:** `docs/experiments/` (harness, fixtures, `expected.json`, `oracle.json`, verdict conditions); existing review verdicts in `docs/reviews/` (a new request file is added); `docs/tasks/T-0002.md`, `T-0004.md`; `docs/tasks/README.md`; `CLAUDE.md`; product docs; ADR-0002, -0004, -0005, -0006, -0007; `planning/03`; `.gitignore` (already modified before this session).

**Not done:** no commit, push, merge or deploy. No production code. No dependency or third-party lint tool installed; `gofmt` and `go vet` are the host Go toolchain's (go1.27.0). No `testdata/reference/` directory created (that is M0 work). The `docs/readiness/06` §7 signature and the `05` §8 block were not signed.

**Built and verified in revision 3 (not M0 implementation):** the Git 2.38.1 reference image (rebuild); the patched harness copy (scratch only); the X-09 probes on two Git versions; the SP-1R3 matrix and test suite on both versions with the patched copy.

---

## 1. Decisions recorded

| ID | Maps to | Selection (as approved) | Approved |
|---|---|---|---|
| D-01 | Milestone authorization | Owner signs the readiness checklist before each milestone | 2026-11-20 |
| D-02 | A-1 product shape | V1 is a standalone CLI | 2026-11-20 |
| D-03 | A-2 license | Apache-2.0 | 2026-11-20 |
| D-04 | A-3 language | Go | 2026-11-20 |
| D-05 | Platforms and Git floor | Linux-first; macOS tested in CI; Git ≥ 2.38.1 initially | 2026-11-20 |
| D-06 | A-9 config location | Repository-committed `ilvuno.toml` | 2026-11-20 |
| A-4 | Windows | Excluded from V1. Inclusion in future versions remains OPEN | 2026-11-20 |
| S-1 | M0 fixture scope | Port F1–F8 and F10 during M0. Retain F9 for fault-injection testing. Defer additional fixture classes | 2026-11-20 |
| S-2 | Tooling | Python 3, Bash and Docker for development and test tooling only. Go remains the production CLI language | 2026-11-20 |
| S-3 | G0 criterion 5 | Replace the obsolete merge-tree criterion with sequential-integration reference conformance, preserving meaningful acceptance coverage | 2026-11-20 |
| S-4 | CI | Linux/macOS GitHub Actions scope approved, subject to review of the actual workflow before merge | 2026-11-20 |
| C-5 | Harness test copy | Parameterize the hard-coded temporary path, preserving existing assertions | 2026-11-20 |
| C-7 | Reference environment | Rebuild Git 2.38.1 from the pinned Dockerfile and record the image digest | 2026-11-20 |

All approvals are by the project owner, as stated in the conversation. No name is recorded in the repository. If the maintainer under AGENTS §10.2 is a different person, that person must countersign the items that AGENTS §10.2 reserves to the maintainer (S-3, S-4, and the G0-3 change).

---

## 2. Remaining open decisions

| ID | Item | Note |
|---|---|---|
| A-4 (future) | Windows in later versions | Undecided. Not needed for M0. |
| D-07 | Candidate ref model and pipeline | M2 blocker |
| D-08 | Merge-driver policy | M2. The floor lacks `check-attr --source` (`09` G-3) |
| D-09, D-10 | Write-scope classes; determinism scope | M2 |
| D-11…D-15 | Scope split, requirements, measurement, claims, adapter deferral | Later gates |
| A-5, A-6, A-7, A-8, A-10 | Passport location, repair executor, AI interpretation, telemetry, monetization gate | Not M0 blockers |
| Q-1…Q-8, T-1…T-14 | Product and technical questions (`open-questions.md` §2–§3) | Not M0 blockers |
| Desktop shell | Not decided. Tauri + React is a candidate only (ADR-0003) | Not needed for V1 |

---

## 3. Status contradictions

### 3.1 Resolved in revision 3

| Contradiction | Resolution |
|---|---|
| ADR-0003 title and decision presented Tauri + React as the approved shell ("preferred when approved", "explicitly accepted" as a risk) | Retitled and rewritten. The ADR approves no framework. V1 being CLI-first does not approve a shell. |
| ADR-0001 "Two-language future… Accepted for now" | Now "not approved", arising only if a shell is approved and Tauri is chosen. |
| ADR index: 0003 "Tauri+React later" as proposed | Accepted (D-02) and described as "desktop shell not decided". |
| planning/02 M8 sketch: "Tauri + React shell" | "Desktop shell, candidate Tauri + React, not approved". |
| G0 criterion 5 asked the withdrawn merge-tree question | Replaced (S-3). |
| G0 criterion 3 and G0-3 said "nine fixtures" | F1–F8 and F10, with F9 as fault-injection test (S-1). |
| Floor "2.38" in planning/05 entry line | Git ≥ 2.38.1 (D-05). |
| B-1 "A-4 unanswered" | A-4 decided. B-1 resolved. |
| E-5 and E-10 pending on Windows and on X-09 evidence | E-5 PASS. E-10 PASS with documented gaps (`09` §4). |
| `open-questions` §1 status note said A-4 OPEN | Updated to DECIDED for V1. |
| AGENTS §4.1 did not state the tooling boundary | S-2 bullet added (policy file, for review). |

### 3.2 Remaining drift (not edited)

| Item | Where | Note |
|---|---|---|
| ADR-0004 says "≥ 2.38" and centres on `merge-tree` | ADR-0004 (proposed) | The reference no longer uses merge-tree for the candidate. Floor should read 2.38.1 if the ADR is kept. M2 concern. |
| ADR-0006 preflight design | ADR-0006 (proposed) | M2. Depends on D-07. |
| `planning/03` lines 57, 125, 137 treat E-1 as an M0 single point | `planning/03` | Not in the approved edit set. Owner may ask for an update. |
| `docs/tasks/README.md` lists T-0204 (merge-tree preflight, M2) | `docs/tasks/README.md` | Deletion proposed in `readiness/02` §11; not applied. |
| `docs/product/05-mvp-scope.md` §4 states "Windows: No, v0.2" as a recommendation | product `05` | Now consistent with the A-4 decision for V1. The v0.2 wording is a future-version claim, which A-4 leaves OPEN. |
| G0-6 wording in `readiness/06` §4 is a gate criterion | `readiness/06` §4 | Updated to S-3 scope. Gate criteria change only with maintainer approval (AGENTS §10.2). |
| `readiness/06` G0-5 (anchored refs) is a different check from `planning/05` criterion 5 | both | Not a conflict. The S-3 approval covers only the merge-tree criterion. |

---

## 4. Entry-gate status (`readiness/06` §1)

Definitions: **PASS**: the named evidence exists and was checked. **OPEN**: waiting on an owner act or an independent review. **FAIL**: named evidence is absent and engineering can produce it.

| # | Result | Evidence |
|---|---|---|
| E-1 | **PASS** | D-01 recorded in `open-questions.md` §0. |
| E-2 | **PASS** | D-02 recorded. |
| E-3 | **PASS** | Decision PASS (D-03). "LICENSE content approved" is met by D-18 (2026-11-20): the exact draft is approved, sha256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`, 11,358 bytes. The root `LICENSE` is created by T-0001 from that text; pre-M0 absence is expected. |
| E-4 | **PASS** | D-04 recorded. ADR-0001 accepted. |
| E-5 | **PASS** | D-05 (Git ≥ 2.38.1, Linux-first, macOS in CI) and A-4 (Windows excluded from V1) recorded. |
| E-6 | **PASS** | D-06 recorded. |
| E-7 | **PASS** | The four files link to `06` §7 and `07` and make no conflicting authorization claim (checked 2026-11-20). The owner may read the criterion more strictly. If so, E-7 becomes OPEN. |
| E-8 | **OPEN** | `T-0005` now specifies R3-1…R3-4, path preflight, isolated fresh oracle, status-preserving trap, no stale reads, and negative tests AC-9…AC-13, including comparison mismatch and invalid-root cases. Independent acceptance is needed. |
| E-9 | **PASS** | Independent Codex review of the corrected `T-0001` (2026-11-20), reported by the project owner: D-17 module path, D-16 lint, platform-specific checks, T-0001/T-0005 CI split, no stub criteria. The verdict text is not committed in this repository. |
| E-10 | **PASS, with documented gaps** | `docs/research/09-git-floor.md`. On Git 2.38.1, every X-09 behaviour that the M0 design uses matches Git 2.55.0, and the SP-1R3 matrix passes 28/28 on both. Gaps G-1…G-6 are recorded. `check-attr --source` is absent on the floor (G-3), which is an M2 concern. |

**Result: 9 PASS · 1 OPEN · 0 FAIL.**

---

## 5. Blockers and prerequisites for starting M0

### 5.1 Blocking the M0 start

| # | Item | Who |
|---|---|---|
| 1 | **M0 authorization.** The owner signs `06` §7 with the statement given there. | Owner (**not given**) |
| 2 | **E-8.** An independent reviewer (not the author) accepts the revised `T-0005`, including path preflight and AC-9…AC-13. | Second reviewer |

**Resolved 2026-11-20, not start blockers:** E-3 (D-18, Apache-2.0 text), E-9 (independent Codex
review of T-0001), lint policy (D-16, M0 lint scope; AGENTS §4.1 carries the exception), and the
canonical module path (D-17).

### 5.2 Blocking later steps (not the start)

| Step | Blocker |
|---|---|
| Creating `.github/workflows/ci.yml` | S-4 approves the scope. The workflow is reviewed before merge (AGENTS §10.2). |
| Applying C-5 in `testdata/reference/` (T-0005) | Happens after authorization. The patch is verified (`09` §5). |
| Using the C-7 image in CI | The image is built (ID in §0). CI must pin it by ID. |
| G0 sign-off | Maintainer countersign of S-3 (G0 criterion 5 and G0-6) and S-1 (G0-3), if the maintainer is not the owner (AGENTS §10.2). |
| macOS verification | Must run in CI before G0 (`09` G-2). |

### 5.3 Engineering choices (made during T-0001, not approvals)

- Go toolchain version and CI runner images (pinned in T-0001).
- Python 3 version for the harness (C-9). The `__pycache__` files show CPython 3.10 and 3.14 were used. Pin one.
- The floor check in CI (2.38.1).

### 5.4 SP-1R3 operational conditions

`T-0005` states the conditions as **R3-1…R3-4** (fresh canonical private scratch; checked exit status
and no consumption of failed output; fresh oracle output with no stale-file read; editable guard
files and no same-UID adversary). They are derived from
`docs/reviews/sp1r3-final-independent-verdict.md` and replace the stale `C-1…C-9` cross-reference
to an earlier revision. **C-5** and **C-7** are approved planning items (§1), not remediation
labels. Any regeneration of `oracle.json` follows R3-3.

---

## 6. Recommendations (for items still open)

| Item | Recommendation |
|---|---|
| Lint tools | **Resolved (D-16):** M0 uses `gofmt`, `go vet` and the import-rule test. `staticcheck` is a later, separately approved dependency. |
| LICENSE | **Resolved (D-18):** the apache.org text is approved as it is, with no copyright-holder line and the standard appendix placeholders. |
| Future Windows (A-4 remainder) | Keep OPEN until a version plan needs it. Nothing in V1 depends on it. |
| Desktop shell | Keep the candidate status. Any proposal needs its own decision and its own ADR. |
| G-4 (image provenance) | Optional before a release claim. Verify the Git tarball signature. |

---

## 7. Verification performed (2026-11-20)

| Check | Result |
|---|---|
| Reference image rebuild: `docker build --no-cache -f Dockerfile.git238` | Build exit 0. Dockerfile Git checksum passed. Image `sha256:1044b988f915…` |
| SP-1R3 matrix on the rebuilt image (Git 2.38.1) | PASS 28/28, exit 0 |
| SP-1R3 `tests.sh` on the rebuilt image, C-5 patched copy | 95 passed, 0 failed, exit 0 |
| SP-1R3 matrix and `tests.sh` on the host (Git 2.55.0), C-5 patched copy | PASS 28/28; 95 passed, 0 failed |
| C-5 patch: three lines changed, no `/tmp/r3-cl` reference left | Confirmed. The patched copy is scratch only |
| X-09 probe on both versions | Matches on X-01, X-02b, X-04, X-04b, X-05, X-06. Differs on X-10 (`check-attr --source`) |
| X-05 driver execution under `merge-tree`, both versions | Driver ran on both (benign marker) |
| Apache-2.0 draft compared with apache.org | Identical: sha256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`, 11,358 bytes. The local nextcloud copy (10,280 bytes) differs and was not used |
| D-16…D-18 recorded | Present in `open-questions.md` §0 with scope and approver; `05` status line updated; `06` E-3 and E-9 PASS; header 9 PASS · 1 OPEN |
| Canonical module path | D-17 records the repository and module path; T-0001 step 1, AC-7 and the review checklist use `github.com/Sumeet-basfore/ilvuno-code`; no other path approved |
| Lint consistency | AGENTS §4.1, D-16, T-0001 step 5 and `06` §7 all say `gofmt` + `go vet` + import-rule test; `staticcheck` deferred; no `golangci-lint` |
| Stale `C-1…C-9` condition references | None in T-0005 or `06` §7; both use R3-1…R3-4 (C-1…C-9 remains only as a historical label) |
| Sign-off blocks (`06` §7, `05` §8) | Both unsigned |
| `docs/experiments/` | Not written to in this revision. Last modified 13:06 today, before the SP-1R3 verdict (13:50) |
| Not run: CI (none exists); macOS (C-8); `make` (no Makefile); `oracle` (rewrites `oracle.json`, C-6); Git 2.39 (G-1) | Not evidence |

Probe and matrix runs used private temporary directories and copies under the session scratchpad. The workspace harness was not changed.

---

## 8. M0 authorization statement

The exact statement for the owner's separate approval is in `docs/readiness/06-m0-readiness-checklist.md` §7. It is not signed. This file does not sign it, and this revision does not authorize M0.
