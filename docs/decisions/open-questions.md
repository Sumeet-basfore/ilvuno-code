# Ilvuno — Open Questions and Decisions Requiring Approval

Status: Phase 2 exit artifact. Nothing here is decided except where marked **Decided**.

**Decision status, updated 2026-11-20:** DECIDED 10 decisions (D-01…D-06, D-16…D-19) and 7 planning items (A-4, S-1…S-4, C-5, C-7) · DEFERRED 0 · OPEN all other rows. Recorded in §0. Nothing else is decided. The SP-1R3 verdict (`docs/reviews/sp1r3-final-independent-verdict.md`) reviews the experimental harness and is not a decision on any row. Decision requests D-01…D-15 are in `docs/readiness/05-owner-decision-register.md`; D-16…D-19 were approved directly and are recorded only here. Mapping: A-1→D-02, A-2→D-03, A-3→D-04, A-4→D-05, A-9→D-06. Reconciliation: `docs/readiness/07-m0-reconciliation.md`.
Owner column: `U` = user (founder/maintainer), `E` = engineering, `PM` = product.
"Blocks" names the first phase or requirement that cannot proceed without an answer.

---

## 0. Recorded decisions (owner-approved, 2026-11-20)

Recorded from the project owner's explicit approvals: six decisions (D-01…D-06, first approval), seven planning items (A-4, S-1…S-4, C-5, C-7, second approval), and four M0 decisions (D-16…D-18, third approval; D-19, fourth approval). The owner's name is not in this repository and is not supplied here, so the approver is recorded as "project owner". All four approvals are recorded here. Every other decision in this file, and D-07…D-15 in `docs/readiness/05-owner-decision-register.md`, remains OPEN.

