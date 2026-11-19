# Ilvuno — Product Requirements Document

Version: 0.1 (Phase 2 draft). Status: requires approval on items in
`docs/decisions/open-questions.md`.
Evidence IDs (`T3-`, `ORC-`, `CR-`) resolve in `docs/research/07-evidence-index.md`.

Requirement priority: **P0** = MVP blocker, **P1** = MVP if cheap, else v0.2, **P2** = post-MVP.
Every requirement has an acceptance criterion and a verification strategy. A requirement
without both is not a requirement.

---

## 1. Executive summary

Multi-agent coding tools have solved parallel *execution*. They have not solved parallel
*integration*. t3code moves conversation context between threads but never code (T3-048).
orca's coordinator receives `merge_ready` and ignores it (ORC-041). cloudroom rebases, then
pushes to the base branch with `--no-verify` and runs no tests (CR-031, CR-032). Every
per-task verification that exists is performed against the task's own base, so it is
strictly weaker evidence than what shipping requires.

Ilvuno is a local-first, open-source tool that closes that gap. Given N independently
produced changes plus the revisions they were built from, it builds a disposable
integration candidate, analyses compatibility without mutating anything, runs the project's
own verification commands on the combination, records the evidence, and stops for a human
decision. Around that core it maintains a **Task Passport**: a durable, versioned,
human-readable record of the task's objective, decisions, runs, revisions, verification
results, and next actions.

