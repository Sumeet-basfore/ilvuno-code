# SP-1R — Harness Correctness and Security Remediation

Status: remediation complete for the harness; production engine unchanged; M0 not started.
Scope: `docs/experiments/` scripts and reports only. No approved PRD, architecture, ADR, decision
record, ownership policy, or global Git configuration was modified. Nothing pushed.

Inputs: the four independent audits (`docs/reviews/sp1-*.md`), all SP-1 reports, the SP-1 script,
its Dockerfile/README, and the readiness decision/checklist registers (read only).

---

## 1. What changed

| Area | Before | After |
|---|---|---|
| Matrix | Recorded rows, exited 0 with a failing Git | Asserts 28 cases against `expected.json` (10 fixtures × 2 approaches + 4 variant pairs); any missing row, malformed JSON, wrong field, or Git failure exits 1 |
| Scratch | Wrote `work/`, `out/`, `fx/` next to the script, inside the repo | Private root (`SP1_SCRATCH` or `mktemp -d`), refused if inside a work tree or the harness dir; every delete validated |
| `SP1_VER` | Interpolated into `rm -rf` | Removed; labels derived from allowlisted fixture/approach/variant and validated |
| Git execution | Inherited environment, hooks functional | `env -i` allowlist, stripped `GIT_CONFIG_*`, command-scoped hooks/attributes/signing/submodule config |
| Drivers/filters | Partial attribute scan; failed open on 2.38 | Refuse on any configured executable feature, independent of attribute source; re-checked before every step |
| Classification | stderr substring match | `merge-base` pre-check + `ls-files -u -z` + `MERGE_HEAD`; unmatched failure = operational error |
| Paths | NUL→newline then line-split | NUL preserved end-to-end; JSON escaped with `surrogateescape` |
| Pins/metadata | Not in the record | Base, ordered sources, policy, git version, OS, identity, candidate/tree, content hashes in every row |
| Fingerprint | HEAD/status/index/stash only | Adds refs, local config, hooks listing, worktree registrations |
| Cleanup | Best-effort `worktree remove` | Journaled states, ownership check, orphan directory detection, ref deletion only when the SHA matches |
| Tests | None (probes only) | 38 negative/security assertions, all passing on Git 2.55.0 and 2.38.1 |

---

## 2. Finding-by-finding

Versions tested for every row: **2.55.0** (CachyOS Linux) and **2.38.1** (Alpine container, built
from pinned source). "Result" is the state after remediation.

### 2.1 Independent verification findings

| ID | Finding | Fix | Test | Result | Remaining limitation |
|---|---|---|---|---|---|
| **V-1** | Matrix is a recorder; exits 0 with a failing Git | `expected.json` + `expected.py --check`; nonzero on missing/extra/malformed/mismatched rows | P1.1–P1.4; also `SP1_GIT_BIN=/bin/false` manual run | **PASS** | Expectations are recorded from a trusted run; a deliberately wrong `--update` would launder a regression — review of that diff is the control |
| **V-2** | `SP1_VER` traversal; stale README/Docker paths | Scratch root validation, label allowlist, `safe_rmrf`, `SP1_VER` removed, README/Docker rewritten | P0.1–P0.7 | **PASS** | A malicious *environment* can still choose the scratch root; it is refused unless private/owned/not-a-symlink, but a user-supplied root is trusted to be theirs |
| **V-3** | Fingerprint omitted refs/config/worktrees | `user_fingerprint` extended; field renamed `user_state_unchanged` | V-3 | **PASS** | It still cannot detect writes *outside* the repository; the harness checks its own output dir separately (P0.7) |
| **V-4** | Original review only partly closed | Every SG case is now executable and in `tests.sh` | SG-1…SG-10 | **PASS** (harness) | Production hook/filter/driver policy remains an owner decision |

### 2.2 Correctness findings

