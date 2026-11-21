# 08 — M0 review remediation: D-19 amendment

**Status: APPROVED by the project owner, 2026-11-21, subject to the pre-commit checks in §8.** The owner's approval
is the written instruction in the session of that date. It approves this amendment, the exceptions E1, E1′, E2, E3, E4
and E5 in §4, the 12-path inventory in §3 and the identity correction in §5. No separate signature was supplied, and
none is recorded here. The approval authorizes one local remediation commit. It does not authorize a push, a PR, a
merge, a settings change, or any change to the original checkout or the accepted snapshot.

The approval does not change the pending G0 requirements in §11.

**Relation to D-19.** D-19 (`docs/decisions/open-questions.md` §0, row D-19, decided 2026-11-20) fixed the M0 delivery
shape as two PRs: the governance branch, then `feat/T-0001` with five task commits. This amendment adds one sixth,
remediation commit to `feat/T-0001`, and it adds the narrow exceptions in §4. D-19 itself is not edited here. The
five task commits are not rewritten.

## 1. Context

An independent engineering review of the M0 implementation returned BLOCKED. The review's text is not held in this
repository. This record addresses the findings as enumerated in the remediation request (items A to E). The
mapping between those items and the review's own wording needs the owner's confirmation.

## 2. The preserved chain (not rewritten, not amended)

Base: `origin/main` `83e91a1`. Branch `feat/T-0001`, in the owner-approved order T-0001, T-0002, T-0004, T-0005, T-0003.

| Commit | Task | Subject |
|---|---|---|
| `c23a899` | T-0001 | chore(repo): add M0 module, Makefile, CI and import-rule harness |
| `efb6d7b` | T-0002 | test(forge): add deterministic git fixture forge |
| `07cf295` | T-0004 | test(safety): add INV-1 fingerprint testkit |
| `e27b06a` | T-0005 | test(reference): add SP-1R3 reference harness, Git-floor evidence and CI jobs |
| `b957128` | T-0003 | test(fixtures): port the SP-1R3 failure corpus onto the forge |

## 3. Sixth commit: scope and path inventory (approved)

One remediation commit, on `feat/T-0001`, after `b957128`. Its subject is
`fix(testkit): remediate review findings on forge containment, F8 evidence and wrong-owner test`.
The inventory is exactly these 12 paths, and no others.

| Path | Change | Finding |
|---|---|---|
| `internal/testkit/forge/forge.go` | modified | A: containment (`containedPath`, shared by the repository and the worktrees); B: `SetEnv` (allowlisted, contained) and `TempRoot` |
| `internal/testkit/forge/containment_test.go` | new | A: regression tests for both escape routes, with external sentinels; B: `SetEnv` traversal, symlink and directory cases |
| `internal/testkit/forge/resolve_test.go` | new | A: nested-path guard (a defect found while fixing A) |
| `internal/testkit/fixtures/f08_driver.go` | modified | B: marker in the fixture's temporary root; checked absent after the refused run |
| `internal/testkit/fixtures/f08_driver_test.go` | new | B: observable-marker control (refused run vs plain git); a symlink created after `SetEnv` is refused before any merge |
| `testdata/reference/check-reference-owner.sh` | new | C: wrong-owner negative test, in the pinned image |
| `testdata/reference/check-reference-test.sh` | modified | C: explicit NOT RUN line for the wrong-owner case; counts unchanged |
| `testdata/reference/README.md` | modified | C: entry point for the wrong-owner test |
| `docs/tasks/T-0002.md` | modified | D: identity and timestamp correction (§5) |
| `docs/readiness/08-m0-remediation-amendment.md` | new | D: this record |
| `docs/research/09-git-floor.md` | modified | D: correction of the wrong-owner status in §9 (AC-13 row) and §12 item 5 only |
| `docs/planning/gate-log.md` | modified | D: one dated correction entry, C-i |

Nothing in `Makefile` changes in the sixth commit. The `forge-verify` recipe already covers the forge, fixtures and
safety packages. The Makefile exception in §4 approves that existing recipe, and it adds no target.

## 4. Narrow scope exceptions requested for approval

| ID | Exception | Why it is needed | Narrowest form |
|---|---|---|---|
| E1 | The T-0003 `forge-verify` change (adds `./internal/testkit/fixtures/...`) to `Makefile` | T-0003 allowlist names `internal/testkit/fixtures/**` and the Makefile is outside it | the one `forge-verify` recipe line; no other recipe or target |
| E1' | The T-0004 `forge-verify` change (adds `./internal/testkit/safety/...`) | Owner-directed in T-0004, outside its allowlist | the same recipe line |
| E2 | Changes to forge internals (`forge.go`) | T-0003 says "forge internals: report a forge bug instead". The containment defect is that bug, and the fix is in T-0002's allowed scope (`internal/testkit/forge/**`) | one shared check, `containedPath`, used by `resolve` (repository paths) and `SetEnv` (the marker under the temporary root); worktree name and destination checks; the additions `SetEnv` (one allowlisted variable, `DRIVER_MARKER`) and `TempRoot` |
| E3 | `check-reference-owner.sh`, a new script that runs a container as root to make the other-owned directory | AC-13 wrong-owner needs a second UID; `sudo` is not available here, Docker is | one script, no Makefile target and no CI change; runs only with Docker and the pinned image; exits 2 with NOT RUN otherwise, and never reports a pass in that case |
| E4 | Correction of the active T-0002 specification (identity and timestamps) | Owner decision 2; the spec text does not match the evidence | one sentence in T-0002 |
| E5 | `docs/planning/gate-log.md` entry C-i | Outside the T-0005 gate-log exception, which covered only T-0005 | one dated entry; earlier entries unchanged |

