# Ilvuno — Required Changes and Human Decisions

This is a review action list, not an implementation plan authorization. No product or architecture fixes were made during the audit. Order items by the gate they block.

## Before M0 can start

| Priority | Required action | Owner/decision | Evidence required to close |
|---|---|---|---|
| Blocker | Reconcile M0 approval status across AGENTS, roadmap, milestones, and G0 entry criteria. Record whether M0 is authorized and which document is authoritative. | Maintainer | One status, consistent in all four sources; recorded approval in the gate log or decision record. |
| Blocker | Record A-1, A-2, A-3, A-4, and A-9 (or dated explicit deferrals), which T-0001 lists as dependencies. | Product/maintainer; A-3 also engineering | Decision entries with owner, rationale, and dependent tasks. No recommendation treated as approval. |
| Blocker | Rewrite the E-1/T-0005 command and expected result around the documented two-branch `merge-tree` interface. Decide how to simulate sequential tuple behavior or demote tuple preflight to advisory. | Engineering recommendation; maintainer approves any guarantee change | Runnable spike spec, tuple fixture, and exact acceptance criterion that matches the algorithm. |
| High | Verify the minimum Git version on the intended CI matrix. The observed local version is 2.55.0; this does not prove the 2.38 floor. | Engineering | Recorded behavior on Git 2.38 and current Git on Linux/macOS, including `merge-tree` syntax and output shape. |
| High | Reduce or justify the M0 scaffolding scope. T-0001 creates stubs for nearly every future package although M0 only needs the repository harness. | Maintainer/engineering | A task scope that contains only approved foundations, with future seams documented rather than prebuilt unless required by an M0 acceptance criterion. |

## Before M2 candidate construction

| Priority | Required action | Owner/decision | Evidence required to close |
|---|---|---|---|
| High | Choose a valid candidate ref/worktree model: a normal branch under `refs/heads/` with an explicitly allowed naming convention, or a detached worktree plus explicit custom-ref management. | Maintainer/engineering; changes the ref invariant | Command sequence and recovery/cleanup protocol that yield exactly the documented ref in all supported Git versions. |
| High | Reconcile the security write boundary with Git-internal and child-process effects. Distinguish Ilvuno files/refs, Git object/worktree metadata, and arbitrary verification/repair effects. | Maintainer/security reviewer | Revised SEC-101 wording, scope policy, user-facing consent, and an instrumented write/effect matrix. |
| High | Decide custom merge-driver policy. Either refuse by default or require explicit consent and record the effective command/configuration. | Maintainer/security reviewer | Threat-model update plus marker-writing custom-driver scenario and a reproducibility/cache-key rule. |
| High | Align the determinism contract across AGENTS, ADR-0007, architecture 06, candidate cache key, and report. Include every semantic input that can change Git's result. | Engineering/maintainer | One guarantee statement; tests for config, attributes, driver/tool version, OS, and Git-version changes. |
| High | Resolve the product requirement-to-milestone map, especially ORQ-102/103 and XDC-102/103. Decide whether manifest and multi-transport behavior are P0, post-MVP, or schema-only properties. | Product/maintainer | Generated traceability: every P0 has one in-scope task and gate; deferred behavior has no MVP acceptance obligation. |

## Before G4/G6 product conclusions

| Priority | Required action | Owner/decision | Evidence required to close |
|---|---|---|---|
| Medium | Give Q-1/M-1 a named owner, date, eligible-run definition, repository/user diversity, exclusions, denominator, interval, and stopping rule. | Product/maintainer | Pre-registered measurement plan consistent with M-1, G4, BG-5, and the monetization gate. |
| Medium | Make BL-1 and false-blocking comparisons reproducible by independent operators. | Engineering/product | Fixed task set, operator script, timing boundaries, gold labels/adjudication method, and raw measurement packet. |
| Medium | Define what a bundle proves: integrity after sealing versus executor identity, execution environment, and approval authenticity. | Security/product | User-facing claims and tests that cover unsigned, signed, tampered, recomputed, and missing-evidence cases. |
| Medium | Specify default handling of absolute paths in child stdout/stderr before persistence/export. | Security/engineering | Fixture outputting repository and home paths; inspection of every persisted artifact and export. |
| Suggestion | Gate or defer passport, repair, approval signing, and auxiliary commands until the candidate-plus-checks-plus-evidence slice has been tried with users. | Product/maintainer | A smaller milestone plan or a written rationale for retaining each feature before M-1 is measured. |

## Before PG1 / any Claude adapter

| Priority | Required action | Owner/decision | Evidence required to close |
|---|---|---|---|
| Medium | Verify the supported Claude CLI/SDK protocol and version band, structured events, cancellation, and authentication behavior from current vendor documentation and conformance runs. | Adapter owner | Versioned conformance fixtures, supported-surface link, and tested typed refusal for unsupported/auth-required environments. |
| Medium | Review the vendor terms for the proposed local-login flow. Do not equate a consumer CLI login with API entitlement or put credentials in Ilvuno. | Maintainer/legal owner | Written terms decision attached to PG1; no credential-reading/storage path. |

## Decisions requiring human approval

`docs/decisions/open-questions.md` marks A-1 through A-10 as recommendations, not decisions. Its §7 requires each to be answered or explicitly deferred with a date. The immediate M0 dependencies are bolded.

| ID | Decision | Current recommendation in the docs | Blocks / timing |
|---|---|---|---|
| **A-1** | Product shape | Standalone CLI | **T-0001 / M0** |
| **A-2** | License | Apache-2.0 | **T-0001 / M0** |
| **A-3** | Implementation language | Go | **T-0001 / M0** |
| **A-4** | Windows in MVP | No; v0.2 | **T-0001 / M0** |
| A-5 | Passport location | Repo-committed by default; local-only optional | Passport tasks / M6 |
| A-6 | Repair executor | External command only | Repair task / M6 |
| A-7 | AI interpretation | Opt-in, non-blocking, removable | Interpretation/report scope |
| A-8 | Telemetry | None by default; optional counts only | Metrics/export implementation |
| **A-9** | Verification config location | Committed `ilvuno.toml` | **T-0001 / M0** |
| A-10 | Monetization gate | No paid feature before 20 weekly actives and M-1 ≥ 5% | Hosting/commercial scope |

Additional maintainer decisions are required for the candidate ref representation, custom merge-driver execution, allowed Git-internal writes, and any change to preflight guarantees. Q-1 also needs a named person and date; “U+PM” is a role label, not an accountable owner.

## Stop condition

This review does not authorize implementation. M0 should be reconsidered after the blocker rows above have recorded evidence, particularly the M0 approval/decision record and an executable E-1 definition. No M1–M6 work should be inferred from the existence of task specs or an “approved MVP scope” label.
