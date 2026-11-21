# 09 — Git floor evidence (X-09), Git 2.38.1

Status: E-10 evidence, written 2026-11-20. Scope: the X-09 requirement in `docs/readiness/02` §9 for the
operations M0 uses, on the floor version **Git 2.38.1** (D-05) and on current **Git 2.55.0**. This is
verification evidence, not M0 implementation. T-0005 extends this note with its own run (`docs/tasks/T-0005.md`).

Raw outputs are in `docs/research/09-git-floor-evidence/`:
`probe.sh` (X-01 to X-06, X-10), `x05.sh` (X-05), `host.log` and `c238.log` (probe output on each version),
`build.log` (floor image build), `h-matrix.log`, `h-tests.log` and `c238b-run.log` (SP-1R3 runs).

---

## 1. Versions and provenance

| | Git 2.38.1 (floor) | Git 2.55.0 (current) |
|---|---|---|
| Source | Image `ilvuno-git238r:rebuild-2026-11-20`, built 2026-11-20 from `docs/experiments/scripts/Dockerfile.git238` with `--no-cache`. Image ID `sha256:1044b988f91593334ab23b73b7b8ecbb308e49eda57c81ed86851677ef6fe859`. Base `alpine:3.17@sha256:8fc3dacfb6d69da8d44e42390de777e48577085db99aa4e4af35f483eb08b989`. Git tarball `git-2.38.1.tar.xz`, SHA-256 `97ddf8ea58a2b9e0fbc2508e245028ca75911bd38d1551616b148c1aa5740ad9`, checked by the Dockerfile during the build (build exit 0). | Host `/usr/bin/git` |
| Platform | Alpine 3.17 (musl), Linux | Linux (host) |

## 2. X-09 requirements and results

"Identical" means the same stdout, exit code and stderr class on both versions.

| Req | Readiness source | Git 2.38.1 | Git 2.55.0 | Result |
|---|---|---|---|---|
| X-01: merge-tree takes two commits; three commits rejected | `02` §2 X-01, X-02 | three commits → exit 129, usage line | exit 129, usage line | identical |
| X-02b: output shape. Clean: `<tree>\0` exit 0. Conflict: `<tree>\0<path>\0` exit 1. Unrelated: exit 128 with `fatal: refusing to merge unrelated histories` | `02` §2 X-02b | clean: stdout `66383ddc…|`, rc 0; conflict: rc 1 with `shared.txt` listed; unrelated: rc 128 | identical | identical |
| `--write-tree --name-only -z --no-messages` accepted | `02` §2 X-02b | accepted (runs above) | accepted | identical. The options are not listed in the `-h` usage text on either version, so acceptance was shown by running them |
| X-03: object pipeline reproduces sequential merges | `02` §2 X-03; E-1′ | SP-1R3 matrix 28/28 (§3) | 28/28 | covered by SP-1R3 |
| X-04: `worktree add -B refs/ilvuno/…` creates `refs/heads/refs/ilvuno/…`, not `refs/ilvuno/…` | `02` X-04 | `refs/heads` entry containing `ilvuno`: 1; `refs/ilvuno` entries: 0 | same | identical. HI-01 confirmed on the floor |
| X-04b: `worktree add --detach` + `update-ref` + cleanup leaves no residue | `02` X-04b | HEAD equals the pinned SHA; worktree `detached`; after cleanup: worktrees 1, `refs/ilvuno` 0 | same | identical |
| X-05: a configured merge driver runs under `merge-tree --write-tree` | `02` §7 X-05; `sp1-security-observations.md` §1 | driver ran; rc 0 (benign marker, `x05.sh`) | driver ran; rc 0 | identical. Driver refusal is an M2 policy (D-08), and the M0 harness refuses before any run |
| X-06: object writes by a conflicting `merge-tree` | `02` §6 X-06 | loose objects 19 → 21 | 19 → 21 | identical |
| X-10: `check-attr --source` available | `02` §2 X-10 | **rc 129, `unknown option`** | rc 0 | **differs.** Absent on the floor. M2 needs a capability probe. `readiness/02` §7 already requires one |

Recorded but not used as evidence: the usage-text line counts for `--`, `--stdin` and `--no-messages`
differ between versions. That is documentation text, not behaviour.

## 3. SP-1R3 coverage (reference matrix and test suite)

