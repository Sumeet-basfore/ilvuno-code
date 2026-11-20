# SP-1R3 — Oracle Boundary Closure Evidence

**Date:** 2026-11-20
**Scope:** harness-only closure of the SP-1R2 rejection: N1 (oracle execution boundary), N2 (oracle
manifest validation), N3 (Git-version binding), the `SP1_CLEAN` scratch leak, and the README
assertion count. No production code, product docs, ADRs, owner decisions, or global Git config were
changed. Nothing pushed; M0 not started.

**Status reported: COMPLETE for this revision, pending targeted independent verification.**
No self-approval is issued.

---

## 1. Exact files changed

```
docs/experiments/scripts/schema.py        NEW   strict manifest schema, shared by harness and oracle
docs/experiments/scripts/oracle.py        rewritten: ownership+registry boundary, controlled Git env,
                                                executable-feature refusal, atomic output, no F8 execution
docs/experiments/scripts/expected.py      run-env binding; exact per-row Git version; spec+oracle builder
docs/experiments/scripts/lib.sh           read_manifest delegates to schema.py; remove_run_root cleanup
docs/experiments/scripts/sp1.sh           oracle registry; run-env sidecar written before the matrix;
                                          matrix passes run-env to expected.py
docs/experiments/scripts/tests.sh         +27 assertions (N1 boundary, N2 manifest classes, N3 binding,
                                          cleanup) → 95 total
docs/experiments/scripts/oracle.json      regenerated: F8 recorded as `refused`, no driver execution
docs/experiments/scripts/expected.json    regenerated from spec+oracle; per-case git_version removed
docs/experiments/scripts/README.md        assertion count 68 → 95
```

No matrix cases were added or removed (still 28). No expectation was regenerated to make a failing
test pass; `expected.json` was rebuilt only after the oracle refused F8 and produced byte-identical
output on repeat runs.

---

## 2. Disposition

| Item | Disposition | Regression tests |
|---|---|---|
| **N1** oracle executes configured Git extensions; accepts arbitrary repositories | **Closed.** Oracle requires the run-owned scratch marker (`format`, `uid`, `id` = `SP1_RUN_ID`, `parent` relation), a registry written by the same invocation, allowlisted fixture names, and repo realpaths under the scratch root. Every Git call uses a sanitised environment with hooks/attributes/signing/submodules neutralised; executable config is refused before checkout and before each merge. Refused fixtures are recorded without running the merge. | N1a–N1i (9) |
| **N2** malformed manifests accepted | **Closed.** `schema.py` is the single validator: allowlisted keys, no blanks, no unknown keys, no duplicates, `FORMAT=1`, full 40/64-hex OIDs, 2–3 unique sources, none equal to base, optional 64-hex digest. `read_manifest` delegates to it; the oracle exits nonzero and writes no output. | N2.1–N2.9 (9) |
| **N3** supported-but-wrong version accepted | **Closed.** `sp1.sh matrix` detects `git --version`, writes `out/run-env.json`, and the checker requires every row's `git_version` and `os` to equal that run's recorded values. The allowed-version profile is still checked separately. | E12, N3.1b, N3.2 (and existing E9) |
| **Cleanup leak** `SP1_CLEAN=1` left a child | **Closed.** `remove_run_root` verifies the marker's format, uid, id and parent relation, then deletes exactly that root (a dedicated root-level path, not `safe_rmrf`'s strict-child rule). Failures print `cleanup: FAILED …` and force exit 2; success prints `cleanup: removed …`. | N4.1–N4.6 (6) |
| **README count** | **Closed.** 68 → **95** (the independently counted total after the targeted additions). | — |

### N1 details

- `oracle.py` no longer accepts a caller-selected repository: fixture names are allowlisted and each
  repo must match `oracle-repos.tsv` written by `sp1.sh oracle` in the same invocation.
- The ownership marker must be present, owned by the current uid, and carry the same id as
  `SP1_RUN_ID`; the root must be `parent` or `parent/run-<id>`.
- Refusal reasons are recorded per fixture (`status: "refused"`, `candidates: []`); **no worktree is
  created and no merge is attempted** for F8. Its expected rows come from `spec.json`
  (`candidate_from: none`).
- The merge algorithm is unchanged and remains independent: native `git worktree add --detach` plus
  sequential `git merge --no-ff` with `ls-files -u -z` for conflict paths and `ls-tree -rz` +
  `cat-file blob` for content.

### N3 details

