# SP-1R Independent Closure Audit

**Date:** 2026-11-20  
**Scope:** SP-1R harness remediation only; no production engine review beyond its use by this harness.  
**Final verdict:** **REJECT**  
**Is SP-1R technically acceptable as a controlled M0 reference harness, assuming the owner separately authorizes M0?** **NO — not in its current form.**

The fixture matrix, security controls, and Git 2.38.1/2.55.0 runs substantially improve SP-1. I reproduced the reported matrix and tests, and independently checked the clean candidate commits and conflict paths. The remaining M0 blockers are in the harness boundary and evidence: an exposed command executes shell from repository metadata and can destroy pre-existing Ilvuno refs; the newline fixture passes with an incorrect content hash; and the scratch directory can delete pre-existing files with matching harness paths. These do not require an M0 production sandbox to fix.

## 1. Exact tests reproduced

All runs used a copied `docs/experiments/scripts/` tree and private directories under `/tmp/ilvuno-sp1r-closure.GBA3Tf`; no source `expected.json` was changed.

| Test | Environment and command | Observed result |
|---|---|---|
| Fixture build | `SP1_SCRATCH=<private mode-700 root> SP1_GIT_BIN=/usr/bin/git ./sp1.sh fixtures` | 10 fixture repositories built. |
| Matrix | Host `/usr/bin/git`, `git version 2.55.0`; `./sp1.sh matrix` | Exit 0; `PASS: 28 cases, all fields match expectations`. Output contained 28 unique case keys. |
| Negative/security suite | Host Git 2.55.0; `./tests.sh` | `38 passed, 0 failed`. |
| Matrix and suite at floor | Rebuilt `ilvuno-git238r-audit` from the checked-in Dockerfile with `docker build --no-cache`; Git source archive passed the Dockerfile SHA-256 check. Runtime reported `git version 2.38.1`; then `./sp1.sh matrix && ./tests.sh`. | Matrix passed 28 cases; suite reported `38 passed, 0 failed`. |
| False Git executable | `SP1_GIT_BIN=/bin/false ./sp1.sh matrix` in a new private scratch root | Exit 1; missing result rows were reported. |
| Original `SP1_VER` traversal | Set `SP1_VER=../../victim`; created a sentinel at the former traversal destination in the disposable harness copy; ran `./sp1.sh run F1-two-clean object plain`. | Exit 0; sentinel remained. `SP1_VER` is no longer used in path construction. |
| Expected-field mutations | Independently changed candidate SHA, verdict, conflict path, base pin, source pin, and cleanup state in copied JSONL output, then ran `expected.py --check`. | All six mutations were rejected with exit 1. Changing only `git_version` was accepted with exit 0. |
| Git-native clean oracle | Reconstructed the sequential candidate with direct `git worktree add` and `git merge --no-ff` outside the SP-1R functions for F1, F2, F5, F7, and F9. | Candidate SHA and tree matched the expected row in all five cases. Independently hashed tree contents matched the manifest for those cases. |
| Git-native conflict oracle | Repeated F3, F4, and F10 with direct Git merges; read unmerged paths with `git ls-files -u -z`. | Conflict step and raw conflict path bytes matched the result rows in all three cases, including F10's newline path. |
| Non-UTF-8 conflict path | Constructed a conflict on byte path `bad-\xff.txt`; ran object and worktree variants with Git 2.55.0, and object variant with Git 2.38.1. | Conflict path round-tripped to bytes `6261642dff2e747874` in all runs. The JSON uses a surrogate escape; this is lossless with Python `surrogateescape`, though interoperability is limited. |
| Hooks, config injection, drivers, filters, attributes, submodule | Re-ran `tests.sh` on both versions; also used a correctly pinned first source commit that adds `.gitattributes` and verified a configured external driver is refused before merge. | Marker hooks/configured driver/filter did not execute; runtime config injection did not affect Git; late-attribute fixture was refused with exit 4; submodule remained uninitialized. |
| Conflict classification | Injected a Git wrapper that returned 1 on `merge` without producing merge state; separately configured an exit-42 pre-merge hook against a real F3 conflict. | No-state failure classified `operational_error`, not `conflict`; the configured hook did not execute and the real conflict remained `conflict` on `shared.txt`. |
| Orphan cleanup | Ran `./sp1.sh probes` and the F9 `fail-cleanup` matrix variant. | P3 reproduced `worktree remove` rc 255, no admin entry, and a remaining directory. The pipeline removed its owned orphan, reported `orphaned_dir_removed`, exit 5, zero refs, and zero registered worktrees. |
| Symlink-swap race | Paused `safe_rmrf` after validation at its `find` step, swapped an in-root parent directory for a symlink to a private outside target, then resumed. | `safe_rmrf` returned 0 and deleted the outside sentinel directory. Payload stayed inside the audit's private `/tmp` root. |
| External `run-existing` target | Ran the exposed command on a disposable repository outside `SP1_SCRATCH` with pre-existing `refs/ilvuno/candidates/r` and `refs/ilvuno/meta/r`. | Both refs were gone afterward while output said `user_state_unchanged:"yes"`. A separate `.git/sp1-meta.env` payload ran shell code and the command still returned success. |
| Reused scratch contents | Placed sentinels in the exact `runs/F1-two-clean-object-plain/` and `out/F1-two-clean-object-plain/` paths under a private supplied scratch root, then ran the fixture. | Both sentinels were deleted. The harness accepts a non-empty root and treats matching paths as disposable. |