## 5. Identity and timestamps (owner decision 2)

Fixture commits use `fixture <fixture@example.com>` with the single date `2020-01-01T00:00:00Z` for every commit,
as `testdata/reference/lib.sh` `sp1_env` does. The oracle's base SHAs depend on it, and the probe with F1's base
reproduces the recorded base SHA `336876da…` and tree `54c4e15d…`.

Integration merges use `ilvuno <ilvuno@localhost>`, with the base commit's committer time as the author and committer
date, in `+0000`. This is how `testdata/reference/oracle.py` builds them, and `expected.json` records it as
`ilvuno <ilvuno@localhost> @<base commit time>`.

The active T-0002 specification now states both (`docs/tasks/T-0002.md`, the reuse paragraph).

## 6. INV-1 protection boundary (owner decision 3)

**Default guarantee.** The fingerprint in `internal/testkit/safety` (T-0004) guarantees that, between two calls, none
of the following changed for a repository it was given. These are the user's **Git-managed integration state**:

- refs outside `refs/ilvuno/` (name and object), via `for-each-ref`;
- `HEAD`: the commit, and the symbolic target;
- index entries (path, mode, object and unmerged stage), via `ls-files --stage -z`;
- working-tree paths that `git status --porcelain=v2 -z --untracked-files=all` reports (tracked changes and
  untracked files), with the mode and SHA-256 of the content, or the target of a symlink;
- the stash list, in order;
- the SHA-256 of `.git/config`.

