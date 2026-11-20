# Ilvuno — Independent Architecture Review

**Review date:** 2026-11-19

**Review scope:** readiness to begin Milestone 0, plus the requested product, architecture, security, verification, recovery, mobile, monetization, and governance questions.

**Final assessment:** **REJECT for M0 start as currently documented**

**Readiness score:** **4.0 / 10** (rubric in [04-build-readiness.md](04-build-readiness.md))

## Executive finding

Ilvuno has a coherent and differentiated product hypothesis: accept Git refs from any producer, construct a combined candidate, run configured checks against that candidate, and leave reviewable evidence. The proposed MVP avoids building another agent workspace like T3 Code or Orca. The research also correctly treats demand, pricing, and the real incidence of combined-only failures as unknown.

The documentation is not implementation-ready. The M0 tuple-conflict spike uses an unsupported `git merge-tree` argument shape; candidate worktree creation does not target the advertised ref namespace; the write-scope promise conflicts with Git's documented writes and with the planned child-process behavior; and M0 approval status is inconsistent across the governing documents. These are concrete blockers to the current M0/M2 plan, not claims about defects in an implementation (there is no implementation yet).

## Method and evidence boundary

I read the required policy, research, product, architecture, ADR, planning, decision, and task-spec documents. I independently inspected the pinned T3 Code and Orca source locations relevant to the integration boundary, and checked the official Git worktree, ref-name, merge-tree, and custom-merge-driver documentation. I also checked Anthropic's current Agent SDK overview for the deferred Claude adapter assumptions.

Repository observations were read-only: branch `main` was clean at review start; `git --version` reported `2.55.0`. This verifies only the local Git version, not the documented 2.38 floor or CI behavior. No tests or builds were run: there is no implementation or build system yet, and this audit did not authorize implementation. Competitor conclusions are limited to static source snapshots pinned in the research documents; absence claims remain inferences, not proof that no competing product has such a feature.

## Verified findings versus assumptions