One initial container attempt set `TMPDIR` inside `SP1_SCRATCH`; the suite's victim then fell inside the root that P0.2 was supposed to treat as external, so three checks failed. I corrected the test setup to use the container's private `/tmp` outside the scratch root and reran successfully. This was an audit invocation error, not a source change.

## 2. Findings closed with evidence

- **Failing Git no longer gives a false pass:** the false executable produced exit 1 and missing-row failures.
- **Matrix row and core result checks work:** 28 rows exist and match the 28 entries in `expected.json`. Changing candidate, result class, conflict path, base/source pins, or cleanup state makes the checker fail.
- **Core clean candidate behavior is reproducible:** direct native Git merges reproduced the expected candidate SHA/tree for F1, F2, F5, F7, and F9 on the host Git.
- **Conflict classification is materially improved:** direct Git conflict checks matched F3/F4/F10; an injected no-state merge failure was classified operationally.
- **Git execution controls work for the tested fixtures:** hooks are redirected, runtime `GIT_CONFIG_*` does not reach child Git, and configured filters/drivers are refused on both tested versions.
- **Static path attacks are closed:** the old `SP1_VER` traversal and ordinary symlink-root/child attempts did not escape the private scratch root.
- **Orphan handling is observable:** the P3 failure is detected and the pipeline reports its cleanup result rather than silently claiming success.
- **Non-UTF-8 conflict names are preserved by the conflict-path path:** bytes round-trip through the JSON representation under Python's surrogate escape handling.

## 3. Remaining failures and issues

### A-1 — `run-existing` executes repository-controlled shell metadata

- **Severity:** Blocker
- **Evidence:** `run_object` and `run_worktree` source `"$repo/.git/sp1-meta.env"` (`sp1.sh:99,197`). The `run-existing` command accepts a caller-supplied repository path without checking that it is under the private scratch root (`sp1.sh:338-345`). This source happens before `check_exec_features` and outside `gitx`'s `env -i` boundary.
- **Reproduction:** In a disposable repository outside the scratch root, append a shell command that writes a marker to `.git/sp1-meta.env`; invoke `SP1_SCRATCH=<private root> ./sp1.sh run-existing <repo> object plain <label>`. The marker was created; the command returned 0 and reported `verdict:"ok"`.
- **Impact:** The harness can execute arbitrary shell code from a repository before its Git extension checks. This contradicts the claim that it only executes harness-owned code and is an unexpected-process execution path.
- **Recommended correction:** Remove `run-existing` from the command interface or constrain it to freshly generated, harness-owned repositories beneath the scratch root. Never `source` repository data; parse a strict, non-executable manifest written by the fixture builder and validate each SHA and field.
- **Required milestone:** M0 harness gate. This requires a narrow harness boundary, not a production sandbox.

### A-2 — Existing `refs/ilvuno/*` can be overwritten and deleted while the fingerprint says unchanged

