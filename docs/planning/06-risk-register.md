# 06 — Risk Register

Status: planning artifact. Reviewed weekly; formally re-scored at each gate. Owner `U` = the
maintainer/decision-maker, `E` = engineering, `PM` = product.

Scoring: **P** probability (1–5), **I** impact (1–5), **S** = P×I. Bands: 15–25 critical,
8–14 high, 4–7 medium, 1–3 low. A risk at ≥15 requires an active mitigation task, not a note.

---

## 1. Product / value risks

| ID | Risk | P | I | S | Trigger (early warning) | Mitigation | Contingency | Owner |
|---|---|---|---|---|---|---|---|---|
| **RK-01** | Real `combination_only` rate is near zero (research spans 27.67% simulated vs 1/834 real) | 4 | 5 | **20** | `M-1` < 2% after 50 multi-source runs | Ship the measurement in M4 (`T-0403`); report it to users; no marketing claim before data | Reposition as a review-artifact tool for large batches, or stop (product `07` §5) | U |
| **RK-02** | Repos lack runnable checks → value collapses to `unverified` | 3 | 4 | 12 | >40% of runs are `unverified` | `init` proposes commands from repo scripts; `INT-310` honesty; single-command path | Target monorepos with CI; drop the "any repo" framing | PM |
| **RK-03** | Developers will not change workflow for a CLI | 3 | 4 | 12 | `M-5` (week-2 repeat) < 15% | Report is the artifact (Marco asks for it); one-command usage; no migration | Ship the host plugin (M7 seam) rather than a UI | PM |
| **RK-04** | A first-party vendor ships combined verification inside one product | 3 | 3 | 9 | Vendor release notes | Cross-tool/cross-vendor scope; portable evidence; determinism | Narrow to "any agent, any repo, auditable evidence" | U |
| **RK-05** | Category churn (Vibe Kanban sunset, Crystal deprecated) signals no willingness to pay | 3 | 4 | 12 | zero team conversations convert | Do not build paid features before traction (`A-10`); stay free-first | Ship as a free OSS utility with no revenue plan | U |
| **RK-06** | "We could do this with a bash script" dismissal sticks | 3 | 3 | 9 | benchmark not cited in adoption conversations | Publish BL-3 comparison (`04` §5) incl. recovery/evidence gaps | Lead with an actual bash script that fails B-17/B-23 | PM |

---

## 2. Correctness / technical risks

| ID | Risk | P | I | S | Trigger | Mitigation | Contingency | Owner |
|---|---|---|---|---|---|---|---|---|
| **RK-07** | `merge-tree --write-tree` cannot see tuple-only conflicts (E-1) | 2 | 5 | 10 | E-1 negative | Run E-1 first (`T-0005`); preflight is `predicted`, always reconciled by a real merge | Demote preflight to advisory; build-then-report; budget +2 days | E |
| **RK-08** | Candidate SHAs are not reproducible across machines (E-2) | 2 | 4 | 8 | E-2 shows differing SHAs | Determinism envelope (ADR-0007); record versions; test on 2 OSes | Narrow guarantee to same-machine; keep cache keys but not "same SHA" claims | E |
| **RK-09** | Predictions disagree with real merges on rename/attribute-heavy repos (E-3) | 3 | 4 | 12 | `DET-4` corpus mismatch | 200-case corpus incl. renames; leak none of it into blocking findings until green | Findings become warnings; the "told you first" flow dies | E |
| **RK-10** | `resume` cannot be made unconditional in the timebox (E-4) | 3 | 4 | 12 | crash matrix has a case with no defined state | Uncertainty model with explicit decisions; 2.5-week M5 budget | Ship v0.1 with `abort` + re-run only, documented; never ship ambiguous resume | E |
| **RK-11** | Verification command mutates the candidate or commits | 3 | 3 | 9 | `candidate_mutated` in fixtures or real use | `tree_before/after` + HEAD check (`T-0404`); fail the run | Detect and report; do not silently accept | E |
| **RK-12** | Worktree add is too slow at 100k files (`NFR-102`/`NFR-108`) | 2 | 3 | 6 | bench > 5 s | `--no-checkout` + `reset --hard`; measure at M2 | Partial checkout mode before v0.1 (revisits sparse refusal) | E |
| **RK-13** | Git version/config variance changes results | 3 | 3 | 9 | user report of a differing result | Forced `-c` overrides; version recorded; `doctor` warns on untested versions | Document "tested with git 2.38–2.x"; add a version check | E |
| **RK-14** | Journal fsync cost or corruption handling fails under real load | 2 | 4 | 8 | `JRN-2` fuzz failures | 10k truncation fuzz in CI; append-only + chain | Shorten journal; drop resume for the affected stage | E |

---

