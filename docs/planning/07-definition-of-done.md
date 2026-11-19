# 07 — Definition of Done

Status: planning artifact. Applies to every task, milestone, and release. Reviewers use this as a
checklist; authors use it before requesting review. "Done" without evidence is not done — this file
is the enforcement of that sentence.

---

## 1. Universal DoD (every task)

A task is done when **all** of these are true.

| # | Requirement | Evidence in the PR / task |
|---|---|---|
| 1 | Acceptance criteria from the task spec are met, item by item | Checklist with each AC ticked and a pointer to the test or artefact |
| 2 | New behaviour has a test that fails without the change | The test name; reviewer can run it against the parent commit |
| 3 | Relevant benchmark scenarios still pass | `make bench SCENARIO=…` output or CI link |
| 4 | `make test` green locally and in CI | CI run link |
| 5 | `make lint` green (format, vet, import rules, banned phrases) | CI link |
| 6 | No new dependency unless the ADR/plan permits it | `go.mod` diff; if changed, an ADR reference |
| 7 | Safety fingerprint test included for any task that writes | Test name (`T-SAFE-*`) |
| 8 | Docs updated if the user-visible surface changed (CLI, config, report, exit code) | Doc diff link |
| 9 | Errors are typed with a stable code and a next action; no stack trace as the primary message | Error catalogue diff |
| 10 | Changelog entry if user-visible | `CHANGELOG.md` entry |
| 11 | Reviewed by someone other than the author (or, solo, a recorded adversarial self-review per §10) | Review link / note |
| 12 | Task spec updated if reality diverged from the plan (scope change recorded, not hidden) | Diff to `docs/tasks/<id>.md` |

Rule of thumb for #2: if the change is a bug fix, the test must reproduce the bug. If it is a
feature, the test must assert the user-visible behaviour, not the internal function.

---

## 2. DoD additions by work type

### 2.1 Git / worktree code

- [ ] Only `refs/ilvuno/*` and `.ilvuno/**` written (asserted, not asserted-by-inspection)
- [ ] Hooks/signing disabled where commits are created (`gitenv` metadata present)
- [ ] Failure path rolls back or journals `orphaned`; no half-state
- [ ] Runs on git 2.38 (floor) and latest
- [ ] Test: user-tree fingerprint unchanged across success, failure, and kill

### 2.2 State machine / journal code

- [ ] Every transition has a test for allowed and rejected cases
- [ ] Intent is journaled before the side effect (test asserts ordering)
- [ ] Crash injection at the new step is covered in the matrix
- [ ] `JRN-1`/`JRN-2` still pass (truncation corpus)
- [ ] No silent state repair: unknown states are typed and surfaced

### 2.3 Verification / process code

- [ ] Process group killed on timeout; orphan scan after the test
- [ ] Env allowlist respected; denied-name precedence tested
- [ ] Output capped and redacted before write
- [ ] Five-state outcome correct for the new path (no sixth state invented)
- [ ] Windows/macOS divergence documented if platform-specific

### 2.4 Evidence / report code

- [ ] Schema versioned (`ilvuno.dev/<kind>/vN`), additive-only change
- [ ] Bundle verification still passes (offline, with the binary removed)
- [ ] Volatile fields confined to the `run` object (determinism)
- [ ] Banned-claims scan green; new wording reviewed against PRD Appendix A
- [ ] No absolute paths, no env values, no identity in output (`INV-9`, `INV-13`)

### 2.5 Fixtures

- [ ] Built by the forge, not checked in as a repo
- [ ] Asserted shape (counts, conflict presence, per-branch green/red)
- [ ] Deterministic: fixed identity/date/TZ; `GIT_CONFIG_GLOBAL=/dev/null`
- [ ] Named after the failure class it represents
- [ ] Fails loudly if git output format changes (structural parse, not substring)

### 2.6 Documentation-only tasks

- [ ] Every command shown was actually run (copy-paste tested)
- [ ] No claim exceeds the banned-claims list
- [ ] Cross-links resolve (checked by `make docs-check`)
- [ ] "What this does not prove" updated if the claim changed

### 2.7 CLI surface

- [ ] `--help` text matches docs (checked by a test)
- [ ] Exit code documented and tested in the matrix
- [ ] `--json` output validates against the schema
- [ ] Works with `--no-color`, no TTY, `TERM=dumb`
- [ ] `--quiet` produces no output on success

---

## 3. Quality bars (measurable)

| Bar | Threshold | Where checked |
|---|---|---|
| Statement coverage of `internal/{engine,gitx,verify,evidence}` | ≥ 80% | CI (`go test -cover`) |
| Coverage of `internal/model` (constructors + transitions) | ≥ 90% | CI |
| Total suite runtime | ≤ 6 min | CI timer |
| Determinism job | 100% identical SHAs/reports | CI |
| Banned-phrase scan | 0 hits | CI |
| Exit-code matrix completeness | every documented code has a scenario | CI |
| Error-catalogue completeness | every code has a `next` action | CI |
| Lint | 0 findings (gofmt, go vet, staticcheck, import rules) | CI |
| Binary size | ≤ 25 MiB | release job |
| Cold start (`--version`) | ≤ 30 ms | bench |