- **Severity:** Blocker
- **Evidence:** The fingerprint intentionally excludes every `refs/ilvuno/*` ref (`lib.sh:169-181`). Success paths update fixed `refs/ilvuno/candidates/r` and `refs/ilvuno/meta/r`, then delete them if the candidate ref still equals the newly computed SHA (`sp1.sh:144-168`, `249-269`). There is no compare-and-swap against the prior value.
- **Reproduction:** On a disposable repository outside scratch, set both refs to its base commit, run `run-existing` on a clean fixture. Both original refs disappeared. The result reported `user_state_unchanged:"yes"`.
- **Impact:** A caller can lose pre-existing refs, and the safety record hides the loss. This also falsifies the scratch-only write claim for `run-existing`.
- **Recommended correction:** Keep existing-repository execution out of the supported harness interface, or require a fresh scratch-owned repository and unique per-run ref names. Use compare-and-swap updates and delete only refs created by that run; include prior values in the safety fingerprint.
- **Required milestone:** M0 harness gate.

### A-3 — F10 passes with an incorrect candidate content manifest

- **Severity:** Blocker
- **Evidence:** `content_manifest` parses `git ls-tree -r --name-only` line-by-line (`sp1.sh:88-94`). For F10's path `line<LF>break.txt`, Git emits a quoted display form; the result row records `"line\\nbreak.txt"` with SHA-256 `e3b0…` (empty content). An independent NUL-safe `git ls-tree -rz` plus `cat-file blob` check found the actual path bytes `line<LF>break.txt` and content SHA-256 `2d27fbdf4e8ca207afbfa388ca9172fbcc6c70e534af2476b3b704f87debadcf`. The matrix nevertheless passes because the wrong value is in `expected.json` too.
- **Reproduction:** Run F10 object/plain, inspect its `content` row, then enumerate the candidate tree using `git ls-tree -rz` and hash the blob bytes. The path and hash differ from the row.
- **Impact:** The asserted content-hash evidence is false for filenames containing LF. A green matrix does not prove candidate content for this supported edge case.
- **Recommended correction:** Preserve NUL-delimited tree records and hash blob object contents without converting path bytes through line-oriented output. Recreate the F10 expected value from an independent tree traversal; do not regenerate the oracle from the faulty manifest.
- **Required milestone:** M0 harness gate.

### A-4 — A supplied scratch root may contain unrelated data that the harness deletes

- **Severity:** High
- **Evidence:** `init_scratch` accepts an existing owned directory, chmods it to mode 700 if needed, and overwrites its `owner` marker without requiring an empty root or validating an existing ownership marker (`lib.sh:19-41`). Each run then recursively removes predictable `runs/<label>` and `out/<label>` paths (`sp1.sh:296-302`).
- **Reproduction:** Put sentinel files under those exact paths in a new private mode-700 `SP1_SCRATCH`, then run the matching fixture. Both files were deleted.
- **Impact:** A caller can select a private directory that is not disposable scratch and lose files with matching names.
- **Recommended correction:** Use an automatically created unique root by default. For caller-supplied roots, require an explicit harness ownership marker and create a unique run subdirectory; refuse non-empty unowned roots. Document the scratch root as disposable only after enforcing that contract.
- **Required milestone:** M0 harness gate.

### A-5 — Expected values are not fully independent, and `--update` accepts malformed input

- **Severity:** High
- **Evidence:** The only built-in update path projects result rows directly into `expected.json` (`expected.py:50-71`); it does not compare to an independent fixture oracle. In a copied tree, `expected.py --update` on `{not json}` returned 0 and wrote an empty case set because `--update` ignores `load()` errors. The docs say to review the diff, but the tool itself permits laundering a failed run. Independent Git checks in this audit verified five clean candidate SHA/tree pairs and three conflict step/path pairs; they do not validate every expected field. F10 demonstrates a bad snapshot can be self-consistent.
- **Reproduction:** Run `python3 expected.py --update malformed.jsonl <temporary-output.json>` with a malformed-only JSONL file; observed exit 0 and zero cases. The repository's `expected.json` was not changed.
- **Impact:** The expectation set is a golden snapshot, not a self-contained statement of intended behavior. Negative checker tests prove mismatch detection, not that every stored value is correct.
- **Recommended correction:** Make update mode reject parse errors, missing/extra rows, and any count other than the declared matrix cardinality. Maintain independent expected trees/paths/content for fixtures or a separately reviewed specification that the update command cannot derive from the tested output.
- **Required milestone:** M0 harness gate.