| ID | Finding | Fix | Test | Result | Remaining limitation |
|---|---|---|---|---|---|
| **C-1** | Pins not bound to the record | `base`, ordered `sources`, `merge_policy`, `metadata`, `git_version` emitted; compared to `expected.json`; moved-ref test | C-1 | **PASS** | Order is asserted as stored; a caller could still change the fixture manifest between builds, which the expected pins would expose but not prevent |
| **C-2** | Already-integrated only tested in one shape | Not extended | — | **OPEN** | Only ancestor-of-base is covered (F5). Source-ancestor-of-base, integration by an earlier step, and duplicate SHAs at different positions are untested |
| **C-3** | stderr-based classification; hook failure read as conflict | `classify_merge_failure` (unmerged NUL list, `MERGE_HEAD`), `merge-base` pre-check for unrelated histories; unmatched → `operational_error` | C-3a, C-3b; SG-1 fixture | **PASS** | A failure that leaves `MERGE_HEAD` *and* unmerged paths is reported as a conflict; a hook that fails after creating state could still be misread (not observed) |
| **C-4** | Newline filenames split; JSON unescaped | NUL preserved; `emit.py` with `surrogateescape`; paths from `ls-files -u -z` | F10 in matrix; C-4 | **PASS** | Non-UTF-8 bytes are represented via surrogate escapes in JSON; a strict JSON consumer may prefer base64 (documented, not implemented) |
| **C-5** | `merge.ff=only` overridden by `--no-ff` | Declared policy recorded in every row (`merge_policy`) | matrix field comparison | **PASS** (declared) | The harness intentionally ignores user `merge.ff`; production must own this policy explicitly |
| **C-6** | Strategy/attributes not pinned; `driver=true` lossy | Config refusal covers every executable feature; `-c` controls; `drivers-text` variant removed; probe P4 documents that `=true` keeps "ours" | SG-3…SG-7; P4 | **PASS** (harness) | Attribute semantics that do not execute code (e.g. `merge=union`) are not pinned; the harness records content hashes, which would expose a difference |
| **C-7** | Commit metadata only for the controlled sample | Fixed identity/date policy recorded; candidate content hashed | matrix `metadata` field | **PARTIAL** | No variation test across identity/date/encoding; the claim is limited to the recorded envelope |
| **C-8** | Equivalence bounded; conflict OIDs not retained | Claims narrowed in the reports; candidate/tree/content recorded per row | matrix | **PASS** (claims) | LFS, filters on content, case-only renames, shallow/alternates, >2 sources with renames remain untested |
| **C-9** | Cleanup/retention differs from recommendation | Cleanup states (`removed`, `orphaned_dir_removed`, `orphaned_dir_left`, `failed`), directory check, ref ownership check, `keep-conflict` variant | matrix F9; P3, P5; P4.1–P4.3 | **PASS** (harness) | The harness's own scratch cleanup is not the product's recovery protocol |
| **C-10** | `.ilvuno` ignore is a prerequisite; symlink redirect | Harness writes outside the repo; SG-8; README documents the product requirement | SG-8, SG-8b, P2 | **PASS** (harness) | Production candidate layout is still an open decision (D-07/D-09) |

### 2.3 Security gates

| ID | Gate | Harness handling | Test | Result |
|---|---|---|---|---|
| **SG-1** | Hooks execute | `-c core.hooksPath=<empty dir>` on every call; markers asserted absent | SG-1 | **PASS** (neutralised) |
| **SG-2** | `GIT_CONFIG_*` env injection | `env -i`; `GIT_CONFIG_COUNT=0`; injection absent from child env | SG-2, SG-2b | **PASS** |
| **SG-3** | `merge.default` selects a driver | Refused when `merge.default` or any non-builtin `merge.*.driver` is configured | SG-3 | **PASS** (refused) |
| **SG-4** | Attributes added by a later source | Config re-checked before every step; refusal precedes any merge | SG-4 | **PASS** |
| **SG-5** | `core.attributesFile` missed on 2.38 | Detection no longer depends on attributes at all: a configured driver is refused regardless of source | SG-5 | **PASS** |
| **SG-6** | Linked-worktree `info/attributes` path | Same config-based refusal; no path assumption | SG-6 | **PASS** |
| **SG-7** | Checkout filters | Any `filter.*.smudge/clean/process` refused before a worktree is created | SG-7 | **PASS** (refused) |
| **SG-8** | `.ilvuno` symlink redirect | Harness never writes through the repo; scratch is validated and outside | SG-8, SG-8b | **PASS** |
| **SG-9** | Candidate commands escape | Only a digest-pinned harness-owned `check.sh` is executed; tampering refuses | SG-9 | **PASS** (harness) / **NOT CONTAINED** (process) |
| **SG-10** | Submodule checkout | `-c submodule.recurse=false`; status asserted uninitialised | SG-10 | **PASS** (asserted) |
| **SG-11** | Orphan directory after admin removal | Directory and admin checked separately; orphan reported | P3; matrix F9 | **PASS** (detected) |