- `expected.py --check <rows> <expected> <run-env>` is now a required three-argument contract.
  Missing run-env fails.
- `run-env.json` is produced by the harness from the binary it actually executes:
  `{"git_version": "2.55.0", "os": "Linux"}` (or `2.38.1` in the container).
- Cross-labelling either supported version is rejected:
  `git_version: run executed '2.55.0', row says '2.38.1'`.

---

## 3. Before / after reproductions

Reproduced with the SP-1R2 code, then with the patch, in private `/tmp` scratch roots.

| Defect | Before | After |
|---|---|---|
| **N1** oracle + configured driver | `./sp1.sh oracle` exit 0; `driver.marker` created; F8 got candidates | exit 0, **marker absent**, F8 `status: refused`, `candidates: 0`, reason `custom merge driver: merge.upper.driver` |
| **N2** malformed manifest | appending `NOT_A_KEY_VALUE_LINE` to a fixture manifest: oracle exit **0**, wrote 2,637-byte output | oracle exit **2**, **no output file** |
| **N3** cross-labelled version | one row changed 2.55.0 → 2.38.1: checker **PASS** | checker **FAIL** — `run executed '2.55.0', row says '2.38.1'` |
| **Cleanup** | `SP1_CLEAN=1 … ./sp1.sh invalid` exit 2, **1 child left**, no cleanup message | exit 2, **0 children left**, `cleanup: removed …`; parent sentinel intact |
| **F10** | `2d27fbdf…` (already correct in R2) | unchanged: `line\nbreak.txt=2d27fbdf4e8ca207afbfa388ca9172fbcc6c70e534af2476b3b704f87debadcf` |

Refusal proof: with `DRIVER_MARKER` exported, the oracle run creates no marker file. A second check
runs the oracle with `GIT_CONFIG_COUNT/KEY/VALUE` pointing `core.hooksPath` at a marker hook and a
repository-configured hook path; the hook marker stays absent.

---

## 4. Test commands and results

Host — Git **2.55.0**, Linux:

```
SP1_SCRATCH=/tmp/r3final ./sp1.sh matrix
  PASS: 28 cases, all fields match expectations        exit 0
SP1_SCRATCH=/tmp/r3final ./tests.sh
  95 passed, 0 failed                                  exit 0
SP1_GIT_BIN=/bin/false SP1_SCRATCH=… ./sp1.sh matrix
  exit 1, 28 missing rows
DRIVER_MARKER=… SP1_SCRATCH=… ./sp1.sh oracle
  exit 0; F8 refused; driver marker absent
```

Floor — Git **2.38.1** (pinned Alpine image `ilvuno-git238r`, `/usr/local/bin/git`):

```
./sp1.sh matrix   PASS: 28 cases                     exit 0
./tests.sh        95 passed, 0 failed                exit 0
```

Test groups (95): path/scratch 12, manifest 10, security 21, content 2, refs 4, repeat 3,
matrix enforcement 5, expectation mutations 12, oracle boundary 9, oracle manifest classes 9,
version binding 2, cleanup 6.

Matrix distribution unchanged: ok 12, conflict 8, stopped 2, unrelated_histories 2,
check_failed 2, harness_refused 2.

---

## 5. Remaining limitations

1. `safe_rmrf` remains check-then-delete; the M0 threat model is one process per fresh private root
   with no concurrent same-UID adversary. Not hostile-filesystem safe; a production gate remains.
2. `spec.json` is hand-declared behaviour; review of that file is the control, not proof.
3. Non-UTF-8 paths use JSON surrogate escapes; strict-UTF-8 consumers must handle them.
4. Two Git versions and Linux only. macOS/Windows unverified.
5. `oracle.json` is written into the scripts directory by `sp1.sh oracle`; regenerating it in a
   checkout produces a diff that must be reviewed.
6. Interrupted runs can still leave their own private scratch root; cleanup on exit is implemented,
   but there is no signal-safe recovery protocol (documented M0 limitation).

---

## 6. Readiness

**Ready for targeted independent verification of N1, N2, N3 and cleanup.** The previous verdict
reproduced three oracle/version defects; each now has a failing-before/passing-after regression,
and the matrix/F10 evidence is unchanged. This document does not assert acceptance or approval.

Remaining M0 blockers outside this patch: owner decisions D-01…D-06 and the AGENTS/checklist status
conflict. Remaining M2 blockers unchanged: executable-feature policy for the product, run-owned refs
and recovery, OS-level containment for project commands, concurrency, platform matrix, image
attestation, edge corpus.
