# SP-1R2 Final Independent Closure Verification

**Date:** 2026-11-20\
**Scope:** Independent verification of SP-1R2 against A-1 through A-9. This is a harness review, not a production architecture review.\
**Final verdict:** **REJECT**\
**Is SP-1R2 acceptable as the controlled M0 reference harness?** **NO**

The matrix and the 68-assertion suite pass on Git 2.55.0 and 2.38.1. The F10 newline path and blob hash are now correct, and the run-scoped refs and scratch child changes address the original data-loss cases under the stated private-root threat model. Acceptance is still blocked because the independent oracle can execute a configured merge driver from its input repository, accepts unregistered repositories when called directly, and accepts malformed manifest rows. The recorded Git version also is not compared exactly to the version for that run.

## 1. A-1 through A-9 closure table

| Finding | Status | Independent result and remaining bypass |
|---|---|---|
| **A-1 — shell manifest / arbitrary repository** | **CONDITIONALLY CLOSED** | `run-existing` is removed; `read_manifest` parses allowlisted `KEY=VALUE` data without evaluation, and M1 rejected `BASE=$(touch …)` without creating the marker. The direct oracle entry point is a remaining bypass: `oracle.py` accepts a caller-selected scratch tree and fixture path without registration or owner-marker validation, then invokes Git merge operations that execute configured merge drivers (N1). |
| **A-2 — pre-existing refs overwritten/deleted** | **CLOSED** | Run refs use a per-invocation 128-bit ID, create-only `update-ref` calls, and expected-old-SHA deletion. The suite's R1/R1b/R1c/R2 assertions passed: pre-existing `refs/ilvuno/keep` and `refs/ilvuno/runs/pre-existing/candidate` survived, and changing an Ilvuno ref changes the fingerprint. No overwrite/delete bypass was reproduced under the controlled single-process model. |
| **A-3 — newline path content hash** | **CLOSED** | `content_manifest` uses `ls-tree -rz` and hashes blob bytes. An independent Git read found raw path bytes `6c696e650a627265616b2e747874` and SHA-256 `2d27fbdf4e8ca207afbfa388ca9172fbcc6c70e534af2476b3b704f87debadcf`; the F10 result row contains the same path and hash. C-4/C-5 passed. |
| **A-4 — deletion of supplied scratch contents** | **CLOSED** | A supplied scratch path is treated as a parent; the harness creates an exclusive `run-<32 hex>` child and an `O_EXCL` marker. P0.8 sentinels at the previous predictable paths survived, and the symlink-parent test passed. This conclusion assumes the documented fresh private parent and no concurrent same-UID mutator. |
| **A-5 — pipeline-derived or malformed expectations** | **CONDITIONALLY CLOSED** | `expected.py --update` now derives rows from `spec.json` and `oracle.json`, checks the observed matrix against them, and refuses malformed output or a mutated candidate. I reproduced a successful 28-row update and a candidate mutation refusal with no output file. The oracle source is not yet trustworthy as an input boundary: it can execute configured drivers and silently accepts malformed manifests (N1/N2). |
| **A-6 — Git version and policy fields not enforced** | **OPEN** | Unsupported version `0.0.0` is rejected by E9; merge-policy and metadata mutations are rejected by E10/E11. However, changing a 2.55.0 row to the other allowed value, 2.38.1, still passes `expected.py --check` (exit 0). The profile validates membership in an allowlist, not the exact version used for this run (N3). |
| **A-7 — cleanup race and deletion bypasses** | **CONDITIONALLY CLOSED** | The documented M0 model expressly excludes hostile concurrent filesystem mutation, so the prior symlink-swap race is not an M0 blocker here. Static outside-root and symlink-child checks passed. There are still direct `rm -rf` calls in fixture/test setup, and interruption can leave private scratch residue; these paths are acceptable only under the stated controlled fixture model. `SP1_CLEAN=1` also fails to remove its child (N4). |
| **A-8 — traversal and late-attribute negative tests** | **CONDITIONALLY CLOSED** | P0.6 now runs the harness with `SP1_VER=../../victim` and checks the sentinel; SG-4 pins the amended source and the refusal included that source SHA. The SG-4 refusal is also caused by the configured external driver being rejected at preflight, before the new attribute is merged. P0.6m is not a true mutant test: it removes the sentinel and confirms it is absent rather than rerunning the harness assertion under a broken condition. The primary P0.6 assertion itself is meaningful. |
| **A-9 — case-count discrepancy** | **CLOSED** | `spec.json` declares 28 cases, `expected.json` has 28, and the reproduced matrix emitted 28 unique rows. The old 30-case claim is corrected. A separate README assertion-count typo remains: it says 67 while the suite actually reports 68 (N5). |

