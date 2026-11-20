# SP-1 Final Verdict

## Decision

# APPROVE WITH CONDITIONS

**Pipeline B is approved as a reference oracle for M0 only inside a controlled, isolated harness. It is not approved as an M0 production engine or as the M2 production baseline in its current state.** The clean-fixture equivalence result is useful, but does not close the execution-security, failure-classification, cleanup, or evidence-quality gaps.

This verdict does not authorize starting M0. `docs/readiness/06-m0-readiness-checklist.md` still marks M0 not ready and lists unchecked evidence; the owner decision register has open decisions. The supplied engineering policy says M0 is the approved scope, while the readiness checklist says its gate is not passed. A maintainer must reconcile scope authorization with gate/start authorization before milestone work begins.

## Readiness score: 4.7 / 10

| Dimension | Score | Basis |
|---|---:|---|
| Narrow corpus correctness | 2.0 / 2.0 | Candidate/tree SHA equality independently reproduced for eight clean cases across two pipelines and two Git versions; ordinary conflict step/path also agreed. |
| Assertion strength | 0.7 / 2.0 | Matrix records outcomes but does not enforce them; a Git executable that always fails still produces exit 0 and zero rows. |
| Execution security | 0.4 / 2.0 | Hooks, filters, and custom drivers execute; config/attribute fallbacks fail open; worktrees do not contain processes. |
| Cleanup and recovery | 0.6 / 1.5 | P3 orphan directory reproduced; conflict state is aborted despite recommendation; stop-after and ref residue need explicit ownership/recovery. |
| Reproducibility and portability | 0.8 / 1.5 | Controlled sample is reproducible on Git 2.38.1/2.55.0; newline filenames break path reporting, Docker provenance and broader platform behavior remain unverified. |
| Governance/readiness | 0.2 / 1.0 | Readiness checklist and owner decisions remain open; recommendation conflicts internally on whether B belongs in M0 or only M2. |
| **Total** | **4.7 / 10** | **A narrow proof is credible; harness gates and production boundary are not ready.** |

## Conditions for M0 use

M0 may use Pipeline B as a reference oracle if all of the following hold:

1. The harness runs only on newly created, controlled temporary fixtures and invokes no project-supplied verification commands.
2. The SP-1 script has a validated temporary output root; rejects traversal/symlink redirection; and all documented commands resolve to the actual script path.
3. Git is invoked with a sanitized, controlled environment/config. Hooks, filters, custom drivers, recursive submodules, and unexpected processes have marker-based assertions that fail the test if they execute.
4. Matrix cases assert expected rows, outcomes, pins, SHAs/trees, and cleanup; any Git invocation failure or missing output returns nonzero.
5. The safety claim names exactly what was checked and covers refs/config/worktree metadata as well as HEAD/index/status/stashes.
6. The owner resolves M0 readiness items and reconciles the milestone-start language before work begins.

These are harness controls; they do not require building a production sandbox for M0 when only trusted harness-owned commands run.

## Conditions before M2 production baseline

Do not adopt B for production integration until the following are addressed and verified:

- Define and enforce hook, filter, custom merge-driver, attribute, and Git configuration policy across every merge step and both common/worktree admin paths.
- Sanitize inherited Git environment/config. If untrusted project commands or executable extensions are supported, contain them with an OS-level sandbox; consent is not containment.
- Classify content conflicts separately from hook, driver, checkout, and Git operational failures using machine-readable state, not stderr substring alone.
- Preserve path names losslessly, including control characters, and emit valid escaped JSON.
- Prove path ownership and cleanup across both filesystem and Git worktree administration; cover orphan directories, stale refs, retries, and process crashes.
- Pin and report source SHAs/order, merge strategy/options, commit metadata, effective Git version/config policy, and expected tree outputs.
- Resolve open design decisions D-07 through D-10 and specify the supported repository/configuration boundary.

## Human decisions required

- **Before M0:** Resolve open M0 owner decisions D-01…D-06 in `docs/readiness/05-owner-decision-register.md`; confirm whether the “M0 approved scope” policy permits milestone start while the readiness checklist says “NOT READY.”
- **Before M2:** Decide D-07 through D-10, including candidate storage/ref ownership, merge-driver and attribute policy, allowed execution effects/sandbox boundary, and determinism contract.
- **New security policy decision:** State whether hooks and checkout filters are refused, allowlisted, or sandboxed; state whether submodules and custom merge drivers are unsupported. Approval prompts alone do not provide containment.
- **Recommendation consistency:** `sp1-recommendation.md` §1 recommends B as merge engine, while §6 says not to put B in production under M0 and places the engine in M2. Record B's role as “M0 reference oracle; M2 candidate engine pending gates,” if that is the intended decision.

## Blocking issues by milestone

| Issue | Severity | Required milestone | Disposition |
|---|---|---|---|
| Script path traversal and non-enforcing matrix | High | M0 | Block running/porting the harness until corrected. |
| Incomplete safety fingerprint | High | M0 harness; M2 contract | Rename or expand before using “untouched” as evidence. |
| Hook/filter/driver/config execution gaps | High | M0 controlled fixtures; M2 policy/enforcement | M0 can defer production sandbox only under trusted-fixture restriction. |
| Conflict misclassification and newline path handling | High | M0 evidence harness; M2 user-facing engine | Block any gate that relies on current output. |
| Cleanup/orphan/ref lifecycle and conflict retention mismatch | High | M0 harness cleanup; M2 recovery | Reproduce and assert cleanup before repeated runs; implement full lifecycle before production. |
| Git version image provenance | Medium | M0 reproducibility | Record image digest/source checksum or build from verified pinned inputs before relying on it as CI evidence. |
| Edge corpus, macOS, scale, concurrent run, mid-merge crash | Medium | M2 or release gate | Can be deferred from harness-only M0, but must be scoped before production/release. |

## Scope and stop statement

This audit created only the four requested review documents. It did not change production code, approved architecture, product specifications, or push/merge any branch. The separate working-tree change to `.gitignore` was present during final inspection and was not modified by this review. The experiment reruns and adversarial fixtures were isolated under `/tmp`.

## Related review files

- [Independent evidence verification](sp1-independent-verification.md)
- [Security gates](sp1-security-gates.md)
- [Git correctness findings](sp1-correctness-findings.md)