| Run | Git 2.38.1 (rebuilt image) | Git 2.55.0 (host) |
|---|---|---|
| `sp1.sh matrix` (28 rows: F1–F10, object and worktree approaches) | PASS 28/28, exit 0 (`c238b-run.log`) | PASS 28/28, exit 0 (`h-matrix.log`) |
| `tests.sh` (95 assertions), patched C-5 copy | 95 passed, 0 failed, exit 0 (`c238b-run.log`) | 95 passed, 0 failed, exit 0 (`h-tests.log`) |

The E-1′ four cases map to F2 (3 clean), F4 (conflict at step 2), F5 (ancestor, skip) and F6 (unrelated). Each
passes on both approaches on both versions.

## 4. Gaps, with dispositions

| Gap | Description | Why it does not block E-10 | Disposition |
|---|---|---|---|
| G-1 | Git 2.39 was not run. X-09 named 2.38 and 2.39. | The floor is 2.38.1. A 2.39 run characterizes a higher version and does not establish the floor. | Not run. Run before any release claim that names 2.39. |
| G-2 | macOS has no Git-version or harness evidence. | D-05 requires macOS tested in CI. CI is M0 work. | Verified by the macOS CI leg, required before G0 (`planning/05` G0 criterion 6). |
| G-3 | `check-attr --source` is absent on 2.38.1 (X-10). | M0 does not use it. M2 driver policy (D-08) does. | M2 capability probe. The fallback that refuses on 2.38.1 is evidenced in `sp1-security-observations.md` §2 ("with fallback: driver_refused"). |
| G-4 | Image provenance. The Dockerfile's Git checksum was pinned from an earlier build and is not verified against an upstream signature (Dockerfile comment; `sp1r-reproducibility.md`). | The checksum matched at the 2026-11-20 rebuild, so the tarball equals the pinned value. | The owner may require signature verification before any release claim. |
| G-5 | The 2.38.1 build is Alpine/musl, not glibc. | The claim is about Git behaviour. The CI legs run on glibc (Ubuntu) and macOS. | Covered by the CI legs (M0 work). |
| G-6 | X-11, X-12 and X-13 are outside the X-09 scope (conflict-style, driver refusal end-to-end, crash during worktree add). | Those are M2 items. | M2. |

## 5. C-5 patch (verified in a scratch copy; applied in T-0005)

Three lines in `tests.sh` change. `T=$SP1_SCRATCH/tests` is already defined at line 9.

```diff
-rm -rf /tmp/r3-cl; mkdir -m700 /tmp/r3-cl
+rm -rf $T/r3-cl; mkdir -m700 $T/r3-cl
-outc=$(SP1_LIB_ONLY=1 bash -c '…' "$SP1_SELF" /tmp/r3-cl 2>&1)
+outc=$(SP1_LIB_ONLY=1 bash -c '…' "$SP1_SELF" $T/r3-cl 2>&1)
-check "N4.5 refused cleanup keeps the root" bash -c "test -n \"\$(find /tmp/r3-cl -maxdepth 1 -type d -name 'run-*')\""
+check "N4.5 refused cleanup keeps the root" bash -c "test -n \"\$(find $T/r3-cl -maxdepth 1 -type d -name 'run-*')\""
```

No assertion changed. Result: 95/95 on both versions (§3). The unpatched suite's `/tmp/r3-cl` exists on this machine
(owner `sumeet`, created 13:09 on 2026-11-20, containing one `run-*` directory). It appears to come from an
earlier unpatched run. That is not verified, and this work did not remove it.

## 6. Conclusion

**E-10: PASS, with documented gaps.** The 2.38.1 floor reproduces every X-09 behaviour that the M0 design
uses. One capability, `check-attr --source`, is absent on the floor, and M2 handles it with a probe and a
refusing fallback. Gaps G-1 to G-6 are recorded in §4 with their dispositions.

## 7. Reproduction

Build the floor image from the pinned Dockerfile. The Dockerfile copies nothing, so any empty context works:

```
docker build --no-cache -f testdata/reference/Dockerfile.git238 -t ilvuno-git238r:rebuild-2026-11-20 <empty-context>   # from the repository root (T-0005)
```

Probes (use a private temporary directory; each probe removes its own temporary repository):

```
GIT_BIN=/usr/bin/git TMPDIR=<private> bash docs/research/09-git-floor-evidence/probe.sh
docker run --rm -u "$(id -u):$(id -g)" -v <private>:/work \
  -v "$PWD/docs/research/09-git-floor-evidence/probe.sh:/probe.sh:ro" \
  -e GIT_BIN=/usr/local/bin/git -e TMPDIR=/work --entrypoint /bin/bash \
  ilvuno-git238r:rebuild-2026-11-20 /probe.sh
GIT_BIN=/usr/bin/git TMPDIR=<private> bash docs/research/09-git-floor-evidence/x05.sh   # and the container form
```