## 2. Exact verification reproduced

I copied `docs/experiments/scripts/` into the new private directory `/tmp/ilvuno-sp1r2-final.9U5LP1`; tests ran against that copy. The repository's `expected.json` and other source files were not edited. The host reported Git 2.55.0. The commands below use:

```bash
AUDIT_ROOT=/tmp/ilvuno-sp1r2-final.9U5LP1
```

| Command / test | Observed result |
|---|---|
| `SP1_SCRATCH="$AUDIT_ROOT/scratch-host" SP1_GIT_BIN=/usr/bin/git TMPDIR="$AUDIT_ROOT/tmp" bash "$AUDIT_ROOT/scripts/sp1.sh" matrix` | Exit 0; `PASS: 28 cases, all fields match expectations`. The actual JSONL had 28 rows. `spec.json` had 28 declared and actual cases; `expected.json` had 28 cases across 10 fixtures. |
| `SP1_SCRATCH="$AUDIT_ROOT/host-test-parent" SP1_GIT_BIN=/usr/bin/git TMPDIR="$AUDIT_ROOT/tmp" bash "$AUDIT_ROOT/scripts/tests.sh"` | Exit 0; `68 passed, 0 failed`. This includes parent sentinels, refs, hooks, filters, driver refusal, source pins, matrix mutations, and cleanup-failure assertions. |
| `SP1_SCRATCH="$AUDIT_ROOT/false-git-parent" SP1_GIT_BIN=/bin/false TMPDIR="$AUDIT_ROOT/tmp" bash "$AUDIT_ROOT/scripts/sp1.sh" matrix` | Exit 1; the checker reported missing rows. It did not treat the unusable Git executable as success. |
| `python3 "$AUDIT_ROOT/scripts/expected.py" --update "$AUDIT_ROOT/scratch-host/run-6a8c4c527aeae08c946a4d09a03cb6ba/out/all.jsonl" "$AUDIT_ROOT/scripts/oracle.json" "$AUDIT_ROOT/scripts/spec.json" "$AUDIT_ROOT/update-copy.json"` | Recorded 28 rows. After changing one observed candidate SHA to 40 zeroes, the same update command exited 1 with “observed run disagrees with the independent oracle” and did not create the output file. |
| Mutations E1–E11 through the reproduced suite | Candidate SHA/tree/content, base/source pins, verdict, conflict path, cleanup, unsupported Git version, merge policy, and commit metadata mutations were rejected. A supported-version mutation was separately accepted; see N3. |
| F10 independent Git enumeration | NUL-delimited `ls-tree` plus `cat-file blob` produced raw path bytes `6c696e650a627265616b2e747874` and SHA-256 `2d27fbdf4e8ca207afbfa388ca9172fbcc6c70e534af2476b3b704f87debadcf`; both match the emitted F10 object and worktree rows. |
| Git 2.38.1 container | Used cached image `ilvuno-git238r-audit:latest`, image ID `sha256:7536f667e6d345ac43ee9a4056be33e1a53fda74d9c61c1d0630e45a0086fa3a`. `/usr/local/bin/git --version` reported `git version 2.38.1`; matrix passed 28/28 and tests reported `68 passed, 0 failed`. The image was not rebuilt during this audit. |

Git 2.38.1 command used:

