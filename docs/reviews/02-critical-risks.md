# Ilvuno — Critical Risks and Findings

Each finding identifies the observed evidence, impact, affected components, a correction, and a verification method. “Verified” refers to the present documentation or cited source behavior; it does not imply any implementation has been tested.

## Blockers

### BLK-01 — The N-way merge-tree preflight command is unsupported

**Evidence.** Architecture 06 §4.1 and task T-0005 step 2 pass `<base> <a> <b> <c>...` to `git merge-tree --write-tree`. The official [git-merge-tree manual](https://git-scm.com/docs/git-merge-tree) documents a modern invocation with two branches. `--stdin` accepts one record per merge, each containing two branches (and optionally a base); it is not an N-way merge interface. The manual also says `--write-tree` creates a top-level tree object. See [architecture 06](../architecture/06-integration-intelligence.md#L86-L112), [ADR-0006](../adr/ADR-0006-candidate-construction.md#L24-L36), [T-0005](../tasks/T-0005.md#L19-L27), and [T-0204](../tasks/T-0204.md#L1-L24).

**Impact.** The current E-1 experiment cannot produce the proposed tuple result, so G0 cannot answer its stated question using its specified command. T-0204 depends on that result and promises pairwise plus tuple predictions. “No mutation” is also too broad for `--write-tree`, which can write objects even though it does not touch the working tree or index.

**Affected component.** Architecture 06, ADR-0006, M0 task T-0005/G0, M2 task T-0204/G2, PRD INT-108.

**Recommended correction.** Choose and document a valid algorithm before M0 is considered complete. To predict sequential integration, simulate the exact order using successive two-branch merges and explicit intermediate results, or make preflight advisory and report conflicts discovered during candidate construction. Do not treat `--stdin` as an N-way solution. Define whether object-database writes are permitted and revise “before mutating anything” accordingly. Record any semantic change in the appropriate decision record.

**How to verify.** On the minimum supported Git and current Git, use the triple-conflict fixture to show all three pairs are clean, then compare the chosen preflight algorithm with the actual A→B→C result. Capture argv, exit status, paths, refs, index, worktree metadata, and object-store effects. G0/G2 acceptance tests must exercise the same supported algorithm.

### BLK-02 — M0 has no single approved baseline

**Evidence.** `AGENTS.md` says only M0 is approved; the roadmap says it is “awaiting M0 approval”; planning 02 says M0–M6 are approved MVP scope. T-0001 requires A-1, A-2, A-3, A-4, and A-9, while `docs/decisions/open-questions.md` says nothing in A-1–A-10 is decided unless marked and that Phase 2 exits only after all ten are answered or explicitly deferred. See [AGENTS.md](../../AGENTS.md#L14), [roadmap](../planning/01-master-roadmap.md#L3), [milestones](../planning/02-milestones.md#L3), [T-0001](../tasks/T-0001.md#L10), and [open questions](../decisions/open-questions.md#L9).

**Impact.** Contributors cannot tell whether they may begin M0 or which foundational choices they are implementing. License, language, CLI shape, platform support, config placement, and task scope are product/governance choices, not safe inferences from recommendations. Current user authorization covers this audit and its four files only.

**Affected component.** `AGENTS.md`, `CLAUDE.md`, planning 01–02, open questions, T-0001, G0 entry criteria.

**Recommended correction.** A maintainer should record A-1–A-10 as decided or explicitly deferred with owner/date, state one authority for milestone authorization, and reconcile the status language across policy, roadmap, and milestone plan. The immediate T-0001 prerequisite set is A-1–A-4 and A-9; other decisions must be resolved before their dependent tasks.

**How to verify.** A clean-clone reviewer can compare the decision table, AGENTS status, roadmap status, T-0001 dependencies, and G0 entry packet. They must agree, with no recommendation presented as an approval.

## High risks

### HI-01 — Worktree creation uses a branch name where the design expects a custom ref

**Evidence.** Architecture 05 runs `git worktree add -B refs/ilvuno/candidates/<run_id> ...` and says this creates the candidate under `refs/ilvuno/`. The [git-worktree manual](https://git-scm.com/docs/git-worktree) defines `-B` in terms of `<new-branch>`; the [git-check-ref-format manual](https://git-scm.com/docs/git-check-ref-format) states branch heads are stored under `refs/heads/`. A read-only `git check-ref-format --branch refs/ilvuno/candidates/audit-probe` on the local Git 2.55.0 accepts that string as a branch shorthand; that does not create a custom ref under `refs/ilvuno/`. See [architecture 05](../architecture/05-worktrees.md#L101), [policy task T-0202](../tasks/T-0202.md#L1), and [T-0205](../tasks/T-0205.md#L1).

**Impact.** The desired ref namespace and the actual worktree-associated branch cannot both be assumed from the documented command. This can violate the only-ref-written invariant, misclassify Ilvuno worktrees, and break abort/recovery cleanup.

**Affected component.** Architecture 05, ADR-0006, `refs/ilvuno/*` invariant, T-0202/T-0205, crash recovery.

**Recommended correction.** Decide between a Git-managed branch whose short name is under a dedicated `ilvuno/...` path (and revise the allowed namespace) or a detached worktree plus explicit custom-ref lifecycle. Document one exact ref representation and use it consistently in policy, discovery, cleanup, and recovery.

**How to verify.** On every supported Git version, create and remove a fixture worktree, then inspect `git worktree list --porcelain -z` and `git for-each-ref`. Assert the exact expected ref, no user ref changes, and successful crash cleanup.

### HI-02 — The write-scope guarantee conflicts with specified side effects

**Evidence.** PRD SEC-101 says writes are limited to `.ilvuno/`, `refs/ilvuno/*`, and the task directory. Architecture 01 lists `.git/worktrees/` as a `gitx` write; architecture 06 uses `merge-tree --write-tree`, which writes Git objects; `init` writes `ilvuno.toml` and `.gitignore`; `key init` writes under `~/.config`; and verification commands can write to `$HOME` and use the network. Architecture 07 explicitly permits those child effects, while architecture 09’s `Scope.WriteRoots` omits Git metadata/object storage and then says subprocesses are constrained. See [SEC-101](../product/02-prd.md#L811-L820), [module map](../architecture/01-overview.md#L74-L87), [worktree shared state](../architecture/05-worktrees.md#L139-L157), [verification environment](../architecture/07-verification.md#L117-L139), and [security scope](../architecture/09-security.md#L174-L211).

**Impact.** The central safety invariant and the tests for it have no unambiguous set of allowed effects. A path audit can fail a legitimate Git operation or pass while missing object-store/worktree metadata changes. A check that writes outside its worktree cannot be made safe by constraining only its `cwd`; the current docs correctly say no sandbox exists, but SEC-101 reads as a stronger guarantee.

**Affected component.** PRD SEC-101/SEC-106, INV-1, policy scope, `init`, key management, Git worktree/preflight, verification, safety fingerprint, benchmark BG-3.

**Recommended correction.** Separate and name three classes: Ilvuno-owned files/refs; Git-managed metadata/object writes; and arbitrary verification/repair child effects. Define which are permitted, which are observed, and which require consent. State that “no network” means no Ilvuno service calls unless children are actually isolated. Ensure consent for executable project configuration is visible in the user journey. Narrow SEC-101 to a promise the implementation can enforce without calling a worktree a sandbox.

**How to verify.** Instrument writes for init, preflight, worktree creation, candidate merge, verify, repair, failure, and crash paths. Assert each class against its declared policy; include an object-writing merge, `.git/worktrees`, a child writing to `$HOME`, and a child making an outbound request. Verify consent and report language match observed behavior.

### HI-03 — Custom Git merge drivers can execute configured commands and alter results

**Evidence.** The official [gitattributes manual](https://git-scm.com/docs/gitattributes) documents custom merge drivers as commands configured in repository or user Git config and selected by attributes. The [git-merge-tree manual](https://git-scm.com/docs/git-merge-tree) says modern merge-tree uses the same merge features as `git merge`. Architecture 05 only forces hooks, signing, and conflict style; ADR-0006 says a user's custom driver is merely recorded. No consent or effective-driver fingerprint is specified.

**Impact.** A merge path may run a configured executable with the user's permissions, potentially writing beyond the candidate or making network calls. Its behavior may vary by machine or installed tool version, so identical refs, Ilvuno binary, Git version, and OS family may still produce different results. This is an execution boundary and a determinism input, not evidence of a vulnerability in an existing binary.

**Affected component.** Candidate/preflight engine, security model, determinism envelope, Git config handling, write-scope audit.

**Recommended correction.** Make the policy explicit: refuse custom merge drivers by default, or require explicit consent after showing the effective driver and its command. Record the relevant attributes/configuration and driver/tool identity in evidence. If drivers are disabled, state that candidate semantics differ from the user's configured merge. Include this input in the reproducibility/cache contract.

**How to verify.** Create a fixture with an attribute-selected custom driver configured to write a marker outside the repo. Assert refusal without consent; after consent, assert exact logged driver identity, bounded behavior, and recorded result. Repeat with changed driver/config/version and ensure the determinism/cache key either changes or the operation is refused.

### HI-04 — Requirements labelled MVP/P0 disagree with deferred scope

**Evidence.** PRD ORQ-102/103 are P1 in §10.7 but §16 says they bind the MVP; product 05 lists stream manifests in scope; the roadmap says it schedules P0 requirements. XDC-102 is P0 and its acceptance requires three transports, while the product defers mobile and relay/hosted transport. See [PRD requirements](../product/02-prd.md#L350-L362), [MVP binding statement](../product/02-prd.md#L600-L607), [cross-device requirements](../product/02-prd.md#L407-L417), and [MVP scope](../product/05-mvp-scope.md#L128-L179).

**Impact.** Estimates, task coverage, and gate traceability are not stable. A team can declare M6 complete while an item still called P0 or “binding MVP” is absent, or expand MVP to include deferred relay behavior without an approved decision.

**Affected component.** PRD, MVP scope, roadmap, milestone/task traceability, G6.

**Recommended correction.** Publish one requirement-to-milestone matrix with one priority and one acceptance test per requirement. Explicitly move transport-agnostic schema properties that are tested locally apart from transport implementations; state whether manifest input/output is part of v0.1.

**How to verify.** A generated traceability check must show every P0 maps to an in-scope task and gate, every deferred item has no MVP acceptance criterion, and no requirement has contradictory priority/status.

## Medium risks

### MED-01 — Product-value and false-block thresholds use inconsistent samples

**Evidence.** M-1 calls for at least 50 integrations across at least 20 users/repos before acting; planning G4 asks for only five real repositories and allows continuation if none show a combined-only failure; BG-5 uses five repositories/100 runs. See [metrics](../product/07-success-metrics.md#L27-L39), [release gate G4](../planning/05-release-gates.md#L129-L150), and [benchmark gates](../planning/04-benchmarks.md#L90-L113).

**Impact.** G4 can advance the project without meeting the sample that the product identifies as decisive. Fifty runs still provide a noisy estimate around a 5% event rate; five repositories are especially vulnerable to repository selection effects.

**Affected component.** M-1, Q-1/U-3, product gates, G4/G6 investment decision.

**Recommended correction.** Keep fixture correctness separate from market evidence. Pre-register a sampling plan with eligible integrations, repo/user diversity, exclusions, denominators, confidence intervals, stopping rules, and the action at each result. Do not treat a small convenience sample as proof of a 5% prevalence.

**How to verify.** Before collection, a reviewer can reproduce the metric from a documented anonymized data table and recompute its interval and verdict. G4 and product scorecard must have distinct names if their gates answer different questions.

### MED-02 — Comparative benchmark protocol has subjective, under-specified parts

**Evidence.** Benchmarks specify exact synthetic outcomes, but BL-1 measures a person's time to “know the answer,” and BG-5 uses manual audit of blocking findings without a stated annotator or gold-label protocol. See [benchmark baselines](../planning/04-benchmarks.md#L1-L28), [BG-5/BG-8](../planning/04-benchmarks.md#L90-L113), and [paired-run protocol](../planning/04-benchmarks.md#L145-L164).

**Impact.** The 5× superiority and ≤2% false-block claims cannot yet be reproduced by an independent team from the written method alone.

**Affected component.** Planning 04, BG-5/BG-8, release claims.

**Recommended correction.** Define a fixed task set, start/stop events, operator instructions, handling of conflict-reading time, adjudication method, denominator, and versioned corpus. Publish raw anonymized measurements alongside summaries.

**How to verify.** Two independent operators using the same documented protocol produce the same classifications and comparable timing distributions; a third party can regenerate gate calculations from the packet.

### MED-03 — Bundle integrity does not prove execution provenance or identity

**Evidence.** The evidence design hashes records and files for offline tamper detection. Security 09 §4.5 states an actor with write access to both repository and `.ilvuno/` can recompute an unsigned chain; T-0602 explicitly permits unsigned approvals and labels them. See [security limits](../architecture/09-security.md#L155-L171), [approval test](../tasks/T-0602.md#L1-L33), and [evidence task](../tasks/T-0304.md#L1-L35).

**Impact.** A valid digest proves consistency of the bundle as received, not that a trusted executor ran the command or that a particular person approved it. Team/compliance use could otherwise infer stronger assurance than delivered.

**Affected component.** Evidence bundle, approval, product claims, future team workflows.

**Recommended correction.** Keep “integrity-verifiable bundle” distinct from signed identity/provenance. Require signatures and independent verification for cross-user approvals; document the single-user/unsigned limitation beside export and approval.

**How to verify.** Tamper-after-seal, recomputed-unsigned-chain, signed approval, key rotation/revocation, and forged passport cases all produce the documented result; user docs distinguish each assurance level.

### MED-04 — Future Claude adapter protocol and authentication need a separate provider gate

**Evidence.** The MVP correctly excludes provider adapters. The current [Anthropic Agent SDK overview](https://docs.claude.com/en/api/agent-sdk/overview) describes the SDK for Python/TypeScript and says other languages can invoke the CLI with `-p --output-format json`; it also cautions that third parties may not offer Claude.ai login/rate limits in their products without approval. The architecture assumes a Claude `stream-json` surface and T-7002 refers to the user's login.

**Impact.** The future adapter cannot infer a stable event protocol, supported auth path, or permitted subscription use from “the CLI works on this machine.” This does not block the provider-neutral MVP but can block PG1/M7.

**Affected component.** Architecture 03, ADR-0010, T-7001/T-7002, PG1.

**Recommended correction.** Keep Claude dispatch provisional. Before PG1, verify the current supported CLI/SDK protocol and version range, test structured events and cancellation, and obtain a provider-terms decision for the proposed local-login flow. Do not store or infer credentials.

**How to verify.** A conformance fixture against the supported version range, documented auth-state behavior, and a recorded terms review are prerequisites in the PG1 packet.

### MED-05 — Default redaction of absolute paths in arbitrary command output is unspecified

**Evidence.** INV-9/observability require no absolute paths in persisted artifacts; SEC-103 makes path redaction an export feature; the redactor's default classes in security 09 §6 cover patterns, entropy, and user rules, not universal path normalization. T-0304 tests secret patterns but not a child printing `$PWD`. See [redaction policy](../architecture/09-security.md#L216-L241), [SEC-103](../product/02-prd.md#L811-L820), and [T-0304](../tasks/T-0304.md#L1-L35).

**Impact.** Evidence logs may contain machine-specific paths or usernames even when structured fields are repo-relative. That weakens the portability/privacy claim and can leak local environment details.

**Affected component.** Evidence capture, redaction, INV-9, bundle export.

**Recommended correction.** Define path normalization for logs before writing or explicitly narrow the invariant and distinguish raw local evidence from exportable redacted bundles. Keep redaction write-time if raw data must never be persisted.

**How to verify.** Run a fixture command that prints absolute repo and home paths to stdout/stderr, then inspect all persisted reports, journals, bundles, and exports for those values. Test configured and default behavior.

## Suggestion

### SUG-01 — Validate the smallest useful vertical slice before the full MVP

**Evidence.** Product 05 §4.3 calls S1–S3 the smallest release that can answer “does this work for me” and permits deferring S4–S6. The roadmap nevertheless plans M0–M6, including passport, approval, repair, and secondary commands, before product-value measurement. Research records no demand or pricing data.

**Impact.** The project can spend most of its planned 46 engineering days on task continuity and approval surfaces before confirming that users need combined verification.

**Affected component.** Product scope, roadmap, milestones M3–M6.

**Recommended correction.** Make the first user-facing experiment a candidate plus configured checks plus integrity-verifiable evidence. Defer passport, repair, signatures, and extra analytics until users demonstrate a need. This is a scope recommendation for the maintainer to decide, not an implemented change.

**How to verify.** The revised plan has a gate that can falsify the core premise using real multi-source runs before adding deferred features; each retained task traces to that gate or a necessary safety invariant.