## 8. Historical T-0005 run (2026-11-20, uncommitted working tree)

> **Historical.** This section records the 2026-11-20 run on the uncommitted M0 working tree. It is kept
> unchanged. The committed state, the CI diagnosis and the residual limits are in §9 to §12, and they
> supersede the residual list at the end of this section.

Run through the M0 wrapper (`testdata/reference/check-reference.sh`, negative tests `check-reference-test.sh`).
Every case uses a fresh private root. Raw logs are in `docs/research/09-git-floor-evidence/`.

| Check | Git 2.55.0 (host) | Git 2.38.1 (pinned image) |
|---|---|---|
| Image | host `/usr/bin/git` | `ilvuno-git238r:rebuild-2026-11-20`, ID `sha256:1044b988f91593334ab23b73b7b8ecbb308e49eda57c81ed86851677ef6fe859` |
| `run-env.json` | `{"git_version":"2.55.0","os":"Linux"}` | `{"git_version":"2.38.1","os":"Linux"}` |
| Matrix (28 rows) | PASS 28/28, exit 0 | PASS 28/28, exit 0 |
| Negative tests AC-9…AC-13 | 86 passed, 0 failed | 86 passed, 0 failed |
| `tests.sh` (95) | 95 passed, 0 failed | 95 passed, 0 failed |

Further evidence:

- **R3-3, fresh oracle:** the oracle regenerated in a fresh copy is byte-identical to the tracked `oracle.json` (wrapper `oracle` mode, PASS, 2.8 s).
- **Failing git (AC-3):** `SP1_GIT_BIN=/bin/false make reference-matrix` exits 2, which is nonzero.
- **Non-vacuity:** two mutants of the wrapper were run against the same negative suite. With the `TMPDIR` preflight removed, the suite fails the mode-755 case, where `tests` mode had published a pass (a real gap, now closed by the base check). With the comparison disabled, the suite fails the three AC-12 checks.
- **AC-5:** `git grep 'merge-tree'` over `cmd`, `internal` (excluding testkit and the importrule fixture), `Makefile` and `.github` finds no match.
- **AC-6:** `go list -deps ./cmd/ilvuno` contains no `testdata` package, and the `build` target does not mention `testdata`.

Residual limitations (AC-7; recorded, not fixed):

1. **macOS reference legs are blocked.** The harness calls `sha256sum`, which macOS runners do not provide. The matrix's version gate also accepts only 2.55.0 and 2.38.1, so a stock macOS `git` will fail it. The macOS reference job is written but expected to fail until both are resolved. This blocks G0-8 on macOS.
2. **Wrong-owner roots are not tested.** Refusing a root owned by another user needs a second account. The owner check is in the wrapper and is untested.
3. **Scratch residue from `tests.sh`.** The unmodified test suite writes `sp1r2-victim-<pid>/sentinel` into `TMPDIR` and does not remove it. The Makefile and the CI job remove the whole base they own.
4. **R3-4.** The ownership marker, the registry and `run-env.json` are editable consistency guards, not attestations. The harness trusts its own process.
5. **Evidence is Linux only.** The macOS and CI legs have not been run; the workflow has not been pushed.

## 9. T-0005 committed state (2026-11-21)

Source: `testdata/reference/` (the approved copy with C-5, plus the adaptations in §11), the Makefile targets
`reference-matrix` and `reference-tests`, and the four reference-era jobs in `.github/workflows/ci.yml`. Logs and
tables are in `docs/research/09-git-floor-evidence/`.

| Check | Git 2.55.0 (host) | Git 2.38.1 (pinned image) |
|---|---|---|
| Git and Python | host `/usr/bin/git` 2.55.0; Python 3.14.7 | image `ilvuno-git238r:rebuild-2026-11-20`, ID `sha256:1044b988f91593334ab23b73b7b8ecbb308e49eda57c81ed86851677ef6fe859`; in-image `git version 2.38.1`; Python 3.10.15 |
| `run-env.json` | `{"git_version":"2.55.0","os":"Linux"}` | `{"git_version":"2.38.1","os":"Linux"}` |
| Matrix, `check-reference.sh matrix` (28 rows) | PASS 28 cases, exit 0 | PASS 28 cases, exit 0 |
| Negative tests AC-9 to AC-13, `check-reference-test.sh` | 87 passed, 0 failed | 87 passed, 0 failed |
| `tests.sh` (95 assertions) | 95 passed, 0 failed | 95 passed, 0 failed |
| `make reference-matrix` / `make reference-tests` | exit 0 / exit 0 | not run through `make`; the CI command set ran in the image with exit 0 |

