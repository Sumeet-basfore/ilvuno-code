# 02 — Milestones

Every milestone has the same eleven sections so reviews can be mechanical. M0–M6 are the approved
MVP scope; M7–M12 are **provisional** and carry no dates (see `01-master-roadmap.md` §3).

Requirement IDs → `docs/product/02-prd.md`. ADRs → `docs/adr/`. Task IDs → `docs/tasks/`.

---

## M0 — Repository setup and deterministic fixtures

**Objective.** Create a repository whose test harness can express every failure class in the
product, deterministically, before any product code exists.

**User benefit.** Indirect but decisive: every later claim ("we detected the conflict", "the run is
reproducible") is only as trustworthy as the fixtures behind it. A weak forge produces a product
that passes its own tests and fails in the real world.

**Deliverables.**
1. Go module, `cmd/ilvuno`, `internal/` skeleton with the package layout from architecture `01` §3.
2. `Makefile` targets: `build`, `test`, `test-scenario`, `bench`, `lint`, `forge-verify`.
3. CI skeleton (GitHub Actions): linux + macOS matrix, unit + scenario jobs, determinism job stub.
4. License (Apache-2.0), `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `.gitignore`.
5. **Fixture forge** (`internal/testkit/forge`): deterministic repo construction with fixed
   identity, dates, `TZ=UTC`, `umask 022`, `GIT_CONFIG_GLOBAL=/dev/null`.
6. The nine failure-class fixtures (architecture `11` §3) as forge specs.
7. **Safety fingerprint** helper (`internal/testkit/safety`): refs + tree + index + stashes + config.
8. E-1 spike result recorded as an ADR amendment or a note in `docs/research/`.

**Dependencies.** Approval of `A-1`/`A-3` (language), `A-2` (license), B-3 (git ≥ 2.38 check).

**Explicit scope exclusions.**
- No `ilvuno` subcommands beyond `--version`.
- No journal, no worktree creation, no config parsing.
- No fixture that requires network, LFS, or submodules (refused paths only).
- No benchmark harness yet (that lands with `T-0305`).

**Development tasks.** `T-0001`…`T-0005` (see `docs/tasks/`).

**Acceptance criteria.**
| # | Criterion |
|---|---|
| A1 | `make test` passes on a clean clone with no network and no global git config |
| A2 | The forge produces byte-identical repo state across two runs on the same OS (refs + tree SHAs compared) |
| A3 | All nine fixture classes exist and each fails/passes in the way its spec says |
| A4 | `make forge-verify` reports the forge's own self-test green |
| A5 | Safety fingerprint detects each of: ref move, dirty tree, staged change, stash creation, config edit |
| A6 | E-1 result is recorded: does `merge-tree` see a triple-only conflict? (yes/no + evidence) |

**Required tests.** `forge_test.go` self-tests; `safety_test.go`; fixture smoke tests asserting each
fixture's expected shape (`git rev-list --count`, conflict presence, per-branch green/red).

**Security considerations.** Fixtures must not read `$HOME` or ambient git config
(`GIT_CONFIG_GLOBAL=/dev/null`, `HOME` redirected in tests) — a developer's global config silently
changing fixture SHAs is the failure this prevents.

**Review requirements.** One human review. Fixture specs reviewed for *intent* ("does this fixture
actually represent the failure class?") not just correctness.

**Definition of Done.** `07-definition-of-done.md` §3 (code) + §5 (fixture) + gate G0.

**Rollback / recovery.** Nothing to roll back (no persistent state). If the forge proves unable to
produce deterministic repos, M0 is re-scoped to a checked-in fixture corpus generated once and
verified by digest — a weaker but workable fallback, recorded as an ADR amendment.

---

## M1 — Read-only git and worktree analysis

**Objective.** Read a repository safely: discover its conditions, resolve configuration, pin inputs
to SHAs, and refuse unsafe states — with zero writes.

**User benefit.** `ilvuno doctor` becomes useful on its own: it tells a developer whether their repo
is in a state Ilvuno can reason about, before they invest a run.

**Deliverables.**
1. `gitx` execution wrapper: argv construction, `-c` overrides, `LC_ALL=C`, `--no-pager`,
   `gc.auto=0`, stdout/stderr capture, typed errors, version probe.
2. `repo` discovery + refusals (bare, shallow, submodule, sparse, unusual layout, LFS-missing).
3. `config` loader/validator for `ilvuno.toml` (strict keys, denied-env list, defaults).
4. `pin` implementation: fetch policy, ref→SHA, merge-base, ahead/behind, drift, duplicate and
   self-source refusal.
5. `worktree` discovery + ownership classification (ours / read-only / orphaned), no creation.
6. `ilvuno doctor` and `ilvuno init` (writes only `ilvuno.toml` + `.gitignore` line).

**Dependencies.** M0 (fixtures, safety fingerprint).

**Explicit scope exclusions.** No worktree creation or removal; no merges; no verification; no
journal (errors are typed but not persisted); no passport; no `integrate` command.

**Development tasks.** `T-0101`…`T-0105`.

**Acceptance criteria.**
| # | Criterion |
|---|---|
| A1 | `doctor` refuses each of the seven repo conditions with the documented code and a fix command |
| A2 | `pin` returns SHAs + merge-bases + drift for 3 sources; a moving ref does not change a pinned SHA |
| A3 | Config validation rejects unknown keys, denied env names, and missing command fields |
| A4 | Ownership classification marks a user worktree read-only and never offers to adopt it |
| A5 | Safety fingerprint is unchanged after every M1 scenario (proves read-only) |
| A6 | Engine overhead for `pin` on the 50k-file fixture ≤ 800 ms (architecture `12` §9) |

**Required tests.** `T-REPO-1..7` (each refusal), `T-PIN-1..5`, `T-CFG-1..4`, `T-SAFE-1`
(read-only variant), `T-WT-1..3` (discovery only).

**Security considerations.** `doctor` output must contain no absolute paths outside the repo, no env
values, no user identity (`T-SEC-9`). Config `env_allow` matching the denied list must fail closed
at load, not at spawn.

**Review requirements.** One human review; specifically re-walk ADR-0005 (no ref re-resolution) and
ADR-0004 (parsers only on documented formats).

**Definition of Done.** §3 code DoD + §4 error-catalogue DoD + gate G1.

**Rollback / recovery.** No persistent state to corrupt beyond `ilvuno.toml` (user-owned, and `init`
refuses to overwrite). If `merge-tree` support is absent, `doctor` reports it and M2 degrades
explicitly — no rollback needed.

---

## M2 — Disposable integration candidate creation

**Objective.** Turn N pinned SHAs into one deterministic candidate commit in a disposable worktree,
without touching anything the user owns.

**User benefit.** The first moment the product does something no other tool does: "these three
branches, together, produce this exact commit — and I can reproduce it."

**Deliverables.**
1. `journal` v1: append-only JSONL, per-record fsync, SHA-256 chain, contiguous `seq`, 32 MiB
   roll-over, reader with torn-tail tolerance, `state.json` as rebuildable cache.
2. Policy gate (`policy`): the single write-scope check for paths and refs.
3. Worktree lifecycle: create (`--no-checkout` + `reset --hard <sha>`), rollback on failure,
   removal with patch archiving, orphaned state.
4. Determinism envelope: fixed identity/date env, version outside the commit.
5. `merge-tree` preflight: pairwise + tuple, prediction vs real reconciliation.
6. Candidate construction: deterministic order, `--no-ff` steps, conflict abort/skip, partial
   integration, `meta` ref for versions.
7. `ilvuno integrate --dry-run` producing a step list and a candidate, no verification.

**Dependencies.** M1.

**Explicit scope exclusions.** No analysis findings beyond conflicts; no verification; no report
beyond a step list; no repair; no passport; no approval.

**Development tasks.** `T-0201`…`T-0205`.

**Acceptance criteria.**
| # | Criterion |
|---|---|
| A1 | Two runs, a week apart, produce the same candidate SHA (DET-1) |
| A2 | The same run on macOS and Linux produces the same SHA on the same input set (E-2 resolved) |
| A3 | Preflight predicts every conflict that the real merge produces, in a 200-case corpus (E-3 resolved) |
| A4 | A triple-only conflict is predicted (`T-CONF-2`) |
| A5 | Safety fingerprint unchanged after success, failure, and SIGKILL, including refs |
| A6 | After a forced failure at any of the five creation stages, no orphan worktree or ref remains |
| A7 | `refs/ilvuno/*` is the only ref namespace written; asserted by diff |

**Required tests.** `DET-1`, `DET-2`, `DET-4`, `T-CONF-1..4`, `T-SAFE-1..3`, `T-INT-2` (merge
crash), `T-WT-1..8`, plus a ref-namespace assertion helper.

**Security considerations.** This is the first milestone that mutates the repository. All writes go
through `policy`; hooks and signing are disabled (`INV-11`); `abort` deletes only `refs/ilvuno/*`.
Threat: a crafted journal pointing at a user worktree must be refused (`T-SEC-4`).

**Review requirements.** Two reviews, one of them specifically adversarial: reviewer attempts to make
the tool write outside scope using config, journal, or argv. Sign-off required from a human who did
not write the code (for a solo maintainer: a recorded self-review against a written checklist).

**Definition of Done.** §3 + §6 (persistence DoD) + gate G2.

**Rollback / recovery.** `ilvuno abort` + `git worktree prune`. If determinism cannot be achieved
cross-machine (E-2 negative), the guarantee narrows to same-machine and the report must say so —
recorded as an ADR-0007 amendment.

---

## M3 — Conflict detection and evidence reporting

**Objective.** Explain what happened: classified findings, per-source attribution, and a verifiable
evidence bundle.

**User benefit.** Priya gets an answer she can paste into a review; Marco gets an artifact he can
check without trusting the tool.

**Deliverables.**
1. `model` package: entities per architecture `02` §3 (subset: Project, IntegrationRun, Candidate,
   VerificationRun stub, EvidenceRecord, Finding), constructors with invariant checks.
2. Findings + classification (`deterministic` / `interpretation`), severity rules, schema
   (`ilvuno.dev/findings/v1`).
3. Deterministic analyses: `file_overlap`, `hunk_proximity`, `dependency_disagreement`,
   `base_drift`, `claim_divergence` (import path), `interface_symbol_touched` (index-command).
4. Evidence store: append-only records, blob refs, write-time redaction, bundle seal + `verify`,
   manifest with digests.
5. Report renderer: Markdown + JSON, ten sections, stable/volatile split, banned-claims scan.
6. Benchmark harness (`docs/planning/04-benchmarks.md`) with baseline recording.

**Dependencies.** M2.

**Explicit scope exclusions.** No verification execution (findings may include `unverified` status but
no commands run); no passport; no approval; no repair; no interpretation model call.

**Development tasks.** `T-0301`…`T-0305`.

**Acceptance criteria.**
| # | Criterion |
|---|---|
| A1 | Every finding carries `classification` + `evidence_refs`; a report with a `blocking`
interpretation fails schema validation (`C-6`) |
| A2 | Report is byte-identical (normalized) across two runs on identical inputs |
| A3 | Bundle verifies offline with the binary deleted from the PATH; a one-byte log edit fails verify |
| A4 | `dependency_disagreement` names both sources' constraints exactly |
| A5 | Banned-claims scan fails the build when a prohibited phrase is added to a template |
| A6 | Redaction corpus test passes both directions (200 secrets, 200 benign) |

**Required tests.** `T-API-1..3`, `T-DEP-1..2`, `T-EVD-1..4`, `RED-1`, `DET-1` (report), contract
tests for schema + banned phrases, `T-SEC-8`.

**Security considerations.** Redaction at write (`INV-13`); bundle must not include env values; report
must not leak absolute paths (`INV-9`). Findings from an index command are `deterministic` but
non-blocking — the asymmetry must be tested, not just documented.

**Review requirements.** Human review of the *wording* of every report section against PRD Appendix A;
human review of the classification table.

**Definition of Done.** §3 + §7 (evidence DoD) + gate G3.

**Rollback / recovery.** Bundles are additive; a bad report version is fixed by regenerating from the
journal (journal is authoritative). If redaction proves too lossy, the entropy class is disabled by
default (E-5) — a config default, not a redesign.

---

## M4 — Combined build/test verification

**Objective.** Run the project's checks against the candidate and record what happened — including
whether the failure exists only in combination.

**User benefit.** The core value, finally: "each branch was green; together they are red; here is
the evidence."

**Deliverables.**
1. `verify` runner: spawn, process group, timeout, TERM→KILL, output caps, `tree_before/after`.
2. Env allowlist + denied-name precedence + `EnvFingerprint` (names + hash, never values).
3. Five-state outcome model + `not_verified` + advisory/skip semantics.
4. Per-command evidence records + logs + digests.
5. **`combination_only` isolation**: re-run only the failing command against each source, record the
   comparison, classify `true | false | unknown`.
6. `ilvuno verify` for an existing ref/candidate; `--runs N` flakiness detection.
7. Exit-code matrix implementation and tests.

**Dependencies.** M2 (candidate), M3 (evidence store, findings).

**Explicit scope exclusions.** No repair loop; no parallel command groups; no JUnit parsing; no
sandbox/container; no passport integration.

**Development tasks.** `T-0401`…`T-0404`.

**Acceptance criteria.**
| # | Criterion |
|---|---|
| A1 | Five outcomes reachable and mutually exclusive; each has a fixture and an exit code |
| A2 | Zero configured commands ⇒ `unverified`, exit 4, confidence-low block present |
| A3 | `T-API-1` yields `combination_only: true` with per-source logs |
| A4 | A command that commits ⇒ `candidate_mutated`, result not attributable, exit 1 |
| A5 | A command that hangs ⇒ group killed, `timed_out`, no surviving children |
| A6 | Ambient secret-shaped env vars absent from the child (asserted) |
| A7 | `tree_after ≠ tree_before` produces a warning + outputs list, not a failure |

**Required tests.** `T-CMD-1..4`, `T-API-1`, `T-COMP-1..3`, `T-SAFE-1`, `T-SEC-1..2`, env-allowlist
tests, timeout/orphan tests, `VR-` matrix.

**Security considerations.** This milestone executes untrusted project code. Every mitigation in
architecture `09` §4.4 and §7 must be present *and* the documentation must state the non-claim
(no sandbox) before release. Env allowlist is security-relevant, not convenience.

**Review requirements.** Human review focused on the process-supervision code and on the exact wording
of the `confidence: low` and `combination_only: unknown` states — both are places where overclaiming
is easy.

**Definition of Done.** §3 + §8 (verification DoD) + gate G4.

**Rollback / recovery.** If `combination_only` proves too expensive on real repos, gate it behind
`--isolate` and keep the measurement opt-in rather than dropping it (it is the metric that decides
the product).

---

## M5 — Reliability, cancellation, and recovery

**Objective.** Make interruption boring: no false verdicts, no orphaned state, no silent data loss.

**User benefit.** The tool can be trusted on a laptop that sleeps, a CI runner that times out, and an
agent that gets killed. This is what makes the difference between a demo and a daily tool.

**Deliverables.**
1. `resume` with the four uncertainty kinds and explicit `--decide` (no auto-resolution).
2. `abort` with patch archiving and orphan cleanup.
3. Advisory project lock with `(pid, start_time, boot_id)` staleness.
4. Idempotency: command ids, `integrate` cache key, duplicate-request returns recorded result.
5. `testhooks` build tag: die-after-stage injection; full crash/signal matrix in CI.
6. Journal corruption handling: torn tail, chain break, seq gap, quarantine.
7. Cleanup verification: no orphan worktrees/refs/processes after any crash.

**Dependencies.** M2 (journal, worktree), M4 (verification to interrupt).

**Explicit scope exclusions.** No daemon, no queue, no cross-host resume, no journal compaction.

**Development tasks.** `T-0501`…`T-0504`.

**Acceptance criteria.**
| # | Criterion |
|---|---|
| A1 | SIGKILL at each of the 12 stages × 3 signals leaves a parseable journal and a defined state |
| A2 | Every uncertain outcome requires an explicit decision; `resume` alone exits 2 with the list |
| A3 | `--decide=remerge` after a killed merge reproduces the same candidate SHA as an uninterrupted run |
| A4 | No orphan worktree, ref, or child process after any crash case (asserted) |
| A5 | A second `integrate` with identical inputs returns the recorded result without re-running checks |
| A6 | A tampered journal is refused (exit 2) and cannot yield an approval |
| A7 | `abort` after any crash leaves the repo byte-identical except for `.ilvuno/` |

**Required tests.** The full crash matrix (`11` §7), `JRN-1`, `JRN-2`, `T-INT-1..3`, `T-SEC-5`,
`T-SEC-10`, lock staleness tests, idempotency tests.

**Security considerations.** Tamper detection is a security control here (journal chain, refusal);
the lock must not be a denial-of-service vector for a second user on the same machine (stale
takeover is explicit and recorded). Kill semantics must never target PIDs outside ownership records.

**Review requirements.** Two reviews; one must be an adversarial crash hunt (reviewer proposes
interruption points the author did not consider). Sign-off only after `JRN-2` passes 10k random
truncations in CI.

**Definition of Done.** §3 + §6 + gate G5.

**Rollback / recovery.** If `resume` cannot be made unconditional in the timebox, ship v0.1 with
`abort` + re-run only (explicit, honest) and record the deferral as an ADR-0002/`PR-2` amendment.
Do **not** ship a `resume` that sometimes produces a different result.

---

## M6 — Functional Ilvuno CLI prototype

**Objective.** Make it usable by someone who is not the author: passport, approval, repair, brief,
stats, docs, exit codes, and a 10-minute first run.

**User benefit.** Everything a first external user touches: `init` → `integrate` → read report →
`approve` → `task show` tomorrow.

**Deliverables.**
1. CLI surface completion: `init`, `doctor`, `integrate`, `verify`, `show`, `approve`, `task`,
   `resume`, `abort`, `stats`, `brief`, `bundle`, `explain`, `gc`, `key init`.
2. Passport v1: write protocol, digest chain, derived state/next-actions, conflict refusal.
3. Approval: TTY gate, typed SHA, digest binding, Ed25519 signature, invalidation.
4. Repair: external command, deterministic brief, budget, per-attempt evidence.
5. `brief` (concurrent-change export), `stats` (local metrics), `explain` (why a finding fired).
6. `--json` contract + docs (quickstart, config reference, "what this does not prove").
7. v0.1.0 release: goreleaser config, checksums, install docs.

**Dependencies.** M3 (report), M4 (verification), M5 (recovery).

**Explicit scope exclusions.** No agent launching; no UI; no Windows; no push/PR; no hosted anything;
no telemetry.

**Development tasks.** `T-0601`…`T-0605`.

**Acceptance criteria.**
| # | Criterion |
|---|---|
| A1 | The 12 PRD acceptance criteria (`02-prd.md` §26) all pass on a clean machine |
| A2 | First integration ≤ 10 min from install on a fresh fixture (`NFR-101`) |
| A3 | `approve` refuses without a TTY and without the typed SHA |
| A4 | Changing a source after approval yields `stale_approval` and a closed gate |
| A5 | Passport renders identically from a clean clone on a second machine |
| A6 | Repair respects budget, never modifies sources, keeps every attempt |
| A7 | All P0 requirements traced to a passing test or a recorded manual check |

**Required tests.** Full PRD §26 traceability matrix; `T-COMP-4` (agent completion ≠ integration);
approval matrix; passport portability; repair matrix; `RED-1`; contract tests; a scripted
"ten-minute" e2e on a fresh container.

**Security considerations.** `approve` path is the highest-risk surface in the MVP (authority). Key
handling: 0600, no passphrase prompt. `stats` must not include repo identifiers. Release artefacts
must be reproducible and checksummed.

**Review requirements.** External-user dogfood: someone who has not seen the code completes
`init → integrate → approve` unassisted, and their confusion is logged as bugs. Gate sign-off
requires that session, not a demo by the author.

**Definition of Done.** §3 + §9 (release DoD) + gate G6.

**Rollback / recovery.** Release rollback: yank the version, publish a patch with the previous
format compatibility (ADR-0014). Data rollback: none needed — user data is git + passports, which the
tool never rewrites destructively.

---

## M7 — Agent runtime and provider adapters *(provisional)*

**Objective.** Let Ilvuno start agents, not only consume their output.

**Deliverables (sketch).** Adapter interface + capability probing; `claude` adapter (Agent SDK or
stream-JSON, user's binary); `codex` adapter (`app-server` JSON-RPC); conformance fixtures per
adapter; session journal; normalized event stream; cancellation ladder; resume semantics.

**Dependencies.** M6 gate (G6) **and** a user-request signal. Not approved.

**Exclusions.** No credential custody (`AUTH-1..3`); no PTY scraping; no hosted execution; no
multi-tenant.

**Acceptance (provisional).** Adapter declares only implemented capabilities (construction-time
check); a killed turn is detected within the cancellation ladder; a provider upgrade drift shows up
as a failed conformance test, not a user bug; `AgentRun.state` still cannot produce an approval
(`INV-7`).

**Security.** Same non-claims as `09` §7; adapters add no capability; approvals remain host-side.

**Rollback.** Adapters are additive and optional; removing one removes a binary path, not a data
format. Sessions are journaled per adapter so a removed adapter leaves readable records.

---

## M8 — Local web and desktop workspace *(provisional)*

**Objective.** A shell for people who will not read a terminal.

**Deliverables (sketch).** Tauri + React shell (ADR-0003); loopback WS daemon; `seq`-based event
replay; report and diff rendering; approval UI; device-local state only.

**Dependencies.** G6, ADR-0003 revisit trigger, and paid/design-partner demand.

**Exclusions.** No cloud sync; no multi-user; no agent launching in v1 of the shell; no telemetry.

**Acceptance (provisional).** A UI action produces exactly the same artifacts as the CLI action
(byte-comparable, normalized); no state exists only in the UI; killing the shell never stops a run.

**Security.** New surface: local listener (loopback only, token-authenticated, constant-time
compare); no `0.0.0.0` default; webview hardened; no `nodeIntegration` equivalent.

**Rollback.** The shell is additive; the CLI remains the product of record. Removing the shell
removes a UI, not capabilities.

---

## M9 — Multi-agent task orchestration *(provisional)*

**Objective.** Dispatch streams, then integrate them — the full loop.

**Deliverables (sketch).** Stream manifest → worktree per stream → adapter launch → `AgentRun`
observation via git only → settle → hand to `IntegrationRun`; admission control and concurrency caps;
delegated-completion treated as a claim.

**Dependencies.** M7, G6, and the measurable M-1 signal proving integration value first.

**Exclusions.** Shared-folder parallelism (`ORQ-101`); DAG scheduling beyond `TaskDependency`;
autonomous merge.

**Acceptance (provisional).** Concurrency is capped and admission-controlled; a child cannot exceed
its parent's permissions; every dispatched stream has its own worktree; integration still requires
explicit approval.

**Security.** Prompt injection becomes a live threat (agent output influencing other agents) — the
`09` §4.2 controls become mandatory, not advisory. No agent-facing MCP server without scoped,
expiring tokens.

**Rollback.** Dispatch is additive; the product remains usable with externally run agents.

---

## M10 — Secure cross-device continuity *(provisional)*

**Objective.** Read and approve from another device, with the host still authoritative.

**Deliverables (sketch).** Pairing (single-use code, fingerprint confirm, DPoP), device registry and
revocation, E2EE relay or bring-your-own transport, `seq` replay, signed remote approvals, mobile
read/approve client.

**Dependencies.** M8 (or a headless daemon), G4-commercial gate, `SEC-105`/`SEC-107` reviews.

**Exclusions.** Live process migration; conversation/transcript sync; `operate` scope on mobile;
legacy protocol versions.

**Acceptance (provisional).** Relay sees ciphertext only (log assertion); revoked device cannot
approve; remote approval re-verifies evidence on the host; a client disconnect never stops a run.

**Security.** New network surface requires its own threat model review before any code: auth,
pairing, revocation, replay, metadata minimisation.

**Rollback.** Transport is optional; the host works offline. Removing relay support leaves file-based
passport sync working.

---

## M11 — GitHub PR and CI integration *(provisional)*

**Objective.** Put the evidence where review happens.

**Deliverables (sketch).** `ilvuno integrate --json` in CI; PR comment/check with the report and
bundle link; base-ref drift check; optional PR body draft (never push).

**Dependencies.** G6; a CI runner with the project's toolchain.

**Exclusions.** Creating or merging PRs; pushing; reading private CI logs; acting on webhooks.

**Acceptance (provisional).** The CI job is the same binary and produces the same verdict as local;
the evidence artifact is attached and verifiable; no secret appears in the artefact.

**Security.** CI environment is hostile to secrets: allowlist must be respected, `--json` must not
echo env. Signed webhooks only if any inbound integration appears.

**Rollback.** CI usage is read-only from Ilvuno's side; removing the workflow removes the feature.

---

## M12 — Optional commercial services *(provisional)*

**Objective.** Fund the project without gating the core.

**Deliverables (sketch).** Hosted evidence retention, team approval queue, encrypted metadata sync,
notifications; billing.

**Dependencies.** Traction gates (`07-success-metrics.md` §7 G4) and **at least one paying team in
writing**.

**Exclusions.** Any gate on local verification, evidence format, or passport; per-seat pricing for
readers; usage metering of verification.

**Acceptance (provisional).** With all services disabled, every P0 scenario still passes
(`OSS-102`); no paid feature is required to read a report or approve locally.

**Security.** SOC2-shaped questions will arrive; publish a data-flow diagram and a written policy
before the first invoice.

**Rollback.** Services are optional by construction; a customer can leave with their files intact.

---

## 11. Multi-agent execution plan

How Claude Code and Codex contribute without producing work that costs more to integrate than to write.
The governing assumption: **two agents is the ceiling for one human reviewer**, and the second agent
earns ~1.7×, not 2× (`04-benchmarks.md` §7 measures this rather than assuming it).

### 11.1 Worktree ownership

| Rule | Detail |
|---|---|
| One agent, one worktree, one branch | `feat/<task-id>` in a dedicated checkout; never two agents in one tree (the `ORQ-101` rule, applied to us) |
| One owner per package at a time | map in `03-dependency-graph.md` §5; transfers are announced at the start of a wave, not mid-task |
| Shared files are frozen during a wave | `internal/model/**`, `internal/policy/**`, `schemas/**`, `Makefile`, `go.mod` change only by the owner, only between waves |
| No agent rebases another agent's branch | history rewriting destroys review evidence; the owner rebases its own branch |
| Worktrees live outside the repo | `.worktrees/<agent>-<task-id>/` (note: these are *our* working trees, not Ilvuno's candidate worktrees) |

### 11.2 Bounded task assignments

| Constraint | Value | Why |
|---|---|---|
| Tasks in flight per agent | 1 | context switching is where quality dies |
| Task size | ≤ 3 engineering days | larger tasks hide scope creep and block integration |
| Scope | the task's "Allowed file scope" list, verbatim | the diff is reviewable at a glance; out-of-scope edits bounce |
| Forbidden without asking | schema changes, new dependency, changing a gate threshold, touching `policy` | these ripple across the whole plan |
| Stop condition | AC met and DoD satisfied | not "when it feels done" |

### 11.3 Expected artefacts per assignment

Every agent-produced change carries, in one PR:

1. The diff (scoped as above).
2. Tests that fail without the change.
3. The task spec updated where reality diverged.
4. Evidence per the task's "Evidence requirements".
5. Four-line adversarial self-review (`07-definition-of-done.md` §10) — required for agent-authored
   code even when a human reviews it afterwards.
6. A one-paragraph note: what was ambiguous in the spec, and how it was interpreted.

Item 6 is the cheapest early signal that a task was underspecified; recurring ambiguity in the same
area means the spec, not the agent, needs work.

### 11.4 Parallel eligibility

Two tasks may run concurrently when **all** hold:

- different packages, or one of them is docs/testkit-only;
- no shared file from the frozen list;
- neither depends on the other's output (per `03-dependency-graph.md` §2);
- the human reviewer has bandwidth for both diffs the same day.

| Pair | Eligible | Note |
|---|---|---|
| `T-0101` gitx + `T-0103` config | yes | different packages |
| `T-0201` journal + `T-0202` worktree | yes | different packages; journal API agreed up front |
| `T-0203` determinism + `T-0204` preflight | yes | one is commit env, one is analysis |
| `T-0204` preflight + `T-0205` candidate | **no** | candidate consumes preflight; owner A does both, sequentially |
| `T-0301` model + anything touching model | **no** | model changes ripple everywhere |
| `T-0401` runner + `T-0402` outcome | **no** | same package (`verify`); sequence them |
| `T-0501` resume + `T-0503` crash harness | **no** | harness asserts resume; same author expected |
| `T-0601` passport + `T-0602` approval | **no** | approval writes passport; sequence |

The pattern: anything touching a shared package or the same file is serialised on purpose. The
parallel slots are real but narrower than a naive "split by milestone" reading suggests.

### 11.5 Review responsibilities

| Who | Reviews | Cannot |
|---|---|---|
| Human maintainer | every diff, every gate | delegate a gate sign-off to an agent |
| Agent (peer review) | clarity, dead code, test quality, doc drift | approve a security-relevant path (`policy`, `redact`, `approval`, `journal`) or a gate |
| Authoring agent | its own diff (self-review §10) | approve its own PR |

Rule: **agent review can reduce human review time, never replace the human's decision**, and never on
the four packages that hold the trust boundary.

### 11.6 Integration checkpoints

| Frequency | Checkpoint | Fail action |
|---|---|---|
| Per task | PR into `main`, squash merge, task ID in subject | revert the merge; the task reopens |
| Per wave (weekly) | Run the benchmark subset for the milestones completed so far | do not start the next wave |
| Per milestone | Gate (`05-release-gates.md`) | milestone stays open |
| Every 2 weeks | **Dogfood**: run Ilvuno on Ilvuno's own in-flight branches and compare its verdict with the human's judgement of mergeability | a disagreement is a bug in one of the two; investigate before continuing |

The dogfood checkpoint is the most valuable and the cheapest: from M2 onward, the project can integrate
its own parallel work. If it cannot, the product does not work in its own development loop — the
fastest possible negative signal.

### 11.7 Git conflict policy

| Situation | Policy |
|---|---|
| Conflict in a frozen/shared file | stop; the file owner resolves; the other agent's branch is rebased by its own owner |
| Conflict in an owned package file | the package owner resolves, even if the other agent's change triggered it |
| Conflict in `testkit` | additive-only convention: both may append; a conflict means a naming collision, so rename and re-run |
| Conflict in docs | last-writer rule, but the losing author reviews the merged text |
| A merge that passes git but fails tests | treated as a real failure: fix forward with a focused commit, never "merge and see" |
| Repeated conflicts on the same file (>2 in a wave) | frozen-file escalation: the file gets an owner for the rest of the wave |

Note the symmetry with the product: we apply the same discipline to our own development that Ilvuno
applies to a candidate — pin, isolate, verify, and never claim success from a clean merge alone.

### 11.8 Escalation procedures

| Trigger | Escalate to | Within | Expected decision |
|---|---|---|---|
| AC ambiguous or contradictory | maintainer | same day | amend the task spec (recorded), not the AC silently |
| Blocked > 4 h on a technical issue | maintainer | 4 h | narrow scope, split the task, or accept a documented workaround |
| A safety invariant is hard to satisfy (`INV-1`, `INV-4`) | maintainer | immediately | stop; a gate risk is never traded for schedule |
| A task needs a new dependency | maintainer | before writing code | ADR amendment or a no |
| Two agents disagree on an interface | maintainer | same day | written decision in the task spec; the loser does not "try anyway" |
| An agent's change makes a benchmark fail | the authoring agent | immediately | revert first, investigate second |
| An agent believes a gate criterion is wrong | maintainer | same day | criteria change is a separate reviewed decision (`05` §12) |

Escalation is cheap by design: the alternative is an agent inventing a workaround that a human
discovers a week later in a diff nobody has time to read.

---

## 12. Milestone → task index

| Milestone | Tasks | Count |
|---|---|---|
| M0 | `T-0001`…`T-0005` | 5 |
| M1 | `T-0101`…`T-0105` | 5 |
| M2 | `T-0201`…`T-0205` | 5 |
| M3 | `T-0301`…`T-0305` | 5 |
| M4 | `T-0401`…`T-0404` | 4 |
| M5 | `T-0501`…`T-0504` | 4 |
| M6 | `T-0601`…`T-0605` | 5 |
| M7 (prov.) | `T-7001`…`T-7002` | 2 |

Total MVP tasks: **33**, each with its own spec in `docs/tasks/<task-id>.md`. Two provisional
adapter tasks are specified at sketch depth only, because M7 is not approved.