### 2.4 Destructive-path handling (P0 objective)

| Requirement | Implementation | Test |
|---|---|---|
| Never delete from an unvalidated env string | `SP1_VER` removed; labels validated against `^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$` | P0.1, P0.6 |
| Private scratch root | `SP1_SCRATCH` or `mktemp -d`; refused if symlink, relative, wrong owner, inside a work tree, or inside the harness dir | P0.4, P0.5, P0.7 |
| Reject traversal | single-component label validation; `realpath` prefix check | P0.1 |
| Reject symlinked targets | `-L` check on the target and every component | P0.3, P0.3b |
| Ownership before cleanup | `stat -c %u` equals `id -u` before `chmod`/`rm` | P0.2; cleanup path |
| Never recursively delete unverified | `safe_rmrf` is the only recursive delete; refuses `/`, `.`, `..`, empty, escapes | P0.2 |
| Never delete user-owned when uncertain | refuses on wrong owner; leaves the path and reports it | P0.2 (outside root), P0.3b (sentinel) |

---

## 3. Exit-criteria check

> **SP-1R2 note.** After the closure audit, the harness was revised again: `run-existing` was
> removed, the fixture manifest is parsed without shell evaluation, refs are run-scoped, the
> content oracle is byte-exact, scratch roots are exclusive children, and expectations come from an
> independent Git-native oracle. Case count is **28**; assertions are **68**. See
> `docs/reviews/sp1r2-closure-evidence.md`. The table below reflects SP-1R and is retained as
> history.

| # | Criterion | Evidence | Result |
|---|---|---|---|
| 1 | Traversal prevented by automated negative tests | P0.1–P0.7, sentinel intact | **PASS** |
| 2 | Matrix fails when Git commands fail | `SP1_GIT_BIN=/bin/false` → 28 missing rows, exit 1 | **PASS** |
| 3 | All expected fixtures asserted | 28/28 cases match `expected.json` on both versions | **PASS** |
| 4 | Unexpected executable Git behaviour detected | SG-1…SG-7 markers absent; refusals recorded | **PASS** (within the declared boundary) |
| 5 | Path reporting preserves arbitrary filenames | F10 and C-4: `["line\nbreak.txt"]` | **PASS** |
| 6 | Cleanup and ownership assertions pass | P4.1–P4.3, P3, P5, matrix F9 | **PASS** (harness) |
| 7 | No unexpected writes outside scratch | P0.7; no writes in the repo; sentinels intact | **PASS** |
| 8 | Repeat runs consistent | Matrix re-run after a scratch wipe produced identical SHAs; P4 repeat | **PASS** |

---

## 4. What this does not fix

- **Production integration policy.** Hook, filter, driver, attribute, submodule, sandbox, and
  scratch-layout decisions for the product remain owner decisions (D-07…D-10, plus the new
  hook/filter policy decision).
- **Containment.** The wrapper is not a sandbox. A repository's own verification command still runs
  with the user's privileges; the harness only avoids running repository-provided code itself.
- **Edge corpus** (C-2, C-8) and **metadata variation** (C-7).
- **macOS** and **Git versions other than 2.38.1 / 2.55.0**.