| Claim | Status | Basis |
|---|---|---|
| The MVP consumes refs and does not launch or manage agents. | **Verified in the specification** | `docs/product/05-mvp-scope.md` §4.2; ADR-0010; architecture 03. |
| The sampled T3 Code snapshot's `thread.merge_back` creates context-transfer records. | **Verified at pinned source** | [T3 Code `Orchestrator.ts`](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/Orchestrator.ts#L6007). This supports a difference in purpose; it does not prove a universal absence claim. |
| The sampled Orca snapshot returns `ignored` for `merge_ready`. | **Verified at pinned source** | [Orca lifecycle reconciliation](https://github.com/stablyai/orca/blob/e348473086c7a064b8abd0a6a70a914269bc11df/src/main/runtime/orchestration/workers/lifecycle/lifecycle-reconciliation.ts#L118-L135). No conclusion is drawn about Orca's overall quality. |
| The real-world rate and buyer demand justify the product. | **Unverified** | Research and PRD state no interviews, usage, or pricing validation; the cited conflict-rate studies are not comparable populations. |
| A three-or-more-ref tuple can be passed to one modern `git merge-tree --write-tree` invocation. | **Contradicted by official documentation** | The supported command takes two branches; `--stdin` accepts separate two-branch merge records. See [git-merge-tree](https://git-scm.com/docs/git-merge-tree). |
| A provider's current CLI/SDK surface and login terms will support the future Claude adapter. | **Partly supported; still unverified for this use** | Anthropic documents `-p --output-format json` as a subprocess option for other languages, and publishes constraints on third-party use of Claude.ai login/rate limits. See [Agent SDK overview](https://docs.claude.com/en/api/agent-sdk/overview). |

## Answers to the 20 audit questions

| # | Assessment |
|---|---|
| 1. Real, differentiated problem? | **Plausible, not validated.** The integration-and-verification layer is distinct in the sampled source set. The PRD cites 27.67% in a simulated dataset versus 1 of 834 pairs in a real Django dataset; these do not establish the rate for target users. Frequency and willingness to pay remain unknown. |
| 2. Avoids cloning T3 Code or Orca? | **Yes in the stated MVP.** No agent dispatch, sessions, desktop/mobile workspace, or provider registry; the core consumes refs and verifies a combined candidate. |
| 3. Initial integration engine scoped appropriately? | **Conceptually yes; plan is too broad.** A local CLI is a sensible first surface. The current M0–M6 plan includes passport, approval signing, repair, analysis, evidence, recovery, and many command surfaces before demand is established. |
| 4. Architecture unnecessarily complex? | **Possibly.** The package boundaries and safety records have clear rationales, but a 33-task/46-engineering-day MVP is not the smallest way to test the core claim. M0 also scaffolds most future packages before they have consumers. |
| 5. Provider assumptions supported? | **MVP: yes, by avoiding them. Future adapters: not yet.** The Claude adapter is post-MVP and should remain gated; protocol/version compatibility and permissible authentication need a targeted spike and terms review. |
| 6. Git worktree handling reliable? | **Not as specified.** The lifecycle and crash handling are detailed, but `worktree add -B refs/ilvuno/...` supplies a branch name where the design expects a custom ref. Git stores branch heads under `refs/heads/`; see [git-worktree](https://git-scm.com/docs/git-worktree) and [git-check-ref-format](https://git-scm.com/docs/git-check-ref-format). |
| 7. Integration candidates reproducible? | **Not established.** Fixed identities/dates are sound foundations, but Git config/custom drivers are not part of a stable, consistent guarantee; the stated guarantee also differs across documents. |
| 8. Verification evidence trustworthy? | **For integrity, with limits.** Digests and offline verification can show whether a sealed bundle changed. They do not independently attest that a command ran in a trusted environment or that an unsigned local approval is authentic. The security document acknowledges this limit. |
| 9. Agent completion separate from integration completion? | **Yes.** INV-7, the model requirements, and T-COMP-4 explicitly prevent agent completion from becoming an integration verdict or approval. |
| 10. Retry/crash recovery sufficient? | **Well specified, unproven.** Intent-before-effect journaling, explicit uncertainty decisions, and the crash matrix are strong. They depend on fixing candidate refs and merge semantics first. |
| 11. Approval boundaries enforceable? | **Mostly specified.** TTY plus typed SHA and digest binding are concrete; unsigned approvals are visibly labelled. The first-run command-consent flow and custom Git merge-driver consent need consistent user-facing treatment. |
| 12. Security threats mitigated? | **Partly.** The docs honestly disclaim sandboxing and specify env allowlists. The write-scope contract omits or contradicts Git metadata/object writes and arbitrary configured merge drivers. |
| 13. Mobile continuity realistic? | **Yes as a deferred feature.** The host-authoritative/thin-client direction is plausible, and mobile is not needed for the local MVP. P0 transport requirements conflict with that deferral. |
| 14. Monetization plausible? | **A hypothesis only.** The proposed $25/seat team price and 3–5-team target are explicitly rough. There is no pricing or buyer evidence; hosting/team features depend on unbuilt security and identity surfaces. |
| 15. Can less deliver most value? | **Yes.** The product scope itself says S1–S3 is the smallest release that can answer whether the tool works for users. Defer passport, repair, and signing until the core workflow earns them. |
| 16. Benchmarks objective/reproducible? | **Synthetic correctness: mostly yes. Real-world comparisons: incomplete.** Fixture outcomes are falsifiable; manual time-to-answer, human false-block review, and small real-repo samples lack a complete sampling and labelling protocol. |
| 17. Milestones independently verifiable? | **The gate format is strong; key gates are not currently executable as written.** G0 depends on the invalid E-1 command, and M2's ref/preflight contracts conflict with Git behavior. |
| 18. Contradictory requirements? | **Yes.** M0 approval status, P0/P1 scope, mobile/transport, write boundaries, and determinism guarantees disagree across documents. See the findings below. |
| 19. AGENTS.md and CLAUDE.md correct/consistent? | **Consistent with each other.** `CLAUDE.md` imports `AGENTS.md` and adds tool-specific guidance. Their M0-only status conflicts with planning documents that call M0–M6 approved or say M0 approval is still pending. |
| 20. Ready to begin M0? | **No, as currently documented.** M0 task prerequisites A-1 through A-4 and A-9 are not recorded as decisions, the roadmap says M0 approval is pending, and T-0005's command is unsupported. |

## Finding index

| ID | Severity | Finding |
|---|---|---|
| BLK-01 | Blocker | The tuple-preflight/E-1 command cannot accept the documented N-way input. |
| BLK-02 | Blocker | M0 authority and required product decisions are unresolved and inconsistent. |
| HI-01 | High | Worktree branch naming cannot create the promised `refs/ilvuno/...` candidate ref. |
| HI-02 | High | The write-scope invariant does not match the specified Git and child-command side effects. |
| HI-03 | High | Custom merge-driver execution is both a code-execution boundary and an omitted determinism input. |
| HI-04 | High | P0/P1 and MVP/deferred requirement assignments conflict. |
| MED-01 | Medium | Real-world premise and false-block gates do not use one adequate, consistent sampling plan. |
| MED-02 | Medium | Human comparative benchmarks lack a reproducible operator/label protocol. |
| MED-03 | Medium | Bundle integrity is not execution provenance or authenticity; claims must stay scoped. |
| MED-04 | Medium | Future Claude adapter protocol/authentication assumptions need a provider-specific gate. |
| MED-05 | Medium | Absolute-path exclusion is required, but default normalization of arbitrary command output is unspecified. |
| SUG-01 | Suggestion | Start with the smallest candidate-plus-checks vertical slice and add secondary surfaces only after evidence. |

Full evidence, impact, corrections, and verification criteria are in [02-critical-risks.md](02-critical-risks.md). Required changes and human decisions are in [03-required-changes.md](03-required-changes.md).