## 3. Security risks

| ID | Risk | P | I | S | Trigger | Mitigation | Contingency | Owner |
|---|---|---|---|---|---|---|---|---|
| **RK-15** | Verification executes malicious repo code (by design) | 4 | 3 | 12 | a user runs Ilvuno on an untrusted repo | Documented non-claim; env allowlist; no auto-approve; `doctor` states isolation: none | Container executor behind the same interface (ADR-0008 revisit) | U |
| **RK-16** | Prompt injection via repo/agent text reaches a shell or a verdict | 3 | 4 | 12 | a finding or brief containing agent-authored commands | Structured briefs, no shell interpolation, classification gate (`09` §4.2) | Disable `--interpret` and brief generation until reviewed | E |
| **RK-17** | A crafted journal/passport forges an approval | 2 | 5 | 10 | digest mismatch found in a report | Hash chain + approvals outside repo + Ed25519 signature | Refuse all approvals until signed; require `key init` | E |
| **RK-18** | Secret leaks into evidence/logs | 3 | 4 | 12 | `RED-1` corpus regression; user report | Write-time redaction, 3 classes, `--keep-raw` refused with bundles | Report a leak process: prompt user to rotate, publish advisory | E |

---

## 4. Schedule / execution risks

| ID | Risk | P | I | S | Trigger | Mitigation | Contingency | Owner |
|---|---|---|---|---|---|---|---|---|
| **RK-19** | Two-agent parallel work creates design conflicts, not just merge conflicts | 3 | 3 | 9 | repeated rebases in W4/W8; rework > 15% | One owner per package (`03` §5); waves; owner-applies-patch rule | Drop to 1 agent; accept 1.0× instead of 1.7× | U |
| **RK-20** | A gate slips and the team "compensates" by skipping M5 | 3 | 5 | **15** | schedule pressure at week 10 | M5 is budgeted explicitly; gate G5 is hard; "ship without" options pre-written (`03` §8) | Release v0.1 as *alpha* with `abort`-only recovery, named as such | U |
| **RK-21** | Scope creep toward dispatch/UI ("while we're here") | 3 | 4 | 12 | any PR touching `agent` package before M7 approval | Import rule test; PRD §13 non-goals; roadmap §5.2 | Reject and cite the ADR | U |
| **RK-22** | Single-maintainer bus factor | 2 | 4 | 8 | no second reviewer available for a gate | Written decision log, ADRs, gate packets; adversarial self-review checklist | Recruit one reviewer before v0.1 | U |

---

## 5. Risk → milestone mapping

| Milestone | Risks resolved or reduced | Risks introduced |
|---|---|---|
| M0 | RK-07 (E-1 answered) | RK-19 (agents start) |
| M1 | RK-13 (version floor), RK-12 (measurement baseline) | — |
| M2 | RK-08, RK-09, RK-11 (fixtures) | RK-14 (journal), RK-17 (tamper surface) |
| M3 | RK-18 (redaction corpus), RK-16 (partial) | — |
| M4 | RK-01 (measurement instrument exists), RK-15 (documented) | RK-15 (execution starts) |
| M5 | RK-10, RK-14, RK-20 | — |
| M6 | RK-02, RK-03, RK-06 (evidence: outsider session) | RK-05 (expectation of revenue) |

Note the deliberate pattern: the risks that can kill the *product* (RK-01, RK-07, RK-09) are
touched in the first four milestones, while the risks that can kill the *schedule* (RK-19, RK-20)
are managed by process.

---

## 6. Weekly review ritual

Ten minutes, same day each week, against this file:

1. Any score changed? Update P/I with a one-line reason.
2. Any trigger fired? If yes, the mitigation becomes a task **this week**, or the contingency is
   invoked in writing.
3. Any risk ≥15 without an owning task? That is a planning bug; fix it in the same session.
4. New risk? Add with a first-pass score; do not debate exact numbers.

Anti-pattern to avoid: treating the register as documentation. A risk with no task and no decision
is a note, and notes do not reduce risk.

---

## 7. Accepted risks (explicitly not mitigated)

Listed so no reviewer thinks they were missed:

| Risk | Why accepted |
|---|---|
| No sandbox for verification | Would break most real toolchains (ADR-0008); documented non-claim |
| Worktrees are not a security boundary | Shared `.git` is inherent to git (architecture `05` §9) |
| No Windows in v0.1 | Cost/benefit at this stage (`A-4`) |
| No secret scanning | Different product; half-accurate scanning breeds false confidence |
| Unsigned approvals allowed locally | Single-user laptops; signatures available via `key init` |
| Determinism only within a git/OS version band | Cross-version merge output genuinely differs; recording versions is the honest answer |
| Enterprise features absent | No buyer yet (RK-05) |