```bash
docker run --rm -u 1000:1000 \
  -v "$AUDIT_ROOT/scripts":/sp1r:ro -w /sp1r \
  -e SP1_GIT_BIN=/usr/local/bin/git \
  -e SP1_SCRATCH=/tmp/sp1r2-parent -e TMPDIR=/tmp \
  ilvuno-git238r-audit:latest /bin/bash -lc \
  'mkdir -m700 -p "$SP1_SCRATCH"; /usr/local/bin/git --version; ./sp1.sh matrix && ./tests.sh'
```

The 68 assertions verify the original `SP1_VER` sentinel, scratch-parent preservation, ref preservation, and the specified Git execution controls. The independent oracle was also run in the copied harness. Its execution findings below mean the suite's pipeline-level “driver did not run” checks do not establish that no driver executes anywhere in the harness.

## 3. Independent oracle assessment

The oracle is algorithmically independent of the object-only and worktree pipeline implementations: it runs native Git worktree and sequential merge commands, reads conflict paths with `ls-files -u -z`, and computes content hashes from `ls-tree -rz` records and blob bytes. The expectation builder uses hand-declared verdict/exit/cleanup values from `spec.json` and candidate/tree/content/path values from `oracle.json`; it does not copy the pipeline output into the expected set. F10 is independently correct.

The operational boundary and input validation are not acceptable yet:

1. `oracle.py` invokes direct `git worktree add` and `git merge` without the pipeline's `check_exec_features` gate. The official `sp1.sh oracle` command built F8, then ran its configured `merge.upper.driver`; a private marker set through `DRIVER_MARKER` was created. The oracle reports F8 as `ok` with candidates even though the matrix spec says `harness_refused` and `candidate_from: none`.
2. Direct `oracle.py <scratch> <out> <fixture>...` calls do not verify a scratch ownership marker or that the fixture repository was created and registered by that invocation. In a disposable fake fixture repository with a local `merge.evil.driver` writing a marker, `python3 oracle.py <scratch> <out> F8-driver` exited 0 and created the marker. This is repository-configured shell execution in the same user context; a Git worktree is not a security sandbox.
3. The oracle's `manifest()` splits each ASCII line at the first `=` but does not reject malformed lines, duplicate keys, unknown keys, or missing/extra keys. Appending `NOT_A_KEY_VALUE_LINE` to a valid private fixture manifest and rerunning the oracle exited 0 and wrote an oracle file. This fails the requested fail-closed behavior.

The direct Git operations are useful as an independent semantic reference for clean and conflicting merges. They are not safe to run on an arbitrary repository until the oracle refuses executable Git extensions and validates its fixture ownership and input schema.

## 4. New confirmed blockers

### N1 — The independent oracle executes configured Git extensions

**Severity:** Blocker\
**Required milestone:** M0 harness gate

**Evidence and reproduction:** In the private copy, this command executed the configured F8 merge driver and created the marker:

```bash
DRIVER_MARKER="$AUDIT_ROOT/oracle-driver.marker" \
SP1_SCRATCH="$AUDIT_ROOT/oracle-parent" SP1_GIT_BIN=/usr/bin/git \
TMPDIR="$AUDIT_ROOT/tmp" bash "$AUDIT_ROOT/scripts/sp1.sh" oracle
```

Separately, I created a disposable repository under `fixtures-F8-driver/repo` with a conflicting file, `.gitattributes` selecting `merge=evil`, a local `merge.evil.driver` that writes a marker, and valid fixture SHAs. Calling `python3 "$AUDIT_ROOT/scripts/oracle.py" "$R" "$R/oracle-out.json" F8-driver` returned 0 and created the marker.

**Impact:** Oracle generation executes commands selected by repository attributes/configuration. Calling the Python script with a caller-controlled scratch tree can run project-configured code outside the pipeline's refusal gate. This contradicts the harness's controlled-execution claim and prevents treating generated oracle output as safe evidence.

**Correction and verification:** Make oracle inputs invocation-owned and fixture-name allowlisted; validate the ownership marker and repository registration; reject external drivers, filters, fsmonitor, and other executable Git features before checkout or merge. The F8 oracle case should be represented as refused without attempting its merge. Re-run the malicious-driver and hook/filter fixtures and require nonzero refusal with no marker.