### A-6 — The checker accepts a wrong Git version and policy metadata

- **Severity:** Medium
- **Evidence:** `git_version` is required in `expected.py`, but is omitted from `COMPARED`; `os`, `merge_policy`, `metadata`, `steps`, and `operational_detail` are not all required/compared either (`expected.py:12-18,50-59`). Changing a matrix row's `git_version` to `0.0.0` still produced `PASS: 28 cases`.
- **Reproduction:** Change only `git_version` in copied `all.jsonl`; run `expected.py --check` against copied `expected.json`. It exits 0.
- **Impact:** The matrix does not enforce the exact runtime Git version or recorded policy, even though the reports describe these as reproducibility evidence. This audit separately verified host Git 2.55.0 and the rebuilt Git 2.38.1 binary.
- **Recommended correction:** Put the intended version/OS/policy/metadata fields in the expected schema and compare them, or make the test runner assert the exact binary version before running the matrix.
- **Required milestone:** M0 reproducibility gate.

### A-7 — `safe_rmrf` and other recursive deletes are not race-safe

- **Severity:** High for a hostile concurrent writer; Medium under the controlled M0 fixture threat model
- **Evidence:** `safe_rmrf` validates path components and ownership, then separately runs `find`/`chmod` and `rm -rf` (`lib.sh:63-85`). I paused the `find` command after validation, swapped the checked parent for a symlink to an outside test directory, and resumed; the outside sentinel was deleted. In addition, `sp1.sh:20` and multiple paths in `tests.sh` use direct `rm -rf`, bypassing `safe_rmrf`.
- **Reproduction:** Use a private mode-700 scratch root with `scratch/a/victim`, pause `safe_rmrf scratch/a/victim` at its `find` step, rename `scratch/a`, replace it with a symlink to a private outside directory containing `victim/sentinel`, then resume. `safe_rmrf` returned success and deleted the sentinel directory.
- **Impact:** The helper does not establish arbitrary hostile-filesystem safety. Another process running as the same UID and able to mutate the private scratch path can redirect deletion between validation and use.
- **Recommended correction:** For M0, use an automatically created per-invocation root, one harness process per root, and explicitly exclude concurrent same-UID mutation; route every recursive deletion through the guarded helper. Before M2 use descriptor-relative/no-follow cleanup or OS-level filesystem isolation where the path may be hostile.
- **Required milestone:** M0 operational condition for isolated fixtures; M2 before hostile or shared filesystem use.

### A-8 — Two automated negative tests overstate what they exercise

- **Severity:** Medium
- **Evidence:** P0.6 only checks that the sentinel from P0.2 still exists (`tests.sh:39`); it does not invoke the harness with `SP1_VER=../../victim`. The manual traversal probe in this audit did pass. SG-4 amends branch `s1` with `.gitattributes` but does not update the `SRCS` pin stored earlier by `mk_two` (`tests.sh:70-75`), so that test's selected input does not actually include the amended commit. A correctly pinned late-attributes fixture was separately refused by the config gate in this audit.
- **Reproduction:** Inspect those test lines; then run with a `SP1_VER` environment value and compare the marker, and amend the SG-4 fixture's `SRCS` to the new commit ID. The current SG-4 setup refuses because the driver is configured, not because the pinned source contains the attribute.
- **Impact:** The remediation report attributes stronger coverage to these assertions than they provide. The implementation behavior is safe in the manual traversal/late-attribute probes, but future regressions in those exact paths could go undetected.
- **Recommended correction:** Make P0.6 invoke the harness with the legacy value and assert a sentinel remains. Update SG-4's metadata to pin the commit that adds the attribute, then assert refusal and no driver marker.
- **Required milestone:** M0 test-quality gate.

### A-9 — Case count is inconsistent across remediation artifacts

- **Severity:** Low
- **Evidence:** The script enumerates 10 fixtures × 2 approaches for 20 plain rows, then 4 variant pairs × 2 approaches for 8 rows (`sp1.sh:10-13,309-329`). `expected.json` has 28 keys and both real matrix runs emitted 28 unique rows. `sp1r-regression-matrix.md` states 28, while the remediation report and README say 30 (`sp1r-remediation-report.md:16,93-95`; `scripts/README.md:10`).
- **Reproduction:** Count the `FIXTURES` list and loops above; run the matrix and count unique `(fixture, approach, variant)` keys.
- **Impact:** The code and checked-in expected set agree at 28; “30” is a stale evidence/documentation claim and reduces auditability.
- **Recommended correction:** Correct the remediation report/README to 28 and make the checker assert the expected cardinality explicitly.
- **Required milestone:** M0 documentation gate.

