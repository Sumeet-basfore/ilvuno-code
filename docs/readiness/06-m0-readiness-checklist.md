# 06 — M0 Readiness Checklist

Status: **M0 AUTHORIZED — 2026-11-20** by the project owner (§7). Scope: §3.1 minimal M0 and tasks
T-0001 to T-0005 only. Gate **G0 is not yet passed**. M1 and later milestones are not authorized.
This file is the single authority for milestone authorization. D-01 is DECIDED 2026-11-20
(`docs/decisions/open-questions.md` §0): the owner signs this checklist before each milestone.

**Entry-gate evaluation, 2026-11-20:** 10 PASS · 0 OPEN · 0 FAIL. Evidence and reasons: the E-rows below and `docs/readiness/07-m0-reconciliation.md` §4.

Gate: **G0**. Milestone: **M0 — repository harness and pipeline proof**. Estimate: **5 engineering
days** (was 4.5; +0.5 for the Git floor matrix).

---

## 1. Entry criteria (all must be true before the first commit of M0 work)

| # | Criterion | Evidence required | Status |
|---|---|---|---|
| E-1 | D-01 recorded: one authority for milestone authorization | entry in `docs/decisions/open-questions.md` | **PASS.** D-01 recorded as DECIDED in `open-questions.md` §0 (2026-11-20) |
| E-2 | D-02 product shape decided | decision entry | **PASS.** D-02 (standalone CLI) DECIDED in `open-questions.md` §0 |
| E-3 | D-03 license decided | `LICENSE` content approved | **PASS.** Decision PASS (D-03). The owner approved the exact text by D-18 on 2026-11-20: `docs/readiness/LICENSE-draft-Apache-2.0.txt`, sha256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`, 11,358 bytes, identical to apache.org. The root `LICENSE` is created by T-0001 from that text (copy verbatim); its absence is expected before M0 |
| E-4 | D-04 language decided | decision entry | **PASS.** D-04 (Go) DECIDED in `open-questions.md` §0. ADR-0001 status accepted |
| E-5 | D-05 platforms + Git floor decided | decision entry naming the floor version | **PASS.** D-05 DECIDED in §0 (Git ≥ 2.38.1; Linux-first; macOS tested in CI). Windows is excluded from V1 by planning item A-4 (DECIDED 2026-11-20) |
| E-6 | D-06 config location decided | decision entry | **PASS.** D-06 (committed `ilvuno.toml`) DECIDED in §0 |
| E-7 | M0 status statements reconciled in `AGENTS.md`, `CLAUDE.md`, roadmap, milestones | the four files link to this checklist instead of asserting status | **PASS.** Verified 2026-11-20: all four link to `06` §7 and `07`, and none claims authorization. Reading used: "instead of asserting status" means no contradicting status claim (see `07` §4) |
| E-8 | `T-0005` rewritten as reference conformance (replaces the E-1 spike) | updated task spec, reviewed | **PASS.** Independent review of the revised `T-0005` (fresh-oracle wrapper, failure handling, path preflight, negative tests AC-9…AC-13) reported by the project owner in the M0 authorization on 2026-11-20. The verdict text is not committed in this repository, as for E-9. The spec is `docs/tasks/T-0005.md` |
| E-9 | `T-0001` scaffolding reduced to M0 needs (§3) | updated task spec, reviewed | **PASS.** Independent Codex review of the corrected `T-0001` (2026-11-20), reported by the project owner. Accepted scope: D-17 module path (`github.com/Sumeet-basfore/ilvuno-code`), lint aligned with D-16 and AGENTS §4.1, platform-specific Linux/macOS checks, T-0001/T-0005 CI ownership split, stub criteria removed. The review verdict text is not committed in this repository |
| E-10 | Git floor verified by X-09, or the floor raised with a recorded reason | `docs/readiness/02` §9 X-09 result | **PASS, with documented gaps.** X-09 result: `docs/research/09-git-floor.md`. Git 2.38.1 matches Git 2.55.0 on X-01, X-02b, X-03 (SP-1R3 matrix), X-04, X-04b, X-05, X-06. `check-attr --source` is absent on 2.38.1 (X-10), so M2 needs a capability probe. Gaps G-1…G-6 are listed in `09` §4 |

**No code may be written until E-1…E-10 are satisfied.** E-7 cannot be satisfied by engineering
alone; it is the visible consequence of D-01.

---

## 2. Historical M0 entry blockers (resolved before authorization)

| Review finding | Blocks | Closed by |
|---|---|---|
| BLK-02 M0 authority / decisions | E-1…E-7 | D-01…D-06, D-16…D-18 |
| BLK-01 invalid E-1 command | E-8 | redesign in `02` §3; task rewrite |
| "M0 scaffolds too much" | E-9 | §3 below |
| Git 2.38 floor unverified | E-10 | X-09 |

---

## 3. Proposed minimal M0 scope

### 3.1 What M0 must produce

| Deliverable | Why it is in M0 | Acceptance |
|---|---|---|
| Go module + `Makefile` + CI (linux, macOS) | every later task runs these commands | `make test` green on a clean clone |
| `cmd/ilvuno --version` | proves the binary builds and is fast | ≤ 30 ms, exit 0 |
| `internal/testkit/forge` | deterministic repositories are the foundation of every test | two builds → identical refs/tree SHAs |
| `internal/testkit/fixtures` (ported SP-1R3 corpus F1–F8, F10; F9 kept as a fault-injection test; S-1 approved 2026-11-20) | the failure corpus | each fixture's verdict matches its `expected.json` row |
| `internal/testkit/safety` | proves "we did not touch your repo" | detects 5 mutations, ignores `refs/ilvuno/*` |
| `internal/lint/importrule` (rule table only) | boundaries enforced from the first package | fails on a deliberate violation |
| `Makefile` targets `gate`, `bench`, `docs-check` (stubs that fail loudly) | the gate contract exists before it is needed | running `make gate G0` says "not implemented" with exit 2, not 0 |
| `docs/planning/gate-log.md` | gate history has a home | file exists with the G0 entry template |
| Reference conformance: SP-1R3 harness under `testdata/reference/` (replaces the E-1′ spike; see `docs/tasks/T-0005.md`) | proves the sequential reference before M2 depends on it | §4 (G0-6 wording is a gate criterion; see `07` §3.2) |
| X-09 result note (`docs/research/09-git-floor.md`) | records what the floor actually does | table of 6 behaviours on 2 versions |

### 3.2 What M0 must **not** create

Deleted from `T-0001` as written (the review's "scaffolding" finding):

```
internal/{cli,model,gitx,repo,engine,analyze,verify,evidence,passport,policy,proc,redact,journal,config}/
schemas/
```

Rationale: 14 empty packages with doc comments are 14 decisions made implicitly, 14 places a reader
must check, and 0 tests. The dependency rule is still enforced — `importrule` carries the *rules*
for those paths without the paths existing. Each package is created by the task that first needs it
(`T-0101` gitx, `T-0103` config, `T-0201` journal, …), together with its first test.

Trade-off, stated: a newcomer sees fewer folders. That is correct for a 5-day milestone; the
architecture document still shows the target shape.

### 3.3 Fixture list change

`T-CONF-2` ("pair-clean, tuple-conflict") is **removed** — no instance was reproducible in 64
attempts (`02` §2 X-08), and no supported primitive exists to detect it. Replaced by two fixtures
that test the actual algorithm:

| New fixture | Shape | Asserts |
|---|---|---|
| `T-SEQ-1` | clean at step 1, conflict at step 2 | the step that failed is named; steps before it are recorded; no ref, no worktree |
| `T-SEQ-2` | source already contained in the accumulator | `already_integrated`; no empty merge commit |
| `T-SEQ-3` | source with unrelated history | `unrelated_histories`, exit 2, no objects committed |

Net fixture count stays at nine.

---

## 4. G0 exit criteria

> **Provenance:** every `LOCAL PASS` row below records a run on the **uncommitted M0 working tree** on
> 2026-11-20. No M0 code is committed at the governance HEAD (`docs/commit-rules`), so these results are
> not reproducible from a clone of it; they become reproducible when PR-2 lands and CI runs. G0 is not
> passed.

| # | Criterion | Evidence | Status |
|---|---|---|---|
| G0-1 | `make test` green on a clean clone, `GIT_CONFIG_GLOBAL=/dev/null`, no network | CI run link | PENDING CI. Local `make test` exit 0. A clean-clone run and the CI run are not done (no push). |
| G0-2 | Forge determinism: two builds → identical refs + tree SHAs | `make forge-verify` output | LOCAL PASS. `make forge-verify`: 68 PASS, 0 FAIL. |
| G0-3 | F1–F8 and F10 exist and assert their `expected.json` verdicts; an F9 fault-injection test exists (S-1, approved 2026-11-20) | fixture list + test output | LOCAL PASS. F1–F8 and F10 match `expected.json` verdicts, candidate SHAs and bases (oracle). F9 cleanup injection: exit 5, orphan reported. |
| G0-4 | Safety fingerprint detects ref move, dirty file, staged change, stash, config edit | test output | LOCAL PASS. All 8 mutations detected and named; `refs/ilvuno/` and `.ilvuno/` ignored; 20 no-op runs equal; best of 5 fingerprints of 500 files 39.7 ms (≤ 50 ms). |
| G0-5 | Anchored ref check: `refs/heads/refs/ilvuno/x` is **rejected** as ours; `refs/ilvuno/x` accepted | test output (regression for X-04) | LOCAL PASS. `TestAnchoredNamespaceCheckRejectsLookalikeRef`: `refs/ilvuno/x` ignored; `refs/heads/refs/ilvuno/x` counted. |
| G0-6 | **Sequential-integration reference conformance (S-3, approved 2026-11-20):** the four E-1′ cases (3 clean; conflict at step 2; ancestor; unrelated) pass on both approaches against the reference oracle | reference conformance log with SHAs and exit codes (T-0005 AC-8) | LOCAL PASS. Reference matrix 28/28 on Git 2.55.0 and 2.38.1. E-1′ cases F2, F4, F5, F6 pass on both approaches (T-0005 AC-8). |
| G0-7 | **X-09:** the same 6 behaviours recorded on Git 2.38 and current | `docs/research/09-git-floor.md` | PARTIAL. `docs/research/09-git-floor.md` holds the X-09 result and the T-0005 run (§8). Owner review pending. |
| G0-8 | CI green on linux + macOS, git version printed, fails below the declared floor | CI link | PENDING CI. Workflow written and YAML-parsed; not run. The macOS reference leg is blocked (`sha256sum`, git version gate; `09` §8). |
| G0-9 | `importrule` fails on a deliberate `engine → agent` import | negative test output | LOCAL PASS. Importrule test detects `engine → agent` and third-party imports in testdata. A planted violation made `make lint` fail, then was reverted. |
| G0-10 | `make gate G0` runs the above and writes `gate-G0.json` | artefact | NOT IMPLEMENTED. `make gate G0` exits 2. No task in T-0001…T-0005 owns the gate runner. |
| G0-11 | Gate packet reviewed by someone other than the author | gate-log entry | PENDING. Requires a reviewer other than the author. |

**E-1′ detail** (replaces the invalid N-way experiment):

```
1. Build a 3-source fixture (all pairs and the sequence clean).
2. Run the object pipeline → candidate_sha_pipeline.
3. In a scratch worktree, run `git merge --no-ff` per source with the identical
   message and committer envelope → reference_sha.
4. Assert: tree equality (required) and commit-SHA equality (required only when
   the reference uses the same envelope).
5. Repeat for a fixture whose step 2 conflicts: assert both report the same paths
   and the same step index.
6. Repeat for an ancestor source: assert skip.
7. Repeat for unrelated histories: assert typed refusal.
```

---

## 5. What M0 explicitly does not include

- No candidate worktree creation (that is M2; M0's spike may create one inside `/tmp` fixtures only).
- No `ilvuno` subcommands beyond `--version`.
- No journal, config loader, verification, evidence, passport, approval, repair.
- No fix for the write-scope classes (M2) or driver policy (M2) — those decisions are recorded, not
  implemented, at M0.
- No changes to any approved document beyond the status reconciliation in E-7.

---

## 6. Recommended narrow spike before full M0 (optional, 2 days)

The review's suggestion — "validate the smallest useful vertical slice" — can be satisfied cheaply
*before* M0's process work:

| Spike | Content | Exit criterion |
|---|---|---|
| **SP-1** (1 d) | Throwaway script in `/tmp`: pin 3 SHAs → object pipeline → `update-ref` → detached worktree → run one real command → bundle of 3 digests | Runs end-to-end on a real repository copy; candidate SHA reproducible on second run |
| **SP-2** (1 d) | Same, on Git 2.38 in a container | Same result; records any parsing difference |

Value: SP-1 would have caught BLK-01 and HI-01 in one day. It is not a substitute for M0 (no tests,
no fixtures, no safety), but it converts the remaining design risk into facts before 5 days of
harness work.

Status 2026-11-20: SP-1 and SP-2 were carried out as the SP-1R3 harness (`docs/experiments/`), and T-0005 imports it. Original recommendation: run SP-1/SP-2 **instead of** `T-0005`, and fold their logs into G0-6/G0-7. This keeps
M0 at 5 days.

---

## 7. Sign-off

**Signed by the owner's authorization, 2026-11-20.** The project owner provided the statement below in the session. It is recorded verbatim. It authorizes M0 only.

```
M0 authorization — Ilvuno (2026-11-20)

I authorize implementation of M0: Repository Setup and Deterministic Fixtures under the
approved minimal M0 scope in docs/readiness/06-m0-readiness-checklist.md §3.1 and the
independently reviewed task specifications T-0001 through T-0005.

This authorization includes:
- Go CLI repository foundation
- Apache-2.0 LICENSE
- Approved development and test tooling
- Controlled SP-1R3 reference harness import
- Deterministic fixture infrastructure
- Safety fingerprint checks
- Linux/macOS CI configuration and verification

All work must follow the recorded owner decisions and SP-1R3 operational conditions.

Not authorized: M1 or later milestones, production integration-engine implementation,
deployment, or pushing/merging changes without separate approval.

The milestone must satisfy its G0 acceptance criteria before being declared complete.

Authorized by: Project owner
Date: 2026-11-20
```

**Implementation authority is limited to the scope above.** A signed block that omits a condition
("approved except E-5") is a scope amendment: record it here, not in a chat.

---

## 8. Post-M0 state (so this does not get re-litigated at G1)

After G0 passes, the project may start **M1 only**. M2 still requires D-07, D-09, D-10 (ref model,
write classes, determinism scope) because those are design decisions the M2 code encodes. No
milestone beyond the next one is ever authorized by a single signature.
