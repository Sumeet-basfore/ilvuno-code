# SP-1R3 Targeted Final Closure Audit

**Audit date:** 2026-11-20  
**Scope:** N1 oracle boundary, N2 manifest validation, N3 Git-version binding, and scratch cleanup.  
**Final verdict:** **APPROVE WITH CONDITIONS**  
**Is SP-1R3 acceptable as a controlled M0 reference harness?** **YES**, under the stated M0 trust boundary and the conditions below.

## Summary

The 28-case matrix and 95-assertion regression/security suite passed on Git 2.55.0 and Git 2.38.1. The targeted malformed-input, feature-refusal, version-label, failing-Git, and cleanup checks reproduced. I found no confirmed bypass that executes a repository-configured driver, filter, or hook during the tested oracle runs.

The closure is conditional because the ownership marker, fixture registry, and run-environment sidecar are ordinary editable files. They establish process-level consistency when the harness is trusted; they are not authenticated attestations against a same-UID editor. A failed oracle invocation also leaves a pre-existing output file untouched, and two exceptional cleanup paths can leave private scratch residue. Those limits do not block use with fresh canonical private scratch, checked exit codes, and no concurrent same-UID attacker.

## Verification environment and commands

All tests ran against a private copy at `/tmp/ilvuno-sp1r3-audit.SPP2Cn/scripts`; fixtures and output were under `/tmp/ilvuno-sp1r3-audit.SPP2Cn`. No untrusted fixture was executed outside that private tree. The workspace copies of `spec.json`, `expected.json`, and `oracle.json` were not changed.

The source test runner contains an unconditional `rm -rf /tmp/r3-cl`; that path already existed. To keep the run isolated, I changed only that literal path in the temporary test copy to `$T/r3-cl`. Assertions and expectations were unchanged.

Host Git 2.55.0:

```sh
SP1_SCRATCH=/tmp/ilvuno-sp1r3-audit.SPP2Cn/host-matrix-parent \
SP1_GIT_BIN=/usr/bin/git TMPDIR=/tmp/ilvuno-sp1r3-audit.SPP2Cn/tmp \
bash /tmp/ilvuno-sp1r3-audit.SPP2Cn/scripts/sp1.sh matrix
# PASS: 28 cases, all fields match expectations; exit 0

SP1_SCRATCH=/tmp/ilvuno-sp1r3-audit.SPP2Cn/host-tests-parent \
SP1_GIT_BIN=/usr/bin/git TMPDIR=/tmp/ilvuno-sp1r3-audit.SPP2Cn/tmp \
bash /tmp/ilvuno-sp1r3-audit.SPP2Cn/scripts/tests.sh
# 95 passed, 0 failed; exit 0

SP1_SCRATCH=/tmp/ilvuno-sp1r3-audit.SPP2Cn/false-host-parent \
SP1_GIT_BIN=/bin/false TMPDIR=/tmp/ilvuno-sp1r3-audit.SPP2Cn/tmp \
bash /tmp/ilvuno-sp1r3-audit.SPP2Cn/scripts/sp1.sh matrix
# exit 1; run-env reports missing git_version or os
```

Git 2.38.1 used cached image `ilvuno-git238r-audit:latest`, image ID `sha256:7536f667e6d345ac43ee9a4056be33e1a53fda74d9c61c1d0630e45a0086fa3a` (not rebuilt during this audit):

```sh
docker run --rm -u 1000:1000 \
  -v /tmp/ilvuno-sp1r3-audit.SPP2Cn/scripts:/sp1r:ro -w /sp1r \
  -e SP1_GIT_BIN=/usr/local/bin/git \
  -e SP1_SCRATCH=/tmp/sp1r3-parent -e TMPDIR=/tmp \
  ilvuno-git238r-audit:latest /bin/bash -lc \
  'mkdir -m700 -p "$SP1_SCRATCH"; /usr/local/bin/git --version; ./sp1.sh matrix && ./tests.sh'
# git version 2.38.1; matrix PASS: 28 cases; 95 passed, 0 failed
```

The same `/bin/false` matrix negative check in the Git 2.38.1 container exited nonzero. F10's independent raw path was `6c696e650a627265616b2e747874`; its blob SHA-256 was `2d27fbdf4e8ca207afbfa388ca9172fbcc6c70e534af2476b3b704f87debadcf`, matching the expected result.

## N1 — Oracle execution boundary

**Status: CONDITIONALLY CLOSED**

- Direct invocation with the registered F8 fixture returned an oracle result with `status: refused`, zero candidates, and no driver marker. The repository-configured custom merge driver was refused before checkout or merge.
- A local checkout filter configuration was refused before checkout; its marker was absent.
- A direct oracle run with a repository `core.hooksPath` and runtime `GIT_CONFIG_*` hook-path injection produced no hook marker. A second private F1 fixture with a `.gitattributes` filter and runtime `filter.spy.smudge` injection completed without running the injected command. The oracle's scrubbed environment and forced empty hooks path were effective.
- The regression suite's unregistered-fixture, invalid-owner-marker, and external-repository cases failed without creating output. The full oracle generation completed and its fixture worktrees were absent afterward.

