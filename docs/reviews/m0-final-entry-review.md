# M0 Final Task and License Review

**Review result:** M0 remains **NOT READY / NOT AUTHORIZED**. I reviewed the entry evidence and task specifications only; no implementation was started. No entry gate is a new FAIL, but E-3, E-8, and E-9 remain OPEN, and the owner authorization is unsigned.

## Entry gates

| Gate | Result | Evidence |
|---|---|---|
| E-1 — authorization policy | **PASS** | D-01 assigns milestone authorization to the owner's signature; [AGENTS.md](../../AGENTS.md) and [06 checklist](../readiness/06-m0-readiness-checklist.md) say M0 is not authorized until §7 is signed. This policy is not itself authorization. |
| E-2 — product shape | **PASS** | D-02 records a standalone CLI in [07 reconciliation](../readiness/07-m0-reconciliation.md) §1. |
| E-3 — license text | **OPEN** | The draft is byte-for-byte the standard Apache-2.0 text: 11,358 bytes, SHA-256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`, matching [the Apache Software Foundation text](https://www.apache.org/licenses/LICENSE-2.0.txt). It includes the complete terms and appendix. The root `LICENSE` is absent and owner approval is not recorded. Text correctness does not close the approval gate. |
| E-4 — language | **PASS** | D-04 selects Go; [ADR-0001](../adr/ADR-0001-language-and-runtime.md) does not approve a second runtime for M0. |
| E-5 — platforms and Git floor | **PASS** | D-05 records Linux-first, macOS CI, Git ≥2.38.1, with Windows excluded from V1. |
| E-6 — config location | **PASS** | D-06 records committed `ilvuno.toml`; it is a later milestone concern, not work added by these tasks. |
| E-7 — status reconciliation | **PASS** | `AGENTS.md`, `CLAUDE.md`, roadmap, and milestones state M0 is proposed and not authorized, consistent with the unsigned [06 §7](../readiness/06-m0-readiness-checklist.md#7-sign-off). |
| E-8 — T-0005 review | **OPEN** | [T-0005](../tasks/T-0005.md) is appropriately narrowed to test infrastructure, but its condition references and oracle-output steps are not executable as written; details below. |
| E-9 — T-0001 review | **OPEN** | [T-0001](../tasks/T-0001.md) removes production package stubs, but still has unresolved toolchain, module-path, portability, and CI ownership criteria; details below. |
| E-10 — Git floor evidence | **PASS, limited to stated scope** | [09 evidence](../research/09-git-floor.md) and raw logs support matching behavior on Linux Git 2.38.1 and 2.55.0, including the 28-row matrix and 95 assertions. Git 2.39 and macOS were not tested; `check-attr --source` is absent on 2.38.1 and remains an M2 capability-probe issue. macOS CI evidence is required before G0, not before E-10 closes. |

## Task and architecture findings

### T-0001 — repository skeleton

**Scope and safety: PASS.** The task limits the binary to `--version`, disallows product behavior and empty production packages, and moves fixture packages to their own tasks. It does not create `internal/engine` or `internal/gitx`; those directories are absent. The import-rule test can assert a future dependency matrix without introducing an integration engine.

**Executable criteria: OPEN.** Before E-9 closes:

- Replace `github.com/<org>/ilvuno` with the actual module path or an explicit owner-supplied value.
- Resolve lint scope. [AGENTS.md](../../AGENTS.md) requires `staticcheck`, while [07 §5.1](../readiness/07-m0-reconciliation.md#51-blocking-the-m0-start) says only `gofmt`/`go vet` are approved for M0 absent approval for an added tool. Specify the exact `make lint` commands and what happens to `staticcheck`.
- Make AC-1 portable: `ldd` is Linux-specific and does not establish the same claim on macOS. State platform-specific commands and the intended meaning of “no runtime deps.”
- Resolve CI ownership/order. T-0001 owns the workflow, but requires the macOS job to run the reference harness; T-0005 adds the Make targets and excludes `.github/workflows/ci.yml` from its allowed scope. T-0005 also requires the failing-Git check in CI. Define which task updates the workflow and obtain the required maintainer review.
- Remove or revise the review checklist item about package stubs importing each other; this task explicitly creates no stubs.

### T-0005 — reference conformance

**No production engine: PASS.** It copies the harness under `testdata/reference/`, restricts Makefile changes to two test targets, and expressly excludes production packages and historical experiment files. This preserves the [SP-1R3 conditional approval boundary](sp1r3-final-independent-verdict.md): the verdict approves a controlled reference harness, not production code or M0 authorization.

**Condition traceability and acceptance: OPEN.** Before E-8 closes:

- `T-0005` says conditions C-1…C-9 are in `07 §5.1`, but current [07 §5.1](../readiness/07-m0-reconciliation.md#51-blocking-the-m0-start) lists M0 blockers. Current §5.4 points to the superseded revision 1 §5.1. The SP-1R3 verdict instead states operational guardrails (fresh canonical private scratch, checked exit status, do not consume stale oracle output, and trust the editable run sidecar). Restate those requirements directly or provide a valid, stable reference; do not treat the unrelated C-1…C-9 remediation findings as verdict conditions.
- Step 3 requires oracle generation “into a fresh path,” but `sp1.sh oracle` has no output-path argument: it writes `$SP1_SELF/oracle.json` (`sp1.sh` lines 312, 326). In the copied harness that is `testdata/reference/oracle.json`, which T-0005 also copies from the existing result. Specify how to produce an isolated fresh output without consuming or overwriting that copy; AC-4’s stale-output rule needs a runner test proving a failed run cannot lead to stale-result use.
- Pin exact tested Git versions and the full container image digest. The experiment allowlist is exactly 2.38.1 and 2.55.0, while the task says 2.55.x and abbreviates the image ID with an ellipsis.
- Align CI ownership for AC-3 as noted under T-0001. AC-5 is currently vacuous before production directories exist; prefer asserting the M0 file/scope boundary or explicitly make it a post-M0 check.
- Remove the stray `</content>` at the end of the task.

### E-10 and future-technology edits

The raw X-09 records show matching results on the declared Linux floor and the tested current version for the listed Git operations. The evidence also clearly limits its claim: no 2.39/macOS result, Alpine/musl floor image, and no upstream signature validation for the pinned Git source. Those limitations do not invalidate E-10 for the declared floor; macOS remains a G0 check and the `check-attr --source` difference is explicitly deferred to M2.

ADR-0001 now correctly makes Rust/Tauri conditional on a separately approved shell. ADR-0003 and planning M8 disclaim framework approval, but ADR-0003 decision #2 says a later shell “would speak WebSocket,” decision #4 selects React over alternatives, and M8 lists a “loopback WS daemon” without candidate status. This is residual future-technology ambiguity, not an M0 blocker; mark those transport/framework choices as unapproved candidates consistently before a future shell task relies on them.

## Recommendation

**E-8: do not close yet. E-9: do not close yet.** Close them after the cited task-spec corrections and a review against the resulting exact acceptance criteria. E-10 may remain PASS with its documented scope limits.

Even after E-8/E-9 close, M0 cannot start until the owner approves the license text and signs [06 §7](../readiness/06-m0-readiness-checklist.md#7-sign-off); the lint-tool question in [07 §5.1](../readiness/07-m0-reconciliation.md#51-blocking-the-m0-start) also needs its recorded resolution. This review does not provide any of those approvals.