The 87 negative tests are the 86 of the 2026-11-20 record plus the AC-11 check that a cleanup failure prints no
PASS line. Both Git versions run the same committed harness, and the same counts pass on each.

**Acceptance criteria.** The status of each criterion, with its evidence:

| AC | Result | Evidence |
|---|---|---|
| AC-1 | met on 2.55.0 and 2.38.1 | matrix 28/28 with `run-env.json` matching; a version outside the pair fails (P1.4 and the wrapper's version gate) |
| AC-2 | met locally | `tests.sh` 95/0. Write-scope check: `git status` (incl. ignored) and the modification-time check show no source path changed during `make reference-tests`. Residue is only the harness's own `sp1r.*` roots (§12) |
| AC-3 | met locally; hosted pending | `SP1_GIT_BIN=/bin/false make reference-matrix` exits 2. The CI job `reference-failing-git` is written and has not run |
| AC-4 | met | R3-3: the fresh oracle is identical to the tracked copy. An oracle failure runs no comparison. A mismatch is reported with a diff, prints no PASS, and leaves the tracked file unchanged (`t0005-negative-exit-codes.txt`) |
| AC-5 | met | `git grep merge-tree` over the scoped paths: exit 1 (no match) |
| AC-6 | met | `go list -deps ./cmd/ilvuno` has no `testdata` package; the `build` target does not mention `testdata` |
| AC-7 | met (this note) | image ID, both Git versions, Python, the C-5 diff, §11, and the limits in §12. Diff: `t0005-diff-vs-experiments.diff` |
| AC-8 | met | from the matrix run, plain rows: F2 ok exit 0 (object and worktree); F4 conflict at step 2, exit 3; F5 ok, exit 0; F6 `unrelated_histories`, exit 3 |
| AC-9 | met | setup failures (mktemp, chmod, cp, rm injected): exit 1, no oracle invoked, no comparison, tracked file unchanged (`t0005-negative-exit-codes.txt`) |
| AC-10 | met | oracle failure: exit 1, no comparison, tracked file unchanged |
| AC-11 | met | cleanup failure after a successful oracle: exit 1, cleanup reported, no PASS line. With an oracle failure too, the original status is kept (exit 1) and the cleanup failure is reported |
| AC-12 | met | mismatch: exit 1, the diff is printed, no PASS line, the wrapper's roots are removed, the tracked file is unchanged |
| AC-13 | met | 18 refusals (6 invalid roots × 3 modes): only the read-only `rev-parse --show-toplevel` is recorded, and only for the inside-worktree root (`t0005-ac13-recording-shim.txt`) |

## 10. Hosted CI: the PR #2 failures, diagnosed

PR #2 (head `a526c43`), hosted run 44797155359, 2026-11-20. Summary and the failing lines: `t0005-ci-pr2-failures.txt`.
The runners' Git was 2.56.1 on both platforms.

| Failing check | Actual failure | Root cause | Fix |
|---|---|---|---|
| reference negative tests and tests.sh (ubuntu-latest) | `P1.4 the real matrix passes` failed (94 passed, 1 failed); the negative tests passed 86/0 | The job ran `tests.sh` on the runner's Git 2.56.1. P1.4 asserts a supported Git (AC-1), so it refuses by design. The 94 other assertions passed. | The host job now runs the negative tests only. `tests.sh` runs in the pinned 2.38.1 image, where it passes 95/0. No assertion was changed. |
| reference negative tests and tests.sh (macos-latest) | `AC-9 cp failure names the injected step`, `AC-9 rm failure names the injected step` and `AC-12 mismatch is reported with a diff` failed (83 passed, 3 failed) | (1) The wrapper ran `chmod 700 -- "$d"`. BSD `chmod` stops option parsing at the first operand, so `--` becomes a file name, and `mk_private_dir` dies before the injected failure can show. (2) A separate, independent cause: the oracle path uses GNU-only `stat -c` in `lib.sh`, so the oracle aborts before any comparison, which is why AC-12 failed. | The `--` is removed. The oracle path uses portability helpers (§11). Reproduced with BSD-semantics shims (`t0005-bsd-shim-results.txt`). The hosted macOS run is still pending. |

The same review found a third issue that the macOS job did not show as a failure. On a host without `sha256sum`,
the "tracked oracle.json unchanged" checks compared two empty strings and passed. The test now probes for a SHA-256
tool and refuses to run without one.

Diagnosis evidence: the reproduction in `t0005-bsd-shim-results.txt` is a Linux simulation of BSD semantics. Its
result matches the hosted macOS log exactly (83 passed, 3 failed, the same three checks), but it is not a macOS run.

macOS also has a conformance gap that the fix does not close. The hosted macOS runner ships Git 2.56.1, outside the
AC-1 pair. So the macOS matrix cannot pass its version gate on that runner (§12, item 1).

## 11. Adaptations to the copied harness (T-0005)

The approved copy changed only in C-5 (§5). T-0005 also changed the following. The full diff against
`docs/experiments/scripts` is `t0005-diff-vs-experiments.diff`, and `testdata/reference/README.md` gives the table
with the reason for each change.

- **Portability (GNU and BSD userland, bash 3.2).** No `--` after an operand (`chmod`, `stat`). SHA-256 through
  `sha256sum`, else `shasum -a 256`. In-place edits through `sed_inplace`, not `sed -i`. `stat -c` with a BSD
  fallback; no `realpath -m`; `rm --one-file-system` only where GNU `rm` is present; no `mapfile -d`.
- **Fail loud.** `read_nul` aborts on a missing file, where it used to return an empty list. The runners in `sp1.sh` refuse
  an output directory that the caller did not create. The P4 test created no output directory and relied on the silent
  empty result; it now uses `mkout`.
- **Output and cleanup.** The wrapper prints its `PASS` line from the exit handler, and only when cleanup succeeded.
  `tests.sh` removes its own `sp1r2-victim` fixture on exit.

The output is unchanged. The oracle regenerated in a fresh copy is byte-identical to the tracked file under GNU and
under the shims. The matrix and the 95 assertions pass with the same counts on both Git versions.

**Owner decision.** The T-0005 review checklist asks for a copy identical to `docs/experiments/scripts` apart from the
C-5 change. That is no longer true, because the portability and fail-loud changes are needed for the macOS negative
tests (AC-9 and AC-12) to be reachable at all. I recommend accepting the changes as listed, since each one is
mechanical and the output is unchanged. The owner decides.

## 12. Residual limits and owner decisions

1. **macOS conformance is unproven, and the runners' Git is unsupported.** The hosted macOS runner ships Git 2.56.1.
   The version gate accepts only 2.55.0 and 2.38.1, so the macOS matrix and `tests.sh` cannot pass there. The negative
   tests are the only macOS check that can pass, and they are not yet run hosted. **Owner decision:** provide a
   supported Git on macOS (for example a pinned build in CI), or accept Linux-only conformance and record it as an open
   item under G-2 and G0-8.
2. **The hosted CI legs have not run.** Pushing needs authorization, which was not given. AC-3 and AC-4 for CI, and the
   macOS negative tests, remain pending hosted evidence.
3. **bash 3.2 is not tested.** `mapfile` was removed and every script passes `bash -n`, but no bash 3.2 is available here.
4. **The BSD shims are approximations.** `t0005-bsd-shim/` mimics the behaviour each construct depends on. It does not
   prove macOS behaviour.
5. **Wrong-owner roots are not tested.** Refusing a root owned by another user needs a second account. The owner check
   is in the wrapper and is untested.
6. **R3-4 is unchanged.** The ownership marker, the registry and `run-env.json` are editable guard files, not
   attestations. The harness trusts its own process.
7. **Scratch residue from direct runs.** When `tests.sh` or `sp1.sh` is invoked directly, its `sp1r.*` roots stay in the
   caller's base directory. The wrapper cleans its own roots, and the Makefile and CI remove the whole base. AC-2 is
   met, because the writes stay inside the harness's own roots.
8. **`tests.sh` does not run on the runners by design.** P1.4 requires a supported Git, so the 95 assertions run only in
   the pinned image (CI) and on a host with 2.55.0 (local only; the hosted runners in PR #2 had 2.56.1).
9. **Harness output before a cleanup failure.** The wrapper's own `PASS` line is withheld. A PASS line printed by
   `sp1.sh` or `tests.sh` before a later cleanup failure is outside the wrapper's control (`testdata/reference/check-reference.md`).
10. **CI image identity.** CI builds its own image (`ilvuno-git238r:ci`) and prints its ID. That ID is not compared with
    the pinned ID in this note.
11. **Gate log.** `docs/planning/gate-log.md` is corrected under owner instruction 9. It is outside this task's document
    allowlist, and the owner should confirm it.
