# Gate log

One entry per gate, appended when a gate is run. Each entry links its packet.

## G0 — M0 test harness is trustworthy enough to build on

- Milestone: M0, authorized 2026-11-20 by the project owner (`docs/readiness/06-m0-readiness-checklist.md` §7).
- Status: **IN PROGRESS. G0 is not passed.**
- Local evidence (2026-11-20, host Linux, Git 2.55.0; pinned image Git 2.38.1):
  - `make build`, `make test`, `make lint`, `make docs-check`: exit 0.
  - `make forge-verify`: 68 PASS, 0 FAIL (G0-2).
  - Fixture suite: F1–F8 and F10 match `expected.json` and `oracle.json` (G0-3); F9 injection test.
  - Safety fingerprint: 8 mutations, namespace, 20 no-op runs, timing 39.7 ms (G0-4); anchored namespace (G0-5).
  - `make reference-matrix`: 28/28 on 2.55.0 and 2.38.1; `make reference-tests`: 86 + 95 checks pass (G0-6).
  - Importrule negative test and a planted violation (G0-9).
  - Race detector: all packages pass.
  - X-09 note with the T-0005 run: `docs/research/09-git-floor.md` (G0-7, owner review pending).
- Not done: CI runs on linux and macOS (G0-1, G0-8; no push; macOS reference leg blocked); `make gate G0` runner (G0-10; no owning task); independent review (G0-11).
- Reviewer (G0-11): **pending.** Must be someone other than the author.

## Corrections and chronology (2026-11-21)

These entries do not overwrite the historical entry above. They record facts established after it was written.

- **C-a (CI result; supersedes "CI not run" above).** PR #2 (`docs/commit-rules`, head `a526c43`, superseded, still OPEN) ran CI as run 44797155359 on 2026-11-20 at 18:54 UTC. `build, test, lint` (ubuntu and macos), the Git 2.38.1 reference job and the failing-git job passed. Two jobs failed: `reference negative tests and tests.sh (ubuntu-latest)`, where tests.sh check `P1.4 the real matrix passes` failed, and `reference negative tests and tests.sh (macos-latest)`, where `check-reference-test` reported 83 passed and 3 failed (AC-9 cp and rm injected-step naming, AC-12 mismatch diff). These failures are not yet diagnosed. No CI pass is claimed for any commit on this branch.
- **C-b (test counts).** "make reference-tests: 86 + 95 checks pass" was a local run on the uncommitted M0 tree on 2026-11-20. It has not been reproduced on CI, and the CI run in C-a contradicts it.
- **C-c (chronology).** The M0 implementation branch `feat/T-0001` is reconstructed in task order (T-0001, T-0002, T-0004, T-0005, T-0003), as approved under D-19 and the owner's order decision. Its commit timestamps are the reconstruction time. The 2026-11-20 runs in this file were executed on the uncommitted tree, not per reconstructed commit. Per-commit verification is recorded only when that commit is checked, with its own dated entry.

## Corrections and chronology (2026-11-21, T-0005)

These entries again leave the historical entry and C-a to C-c unchanged.

- **C-d (diagnosis of C-a; supersedes "not yet diagnosed").** Both failing jobs were diagnosed from the hosted logs. The
  evidence is in `docs/research/09-git-floor-evidence/t0005-ci-pr2-failures.txt`.
  - *ubuntu-latest:* the `P1.4` failure is by design. The job ran `tests.sh` on the runner's Git, which was 2.56.1. P1.4
    asserts a supported Git (2.55.0 or 2.38.1), so it refused. Fix: the host job runs the negative tests only. `tests.sh`
    runs in the pinned Git 2.38.1 image, where it passes 95/0.
  - *macos-latest:* the three failures had two causes. `chmod 700 -- "$d"` is read as a file name by BSD `chmod`. The oracle path
    also used GNU-only `stat -c`, so the oracle aborted before the comparison. Both are fixed. A Linux reproduction with
    BSD-semantics shims gives the same 83 passed and 3 failed (`docs/research/09-git-floor-evidence/t0005-bsd-shim-results.txt`).
    The reproduction is not a macOS run.
- **C-e (local verification, T-0005 working tree, 2026-11-21; not CI).** On Git 2.55.0 (host): `make reference-matrix` 28/28,
  exit 0. `SP1_GIT_BIN=/bin/false make reference-matrix` exit 2. `make reference-tests` exit 0, with 87 negative and 95
  `tests.sh` assertions passing. On Git 2.38.1 (pinned image `ilvuno-git238r:rebuild-2026-11-20`, ID
  `sha256:1044b988f91593334ab23b73b7b8ecbb308e49eda57c81ed86851677ef6fe859`): 87/0, matrix 28/28, `tests.sh` 95/0, exit 0.
  The fresh oracle is byte-identical to the tracked `oracle.json`. The Git-floor note is `docs/research/09-git-floor.md` §9.
- **C-f (test counts).** The negative-test count is 87 from 2026-11-21 on. It was 86 on 2026-11-20, and the difference is the
  AC-11 check that a cleanup failure prints no PASS line. Under C-b, the 86 + 95 record stays a 2026-11-20 local result.
- **C-g (macOS is unverified).** No macOS reference result exists. The hosted macOS runner ships Git 2.56.1, outside the
  supported pair, so the macOS matrix cannot pass its version gate. G0-8 stays open on macOS (`09-git-floor.md` §12, item 1).
- **C-h (docs-check).** `DOCS_CHECK_DEFERRED` is removed. `make docs-check` is strict and also covers the `testdata/reference`
  paths cited by T-0005 and the Git-floor note. A missing cited path fails the check, as tested on 2026-11-21.
- **G0 status after T-0005.** G0-6 (reference conformance) is locally met on 2.55.0 and 2.38.1 (C-e). G0-7 has its note
  (`09` §9 to §12) and awaits owner review. G0-1 and G0-8 are pending hosted CI, and macOS is blocked (C-g). G0-10 and G0-11
  are unchanged. G0 is not passed.

## Corrections (2026-11-21, M0 review remediation)

These entries leave the historical entry and C-a to C-h unchanged. C-i is recorded under the owner's approval of the
sixth commit, 2026-11-21 (`docs/readiness/08-m0-remediation-amendment.md`, approved subject to its §8 checks).

- **C-i (corrections to stale evidence claims).**
  - *Forge containment.* The T-0002 review checklist said the forge "cannot accidentally write outside t.TempDir()". The
    lexical check it relied on did not cover symlinks, and worktree names were not checked. Both escape routes are now
    refused, with regression tests that check an external sentinel is unchanged. The `DRIVER_MARKER` path is held to the
    same containment rule.
  - *F8 driver marker.* The marker was described as unobservable. It is observable inside the fixture's temporary root,
    and the refused run leaves it absent. The control is in `f08_driver_test.go`.
  - *AC-13 wrong owner.* The wrong-owner case was described as untested. It runs locally in the pinned Git 2.38.1 image,
    through `testdata/reference/check-reference-owner.sh`. **Pending G0 requirement:** the case must run in an authorized
    hosted CI path, or G0 must record it as not met (amendment §11). It is not in any CI path yet, and macOS evidence does
    not exist.
  - *Counts.* The negative-test count stays at 87, and the `tests.sh` count stays at 95. The wrong-owner case is counted
    separately and is not part of either count.