## 4. Fixture-count reconciliation

The executable matrix has **10 fixtures**. It runs the plain variant for each fixture on object and worktree approaches (**20 rows**), then runs four selected variants (`fail-cleanup`, `stop-after1`, `index-lock`, `keep-conflict`) on both approaches (**8 rows**). Total: **28 rows**. I independently counted 10 fixture repositories from `./sp1.sh fixtures`, 28 actual JSONL rows, 28 unique keys, and 28 `expected.json` cases. There is no thirtieth case in the current matrix. The remediation report's “30 cases” and false-Git “30 missing rows” text is stale; the regression matrix and expected file reflect 28.

## 5. Security boundary assessment

The environment wrapper, command-scoped empty hooks path, driver/filter refusal, submodule setting, and conflict-state classifier passed the tested cases on Git 2.55.0 and 2.38.1. Static symlink and traversal attempts were also rejected. These are meaningful harness controls.

The harness is **not** safe against an arbitrary repository passed to `run-existing`: it evaluates repository metadata as shell and can mutate namespaced refs. The harness also does not prove hostile-filesystem safety: a same-UID concurrent symlink swap bypasses `safe_rmrf`'s check/use sequence. These are separable from a production sandbox. M0 can remain a fixture-only milestone if it restricts all inputs to harness-created repositories under an automatically created private scratch root, runs one harness per root, and invokes no project verification commands. Do not use this harness to process arbitrary repositories until the boundary is fixed.

## 6. Open issues by milestone

| Milestone | Required disposition |
|---|---|
| **M0 blockers** | Close A-1/A-2 by restricting or removing `run-existing`, parsing only trusted fixture metadata, and preserving pre-existing refs; fix A-3 with NUL-safe content hashing; fix A-4 with owned unique scratch roots; add independent expected behavior and make update mode fail closed (A-5); assert Git version/policy (A-6); repair misleading P0.6/SG-4 tests and correct the row count (A-8/A-9). These are harness-only changes. |
| **M0 operational condition** | Use one process per fresh private mode-700 scratch root and do not allow a concurrent same-UID process to mutate it. The demonstrated `safe_rmrf` race can be deferred under this explicit controlled fixture threat model; do not claim general filesystem safety. |
| **M2 production blockers** | Do not treat this harness audit as approval of production B. Resolve product policy for hooks, filters, merge drivers, attributes, submodules, run-owned refs, scratch layout, arbitrary verification commands, process containment, concurrency, crash recovery, and supported Git/platform matrix. Use OS-level containment before executing untrusted project commands. |
| **Release blockers** | Establish supported Git/OS behavior beyond the two tested Linux builds; authenticate/attest the Git floor image inputs if they become release evidence; define strict JSON handling for non-UTF-8 path bytes; and complete the broader edge corpus for LFS, shallow/alternate repositories, case-only renames, and metadata variation. |
| **Deferred support limitations** | macOS/Windows, broad merge-option/config fidelity, unsupported custom executable Git features, and strict non-UTF-8 JSON consumers can remain unsupported for M0 if stated explicitly. Non-UTF-8 path bytes did round-trip in the tested Python harness. |

## 7. Final verdict

**REJECT SP-1R as currently closed.** The 28-row matrix and 38 assertions pass on Git 2.55.0 and a separately rebuilt Git 2.38.1 image, and much of the original SP-1 harness evidence is now reproducible. However, the test-only repository entry point still enables shell execution and ref loss, and the F10 content assertion is demonstrably wrong. A valid, independently trustworthy M0 reference harness requires those harness-only blockers to be closed first.

**YES/NO answer: NO.** Under a controlled fixture threat model, a production security sandbox is not needed to fix these M0 blockers. Restrict the harness to its own private fixtures, correct the content oracle, and protect user data/refs; then the M0 harness can be reassessed without expanding M0 into production security infrastructure.