Excluded by design: `refs/ilvuno/*` and `.ilvuno/` (Ilvuno's own candidate namespace).

**Not covered (stated limits, not defects to fix in M0):**

1. **Git-ignored files are outside the guarantee.** `git status` omits them, so their creation, change and deletion
   are invisible. That includes build outputs, local configuration such as `.env`, and the `ilvuno` binary.
2. **Other `.git` state is not compared:** `.git/hooks`, `.git/info/exclude`, `config.worktree`, included config
   files, reflogs, and the object database. Objects are immutable and are not compared.
3. **Stat cache.** `git status` trusts index stat data (size and mtime) for paths it considers clean. A tracked file
   rewritten with the same size and mtime, outside git's racy-timestamp window, is not reported, so the fingerprint
   cannot see it. The fingerprint reads content only for paths that status reports.
4. **Configuration and environment.** The fingerprint runs git with the caller's environment, except the
   variables that redirect the repository (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_COMMON_DIR`,
   `GIT_OBJECT_DIRECTORY`, `GIT_ALTERNATE_OBJECT_DIRECTORIES`, `GIT_NAMESPACE`). User and system configuration still
   applies, so `status.*` and `core.fsmonitor` can change what status reports. A configured `core.fsmonitor` program
   is run by git. The fingerprint is not sandboxed against user configuration.
5. **Not atomic.** Two snapshots are taken at separate times. A concurrent writer between them is not excluded.
6. **Platform.** The macOS `git status` output has not been tested (T-0004 residual risk).

**Operations that could mutate ignored files.** None of these is performed by the M0 code, except where stated. Each
one needs its own safety decision before Ilvuno runs it on a user repository:

- `git clean -x` or `-X` (deletes ignored files). AGENTS §8 forbids it without approval.
- `git stash -a` or `--all` (stashes ignored files and removes them from the tree).
- `git worktree remove --force`, or deleting a worktree directory, which removes that worktree's ignored files.
- `git checkout`, `git merge` or `git reset` when a tracked path arrives where an ignored file exists. Git's documented
  behaviour is to overwrite ignored files in that case. This needs checking before any candidate is applied to a
  user's working tree.
- Ilvuno's own candidate cleanup inside `.ilvuno/`, which is ignored and excluded from the fingerprint by design.

The reference harness (`testdata/reference/`) copies itself into private roots and never runs in a user repository.

## 7. Corrected evidence claims (necessary corrections only)

| Earlier claim | Where | Correction |
|---|---|---|
| "Forge cannot accidentally write outside t.TempDir() (path prefix assertion)" | T-0002 review checklist; `efb6d7b` | The lexical check did not cover symlinks. Both escape routes are now refused, and regression tests show it (A). |
| "The driver marker cannot be observed" | `b957128` commit message; `f08_driver.go` | Superseded. The marker is observable in the fixture's temporary root, and the refused run leaves it absent (B). |
| "Wrong-owner roots are not tested" | `docs/research/09-git-floor.md` §12, item 5 | Superseded. The test runs locally in the pinned image. It is not yet in hosted CI (C). |
| "AC-13 met" | `docs/research/09-git-floor.md` §9 | Narrowed. The owner-side negative cases are met; the wrong-owner case is met locally, and hosted evidence is pending. |

## 8. Pre-commit checks and their results

The approval is subject to these checks. Each one was run on the final working tree before the commit.

1. **HEAD and inventory.** `feat/T-0001` is at `b957128`, nothing was staged, and the working changes are exactly the
   12 paths in §3.
2. **Amendment read in full.** Its status, the D-19 relation, the approvals, the exceptions and the pending requirement
   in §11 match this record.
3. **DRIVER_MARKER containment.** A probe confirmed two defects in the first version of `SetEnv`. A traversing value
   (`<root>/../x`) passed the string-prefix test, and a symlink at the marker path was accepted. Both are fixed. The
   check is now a clean absolute path strictly inside the temporary root, with no symlink among its components
   (`containedPath`, the same helper as the repository check). The F8 control also re-checks the marker path
   before any merge runs. Regression tests: five `SetEnv` cases (traversal, symlinked marker, symlinked directory,
   root and other variables, plain control) and one case for a symlink created after `SetEnv`. Mutants: removing the
   containment check fails the two symlink tests, and removing the marker guard fails the post-`SetEnv` test. The
   traversal case is refused by two layers, so removing only the containment check does not fail it. No thirteenth
   path was needed.
   The check is made when the variable is set, and again before the control runs. Neither check is atomic with the
   write that follows, so the remaining window is recorded in `resolve`, `SetEnv` and §6.
4. **Wrong-owner test.** It is not in any authorized CI path, so it is recorded as a **pending G0 requirement** (§11),
   and no pass is claimed for it. The negative suite prints NOT RUN, and the counts exclude it.
5. **Verification on the final tree:**
   - `make build`, `make test` (5 packages ok), `make lint`, `make docs-check` (strict): exit 0 each.
   - `make forge-verify`: 96 PASS, 0 FAIL, covering forge, fixtures and safety.
   - `go test -race` and `-count=3` on `internal/testkit/...`: exit 0.
   - The same Go suites inside the pinned Git 2.38.1 image: 96 PASS, 0 FAIL.
   - `make reference-matrix`: exit 0, 28/28. With `SP1_GIT_BIN=/bin/false`: exit 2.
   - `make reference-tests`: exit 0. Negative tests 87/0, `tests.sh` 95/0, plus the NOT RUN line.
   - The reference CI command set inside the pinned image: exit 0, 87/0, 28/28, 95/0, no scratch residue.
   - `check-reference-owner.sh`: exit 0, 4 of 4 PASS.
   - Earlier mutants: the wrapper ownership refusal removed, so the wrong-owner message assertion fails.
     Nested-path regression: a defect of the first `resolve` change, caught by the fixture suite and fixed.
6. **Staging and commit.** Recorded in the remediation report. Only the 12 paths were staged, the whitespace check
   ran on that exact set, and the commit is one local commit on `feat/T-0001`.

## 9. Owner decisions

1. **Decided: approved** (2026-11-21). The amendment, the inventory (§3) and the exceptions (§4).
2. **Decided: approved.** The identity correction (§5).
3. **Decided: approved.** The INV-1 boundary and the operations list (§6) as the default, with the limits as stated.
4. **Open: a G0 requirement (§11).** The wrong-owner test is not in a hosted CI path. Joining `ci.yml` is a separate
   decision and is not authorized by this approval.
5. **Open: a separate safety review** for ignored-file mutation (§6). It must come before any Ilvuno operation that could
   run one of the listed commands on a user repository. It is not part of this commit.

## 11. Pending G0 requirement: AC-13 wrong-owner test in a hosted CI path

The wrong-owner test (AC-13, `testdata/reference/check-reference-owner.sh`) has run locally, on one host, in the pinned
image. It is not in any CI path. G0 needs one of two outcomes, and the owner chooses:

- **(a)** the test runs in a hosted CI path the owner has authorized, most likely the `reference-git-2-38-1` job in
  `ci.yml`, which needs its own approval; or
- **(b)** G0 records AC-13's wrong-owner case as **not met**, and G0 does not pass on that basis.

Until one of these is recorded, the wrong-owner case is not counted as a pass anywhere in the repository. The negative
suite prints it as NOT RUN, and its counts exclude it.

## 10. Not proven by the local evidence

- Any macOS behaviour (no macOS run exists).
- Hosted CI for any commit in this chain (no push).
- The forge's containment against a concurrent same-UID adversary. The checks are not atomic (§6, and `resolve`).
- The wrong-owner test on the hosted runner. It has run only on this host with Docker.