Coverage is a floor, not a goal: a 95%-covered function with no assertion on its error path is not
better than an 80%-covered one. Reviewers judge assertions; the number only catches neglect.

---

## 4. Error-catalogue DoD

Every new error must ship with:

```go
{Code: "unpinnable_source", Class: 2, Message: "…", Next: "commit your changes, or pass a SHA"}
```

- [ ] Stable code, never renamed after release (add `deprecated:` instead)
- [ ] Class maps to an exit code in the documented table
- [ ] `Next` is a runnable command or a decision, not "check the docs"
- [ ] Catalogue table in docs regenerated (`make docs-check` fails on drift)
- [ ] A fixture that triggers it

---

## 5. Fixture DoD

See §2.5. Additional:

- [ ] Fixture is used by at least one scenario/benchmark or is removed (no orphan fixtures)
- [ ] Fixture name appears in the traceability matrix if it backs a PRD requirement

---

## 6. Persistence / recovery DoD

- [ ] Journal record has `seq`, `at`, `kind`, `phase`, `prev`, `digest`
- [ ] Crash case added to the matrix for the new step
- [ ] `resume` behaviour for the new uncertainty is documented and tested
- [ ] No automatic repetition of uncertain effects (asserted by a test counting side effects)
- [ ] Corruption path: what happens on truncation, chain break, missing file — tested

---

## 7. Evidence DoD

- [ ] Every claim in the output has an `evidence_ref` or is explicitly `unverified`
- [ ] `passed` implies a bound commit SHA and a command result (`C-7`)
- [ ] `not_verified` section present, even if empty
- [ ] Bundle manifest lists every file with digest
- [ ] `bundle verify` passes on the produced bundle
- [ ] Redaction applied at write; no raw fallback in bundled output

---

## 8. Verification DoD

- [ ] Outcome is one of the five, never a sixth
- [ ] Exit code matches the outcome table
- [ ] Advisory failures do not change the outcome but are visible
- [ ] Skips have a reason recorded
- [ ] `tree_before/after` recorded; mutation warning present when applicable
- [ ] Env fingerprint stored (names + hash only)

---

## 9. Release DoD (v0.1.0 and later)

All of `05-release-gates.md` §11, plus:

- [ ] Gate G6 packet complete and signed
- [ ] Traceability matrix: every P0 requirement → test or recorded manual check
- [ ] Rollback instructions tested once (not just written)
- [ ] Known-issues doc names Windows, submodules, LFS, sparse, network FS
- [ ] "What this does not prove" linked from README, `init`, and report footer
- [ ] Version and schema versions recorded in the release notes
- [ ] No network calls in the P0 suite (egress test)

---

## 10. Adversarial self-review (for solo work)

When no second reviewer exists, the author performs and records this checklist in the PR. It is
required, not optional, and a reviewer (future) should be able to see it was done honestly.

1. **Read your own diff backwards** (from the last hunk up). List anything that surprised you.
2. **Try to break it**: which input would make this write outside scope, double-charge, or hang?
   Write the answer down; add the test.
3. **Check the unhappy path**: kill it mid-way. What state remains? Is it named?
4. **Check the claim**: quote the user-facing sentence your change enables. Does it overclaim?
5. **Check the absence**: what did you *not* do that the task spec asked for? Say it explicitly.

Recorded as four lines in the PR body: what surprised me / what I tried to break / what remains on
kill / what I did not do.

---

## 11. Review checklist (reviewer's copy)

**Correctness**
- [ ] Does the change do what the task spec says, and only that?
- [ ] Would the new test fail on the parent commit?
- [ ] Are error paths handled, typed, and documented?

**Safety**
- [ ] Any write outside `refs/ilvuno/*` or `.ilvuno/**`?
- [ ] Any path that could touch a user ref, worktree, stash, or `HEAD`?
- [ ] Is the failure path journaled/rolled back rather than silently left?

**Determinism**
- [ ] Does the change introduce wall-clock, ordering, locale, or path dependence?
- [ ] Is the volatile data confined to the `run` block?

**Honesty**
- [ ] Does any output string overclaim (banned list)?
- [ ] Is an interpretation able to reach a gate or an exit code?
- [ ] Would a reviewer reading only the report be misled?

**Evidence**
- [ ] Are AC and tests linked?
- [ ] Is there a benchmark/scenario reference?
- [ ] Is the task spec updated where reality diverged?

**Cost**
- [ ] Any new dependency? Justified in an ADR?
- [ ] Binary size / startup impact considered?

---

## 12. What "done" is not

| Not done | Because |
|---|---|
| "Works on my machine" | the fixture forge exists precisely to remove this defence |
| "Tests pass locally" | determinism and crash matrices run in CI |
| "I'll add tests in the follow-up" | the follow-up does not exist; the test ships with the change or the change does not ship |
| "The docs are obvious" | the docs are part of the deliverable, and `docs-check` enforces it |
| "It only fails in the timeout path" | timeouts are a P0 path (B-18, `T-CMD-2`) |
| "It's just a warning" | warnings are how a gate erodes; they have tests too |
| "The benchmark is flaky" | fix the benchmark (2× headroom), do not skip it |
