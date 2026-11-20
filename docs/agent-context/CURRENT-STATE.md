# CURRENT STATE — Ilvuno

As of: 2026-11-20 · Source of truth for status: `docs/readiness/06`, `docs/planning/gate-log.md`,
the Git working tree. This file summarizes; it does not authorize.

## Authorization

- **M0: AUTHORIZED** 2026-11-20 by the project owner (`docs/readiness/06` §7), scope §3.1 and
  T-0001…T-0005. [approved]
- **M1–M6: NOT AUTHORIZED.** **G0: IN PROGRESS, NOT PASSED.**
- Policy: AGENTS §15 Git branching and PR review is **APPROVED 2026-11-20** by the project owner,
  and governs where it conflicts with §§7–9. [approved]

## Implemented [implemented, working tree only]

All M0 code is **uncommitted/untracked**. Nothing below is on `main`.

- T-0001 scaffold: `go.mod`, `Makefile`, `cmd/ilvuno`, `internal/lint/importrule`,
  `.github/workflows/ci.yml`, LICENSE, community files.
- T-0002 forge, T-0003 fixtures, T-0004 safety fingerprint.
- T-0005 reference harness under `testdata/reference/` (28-case matrix; 95 assertions).
- `ilvuno --version` and `--help`; no product behaviour.

Recorded local evidence (2026-11-20, host Linux/Git 2.55.0 + container Git 2.38.1):
`make build`, `test`, `lint`, `docs-check` exit 0; forge 68/68; fixtures match
`expected.json`/`oracle.json`; safety mutations detected; reference matrix 28/28 on both Git
versions; race detector clean. Source: `docs/planning/gate-log.md`. **Not re-run for this file.**

## G0 status

| Criterion | Status |
|---|---|
| G0-1 clean-clone/CI `make test` | PENDING CI |
| G0-2 forge determinism | LOCAL PASS |
| G0-3 fixture corpus | LOCAL PASS |
| G0-4 safety fingerprint | LOCAL PASS |
| G0-5 anchored refs | LOCAL PASS |
| G0-6 reference conformance | LOCAL PASS |
| G0-7 X-09 note | PARTIAL — owner review pending |
| G0-8 CI Linux + macOS | PENDING — not run; macOS reference leg blocked |
| G0-9 importrule negative test | LOCAL PASS |
| G0-10 `make gate G0` runner | NOT IMPLEMENTED (exits 2) |
| G0-11 independent gate review | PENDING |

G0 does not pass until CI runs, the gate runner exists or is explicitly waived, and a reviewer
other than the author signs the packet. **Merging M0 code does not pass G0.**

## Git state

- Branch `docs/commit-rules` (pushed; governance G1–G5 committed **locally**, **6 commits ahead**
  of `main` `4d6dd4c`, not yet pushed).
- Working tree: **2 modified** (`README.md`, `.gitignore`, both T-0001 paths), **82 untracked files** —
  M0 implementation (`cmd/`, `internal/`,
  `testdata/`, `.github/`, `go.mod`, `Makefile`, LICENSE, community files), `docs/planning/gate-log.md`,
  `docs/research/09-*`, the 20 deferred `docs/experiments/` files, the two generated `.pyc`, and the
  ten `ilvuno-concept-*.svg` brand assets.
- Ignored/local: `ilvuno` binary; `docs/experiments/scripts/__pycache__/` is **not** ignored yet.
- No staged files, no stash, one worktree. [verified by `git status`, `git stash list`,
  `git worktree list`]

## Decisions

- Decided: D-01…D-06, D-16…D-19; A-4, S-1…S-4, C-5, C-7.
- Open, M2-blocking: D-07 refs, D-08 drivers, D-09 write scopes, D-10 determinism.
- Open, later: D-11…D-15; A-5…A-8, A-10.
- Open product premise: Q-1 (real combined-failure rate), no owner/date.

## Unverified claims

- **E-8 and E-9 PASS** rest on owner-reported independent reviews; the verdict texts are not
  committed. Do not cite them as reproducible evidence.
- CI has never run. macOS reference leg is blocked (`docs/research/09` §8).
- G0-7 owner review and G0-11 independent review are pending.
- The 2026-11-20 local results are a gate-log record, not a fresh verification by this file.

## Next actions

1. Owner reviews and pushes PR-1 (`docs/commit-rules`, G1–G5); run CI on it.
2. Merge PR-1, then create PR-2 from the post-merge `origin/main` and commit T1–T5.
3. Complete G0-1, G0-7, G0-8, G0-10 or record an explicit waiver, and G0-11.
4. Do not start M1 until G0 passes and M1 is authorized.