**Provenance limit:** I also constructed a private scratch marker and `oracle-repos.tsv` by hand, using the current UID and a matching run ID, and direct invocation accepted the in-root F1 repository. This confirms that the marker/registry protocol is forgeable by the same user; it does not authenticate that a particular live `sp1.sh` process created them. Same-UID forgery is outside the stated M0 threat model. Treat the registry as a guard against accidental/arbitrary paths in a trusted harness run, not as an authorization token.

**M0 disposition:** No executable-feature bypass was reproduced within the controlled process boundary. Keep fixtures in a fresh private root and do not treat worktrees or the registry as a security sandbox.

## N2 — Manifest validation

**Status: CONDITIONALLY CLOSED**

`lib.sh` delegates manifest parsing to `schema.py`; `oracle.py` imports and calls that same parser. The 95-assertion suite passed malformed-line, unknown/duplicate-key, missing-field, short-OID, duplicate-source, source-equals-base, and unsupported-source-count cases. Additional private mutations covered invalid OID characters, one and four sources, and a full-length but absent base object ID. Malformed inputs returned nonzero and did not write a new output file. A syntactically valid but absent base was rejected by Git before an oracle result was written; the syntax-only schema accepts it, as expected.

Two boundaries remain:

1. The oracle leaves an existing output path untouched when validation fails. Reproduced by copying a valid oracle result to `stale-oracle.json`, corrupting the private fixture manifest, then invoking `oracle.py` with that same output path: exit 2, while the old file remained byte-identical. This is not a newly published result, but callers must use the exit status and must not consume a prior result after a failed invocation. Prefer a fresh output path per run or remove/quarantine the prior output before invoking the oracle.
2. A full-length, lowercase OID that is absent from the repository passes schema syntax validation. Oracle Git operations then fail without publishing a new result. In a manually corrupted private fixture, the object pipeline classified the absent base as `unrelated_histories`; generated harness fixtures use real object IDs, so this is a fixture-corruption limitation rather than a blocker for the controlled matrix.

## N3 — Git-version binding

**Status: CONDITIONALLY CLOSED**

On actual Git 2.55.0 and 2.38.1 matrix outputs, changing all result rows to the *other supported version* while leaving `run-env.json` unchanged failed. Changing only the sidecar also failed. Changing both rows and sidecar together to the other supported version passed all 28 cases; changing both to `9.9.9` failed the allowlist check.

The harness observes `SP1_GIT_BIN --version` when creating the run and records it in `run-env.json`. The checker verifies each row equals that sidecar and that the version is allowlisted. It does **not** re-run or hash the Git executable during checking, and it does not authenticate the sidecar. The guarantee is internal consistency plus version allowlisting, not tamper-evident proof of which executable ran. This is acceptable for a trusted, controlled M0 run; do not present the sidecar as a signed or independently attested record.

## Scratch cleanup

**Status: CONDITIONALLY CLOSED**

- `SP1_CLEAN=1` on a successful 28-case matrix removed the invocation-owned run child and preserved its parent sentinel. The regression suite's invalid-command error path also removed its run child and exited nonzero.
- Invalid marker format/UID, mismatched run ID, and marker symlink were refused; the owned root was preserved. An injected `rm` failure was reported and forced exit 2, leaving the private root for inspection.
- A symlink-aliased parent path was rejected as `scratch child escaped its parent` but left the just-created private run child because initialization failed before the cleanup trap was installed. Use a canonical, non-symlink scratch parent for M0 runs.
- Separately, an injected Git wrapper that failed `worktree remove` caused `oracle.py` to return success and publish its result while leaving a private oracle worktree/admin entry. The `finally` block ignores the removal/prune return codes. This was confined to the private scratch root and manually cleaned after the probe. Normal oracle runs on both tested Git versions removed their worktrees.

These are confirmed cleanup limitations, not hostile-filesystem escapes. Under the M0 model, use a canonical fresh private parent, inspect the run root when cleanup reports failure, and do not reuse scratch roots after an interrupted or failed run.

## Findings by milestone

| Classification | Finding |
|---|---|
| **M0 blocker** | None reproduced when the harness is run in its stated controlled boundary, command exit codes are checked, and a fresh canonical private scratch root is used. |
| **M0 documented limitations** | Marker/registry and run-env are editable by the trusted owner; an old oracle output survives a failed invocation; symlink-aliased initialization and injected Git removal failures can leave private scratch residue; syntactically valid absent object IDs are not schema-detectable. |
| **M2 production requirement** | Authenticate run provenance if it must resist same-UID edits; sandbox project-controlled Git extensions/commands; make cleanup robust against hostile/concurrent filesystem changes; surface and fail on oracle worktree cleanup errors. Git worktrees alone provide no security isolation. |
| **Release/future coverage** | The Git 2.38.1 image was cached, not rebuilt; this audit covered Linux only. Verify image provenance and additional platforms before using these results as release evidence. |

## Final decision

Use SP-1R3 as a **controlled M0 reference harness** only when the runner checks every command's exit status, uses fresh canonical private scratch, trusts the harness process and its sidecar/registry, and does not consume an oracle file after a failed run. The reproduced edge failures leave private residue or preserve prior output; they did not escape the fixture boundary or execute repository-configured commands.

**YES — SP-1R3 is acceptable as a controlled M0 reference harness under those conditions.**  
**Final verdict: APPROVE WITH CONDITIONS.**
