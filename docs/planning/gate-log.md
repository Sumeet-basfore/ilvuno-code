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