### N2 — Oracle accepts malformed fixture manifests

**Severity:** High\
**Required milestone:** M0 harness gate

**Evidence and reproduction:** Append the line `NOT_A_KEY_VALUE_LINE` to `$AUDIT_ROOT/oracle-injected-input/fixtures-F8-driver/repo/.git/sp1-meta.env`, then run:

```bash
python3 "$AUDIT_ROOT/scripts/oracle.py" \
  "$AUDIT_ROOT/oracle-injected-input" \
  "$AUDIT_ROOT/oracle-injected-input/malformed-oracle-out.json" F8-driver
```

It exited 0 and emitted output because `manifest()` silently stores the malformed row and uses the remaining `BASE`/`SRCS` keys.

**Impact:** Oracle results can be generated from inputs the fixture pipeline itself rejects, so independent expectations may be built from malformed or ambiguous pins.

**Correction and verification:** Validate the complete manifest format, keys, uniqueness, full object IDs, source count, and source/base constraints before any Git operation. Verify malformed, duplicate, unknown, missing, and truncated manifest cases all return nonzero and write no output.

### N3 — Version checker accepts a supported but incorrect version label

**Severity:** Medium\
**Required milestone:** M0 reproducibility gate

**Evidence and reproduction:** On a 2.55.0 matrix, change one row's `git_version` to `2.38.1` and run `expected.py --check`. It returned 0 and passed all 28 rows. `expected.py` stores `2.55.0` in the expected row but intentionally skips comparing that field, accepting either profile version.

**Impact:** A result file can claim the other supported Git version without detection. The independent command log still records which binary was run, but the checker does not bind each row to that exact execution as requested.

**Correction and verification:** Pass the expected runtime version into the checker or create version-specific profiles, then prove both allowed versions pass in their own runs and that cross-labeling either version fails.

## 5. Remaining limitations by milestone

| Classification | Item |
|---|---|
| **M0 blocker** | N1 and N2: oracle execution and malformed-input boundaries. N3: exact runtime-version labeling is not enforced. |
| **M0 documented limitation** | `safe_rmrf` remains check-then-delete and is not safe against a concurrent same-UID filesystem mutator. This audit accepts that limitation for one process per freshly generated private fixture root, as directed. Interrupted runs may leave that private root and worktree/ref residue for inspection; the harness has no signal-safe recovery protocol. |
| **M0 cleanup limitation** | `SP1_CLEAN=1 SP1_SCRATCH="$AUDIT_ROOT/clean-parent" TMPDIR="$AUDIT_ROOT/tmp" bash "$AUDIT_ROOT/scripts/sp1.sh" invalid` exited 2 and left one `run-*` child. `cleanup_scratch` passes the run root to `safe_rmrf`, which accepts only a strict child of that root, and suppresses the refusal. This leaks private scratch data but did not affect parent sentinels. |
| **M2 blocker** | Before production use, arbitrary project verification commands need OS-level containment; worktrees do not provide that isolation. Production also needs explicit concurrency and crash-recovery behavior and hostile/shared-filesystem cleanup guarantees. These requirements do not independently block a controlled harness once N1–N3 are closed. |
| **Release gate / future support** | This audit ran Linux with Git 2.55.0 and a cached Git 2.38.1 image; the latter was not rebuilt here. Verify pinned-image provenance before using the image as release evidence. macOS/Windows and broader Git/repository edge support remain unverified. |
| **Documentation** | `scripts/README.md` says 67 assertions; both independent runs report 68. The matrix count itself is correctly 28. |

## 6. Final decision

**YES/NO — Is SP-1R2 acceptable for use as the controlled M0 reference harness?** **NO.**

**Final verdict: REJECT.** The core matrix, ref preservation, scratch-parent sentinels, and byte-exact F10 result reproduced successfully. The oracle is part of the evidence path and presently executes configured Git drivers, accepts unregistered input repositories, and accepts malformed manifests. The Git-version checker also does not bind a row to the exact version used. Close N1–N3 and repeat this gate before treating SP-1R2 as an M0 reference harness.
