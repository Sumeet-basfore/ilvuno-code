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
cd docs/experiments/scripts && docker build --no-cache -f Dockerfile.git238 -t ilvuno-git238r:rebuild-2026-11-20 <empty-context>
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

## 8. T-0005 run (2026-11-20)

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