The recommended first product is a **standalone CLI** (one binary, git-native, no server,
no account), designed so a plugin surface can expose the same engine to existing agent
workspaces later. We do not build a desktop app, a browser IDE, or a dispatch layer in the
MVP; those are commoditized (research `04` §6) and commercially fragile (Vibe Kanban's
sunset, Crystal's deprecation).

The central risk is not technical. It is that the real-world combined-failure rate is low
enough that developers do not care. Published evidence spans 27.67% of simulated agent PRs
with textual conflicts (AgenticFlict) to 1 of 834 real merged-Django PR pairs (the "stale"
benchmark). The same benchmark found that *telling* an agent about a concurrent change
recovered 82% of runs — which suggests prevention via context sharing may be worth more
than detection. Ilvuno therefore treats measurement of the user's own repository as a
first-class output of the MVP, not as a research exercise preceding it.

---

## 2. Problem statement

### 2.1 Core problem

Verification in multi-agent development is performed against the wrong artifact.

An agent branch passes its own tests. The integration candidate — the actual thing that
would ship — is never built, never tested, and never recorded. The developer discovers the
failure at the last possible moment: after merging N branches locally, or after CI fails on
a PR whose author believed it was green.

This is not a tooling gap in any one product; it is a category-wide stopping point. Ten
distinct failure modes are documented in research `05`, and **not one is Solved for the
combined case**.

### 2.2 Why this hurts, concretely

| Failure mode | Mechanism | Current handling |
|---|---|---|
| Textual conflict between agent branches | Overlapping hunks; git refuses | Resolved per branch against base only (ORC-052, CR-031) |
| Semantic conflict, clean merge | Both add the same export; one renames what the other calls | Nothing. Not detected by any surveyed tool (`05` §2.2) |
| Dependency conflict | Different versions of one package; lockfile merges into an inconsistent state | Nothing. Per-worktree setups do not reconcile (T3-074, CR-021) |
| API/interface mismatch | A changes a contract; B builds against the old one | Nothing combined (ORC-054) |
| Stale base at integration time | B passed on a base that has since moved | Checked at worktree creation only (ORC-039, T3-033) |
| Combined test failure | Each passes; the pair fails | No local combined run anywhere (`05` §2.7) |
| Unverified "done" | Agent says done; nothing checks the claim | Self-reported (ORC-042, CR-026, T3-030) |
| Interrupted integration | Crash between merges | No integration state machine exists (`05` §2.9) |
| Failed repair loop | Fix attempt fails, no budget, no record | Attempt caps exist for dispatch and single PRs only |

### 2.3 Secondary problem: continuity

A multi-stream task's state lives in three places that do not talk to each other: the
conversation of each agent, the git graph, and the developer's head. Nothing records what
was decided, why, what was verified, and what is still pending. That is why returning to
Friday's work on Monday costs an hour, and why "moving to mobile" is not a feature request
anyone can act on.

Ilvuno does not migrate conversations. It records the *task* durably enough that any client
— a CLI today, a phone later — can show what needs to happen next.

---

## 3. Evidence from research

### 3.1 Confirmed absences (the gap is real; not necessarily large)

| Claim | Evidence | Confidence |
|---|---|---|
| t3code never merges code between agent threads | T3-048 (`merge_back` is context transfer only), T3-051 | Confirmed / Inferred (absence) |
| orca ignores the integration signal it defines | ORC-041 (`merge_ready` present, `merging` never assigned) | Confirmed |
| cloudroom pushes without verification | CR-032 (`--no-verify`, no tests) | Confirmed |
| No surveyed tool runs the project's checks on a combined candidate | research `05` §3 | Inferred from absence |
| No surveyed tool ties completion to an artifact | T3-030, ORC-042, CR-026 | Confirmed |
| No surveyed tool has an integration state machine | research `05` §2.9 | Inferred from absence |

### 3.2 Anti-patterns we inherit knowledge from and will not repeat

| Anti-pattern | Where | Ilvuno rule |
|---|---|---|
| Fail-open guard | cloudroom Command Guard allows the command on error (CR-037) | Every guard fails **closed** |
| Permissive default | t3code defaults to Full access (T3-060) | Read-only unless explicitly elevated |
| Non-constant-time token comparison | orca (ORC-072) | Constant-time compare, always |
| Silent SSH TOFU | orca (ORC-073) | Refuse unknown hosts |
| Plaintext secret fallback | orca (ORC-074) | Refuse to store rather than downgrade |
| Unsigned webhooks | t3code schedule_task (T3-064) | Sign or do not accept |
| Editing global agent config | orca writes `~/.claude/settings.json` (ORC-016) | Never mutate global tool config |
| Shared-folder parallelism | T3-031, ORC-033, CR-024 | One working tree per stream, enforced |
| Log pruned below rebuildability | t3code ~1 day, projections authoritative (T3-021) | Evidence store is append-only and self-contained |

### 3.3 Evidence that constrains the product (the counter-case)

- AgenticFlict: **27.67%** of ~107,000 *simulated* agent PRs had textual conflicts.
- "stale" benchmark: **1 of 834** real merged Django PR pairs conflicted; the authors say
  constructed-task rates do not estimate real frequency.
- Same benchmark: telling an agent about the concurrent change recovered **82%** of runs.

Conclusion, and it is a design driver, not a footnote: conflict *detection* may be the
weakest form of the idea. The stronger bets are (a) combined verification with evidence,
(b) feeding the concurrent change to the agent, (c) making "done" checkable. The MVP
includes (b) as a cheap export path (`INT-260`) precisely because the evidence points there.

### 3.4 What research does not support

The research contains **no demand-side evidence**: no user interviews, no revenue data, no
pricing, no usage numbers beyond star counts and one unverified "500,000 users" claim
(T3-070). Any statement in this PRD about willingness to pay is a hypothesis, marked as one.

---

## 4. Target users

Primary: **staff/senior engineers running 2–6 parallel agent streams on one repository with
a runnable test suite**, on a team that reviews code.

| Segment | Fit | Why |
|---|---|---|
| Individual power user (2–5 agents/day, one repo) | Strong | Feels the failure directly; owns the integration pain |
| Small team (3–8 devs, shared repo, PR review) | Strong | Integration evidence is what reviewers ask for |
| Platform/infra team standardizing agent usage | Medium | Wants policy + reproducibility, cares about determinism/audit |
| Non-technical builder ("vibe coder") | Weak — excluded | No runnable checks, no git fluency; value collapses |
| Enterprise regulated org | Later | Needs SSO, audit export, policy; not MVP |

**Explicit exclusion for MVP:** repositories without a runnable verification command. The
core value proposition depends on being able to execute the project's own checks. We will
say so on the tin rather than fake value with static analysis (`INT-310`).

---

## 5. Jobs to be done

| ID | Job | Current workaround | Success signal |
|---|---|---|---|
| J1 | "Tell me whether my parallel agent branches actually work together, before I spend an hour merging." | Manual merge into a scratch branch, run tests, throw it away | Answer in < 5 min, reproducible |
| J2 | "When the combination fails, show me *what* failed with evidence, not a summary." | Read CI logs after pushing | Failure artifact a colleague can read |
| J3 | "Don't let an agent's 'done' stand unverified." | Trust review by diff reading | Claim → SHA + command + exit code |
| J4 | "Let me come back to this task in three days and know where I am." | Scattered notes, agent histories | Passport answers in < 60 s |
| J5 | "Give me a repair attempt with a budget, and keep the record when it fails." | Retry manually until bored | Bounded attempts, each with evidence |
| J6 | "Don't make me change my agent, editor, or workflow." | Copy-paste between tools | Works on plain git refs |
| J7 | "Let me check and approve from my phone later." | Nothing exists | Read/approve client (post-MVP, `XDC-2xx`) |

---

## 6. User personas

Full profiles in `03-personas.md`. Summary:

| Persona | Role | Primary job | Would abandon if |
|---|---|---|---|
| **Priya** — the parallel power user | Staff engineer, 5-agent Friday | J1, J2 | Verification takes as long as merging manually, or is flaky |
| **Marco** — the reviewer | Tech lead, reviews agent-heavy PRs | J3 | Report is untrustworthy or unreadable, or adds a meeting |
| **Dana** — the platform owner | Staff infra, 40 devs, agents sanctioned | Policy, reproducibility | Tool phones home, or mutates global config |
| **Sam** — the solo builder | Indie, no tests, 1 repo | (anti-persona) | n/a — we do not serve them at MVP |

---

## 7. Core use cases

**UC-1 Verify a parallel batch before merging.** Developer has 3 branches from 3 agents.
Runs `ilvuno integrate --base main --branch a --branch b --branch c`. Gets: candidate
built or refused, conflicts listed deterministically, checks run on the combination,
evidence recorded, exit code non-zero on failure. *(J1, J2)*

**UC-2 Attach a passport to a task and resume it later.** `ilvuno task new`, add objective
and acceptance criteria, link agent runs and SHAs as work proceeds. Three days later,
`ilvuno task show` restores full context including what was verified and what is pending.
*(J4)*

**UC-3 Prove a claim.** An agent says "tests pass". `ilvuno verify` runs the configured
commands and records evidence; the passport shows `claim: tests pass — status: unverified`
until an artifact exists. *(J3)*

**UC-4 Bounded repair.** Verification fails. `ilvuno repair` generates a repair brief from
the recorded evidence, invokes the developer's configured repair command on a new branch,
re-verifies, and stops after N attempts with every attempt recorded. *(J5)*

**UC-5 Hand the batch to another agent with conflict context.** `ilvuno brief` emits a
concurrent-change brief in a format consumable by any agent (the 82% lever). *(J1)*

**UC-6 Refuse an unsafe integration.** Dirty source, moved ref, uncommitted changes, shared
working tree, unknown host — the tool refuses with a specific reason and does not proceed.
*(J6, security)*

Full step-by-step flows in `04-user-journeys.md`.

---

## 8. Product differentiation

Four defensible claims, ordered by strength of evidence:

1. **Combined verification with recorded evidence.** No surveyed product builds N-branch
   candidates and runs project checks on the result. Vendors can add this to a single
   surface; they cannot do it cross-tool, cross-agent, and cross-vendor. This is the moat.
2. **Claim → artifact binding.** "Done" is a claim. Ilvuno records SHA, command, exit code,
   and output digest, derived from git and process results. t3code's own PR #30507 confirms
   nothing exists (T3-030).
3. **Determinism separated from interpretation.** Findings are typed `deterministic` or
   `interpretation`; only deterministic findings affect exit status or block approval.
   Nobody in the category does this, and everyone in the category produces AI output that
   reviewers cannot audit.
4. **A durable task artifact, not a session.** Passports are plain files in the repo, owned
   by the developer, readable by any tool, diffable in review — unlike SQLite-bound session
   state (ORC-025, CR-051) that dies with the product.

What is **not** differentiation, and will not be marketed as such: worktrees, parallel
sessions, diffs, PR creation, phone control, cloud sandboxes. All commoditized or
vendor-owned (research `04` §6).

---

## 9. Competitive positioning

| | Dispatch | Isolation | Per-change verify | **Combined verify** | **Evidence ledger** | Cross-tool |
|---|---|---|---|---|---|---|
| t3code | ✅ | ✅ | CI only | ❌ | ❌ | ❌ |
| orca | ✅ | ✅ | CI only | ❌ | partial | ❌ |
| cloudroom | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Conductor / Paradev / Sculptor | ✅ | ✅ | CI only | ❌ | ❌ | ❌ |
| Copilot agent | ✅ | ✅ (cloud) | ✅ per PR | ❌ | partial | ❌ |
| Cursor cloud | ✅ | ✅ | ✅ per agent | ❌ | artifacts | ❌ |
| Clash / Grove | ❌ | ❌ | ❌ | predict only | ❌ | ✅ |
| Graphite | ❌ | n/a | ✅ queue CI | stacks only | ❌ | ❌ |
| **Ilvuno MVP** | ❌ (by design) | ✅ (git-native) | ✅ | **✅** | **✅** | **✅** |

Positioning statement: *Ilvuno sits between the agents you already run and the branch you
actually ship. It is the only step that verifies the combination and keeps the receipt.*

Distribution logic follows from the table: every row above is a potential **input source**
(they produce branches/worktrees) and a potential **host** (they can call the CLI or an MCP
tool). Being a layer, not a workspace, means we do not need to win the dashboard market.

**Competitive risks.** (a) A first-party vendor ships combined verification inside one
product — mitigated by cross-tool/cross-vendor scope and by evidence portability. (b)
Category churn: Vibe Kanban sunset, Crystal deprecated, reported Conductor acquisition —
read as a warning about standalone orchestrator UIs, which we deliberately are not building.

---

## 10. Functional requirements

Requirement format: ID · description · priority · user value · dependencies · acceptance
criteria (AC) · verification strategy (VS). Verification is by automated test unless a
manual/observational method is named.

### 10.1 Core: sources and candidate construction (`INT-1xx`)

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| INT-101 | Accept integration sources as git refs (branch, tag, SHA) from any producer: manual, orca, t3code, cloudroom, script. No agent-specific integration. | P0 | J1, J6 | — | Three refs from three different tools integrate in one run; no tool-specific config required | Fixture repo with 3 refs created by 3 different commit paths; e2e test |
| INT-102 | Resolve each source to an immutable commit SHA at start; record it. Refuse anything that cannot be pinned. | P0 | J2, J3 | INT-101 | Refused sources produce a typed `unpinnable_source` error and zero side effects | Test with dirty tree, bare `HEAD`, and a ref moving during run (race fixture) |
| INT-103 | Record each source's base ref and merge-base with the integration base. | P0 | J1 | INT-102 | Report includes per-source `base_ref`, `merge_base_sha`, `ahead`/`behind` counts | Assert against `git merge-base` in fixture |
| INT-104 | Fetch/refresh the integration base before building; fail if the refresh fails, unless `--allow-stale-base` is passed explicitly. | P0 | J1 | INT-103 | Default run fails closed on fetch error; `--allow-stale-base` records a warning in the report | Test with unreachable remote; assert exit code and report flag |
| INT-105 | Build the candidate in a disposable git worktree, never in the user's working tree, on a branch under `refs/ilvuno/candidates/<id>`. | P0 | J1, J6 | INT-102 | User's working tree and `HEAD` unchanged after any run, including failure | Hash working-tree state before/after across a matrix of success/failure/interrupt |
| INT-106 | Merge sources in a deterministic, recorded order (default: by base ancestry then source order; overridable). | P0 | J2 | INT-105 | Same inputs → identical candidate SHA across two runs (given fixed timestamps via env commit date) | Determinism test: run twice, compare candidate SHA and report hash |
| INT-107 | Record every merge step: pre-SHA, post-SHA, source, strategy, result (`clean`/`conflicted`/`skipped`). | P0 | J2 | INT-106 | Report contains a complete ordered step list; a failed step retains pre-SHA | Fixture with a conflict at step 2; assert step list |
| INT-108 | Detect textual conflicts **before** mutating anything, using `git merge-tree`-style analysis across all pairs vs base and the accumulated candidate. | P0 | J1 | INT-104 | Conflict prediction matches actual merge outcome for every fixture pair; prediction reported as `deterministic` | Property test over generated branch pairs compared against real `git merge` |
| INT-109 | Support partial integration: integrate a subset, report per-source inclusion. | P1 | J1 | INT-107 | Report lists `included` / `excluded` with reason for each source | Subset run fixture |
| INT-110 | Validate candidate creation and roll back cleanly on any failure (create, merge, verify). | P0 | J6 | INT-105 | After forced failure at each stage, no orphan worktree/branch remains | Fault-injection test per stage; assert `git worktree list` and ref list clean |
| INT-111 | Cap candidate size and refuse pathological inputs (branch count, file count, merge depth) with typed errors. | P1 | J1 | INT-106 | Limits configurable; breach refuses before any merge | Boundary tests at cap ± 1 |

### 10.2 Compatibility analysis (`INT-2xx`)

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| INT-201 | Report file-level overlap between sources: files touched by ≥2 sources. | P0 | J1, J2 | INT-103 | Overlap list is complete and derived from `git diff --name-only base..source` per source | Fixture with known overlap sets; assert exact set |
| INT-202 | Report hunk-level proximity: overlapping hunks merged cleanly but within a configurable line window. | P1 | J1 | INT-201 | Findings typed `deterministic`, severity `warning` | Fixture with adjacent-but-clean hunks |
| INT-203 | Detect dependency-manifest changes per source and flag version disagreements across sources (lockfiles and manifests from a configured list). | P0 | J1 | INT-201 | Two sources adding different versions of one package produce one `dependency_disagreement` finding naming both SHAs and both versions | Fixture monorepo with pnpm-lock and package.json divergence |
| INT-204 | Detect changed exported interface surfaces per source (declaration files, exported symbols, schema files) and flag a symbol changed by one source and referenced by another, when a configured symbol-index command is available. | P1 | J1, J2 | INT-201 | Findings typed `deterministic` when derived from index output; `interpretation` when derived from AI | Fixture with `index_command` stub; assert finding classification |
| INT-205 | Report base drift per source (how far the source's base has moved) as a `deterministic` warning above a configurable threshold. | P0 | J1 | INT-103 | Drift count and threshold in report; never silently ignored | Fixture with a moved base |
| INT-206 | Never present an AI interpretation as a fact. Every finding carries `classification: deterministic \| interpretation` and `evidence_ref`. | P0 | J2, J3 | INT-201 | Report fails validation if any finding lacks classification or evidence ref; schema test enforces it | Schema validation test over all fixture reports |
| INT-207 | AI-assisted interpretation is opt-in, off by default, and cannot change exit status or approval eligibility. | P0 | J3 | INT-206 | With interpretation enabled and producing findings, exit status and approval gate identical to disabled run | Paired test: same inputs, interpretation on/off, compare exit code and gate |
| INT-208 | Produce a machine-readable compatibility summary (JSON) and a human summary (Markdown) from one analysis pass. | P0 | J2 | INT-201 | Both artifacts derived from the same in-memory result; no divergence | Golden-file test; assert JSON→Markdown consistency |
| INT-210 | Estimate which findings would have been missed by per-branch verification (i.e., combination-only failures). | P1 | J1, J7 | INT-3xx | Report states `combination_only: <count>`, computed from findings whose cause requires ≥2 sources | Fixture where each branch is green and only the pair fails |
| INT-260 | Export a **concurrent-change brief**: a compact, tool-agnostic document describing what other streams are changing, suitable for injecting into an agent prompt. | P1 | J1, J6 | INT-201 | Brief is < configurable token budget; contains per-source file list and overlap warnings; no proprietary format | Snapshot test on fixture; second test injects brief into a stub agent command and asserts receipt |

### 10.3 Verification runner (`INT-3xx`)

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| INT-301 | Run project-configured verification commands against the candidate, in the candidate worktree, in declaration order. | P0 | J1, J2 | INT-105 | All configured commands execute against candidate only; cwd recorded | Test asserts cwd and env for each command |
| INT-302 | Support a named pipeline with per-command timeout, env overrides, and `required` flag. | P0 | J1 | INT-301 | Timeout kills the process group; `required: false` failure is recorded as `advisory` and does not fail the run | Timeout test with a sleeping command; advisory test |
| INT-303 | Refuse to skip verification: no `--no-verify` equivalent; a run with zero configured commands is `unverified`, not `passed`. | P0 | J3 | INT-301 | Repo without commands yields status `unverified` and exit code distinct from both pass and fail | Exit-code matrix test |
| INT-304 | Capture per-command evidence: argv, cwd, exit code, duration, truncated output, full-log reference, output digest. | P0 | J2, J3 | INT-301 | Evidence validates against schema; digest reproducible from stored log | Schema test + digest recomputation test |
| INT-305 | Run verification in an environment separated from the user's shell where possible: explicit env allowlist, no inherited agent credentials. | P0 | J3, security | INT-301 | Ambient tokens (e.g. `*_TOKEN`, `*_KEY`) absent from the child env unless explicitly allowed | Test asserts child env contents |
| INT-306 | Detect and report when a command's failure is caused by the combination rather than a single source: re-run the failing command against each source individually and record the comparison. | P1 | J1, J2 | INT-301 | Failing combined command yields per-source results; `combination_only` computed | Fixture: pair fails, each alone passes |
| INT-307 | Stream progress to stderr with per-stage status; never require a TTY. | P1 | J1 | INT-301 | Works in CI with `TERM=dumb`; no ANSI when not a TTY | Pipe test |
| INT-308 | Bound total run time; on interruption, leave the candidate and evidence in a resumable state (`REC-1xx`). | P0 | J4 | INT-301 | SIGINT yields resumable state, not corrupt state | Signal-injection test |
| INT-310 | Explicitly state, in the report and on first run, when a repository has no runnable verification, and what that means for the confidence of the result. | P0 | J3 | INT-303 | Report contains a `confidence: low` block with the reason; not hidden in logs | Report assertion on a no-tests fixture |

### 10.4 Evidence ledger (`EVD-1xx`)

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| EVD-101 | Append-only evidence store, self-contained per integration or per task, readable without Ilvuno. | P0 | J3, J4 | INT-304 | Deleting the binary leaves readable JSONL + logs; re-running analysis from the store reproduces the summary | Round-trip test: emit, remove binary, parse with a script |
| EVD-102 | Every claim is a record with `claim`, `status` (`verified`/`unverified`/`contradicted`), `evidence_refs[]`. | P0 | J3 | EVD-101 | Claim without evidence cannot be stored as `verified` (schema-level refusal) | Negative test |
| EVD-103 | Derive file-change claims from git, never from agent text; store agent-reported file lists separately and flag divergence. | P0 | J3 | EVD-102 | Divergence between reported and actual file list produces a `claim_divergence` record | Fixture with an agent report listing a file it did not change |
| EVD-104 | Record git provenance for every verified artifact: commit SHA, tree SHA, base SHA, ref. | P0 | J3 | EVD-102 | Evidence without a commit SHA cannot be `verified` | Schema test |
| EVD-105 | Content-address logs and outputs (digest) so evidence can be checked against tampering. | P1 | J3 | EVD-101 | Recomputed digest mismatch yields `tampered` status, not silently accepted | Tamper test |
| EVD-106 | Evidence is exportable as a single portable bundle (report + logs + digests + manifest) for attachment to a review. | P0 | J2 | EVD-101 | Bundle verifies offline; manifest lists every file with digest | Bundle verify test |
| EVD-107 | Record negative evidence: what was not run, what was skipped, and why. | P0 | J3 | EVD-102 | Report has an explicit `not_verified` section; empty only if truly all verified | Report assertion on skip fixtures |
| EVD-108 | Timeline: every record carries a monotonic sequence and UTC timestamp; ordering survives merge of two stores from different machines. | P2 | J4 | EVD-101 | Two stores merged produce a total order without duplicate sequence collisions | Merge test |

### 10.5 Repair (`INT-5xx`)

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| INT-501 | Generate a **repair brief** from recorded evidence: failing command, exit code, relevant log excerpt, conflicting sources, and the constraint that sources must not be modified. | P0 | J5 | EVD-104 | Brief contains all fields; brief is deterministic given the same evidence | Snapshot test |
| INT-502 | Invoke a developer-configured repair command with the brief on a new branch off the candidate; the tool does not embed an agent. | P0 | J5, J6 | INT-501 | Any executable works; no provider SDK dependency in MVP | Stub command test; dependency audit |
| INT-503 | Enforce an attempt budget (default 2) and re-verify after each attempt. | P0 | J5 | INT-502 | Exceeding the budget stops with `budget_exhausted`, all attempts recorded | Loop test with an always-failing stub |
| INT-504 | Preserve every failed attempt as evidence with its diff; never delete a failed attempt. | P0 | J5, J3 | INT-503 | Attempt count in report equals stored attempt diffs | Assertion over a 2-attempt failing run |
| INT-505 | Never modify source branches. Repair produces new refs only. | P0 | J6 | INT-502 | Source SHAs identical before/after any repair | Assertion across repair matrix |
| INT-506 | Escalate to the developer when the budget is exhausted, including a recommended next action. | P0 | J5 | INT-503 | Report contains `escalation` block with the last failure and a recommendation | Fixture assertion |

### 10.6 Review and approval gate (`INT-6xx`)

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| INT-601 | Produce a combined diff with per-source attribution (which source last touched each hunk/file). | P0 | J2 | INT-107 | Every changed file attributes to a source or to a merge step | Fixture assertion |
| INT-602 | Require explicit developer approval before any final integration action; there is no auto-approve flag. | P0 | J2, J3 | INT-601 | Non-interactive mode cannot approve; `--auto-approve` does not exist | CLI surface test asserting absence + non-interactive refusal |
| INT-603 | Approval is recorded with approver identity (local user + optional key), timestamp, candidate SHA, and the exact evidence set approved. | P0 | J3 | EVD-104 | Changing any input after approval invalidates it (typed `stale_approval`) | Test: approve, mutate a source, assert invalidation |
| INT-604 | Landing is out of scope: print the exact git commands for the developer to run, plus a PR body draft. | P0 | J6 | INT-601 | Tool performs no push/merge/PR creation | Assert no network writes in an instrumented run |
| INT-605 | Report is diffable and stable: same inputs → byte-identical report except timestamps/durations. | P1 | J3 | INT-601 | Normalized comparison passes | Determinism test |

### 10.7 Orchestration (`ORC-1xx`) — largely post-MVP

Ilvuno's MVP deliberately has no dispatch. It consumes the output of tools that dispatch.
These requirements exist so the seam is designed, not built.

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| ORC-101 | Never run two agent streams in one working tree; refuse if a source is a shared checkout. | P0 (seam) | J6 | INT-101 | A source that is the user's working tree is refused with `shared_workspace` | Test with the repo root as source |
| ORC-102 | Accept a machine-readable stream manifest (yaml) describing N planned streams: objective, base, file ownership hints, agent, worktree. | P1 | J1 | INT-101 | Manifest validated; unknown fields rejected (strict) | Schema tests |
| ORC-103 | Emit a stream manifest from an existing run so integration is reproducible without the originating tool. | P1 | J4 | ORC-102 | Manifest + pinned SHAs reproduce the integration on a clean clone | Clean-clone reproduction test |
| ORC-104 | Detect overlapping declared file ownership in a manifest at plan time (the C3 pre-flight, MVP-C style) without requiring merges. | P2 | J1 | ORC-102 | Overlap report produced from manifest alone | Manifest fixture |
| ORC-105 | Dispatch (starting agents) is explicitly out of MVP scope; if added, it must reuse the same isolation and evidence primitives and must cap concurrency. | P2 | — | ORC-102 | n/a until scheduled | Design review gate |
| ORC-106 | Expose the engine to host tools via a stable non-interactive interface (CLI JSON contract first; MCP tool later). | P1 | J6 | INT-208 | JSON schema versioned; breaking change requires major version | Contract test pinned in a schema file |
| ORC-107 | Provide a plugin contract for workspaces (orca/t3code-style) to call integration as a step, without Ilvuno owning their state. | P2 | J6 | ORC-106 | A stub host can invoke, receive findings, and display them | Stub host integration test |

### 10.8 Task Passport (`TSK-1xx`)

See §17 for the data model.

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| TSK-101 | Create a task with objective, acceptance criteria, and required checks. | P0 | J4 | — | Passport valid against schema; unusable until objective + ≥1 criterion present | Schema tests |
| TSK-102 | Record approved decisions as immutable entries with rationale, author, timestamp. | P0 | J4 | TSK-101 | Editing an approved decision requires a superseding entry, not mutation | Immutability test |
| TSK-103 | Link agent runs: agent name, model if known, run id, start/end, produced refs. | P0 | J4 | TSK-101 | Links resolve to real refs; dangling refs flagged | Dangling-ref test |
| TSK-104 | Record git references per task: base, stream branches, candidate, repair branches. | P0 | J4 | TSK-103 | Passport renders a git graph summary from refs alone | Fixture assertion |
| TSK-105 | Record verification results by reference to evidence bundles (not by copy). | P0 | J3, J4 | EVD-106 | Passport with missing bundle renders `evidence_missing`, not "pass" | Negative test |
| TSK-106 | Track pending approvals with `what`, `why`, `blocking`, `since`. | P0 | J4 | TSK-102 | `next actions` derived deterministically from pending approvals + failed checks | Derivation test |
| TSK-107 | Derive current state from records: `planning`, `in-progress`, `awaiting-verification`, `awaiting-approval`, `blocked`, `done`, `abandoned`. | P0 | J4 | TSK-106 | State is computed, never hand-set; illegal transitions rejected | State-machine property test |
| TSK-108 | Remain human-readable: one file, diffable, reviewable in a PR. | P0 | J4 | TSK-101 | File parses as plain YAML/JSON; a human can read it without the tool | Manual read + format test (no binary, no proprietary encoding) |
| TSK-109 | Store no absolute paths and no machine-identifying data in the passport. | P0 | J4 | TSK-108 | Lint rule fails the build if an absolute path appears | Path lint test |
| TSK-110 | Passport is portable: cloning the repo + fetching refs reproduces the task view on another machine. | P0 | J4 | TSK-109 | Second-machine reproduction test | Clean-clone test |
| TSK-111 | Support task linking (parent/child) for batches, without requiring a DAG engine. | P2 | J4 | TSK-107 | Parent renders rollup of child states | Rollup test |
| TSK-112 | Explicitly record what the passport does **not** contain: no conversation transcripts, no provider session ids presumed valid elsewhere. | P0 | J4 | TSK-108 | Schema has a `not_included` block, and docs restate it | Documentation + schema test |
| TSK-113 | Compaction: old evidence bundles may be pruned with a recorded tombstone retaining digests and summary. | P1 | J4 | TSK-105 | Pruned bundle leaves verifiable summary; report says `pruned` | Pruning test |

### 10.9 Verification and evidence policy (`VER-1xx`)

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| VER-101 | Never claim that independent success implies integration success, in any user-facing string. | P0 | J3 | — | String audit list in repo; test greps user-facing strings for banned claims | Automated banned-phrase test |
| VER-102 | Distinguish `passed`, `failed`, `unverified`, `inconclusive`, `error` — five states, never collapsed. | P0 | J3 | INT-303 | Exit-code map documented and tested; `inconclusive` used when a command did not run | Exit-code matrix test |
| VER-103 | A verification result is bound to a commit SHA; results for a different SHA are `stale`. | P0 | J3 | EVD-104 | Changing the candidate invalidates prior results with `stale` | Invalidation test |
| VER-104 | Report flakiness risk: repeated runs of the same candidate record per-run results, and a command that passes and fails is flagged. | P1 | J3 | INT-301 | Two-run disagreement yields `flaky` flag, not a coin-flip pass | Double-run fixture |
| VER-105 | Machine-checkable report schema versioned independently of the CLI version. | P1 | J6 | INT-208 | Consumers can pin schema major; additive changes allowed | Schema compatibility test |
| VER-106 | Any AI-produced text in artifacts is labelled and separable from deterministic sections at parse level. | P0 | J2 | INT-206 | JSON puts interpretation under a distinct key; removal leaves a valid report | Report validity test after stripping interpretation |

### 10.10 Failure and recovery (`REC-1xx`)

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| REC-101 | Integration is a state machine with explicit states and journaled transitions. | P0 | J4, J5 | INT-105 | Every transition persisted before its side effect | Crash-injection test at each transition |
| REC-102 | Never automatically repeat an uncertain side effect after a crash (the cloudroom rule, CR-016). | P0 | J3 | REC-101 | After crash mid-merge, resume reports `unknown` and requires an explicit decision | Crash + resume test matrix |
| REC-103 | `ilvuno resume` continues a run from recorded state or explains why it cannot. | P0 | J4 | REC-102 | Resume after SIGKILL at each stage either completes or states the exact reason | Signal matrix test |
| REC-104 | `ilvuno abort` cleans candidate worktrees/refs and records the reason. | P0 | J4 | REC-101 | Abort leaves no orphan artifacts | Orphan scan assertion |
| REC-105 | Idempotent commands with request ids for non-interactive use. | P1 | J6 | REC-101 | Replaying a command id returns the original result | Replay test |
| REC-106 | Every error is typed with a stable code and a next action; no stack trace as the primary message. | P0 | J4 | — | Error catalog in repo; test asserts every code has a documented action | Error-catalog completeness test |
| REC-107 | Never delete user work: candidate cleanup never touches source branches, dirty trees, or stashes. | P0 | J6 | INT-110 | Post-cleanup integrity scan finds user refs and stash intact | Cleanup matrix test |

### 10.11 Cross-device (`XDC-1xx`)

See §21.

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| XDC-101 | Core works fully offline with no account and no network call to Ilvuno. | P0 | J6 | — | Network-instrumented run shows zero outbound calls to Ilvuno endpoints | Egress test |
| XDC-102 | Passport and evidence are transport-agnostic: git, a file sync tool, or a future relay all produce an identical task view. | P0 | J4 | TSK-110 | Three transports, one view | Transport matrix test |
| XDC-103 | Passport schema is designed for a thin read/approve client: no server-side state required to render it. | P0 | J7 | TSK-108 | A static renderer produces the view from files alone | Static-render test |
| XDC-104 | Remote execution and notifications are optional services, never required for the core flow. | P0 | J6 | XDC-101 | Disabling services leaves every P0 scenario working | Feature-flag-off test |
| XDC-105 | Any future sync is end-to-end encrypted with no plaintext metadata on the server; approval actions are signed. | P2 | J7, security | XDC-104 | Design review gate before implementation | Threat-model review |

### 10.12 Open source (`OSS-1xx`)

| ID | Requirement | Pri | User value | Deps | Acceptance criteria | Verification |
|---|---|---|---|---|---|---|
| OSS-101 | Core is open source with an OSI-approved license; paid features live outside the core repo or behind a documented boundary. | P0 | J6 | — | License file present; no proprietary code in core paths | Repo audit |
| OSS-102 | No feature that a P0 requirement needs may require a network service or a paid key. | P0 | J6 | OSS-101 | Capability test with all network blocked passes P0 suite | Offline CI job |
| OSS-103 | Documented extension points (verification commands, symbol index, repair command, report schema) so third parties can extend without forking. | P1 | J6 | ORC-106 | A third-party adapter can be written using docs only | External integration test (dogfood) |
| OSS-104 | Contributions are testable without a proprietary harness: fixtures and golden files in-repo. | P1 | — | OSS-103 | `make test` green on a clean clone with no secrets | Clean-clone CI |

### 10.13 Nonfunctional requirements (`NFR-1xx`)

| ID | Requirement | Target | Verification |
|---|---|---|---|
| NFR-101 | Time to first integration from install: ≤ 10 minutes (install, init, configure one command, run) | Timed usability run on a fresh fixture repo, 3 users |
| NFR-102 | Overhead excluding project checks: ≤ 5 s for 5 sources on a 50k-file repo | Benchmark in CI, tracked per release |
| NFR-103 | Determinism: same inputs → same candidate SHA and byte-identical normalized report | Determinism test in CI |
| NFR-104 | Zero runtime dependencies beyond git and the project's own toolchain | Dependency audit; container test with minimal image |
| NFR-105 | Single binary, no interpreter, no node_modules for the tool itself | Distribution test on macOS/Linux; Windows in v0.2 |
| NFR-106 | Report and passport survive `ilvuno` uninstall | Round-trip test with binary removed |
| NFR-107 | Crash-safe: SIGKILL at any stage leaves recoverable state, never corrupt evidence | Signal matrix test |
| NFR-108 | Works on repositories up to 100k tracked files and 100 branches without pathological memory | Scale fixture benchmark |
| NFR-109 | No telemetry by default; opt-in only, with a visible local log of what would be sent | Instrumented test |
| NFR-110 | Accessibility of human output: report is plain Markdown, degrades to text-only terminals | Manual + snapshot |

---

## 11. Nonfunctional requirements (narrative)

Beyond the table: the tool must be **boring to operate**. It runs, it prints, it exits with
a meaningful code, and it leaves files a human can read. It must not require a daemon, a
database server, a login, a background process, or a browser. Every one of those is a
reliability liability and an operational surface we do not want in v0.1.

Performance posture: verification time is dominated by the project's own checks, so our
own overhead must be irrelevant by comparison. If our overhead is ever visible in a user's
timing, it is a bug.

---

## 12. MVP scope

Full boundary in `05-mvp-scope.md`. Summary:

**In:** single binary CLI; git refs as sources; deterministic candidate construction;
file/hunk/dependency/interface/drift analysis; verification runner with evidence capture;
append-only evidence store + portable bundle; repair via external command with budget;
combined diff with attribution; approval gate; Task Passport as a repo-committed file;
JSON + Markdown reports; macOS/Linux.

**Out:** dispatch, agent adapters, TUI/dashboard, desktop app, browser UI, hosted service,
relay, mobile, team features, billing, Windows (v0.2), AI interpretation on by default,
automatic conflict resolution, PR creation, push.

**Sequencing inside MVP:** (1) candidate + determinism + evidence; (2) analysis findings;
(3) verification runner; (4) passport; (5) repair; (6) approval/report polish. Ship 1–3
before 4–6 if time-boxed.

---

## 13. Explicit non-goals

1. Starting, steering, or hosting agents (MVP).
2. Merging, pushing, or opening PRs on the developer's behalf.
3. Resolving conflicts automatically, with or without AI.
4. Building a chat UI, terminal multiplexer, or file editor.
5. Being the developer's editor integration.
6. Migrating conversation transcripts between agents or devices.
7. Guaranteeing correctness. We report evidence; we do not certify software.
8. Serving repositories without runnable verification (we state low confidence instead).
9. Supporting Windows in the MVP.
10. Enterprise features: SSO, RBAC, audit export, retention policy.
11. Replacing CI. We complement it with pre-merge evidence; we do not run a build farm.
12. Monetizing the core. No license key, no seat check, no phone-home for local use.

---

## 14. User journeys

Full flows in `04-user-journeys.md`. Journey set: J-1 first run, J-2 happy-path batch,
J-3 conflict discovered deterministically, J-4 combination-only test failure, J-5 repair
loop with budget, J-6 returning after three days, J-7 refusal (unsafe input), J-8 CI usage
by a platform team.

Each journey specifies: trigger, steps, what the user sees, state written, exit code, and
the failure branches.

---

## 15. Integration Intelligence specification

### 15.1 Pipeline

```
sources ──▶ PIN ──▶ PREFLIGHT ──▶ CANDIDATE ──▶ VERIFY ──▶ CLASSIFY ──▶ RECORD
             │            │             │            │
             │            └── refuses before mutation
             └── immutable SHAs, no dirty trees
                                        └── disposable worktree, refs/ilvuno/*
                                                       └── project commands, exit codes
                                                                   └── deterministic vs interpretation
                                                                              └── append-only evidence
                                                          REPAIR (bounded, optional) ──▶ re-VERIFY
                                                          APPROVE (human, mandatory) ──▶ landed by developer
```

### 15.2 Stage contract

| Stage | Input | Output | Invariant | Failure mode |
|---|---|---|---|---|
| `pin` | refs, base | source list with SHAs, merge-bases, drift | nothing is mutated | `unpinnable_source`, `base_fetch_failed` |
| `preflight` | pinned sources | findings: conflicts, overlaps, dependency disagreement, interface touch, drift | predictions are marked `predicted`; no merge performed | `preflight_error` (fails closed) |
| `candidate` | pinned sources + order | candidate ref, step list, combined tree | user's tree untouched; rollback on any error | `merge_conflict`, `candidate_build_failed` |
| `verify` | candidate + pipeline | per-command evidence | zero configured commands ≠ pass | `command_timeout`, `command_error`, `no_verification_configured` |
| `classify` | findings + evidence | typed findings | interpretation cannot affect gate | `classification_missing` (report invalid) |
| `record` | everything | evidence bundle + report | append-only, content-addressed | `evidence_write_failed` |
| `repair` | failing evidence | new refs + attempts | sources immutable; budget enforced | `budget_exhausted` |
| `approve` | report + user | approval record bound to SHA | no auto-approve path | `stale_approval` |

### 15.3 Deterministic vs interpreted findings

Two separate namespaces in the data model, not two severities of the same thing.

**Deterministic** — reproducible from git, files, or process results. May block approval.

| Finding | Source | Blocks |
|---|---|---|
| `merge_conflict` | `git merge-tree` / real merge | yes |
| `file_overlap` | `git diff --name-only` per source | no (warning) |
| `hunk_proximity` | diff positions | no |
| `dependency_disagreement` | parsed manifests + lockfiles | yes if `on_conflict: block` |
| `base_drift` | `rev-list --left-right --count` | configurable |
| `interface_symbol_touched` | configured symbol index | warning |
| `check_failed` | exit code | yes |
| `check_timeout` | process supervision | yes |
| `claim_divergence` | git vs agent report | warning |
| `stale_approval` | SHA mismatch | yes |
| `flaky_check` | two runs disagree | warning |

**Interpretation** — model-generated, always labelled, never blocking, always removable.

| Finding | Why it exists | Rule |
|---|---|---|
| `likely_semantic_conflict` | points a human at a suspicious region | must cite file:line in both sources; no citation → dropped |
| `suggested_repair` | seed for the repair brief | cannot be the only content of a repair brief |
| `risk_summary` | review aid | absent from the machine findings array |

Schema rule: findings array entries carry `classification`. The report validator rejects a
report whose interpretation entries are indistinguishable from deterministic ones
(`INT-206`). Exit status is computed **only** from deterministic findings (`INT-207`).

### 15.4 What the report may and may not claim

Must:
- state the exact source SHAs and base SHA;
- state which verification commands ran, and which were configured but skipped;
- distinguish `combination_only` failures from per-source failures where measured;
- state confidence (`low`) when no verification was configured.

Must not:
- say "ready to merge" (say "no deterministic blocking findings");
- say "verified" when status is `unverified` or `inconclusive`;
- imply that a clean candidate proves correctness in production;
- present an interpretation in the same visual weight as a deterministic finding.

A banned-phrase test enforces the most important ones (`VER-101`).

### 15.5 Ordering and conflict policy

Default merge order: ascending by merge-base age (oldest base first), then source order as
declared. Rationale: reduces cascading rebases and makes the first conflict the most
informative. Overridable with `--order`. When a conflict occurs at step *k*, the run stops
by default; `--continue-on-conflict` records the conflict, skips that source, and continues
so the developer sees all conflicts in one pass. Both behaviours are recorded in the step
list, so the report is never ambiguous about what happened.

Unresolvable-by-policy cases (huge candidate, >N sources, merge requires network) refuse
with a typed error rather than degrading silently.

---

## 16. Agent orchestration requirements

MVP position: **no dispatch.** Ilvuno is an integration layer that consumes artifacts.
This is a deliberate product decision, not a limitation to apologize for, and it is the
research's recommended direction (research `06` §9).

Requirements that still bind the MVP:

| ID | Requirement | Why |
|---|---|---|
| ORC-101 | Never share a working tree between streams | Shared-folder parallelism is an open failure mode in all three products (T3-031, ORC-033, CR-024) |
| ORC-102 | Stream manifest is the plan format, strict-validated | Reproducibility and cross-tool input |
| ORC-103 | Manifest can be emitted from an existing run | Lets users adopt without changing tools |
| ORC-106 | Stable non-interactive contract (JSON, schema-versioned) | Host tools call it; hosts own their own UX |
| ORC-107 | Plugin contract, post-MVP | A workspace can add "integrate" as a step |

When dispatch is eventually added (post-MVP, gated by metrics in §24), it must:
- reuse the same isolation and evidence primitives, not a parallel implementation;
- cap concurrency and enforce admission control (cloudroom's shared-sandbox OOM, CR-044);
- treat delegated completion as a claim, never as a verified result (T3-030, ORC-042);
- never let a child inherit broader permissions than its parent (T3-028 dispatch-mode limit
  is prior art worth copying);
- hand the child the concurrent-change brief, given the 82% result (research `04` §5).

---

## 17. Task state and session requirements

### 17.1 Passport model

One file per task, committed to the repo (default `docs/ilvuno/tasks/<slug>.yaml`), plus
local evidence under `.ilvuno/` (gitignored) and exportable bundles.

```yaml
schema: ilvuno.dev/passport/v1
task: checkout-split                # stable slug
objective: >
  Split checkout into payment-intent and fulfilment flows without changing the
  public API of packages/api.
acceptance_criteria:
  - id: AC-1
    text: "packages/api public surface unchanged (symbol index diff empty)"
    check: api-surface              # references a configured check
  - id: AC-2
    text: "checkout e2e suite green on the integrated candidate"
    check: e2e
decisions:
  - id: D-1
    text: "Keep legacy /checkout endpoint; add /checkout/v2"
    rationale: "Mobile 1.x still calls the old route"
    decided_by: priya
    at: 2026-11-19T10:04:00Z
    supersedes: null
state: awaiting-verification        # derived, not authored
streams:
  - source: refs/heads/agent/payments-intent
    agent: claude-code
    run_id: local-2026-11-19-01
    sha: 4b825dc642cb6eb9a060e54bf8d69288fbee4904
    base_sha: 7c4758d5e89f330f645bba6d7099e97192fe45e7
verification:
  - bundle: .ilvuno/bundles/2026-11-19T14-02-11Z/manifest.json
    status: failed                  # passed | failed | unverified | inconclusive | error
    candidate_sha: 9f2c...
    failed_checks: [e2e]
    combination_only: true
approvals:
  - id: A-1
    what: "land candidate 9f2c onto main"
    blocking: true
    since: 2026-11-19T14:05:00Z
next_actions:                        # derived
  - "Repair e2e failure on candidate 9f2c (1 attempt used of 2)"
  - "Re-verify after repair"
not_included:
  - conversation transcripts
  - provider session ids (not portable across machines)
  - credentials or environment values
```

### 17.2 Requirements

- **Durable** — plain text, versioned schema, committed to git, readable without the tool
  (`TSK-108`).
- **Derived, not authored, where possible** — `state`, `next_actions`, and `verification`
  are computed from records so they cannot drift (`TSK-107`).
- **Immutable decisions** — a changed decision is a new entry with `supersedes`
  (`TSK-102`).
- **Portable** — no absolute paths, no machine identity, no conversation content
  (`TSK-109`, `TSK-110`).
- **Honest about boundaries** — `not_included` is part of the schema, not a footnote
  (`TSK-112`).

### 17.3 Task persistence is not conversation migration

Explicit and important:

- A passport records **what was decided and what was verified**. It does not contain a
  transcript, a token stream, or a provider session.
- Native agent resume is provider-specific and environment-specific: t3code resumes via
  adapter `resumeThread`; orca via `--resume <id>`; cloudroom via a native transcript file
  (T3-009, ORC-022, CR-016). None of these survive being moved to another machine with the
  same guarantees.
- Therefore Ilvuno never promises "continue the same conversation elsewhere." It promises
  "know exactly where this task stands and what to do next, anywhere." A new agent session
  can be started from that state — deliberately, with the objective and constraints in
  front of it — which is a better outcome than pretending a transcript migrated.
- Prior art on partial transfer is instructive: cloudroom's Teleport moves the native file
  exactly but leaves uncommitted tracked edits on the laptop (CR-021). That asymmetry is
  precisely why we separate durable *task* state from fragile *session* state.

---

## 18. Verification and evidence requirements

Covered by `VER-1xx` and `EVD-1xx`. The three load-bearing rules:

1. **Five states, never three.** `passed`, `failed`, `unverified`, `inconclusive`,
   `error`. A run with no checks is `unverified`, never `passed` (`INT-303`, `VER-102`).
2. **Evidence outlives the tool.** Self-contained, content-addressed, portable
   (`EVD-101`, `EVD-106`, `NFR-106`).
3. **Claims are claims.** Agent-declared facts are recorded as claims and marked
   `unverified` until an artifact exists; agent-reported file lists are compared against
   git and divergence is recorded (`EVD-102`, `EVD-103`).

Negative evidence is a requirement, not a nicety: `not_verified` is an explicit section
(`EVD-107`). A review comment that says "these three checks ran, these two did not, here is
why" is more useful than a green badge.

---

## 19. Failure and recovery UX

### 19.1 Principles

- **Fail closed, explain plainly, name the next action.** Every error has a stable code, a
  one-sentence cause, and a concrete command or decision (`REC-106`).
- **Never repeat an uncertain side effect** (`REC-102`). After a crash, the tool says what
  it does not know.
- **Never destroy work** (`REC-107`). No cleanup path touches user refs, stashes, or dirty
  trees.
- **Resumable, not restartable.** `ilvuno resume` uses recorded state; it does not redo
  merges it already performed.

### 19.2 Message anatomy

```
✗ integration failed — check_failed (e2e)

  candidate   9f2c4a1  (3 sources, base 7c4758d)
  failed      pnpm test --filter checkout   exit 1   18.4s
  alone       payments-intent: passed · fulfilment: passed
  combined    failed  ← combination-only

  evidence    .ilvuno/bundles/2026-11-19T14-02-11Z/
  next        ilvuno repair --bundle 2026-11-19T14-02-11Z
              ilvuno show  --bundle 2026-11-19T14-02-11Z --logs e2e
```

### 19.3 Failure catalogue (MVP)

| Code | Meaning | Exit | Next action |
|---|---|---|---|
| `unpinnable_source` | dirty tree / moving ref / no SHA | 2 | commit or pass a SHA |
| `base_fetch_failed` | cannot refresh base | 2 | retry, or `--allow-stale-base` (recorded) |
| `shared_workspace` | source is a shared checkout | 2 | create a worktree |
| `merge_conflict` | textual conflict | 3 | see conflicting files + sources |
| `dependency_disagreement` | manifest versions disagree | 3 | `--on-dependency-conflict=warn` to proceed |
| `no_verification_configured` | no checks | 4 | configure `verification.commands` |
| `check_failed` | command exit ≠ 0 | 1 | `ilvuno repair` |
| `check_timeout` | timeout | 1 | raise timeout or split check |
| `flaky_check` | two runs disagree | 1 | investigate; result is not a pass |
| `budget_exhausted` | repair attempts used | 1 | human decision |
| `interrupted` | crash/signal mid-run | 130 | `ilvuno resume` |
| `stale_approval` | inputs changed after approval | 3 | re-run and re-approve |
| `evidence_write_failed` | disk/permissions | 2 | fix storage; run is invalid |

Exit codes are part of the public contract (`ORC-106`), documented, and tested as a matrix.

### 19.4 Degradation ladder

1. Verification commands configured and green → full confidence report.
2. Commands configured but flaky/inconclusive → report says so, gate stays closed.
3. No commands configured → `unverified`, confidence low, explicit statement (`INT-310`).
4. No git available or repo shallow in a way that breaks merge-base → refuse with reason.
5. Disk full, permission denied → refuse before mutating.

Never a silent downgrade. Every rung is visible in the output and in the report.

---

## 20. Security and privacy

### 20.1 Posture

- Local-first, no account, no telemetry by default (`XDC-101`, `NFR-109`).
- Least privilege: read-only on sources, write only to `refs/ilvuno/*` and `.ilvuno/`
  (`INT-105`, `INT-505`).
- No secrets handling beyond not leaking them: verification child processes get an
  allowlisted env, never ambient agent credentials (`INT-305`).
- Evidence may contain source code and log output. It is local by default and only shared
  when the user exports a bundle. Bundles can be exported redacted (`SEC-1xx` below).

### 20.2 Rules that are non-negotiable from day one

Derived from the anti-pattern table (§3.2) and research `06` §6:

| Rule | Rationale |
|---|---|
| Guards fail closed | cloudroom's guard fails open (CR-037) |
| Restrictive defaults, explicit elevation | t3code defaults to full access (T3-060) |
| Constant-time comparison for any token | orca (ORC-072) |
| No silent trust on first contact | orca SSH TOFU (ORC-073) |
| Secrets: refuse rather than downgrade to plaintext | orca (ORC-074) |
| Signed webhooks only | t3code (T3-064) |
| Never mutate global agent/tool config | orca writes `~/.claude/settings.json` (ORC-016) |
| No custom crypto in MVP | orca's E2EE with a v1 downgrade path (ORC-067) |

### 20.3 Requirements

| ID | Requirement | Pri | AC | VS |
|---|---|---|---|---|
| SEC-101 | Tool never writes outside `.ilvuno/`, `refs/ilvuno/*`, and the configured task directory. | P0 | Path audit shows no other writes, including on failure paths | Strace/instrumented FS test across scenario matrix |
| SEC-102 | Verification child env is an allowlist; a denied variable's presence in the child fails the test suite. | P0 | Child env contains only allowlisted vars + explicit overrides | Env assertion test |
| SEC-103 | Evidence bundle export supports redaction rules (paths, env values, regex patterns) applied before write. | P1 | Redacted bundle contains no match for configured patterns | Redaction test with seeded secrets |
| SEC-104 | Report and evidence never contain values of env vars matching the secret-name heuristic. | P0 | Grep over artifacts finds none | Fuzz fixture with seeded values |
| SEC-105 | When future network features exist, TLS with certificate validation, no plaintext fallback, no downgrade path. | P2 | Design review before merge | Review gate |
| SEC-106 | Dependency surface is auditable: no transitive runtime deps in the CLI. | P0 | Dependency audit lists only stdlib + git subprocess | `NFR-104` audit in CI |
| SEC-107 | Any hosted component receives ciphertext and opaque ids only; no repo names, file names, or diffs. | P2 | Metadata inspection of relay logs shows only opaque tokens | Design review + log audit |

### 20.4 Privacy of the passport

The passport contains repository-internal information (objectives, decisions, SHAs). It
lives in the repo and is therefore shared with everyone who can read the repo. We state
this plainly in docs, and `TSK-112` records what is deliberately absent. Users who must not
commit it can keep it locally with a documented flag — but then portability is their
responsibility.

---

## 21. Cross-device continuity strategy

### 21.1 Position

Continuity at MVP means: **the task state is portable**, not that a session follows you.
A developer can read and resume the task from a laptop, a second machine, or (later) a
phone, because the state is plain files with no machine coupling.

Explicitly rejected for MVP: a headless always-on runtime (orca/t3code model), and
snapshot-and-resume session transfer (cloudroom Teleport model). Both are significant
engineering investments justified only after task continuity demonstrably matters
(`XDC-1xx`).

### 21.2 Transport ladder

| Rung | Mechanism | Cost | When |
|---|---|---|---|
| 1 (MVP) | Passport in git; evidence bundles shared manually or via any file sync | zero | now |
| 2 (v0.3) | Optional encrypted metadata sync (passport + bundle manifest, not contents) | low | when users report multi-machine friction |
| 3 (v0.5) | Optional relay for remote verification runs and push notifications | medium | when a paying cohort asks for it |
| 4 (v1+) | Mobile read/approve client over the same relay | high | when rung 3 has revenue |

### 21.3 Design constraints that keep every rung open

- Passport shape independent of transport (`XDC-102`).
- Static renderable (`XDC-103`) — a phone client needs no server logic to display a task.
- All network features optional and off by default (`XDC-104`).
- Any sync E2EE with signed approvals (`XDC-105`).
- Evidence addressing is content-based, so a device can fetch only what it needs.

---

## 22. Open-source strategy

**License:** recommend Apache-2.0 (patent grant matters for a tool that runs in build
pipelines; permissive enough for platform teams). Alternative: AGPL-3.0 to discourage
hosted clones — but that shrinks enterprise adoption, which is where the paid cohort lives.
Decision required (`docs/decisions/open-questions.md`, A-2).

**Boundary:** the CLI, the engine, the schemas, the passport model, and all verification
functionality are open source forever and require no service (`OSS-102`). Revenue comes
from hosting, continuity, and collaboration — never from gating local verification.

**Governance:** single-maintainer benevolent-dictator for v0.x, with a public roadmap and a
written decision log. Contributor requirements are minimal: tests, fixtures, no secrets in
CI (`OSS-104`).

**Ecosystem posture:** explicit third-party extension points (`OSS-103`) and a stable
machine contract (`ORC-106`). The goal is for orca/t3code-style workspaces to call Ilvuno
as a step and for Clash/Grove-style predictors to feed it. Being a layer means surviving
someone else's product churn.

**Anti-goal:** do not fork or embed an existing workspace, even in a future desktop shell.
That path leads directly into the commoditized, commercially fragile segment.

---

## 23. Monetization hypotheses

Details in `06-monetization.md`. Summary of what is *not* monetized: local execution, git
integration, integration intelligence, verification, evidence, passport.

| Hypothesis | Offering | Evidence strength |
|---|---|---|
| H1 | **Hosted relay**: run verification on a remote/always-on machine, reachable from any device | Medium — remote execution is a real pattern (T3-052, ORC-063), but people already have CI |
| H2 | **Encrypted metadata sync + notifications**: passport and approval state across devices, push when a batch is ready or blocked | Medium — mobile continuity is universally desired, unreliably delivered (ORC-097) |
| H3 | **Team workflows**: shared approval queue, evidence retention, policy checks on integration | Medium-high — this is the only offering a team buys rather than an individual |
| H4 | **Managed CI integration**: an integration step that runs in the team's existing CI with hosted evidence retention | High, but competes with "just run the CLI in CI", which is free |
| H5 | **Support/services**: setup, custom verification adapters | Low ceiling, high effort |

Target: **$500–$1,000 MRR is a hypothesis, not a plan.** At $12–20/user/month that is
roughly 30–80 paying individuals; at $200–500/team/month that is 2–5 teams. The second is
far more plausible given that the buyer is a team and the value is shared evidence.

---

## 24. Success metrics

Details in `07-success-metrics.md`. Headline set:

| Metric | Target (first 6 months) | Why |
|---|---|---|
| **North star:** verified integrations per active developer per week | ≥ 3 | Measures real use, not installs |
| Combination-only failures detected / total integrations | ≥ 5% (the value signal; if ~0%, the pitch is wrong) | Direct test of the core hypothesis |
| Time to verified candidate (p50) | ≤ 5 min excluding project check time | Must be cheaper than manual merging |
| First integration within 10 min of install | ≥ 70% of new users | Onboarding viability |
| Integrations with complete evidence | ≥ 95% | Evidence is the product |
| False-blocking rate (deterministic finding that was not a real problem) | ≤ 2% | Determinism must be trustworthy |
| Approval-then-land conversion | ≥ 60% | Report is actually used to decide |

Guardrail metrics: crash rate per 100 runs, `resume` success rate, report determinism
failures, verification timeout rate, uninstall-without-first-run.

---

## 25. Risks and unknowns

| ID | Risk | Likelihood | Impact | Mitigation / test |
|---|---|---|---|---|
| R-1 | Real combined-failure rate is low → nobody cares | Medium-high | Fatal | Measure on user repos from day one; ship the metric in the report |
| R-2 | Projects lack runnable checks → value collapses | Medium | High | Target selection; `INT-310` honesty; allow single-command verification |
| R-3 | Verification is slow → tool is not in the loop | Medium | High | Allow subset checks; parallel commands; report check duration ranking |
| R-4 | A vendor ships combined verification first | Medium | Medium | Cross-tool scope, portable evidence, layer not workspace |
| R-5 | Determinism breaks on real repos (submodules, LFS, sparse) | Medium | Medium | Fixture matrix; refuse rather than degrade |
| R-6 | Conflict predictions disagree with real merges → trust loss | Low-medium | High | Predictions are always verified by the real merge; never the sole basis |
| R-7 | Evidence stores grow unbounded | Medium | Medium | Compaction with tombstones (`TSK-113`) |
| R-8 | Windows users excluded at MVP | High | Low | v0.2, documented |
| R-9 | Willingness to pay below the hypothesis | Medium-high | High | Team-first monetization; price test before building hosted features |
| R-10 | Maintenance burden of cross-platform process supervision | Medium | Medium | One binary, minimal surface, no daemon |

Unknowns that must be resolved before scaling engineering (see `open-questions.md`):
real conflict rate; whether evidence changes review behavior; whether teams or individuals
pay; whether repair-with-budget is used at all or is theater.

---

## 26. Acceptance criteria

An MVP is acceptable when **all** of the following pass on a clean machine:

1. `install` → `init` → configure one command → `integrate` completes in ≤ 10 minutes on
   the fixture repo, with no account, no network, no daemon. (`NFR-101`, `OSS-102`)
2. Three branches from three different producers integrate into a candidate; the same
   command twice yields identical candidate SHA and identical normalized report.
   (`INT-106`, `INT-107`, `NFR-103`)
3. A conflicting pair is predicted before any mutation and confirmed by the real merge;
   exit code is distinct from a verification failure. (`INT-108`, `VER-102`)
4. A pair that is individually green and jointly red produces
   `combination_only: true`, with per-source re-runs recorded as evidence. (`INT-306`,
   `INT-210`)
5. A repo with no configured checks reports `unverified` with confidence low, and never
   `passed`. (`INT-303`, `INT-310`)
6. Agent-reported file lists that disagree with git produce `claim_divergence`. (`EVD-103`)
7. The evidence bundle validates offline after the binary is deleted, and its manifest
   digests match. (`EVD-101`, `EVD-106`, `NFR-106`)
8. SIGKILL at each of the six pipeline stages followed by `resume` either completes or
   states a precise reason; no orphan worktree, branch, or partially written bundle
   remains. (`REC-101`–`REC-104`, `NFR-107`)
9. The user's working tree, `HEAD`, stashes, and source branches are byte-identical after
   every scenario, including failures and repair runs. (`INT-105`, `INT-505`, `REC-107`)
10. A passport written on machine A renders identically from a clean clone on machine B,
    containing no absolute paths. (`TSK-109`, `TSK-110`)
11. Banned marketing phrases ("ready to merge", "verified" for unverified states, "guarantees")
    are absent from all user-facing strings — enforced by test. (`VER-101`)
12. No outbound network call to any Ilvuno endpoint exists in the P0 suite. (`XDC-101`)

---

## 27. Release readiness requirements

**Before public v0.1:**

- [ ] All P0 acceptance criteria green in CI on macOS + Linux, two git versions.
- [ ] Fixture corpus in-repo: clean pair, conflicting pair, semantic-only pair, dependency
      divergence, no-tests repo, monorepo, submodule repo, LFS repo. (`OSS-104`)
- [ ] Error catalogue complete; every code documented with a next action. (`REC-106`)
- [ ] Report and passport schemas published with a version and a compatibility policy.
      (`VER-105`)
- [ ] Security review against §20.2 rules, with a written result per rule.
- [ ] Egress test proving zero Ilvuno network calls. (`XDC-101`)
- [ ] Docs: 10-minute quickstart, verification configuration guide, passport reference,
      "what this tool does not prove" page.
- [ ] Determinism test in CI comparing report bytes across runs. (`NFR-103`)
- [ ] Rollback story documented: uninstalling leaves no state the user cannot read.
      (`NFR-106`)
- [ ] License, CONTRIBUTING, decision log, and a public statement of non-goals (§13).
- [ ] Known-issues doc naming Windows, submodules, and non-git VCS as unsupported.

**Before v0.2:** Windows support, `--continue-on-conflict` matrix, flaky-check double-run
default, compaction. (`TSK-113`, `VER-104`)

**Before any paid feature:** at least 20 weekly-active developers, ≥ 5% combination-only
failure rate observed, and ≥ 3 teams asking for the same continuity feature. Otherwise do
not build it.

---

## Appendix A: Banned claims (enforced by test)

- "ready to merge" / "safe to merge"
- "guarantees" / "proves correctness"
- "verified" applied to a state other than `passed` with a bound SHA
- "all checks pass" when any configured check was skipped
- "works with any agent" without the qualifier "any agent that produces git commits"
- "replaces CI"

## Appendix B: Requirement count

| Domain | Count | P0 | P1 | P2 |
|---|---|---|---|---|
| INT-1xx (construction) | 11 | 9 | 2 | 0 |
| INT-2xx (analysis) | 10 | 7 | 3 | 0 |
| INT-3xx (verification) | 9 | 6 | 2 | 0 (1 gap: INT-309 unused) |
| INT-5xx (repair) | 6 | 6 | 0 | 0 |
| INT-6xx (review/approval) | 5 | 4 | 1 | 0 |
| EVD-1xx | 8 | 6 | 2 | 0 |
| ORC-1xx | 7 | 2 | 3 | 2 |
| TSK-1xx | 13 | 11 | 1 | 1 |
| VER-1xx | 6 | 4 | 2 | 0 |
| REC-1xx | 7 | 7 | 0 | 0 |
| XDC-1xx | 5 | 4 | 0 | 1 |
| OSS-1xx | 4 | 2 | 2 | 0 |
| SEC-1xx | 7 | 4 | 1 | 2 |
| NFR-1xx | 10 | — | — | — |