| ID | Maps to | Selected option (as approved) | Status | Approved |
|---|---|---|---|---|
| **D-01** | Milestone authorization | Owner signs the readiness checklist before each milestone (`docs/readiness/06` §7). | DECIDED | 2026-11-20, project owner |
| **D-02** | A-1 product shape | V1 is a standalone CLI. | DECIDED | 2026-11-20, project owner |
| **D-03** | A-2 license | Apache-2.0. | DECIDED | 2026-11-20, project owner |
| **D-04** | A-3 implementation language | Go. | DECIDED | 2026-11-20, project owner |
| **D-05** | A-4 platforms and Git floor | Linux-first; macOS tested in CI; Git ≥ 2.38.1 initially. Windows is handled by planning item A-4 (excluded from V1). | DECIDED | 2026-11-20, project owner |
| **D-06** | A-9 verification config location | Repository-committed `ilvuno.toml`. | DECIDED | 2026-11-20, project owner |
| **D-16** | M0 lint policy | `gofmt`, `go vet`, and the in-repository import-rule test only. `staticcheck` deferred until separately approved as a dependency; `golangci-lint` not used in M0. | DECIDED | 2026-11-20, project owner |
| **D-17** | Canonical repository and Go module path | Repository `https://github.com/Sumeet-basfore/ilvuno-code`; Go module path `github.com/Sumeet-basfore/ilvuno-code`. No other path is approved. | DECIDED | 2026-11-20, project owner |
| **D-18** | Apache-2.0 LICENSE text | The exact text in `docs/readiness/LICENSE-draft-Apache-2.0.txt` (sha256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`, 11,358 bytes, identical to apache.org) is approved unmodified, including the appendix placeholders. | DECIDED | 2026-11-20, project owner |
| **D-19** | M0 delivery shape | Two PRs: governance (`docs/commit-rules`) then implementation (`feat/T-0001`, five task commits). Includes: T-0001 scope → `cmd/ilvuno/**`; T-0005 scope → `docs/research/09-git-floor-evidence/**`; defer 20 superseded `docs/experiments/**` files; exclude 2 `.pyc` and 10 `ilvuno-concept-*.svg`; gate-log Option A (attributed evidence, original provenance and dates preserved). | DECIDED | 2026-11-20, project owner |

**Rationale and documents updated.** The owner approved each selection. The rationale below is the basis already recorded in `docs/readiness/05` §2 and §5. The owner has not restated it.

- **D-01.** Basis: one signature, one checklist, one log. Without it, contributors act on contradictory status (`05` §2). Documents: AGENTS.md §1; roadmap status line; `planning/02` status line; `readiness/05` header; `readiness/06` header and §7 note. CLAUDE.md §1.4 already matches and is unchanged.
- **D-02.** Basis: CLI-only product shape (`docs/product/05` §2). A shell built before the claim is tested costs about 6× (`05` §2). Documents: AGENTS.md §1; T-0001 dependencies; roadmap B-1 row; ADR-0003 (status: accepted, since it records A-1).
- **D-03.** Basis: a patent grant matters for CI use, AGPL inhibits enterprise adoption, and MIT has no patent grant (A-2). Documents: T-0001 (`LICENSE`). The exact text is approved by D-18; the root `LICENSE` is created by T-0001 from that text, so E-3 is satisfied.
- **D-04.** Basis: single static binary, trivial cross-compile, cheap subprocess and concurrency. A rewrite after v0.1 is expensive (A-3). Documents: ADR-0001 (status: accepted); T-0001 (`go.mod`); AGENTS.md §4.1 (already states Go).
- **D-05.** Basis: the floor-risk rationale in `05` §2 (a wrong floor breaks parsing on users' machines). The SP-1R3 matrix passed on 2.38.1 (Linux container) and 2.55.0 (Linux host); see `docs/readiness/07` §7. The approval says "initially", so the floor may move. Documents: T-0001 (CI floor and macOS leg); `readiness/06` E-5 and E-10; `planning/02` M0 dependencies.
- **D-06.** Basis: a committed config makes verification a reviewable team decision (A-9). Documents: this file. No M0 document depends on it. Config loading is M1 (T-0103).

- **D-16 (M0 lint).** Scope: `make lint` in M0 = `gofmt -l` over tracked Go files (fail on output), `go vet ./...`, and `go test ./internal/lint/importrule/...`. `staticcheck` remains the target quality bar but is deferred until separately approved as a tool dependency; AGENTS §4.1 preserves that requirement. `golangci-lint` is not added. This approves no tool installation and no dependency.
- **D-17 (module path).** Scope: the `module` directive in `go.mod` and every import path. Base repository for CI, releases and clone instructions. Approval of the path does not approve the repository's visibility, creation, or any push.
- **D-18 (LICENSE text).** Scope: the exact 11,358-byte Apache-2.0 text identified by the recorded SHA-256, with no copyright-holder line and the standard appendix placeholders. T-0001 copies it verbatim to root `LICENSE`; changing the text, adding a copyright line, or substituting another copy requires a new decision. This is the E-3 evidence, not M0 authorization.

**Planning approvals (second approval, 2026-11-20, project owner).** Each row gives the approved selection, its basis, and the documents updated in this revision.

| ID | Approved selection | Basis | Documents updated |
|---|---|---|---|
| **A-4** | Windows is excluded from V1. Inclusion in future versions remains OPEN. | The A-4 register row (scope and honest support claims; process-group semantics differ). | `open-questions` §1 note; `T-0001`; `readiness/06` E-5; `planning/01` B-1; `planning/02` M0 dependencies |
| **S-1** | Port F1–F8 and F10 during M0. Retain F9 for fault-injection testing. Defer additional fixture classes. | F1–F10 are verified by SP-1R3 (28/28 on two Git versions). The other classes need product code before they can be asserted. | `T-0003`; `readiness/06` §3.1 and G0-3; `planning/05` G0 criterion 3; `planning/02` M0 deliverable 6 |
| **S-2** | Python 3, Bash and Docker are permitted for development and test tooling only. Go remains the production CLI language. | D-04 (Go). The SP-1R3 harness is Python and Bash, and the 2.38.1 reference uses Docker. | `T-0005`; `AGENTS.md` §4.1 (policy file, for human review) |
| **S-3** | Replace the obsolete merge-tree criterion (G0 criterion 5) with sequential-integration reference conformance, preserving meaningful acceptance coverage. | `readiness/02` X-02: `merge-tree` takes two commits only, so the tuple question has no preflight to decide. Reference conformance covers the four E-1′ cases on both approaches. | `planning/05` G0 criterion 5, entry line and sign-off artefact; `readiness/06` G0-6; `T-0005` AC-8 |
| **S-4** | Approve the proposed Linux/macOS GitHub Actions CI scope, subject to review of the actual workflow before merge. | `AGENTS.md` §10.2 requires maintainer approval for `.github/`. Scope as proposed in `readiness/07` §6. | `T-0001` CI outline and AC-4. The workflow file is not created yet. |
| **C-5** | Parameterize the hard-coded `/tmp/r3-cl` path in the harness test copy, preserving existing assertions. | The SP-1R3 runner condition. The patched copy changes three lines and passes 95/95 on Git 2.55.0 and 2.38.1 (`docs/research/09-git-floor.md` §5). | `T-0005` step 2; `readiness/07` §5.1 |
| **C-7** | Rebuild the Git 2.38.1 reference environment from the pinned `Dockerfile.git238` and record the image digest. | The SP-1R3 verdict's "Release/future" item. Rebuilt 2026-11-20 with `--no-cache`. The Dockerfile's Git checksum check passed. Image ID `sha256:1044b988f91593334ab23b73b7b8ecbb308e49eda57c81ed86851677ef6fe859`. | `T-0005` step 5; `docs/research/09-git-floor.md` |

---

## 1. Decisions requiring approval now (block Phase 3)

| ID | Decision | Options | Recommendation | Why it must be explicit | Owner | Blocks |
|---|---|---|---|---|---|---|
| **A-1** | Initial product shape | CLI / plugin / desktop / browser | **Standalone CLI** | Commits 3 months; excludes UI work entirely | U | All work |
| **A-2** | License | Apache-2.0 / AGPL-3.0 / MIT | **Apache-2.0** | Patent grant matters for CI usage; AGPL inhibits enterprise adoption which is the paid cohort | U | OSS-101, public repo |
| **A-3** | Implementation language | Go / Rust / TypeScript | **Go** — single binary, trivial cross-compile, cheap subprocess+concurrency, low contributor friction vs Rust | Reversal after v0.1 is expensive | U+E | S1 |
| **A-4** | Windows in MVP | yes / no | **No, v0.2** | Scope and honest support claims; process-group semantics differ | U | NFR-105, docs |
| **A-5** | Passport location | repo-committed / local-only / both | **Repo-committed default, `--local-only` optional** | Privacy vs portability; the portability story depends on it | U | TSK-108, TSK-110 |
| **A-6** | Repair executor | external command / bundled agent / none | **External command only** | Keeps the tool agent-neutral and dependency-free; also avoids owning a provider roadmap | U | INT-502 |
| **A-7** | AI interpretation | off / on / on-but-non-blocking | **Opt-in, non-blocking, structurally removable** | Determines whether the product is auditable — the core differentiator | U | INT-206, INT-207 |
| **A-8** | Telemetry | none / opt-in aggregate / default-on | **None by default; opt-in aggregate, counts only** | Trust precondition for Dana persona; changing later is a reputational event | U | NFR-109 |
| **A-9** | Verification config location | committed `ilvuno.toml` / per-user / both | **Committed** | Makes verification a reviewable team decision, which is what reviewers want | PM | INT-301 |
| **A-10** | Monetization gate | build paid now / gate on traction | **Gate: no paid feature before 20 weekly actives and M-1 ≥ 5%** | Prevents building infrastructure for an unmeasured problem | U | 06-monetization §6 |

---

**Status of §1 rows after §0.** A-1 → D-02 DECIDED. A-2 → D-03 DECIDED. A-3 → D-04 DECIDED. A-9 → D-06 DECIDED. **A-4: DECIDED for V1 (Windows excluded), 2026-11-20. Its inclusion in future versions remains OPEN.** D-05 covers platforms and does not address Windows; A-4 does. A-5, A-6, A-7, A-8 and A-10 remain OPEN. The §1 table is kept unchanged as the original request record.

---

## 2. Open product questions

| ID | Question | Current best answer | How to resolve | Owner | Blocks |
|---|---|---|---|---|---|
| **Q-1** | What is the real combined-failure rate on real repositories? | Unknown. Evidence spans 27.67% (simulated PRs) to 1/834 (real Django pairs). **This is the product's premise.** | Measure via `INT-306` across ≥ 50 integrations on ≥ 20 users' repos; publish the number even if unfavorable | U+PM | G3 gate; the whole value case |
| **Q-2** | Is combined verification the right lever, or is concurrent-change *context* stronger? | The 82% recovery result suggests context may dominate detection (research `04` §5). We ship the brief (`INT-260`) as a hedge | Run an A/B in the wild: integrations preceded by a brief vs not; compare M-1 and M-3 | PM | INT-260 priority |
| **Q-3** | Do reviewers actually consume evidence bundles, or is a green check enough? | Assumed yes (Marco persona). Unverified — no interviews | M-2 plus 5 reviewer interviews with a real bundle | PM | M-2 target; report design |
| **Q-4** | Is `unverified` (no configured checks) a useful honesty signal or a dead end for new users? | Honest but possibly a cliff. `init` proposes commands; that may be insufficient | Track the `unverified` share; interview users who never configure checks | PM | INT-310 UX |
| **Q-5** | Is a 2-attempt repair budget right? | A guess. Prior art: 3 failures (ORC-038), 10 wakes (T3-047), 4 retries (CR-049) | Measure budget-exhaustion rate and success-per-attempt; tune, or remove repair if it never helps | PM | INT-503 default |
| **Q-6** | Should the passport be one file per task or a directory? | One file — diffable, reviewable, simple | Test with 3 real multi-week tasks; split only if files exceed readable size | E | TSK-108 |
| **Q-7** | Should overlapping-file warnings block by default? | No, warning only. Blocking on a heuristic destroys trust | False-blocking rate data; ask users | E | INT-201 severity |
| **Q-8** | Does anyone want pre-dispatch conflict prediction (MVP-C)? | Unlikely alone; could be an input to the brief (`INT-260`) | Test with the manifest path (`ORQ-104`); do not build standalone | PM | ORQ-104 |

---

## 3. Open technical questions

| ID | Question | Options | Recommendation | Blocks |
|---|---|---|---|---|
| **T-1** | Is the evidence store the source of truth, or is the passport? | passport authoritative / store authoritative / passport derived | **Passport is the human-facing record; the store is authoritative for evidence.** Passport references bundles by id+digest, never copies (`TSK-105`). Avoids t3code's mistake of making projections authoritative after pruning the log (T3-021) | EVD-101, TSK-105 |
| **T-2** | How to make candidate SHAs deterministic given commit timestamps | fixed committer date env / content-hash identity / accept nondeterminism | Fixed `GIT_COMMITTER_DATE`/`GIT_AUTHOR_DATE` derived from inputs; verify with a determinism test (`INT-106`, `NFR-103`) | INT-106 |
| **T-3** | Env allowlist mechanism for verification children | fixed list / per-project config / both | Fixed safe base + explicit per-project additions; denied-by-default (`INT-305`) | INT-305 |
| **T-4** | How to handle submodules, LFS, sparse checkout | support / refuse with typed error / partial | **Refuse with a typed error at v0.1**; silent wrong answers are worse than refusal (`REC-106`, `R-5`) | INT-110, docs |
| **T-5** | Symbol index for interface analysis — ship one or require one? | bundle ctags / require configured command / language-specific | **Require a configured command** at v0.1; ship a ctags-based example. Avoid owning per-language analyzers | INT-204 |
| **T-6** | Parallelize verification commands? | sequential / parallel with deps / per-command flag | Sequential by default; opt-in parallel for commands declared independent. Parallel by default breaks order-sensitive suites and makes evidence harder to reason about | INT-301, INT-302 |
| **T-7** | Report normalization for byte-identical determinism | strip timings / separate volatile section / both | Volatile fields in a separate `run` block excluded from the normalized comparison; the findings section is byte-stable (`INT-605`) | VER-105 |
| **T-8** | Journal format | JSONL append-only / SQLite / both | **JSONL append-only** — the cloudroom receipt model (CR-012, CR-013), no DB dependency, human-readable, no migration burden | REC-101, NFR-104 |
| **T-9** | Crash-detect strategy for "do not repeat uncertain effects" | advisory lock file / fsynced state before each side effect / git-based detection | fsynced intent record before each mutating step, on resume compare git state against the intent (`REC-102`) | REC-102 |
| **T-10** | Bundle format and compaction | tar.zst + manifest / plain dir / both | Plain directory with manifest + optional tar for transport; compaction writes tombstone with digests (`TSK-113`, `EVD-105`) | EVD-106 |
| **T-11** | MCP tool vs CLI JSON as the host plugin contract | CLI JSON first / MCP first / both | **CLI JSON first** (`ORQ-106`), MCP wrapper in v0.3. CLI is testable without a host; MCP adds protocol churn | ORQ-107 |
| **T-12** | Base fetch policy when offline | fail / use local base with recorded warning | Fail closed by default; `--offline` uses the local base and records `stale_base_accepted` in the report (`INT-104`) | INT-104 |
| **T-13** | Identity for approvals in a local-only tool | OS user + hostname / optional signing key / both | OS user + optional key; never machine-identifying data in the passport (`TSK-109`, `INT-603`) | INT-603 |
| **T-14** | Monorepo support level | whole-repo checks only / per-package check selection | Whole-repo at v0.1 with per-command `cwd` support; package selection v0.2 | INT-302 |

---

## 4. Open questions the research could not answer

These are limits of the Phase 1 research, recorded so we do not mistake absence of evidence
for evidence of absence.

| ID | Gap | Impact |
|---|---|---|
| **R-1** | All "no tool does X" claims are **absence claims** from static reading at pinned commits (research `05` §8). A feature could exist in an unread area, or ship next week | Differentiation claims must be re-checked at each release; never state "nobody does this" in marketing |
| **R-2** | No demand-side evidence: no interviews, no pricing, no usage data, no revenue. Star counts are popularity, not adoption | Every willingness-to-pay statement in `06-monetization.md` is a hypothesis |
| **R-3** | Conflict-rate figures come from preprints: 27.67% on *simulated* PRs (AgenticFlict) and 1/834 on real Django pairs ("stale"), whose authors say constructed rates do not estimate real frequency | Direct driver of Q-1 and the measurement obligation in §5 of `05-mvp-scope.md` |
| **R-4** | Runtime behavior of all three analyzed products was never observed — static reading only | Prior-art claims about *behavior* (e.g. "merge_ready is ignored") are code-level, not runtime-verified |
| **R-5** | cloudroom's hosted side (sandbox provider, website logic) is not in the public repo | Any claim about hosted sandbox economics is unverified (`06-monetization.md` H1) |
| **R-6** | Conductor's reported acquisition, Graphite's merge-queue behavior, Clash/Grove feature claims are secondary-source only | Competitive conclusions about those four are provisional |
| **R-7** | No data on how often teams require verification evidence on PRs | M-2 target is a guess; T1 test resolves it |
| **R-8** | Not established whether vendors can add combined verification quickly inside one product | If they can, the moat narrows to cross-tool + portable evidence + determinism (`02-prd.md` §8) |

---

## 5. Explicitly deferred decisions

Not open — deliberately postponed, with the condition that would reopen them.

| Decision | Deferred until | Reopening condition |
|---|---|---|
| Desktop shell (shape C) | Post-revenue | A paying team requires a GUI to adopt; or CLI adoption is blocked by UI-only friction |
| Browser workspace (shape D) | Post-desktop | Never, unless a concrete user workflow demands server-side state |
| Host plugin (shape B) | After `ORQ-106` contract ships | Any host asks to integrate; or M-2/M-5 show CLI-only adoption caps out |
| Dispatch / agent control | Post-revenue, gated by `ORQ-105` | G4 passed and users ask for it by name |
| Mobile client | After rung-3 transport exists (`21.2`) | H2/H3 revenue exists and users request it |
| Hosted runner (H1) | Bundled into H3, post-G5 | Security review of the hosted surface complete |
| Team collaboration (H3) | Post-G4 | 3 teams ask for the same feature unprompted |
| Windows support | v0.2 | Any paying user on Windows |
| Non-git VCS | Never in v1 | A paying team's primary VCS is not git |
| Enterprise (SSO, RBAC, audit export) | Post product-market signal | An enterprise deal in writing |
| Plugin marketplace / skill ecosystem | Post-v1 | Third-party adapters appear organically (`OSS-103`) |

---

## 6. Risk register items needing an owner

From `02-prd.md` §25, the ones that cannot be mitigated by engineering alone:

| Risk | Why it needs a decision, not a fix | Owner |
|---|---|---|
| R-1 combined-failure rate is low | Determines whether the product exists. Must be measured and acted on, not rationalized | U |
| R-9 willingness to pay below hypothesis | Requires conversations with real teams, not code | U |
| R-4 vendor ships it first | Requires a positioning decision (layer vs product) and speed | U+PM |
| R-10 cross-platform process supervision | Requires accepting a narrower platform set at v0.1 | U (A-4) |

---

## 7. Exit condition for Phase 2

Phase 2 is complete when:

1. A-1 through A-10 are answered (or explicitly deferred with a date).
2. Q-1 has a measurement plan with a named owner and a date, not a promise to "look into it".
3. The MVP boundary in `05-mvp-scope.md` §4 is accepted, including its exclusions.
4. The banned-claims list (PRD Appendix A) is accepted as a release gate, not a style guide.

Until then, no production code is written. Phase 3 begins with slice S1
(`init`/`doctor`/`pin`/candidate) only after A-1, A-2, A-3, A-4, A-9 are answered. All five (A-1, A-2, A-3, A-4, A-9) are now DECIDED (§0).
