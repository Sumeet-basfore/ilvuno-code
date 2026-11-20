# SP-1 Independent Verification

**Reviewer role:** independent Git systems and security review\
**Scope:** SP-1 evidence and Pipeline B as a possible baseline\
**Verdict:** APPROVE WITH CONDITIONS; Pipeline B is suitable as an M0 reference oracle in a controlled harness, not as an M0 production engine or an M2 production engine in its current form.

## Scope and method

I read the five SP-1 reports, `docs/experiments/scripts/sp1.sh`, its README and Git 2.38 Dockerfile, all files under `docs/readiness/`, and the original `gemini_red.md` red-team review. I copied the experiment scripts into an isolated directory under `/tmp/ilvuno-sp1-audit.nngjvu` and ran them only against disposable temporary repositories. No production code or approved architecture/product document was changed. The existing Docker image `ilvuno-git238:latest` was used; its Git binary reported 2.38.1. The installed host Git reported 2.55.0. The Dockerfile was not rebuilt, so its supply chain was not independently verified.

The matrix and probe commands exited 0 on both Git versions and emitted 28 matrix rows each. Independent parsing confirmed the reported clean-case candidate and tree SHA equality for the eight cases listed in the comparison report. The two content-conflict cases agreed on step and path for the tested ordinary filenames. This supports a narrow equivalence result for this corpus. It does not support the reports' broader claims that the checks reliably gate outcomes, that Git configuration is neutralized, that hook/driver execution is controlled, or that the user's full repository state remains unchanged.

## Findings

### V-1 — The matrix is a recorder, not an enforcing test suite

- **Severity:** High
- **Evidence:** `sp1.sh matrix` exits 0 with `SP1_REAL_GIT=/bin/false`, `SP1_VER=failed-commands`, and prints zero rows. The script records outcomes; it does not assert expected verdicts, exit classes, candidate SHAs, path sets, or cleanup invariants. Its exit status therefore cannot establish a passing matrix.
- **Reproduction:** From a copied script directory, run `SP1_REAL_GIT=/bin/false SP1_VER=failed-commands ./sp1.sh matrix`; observed exit status 0 and no rows.
- **Impact:** Reports that describe the matrix as “pass” can remain green when fixture creation or Git invocations fail. The SHA comparisons were independently corroborated for the listed cases, but the script alone does not enforce them.
- **Recommended correction:** Add explicit expected-result assertions and nonzero exit on missing rows, command failures, malformed JSON, unexpected verdicts, candidate/tree mismatch, and cleanup residue. Keep raw output as evidence and validate its schema.
- **Required milestone:** M0 harness acceptance.

### V-2 — Script environment and output location are stale or unsafe

- **Severity:** High
- **Evidence:** The script writes `work/`, `out/`, and `fx/` relative to its own directory, despite README/recommendation language promising output under the caller's working directory or `/tmp`. The recommendation's Docker example invokes `./sp1.sh` from a mount where that path does not exist; the script is actually nested at `docs/experiments/scripts/sp1.sh`. `SP1_VER` is interpolated into recursive cleanup paths without validation or canonicalization.
- **Reproduction:** Copy the script to `/tmp`, create a sentinel at `/tmp/.../victim/F1-two-clean-object-plain/sentinel`, then run `SP1_REAL_GIT=/usr/bin/git SP1_VER=../../victim ./sp1.sh run F1-two-clean object plain`. The sentinel was deleted and fixture files were created outside the intended script work directory.
- **Impact:** A documented invocation may fail or write into the repository; an untrusted or accidental version string can redirect `rm -rf` to an arbitrary reachable path. This is a harness defect even though it is not production code.
- **Recommended correction:** Require an explicit temporary output root, validate version labels against a narrow allowlist, canonicalize and assert every deletion target is a strict child of the isolated root, and fix all documented invocations. Have the harness refuse to run from or write into the source checkout.
- **Required milestone:** Before M0 runs SP-1 or ports the fixtures.

### V-3 — “Main untouched” evidence checks only a narrow worktree fingerprint

- **Severity:** High
- **Evidence:** The script's fingerprint hashes `HEAD`, status, index entries, and stash list; it omits refs, config, hooks, worktree registrations, files outside the indexed tree, and other repository metadata. In a temporary copy, a fixture check command added a user branch ref and repository config key while B still reported `main_untouched:"yes"`.
- **Reproduction:** Add to a temporary fixture check script `git -C <main> config audit.marker mutated` and `git -C <main> update-ref refs/heads/audit-probe HEAD`; run B. The output said `main_untouched:"yes"`, while both mutations were present.
- **Impact:** This label overstates what was measured and can conceal changes to user-owned refs or configuration.
- **Recommended correction:** Rename the field to state the exact measured dimensions or implement a before/after safety fingerprint covering all in-scope refs, config, index, worktree registrations, stashes, and filesystem state. Use independent assertions rather than trusting the report field.
- **Required milestone:** M0 harness acceptance; production safety contract before M2.

### V-4 — The original review's key corrections are only partly closed

- **Severity:** Medium
- **Evidence:** The supplied Gemini review identified invalid multi-head `merge-tree` invocation and `-B` ref placement. Current SP-1 tests do not use the invalid multi-head invocation and correctly use detached worktrees in the tested path, so those specific defects are not reproduced in the current clean corpus. However, the report's custom-driver refusal claim does not hold for several configuration and attribute sources described in `sp1-security-gates.md`.
- **Reproduction:** See SG-2 through SG-6 in `sp1-security-gates.md`; execute their isolated temporary-repository fixtures on Git 2.38.1 and 2.55.0.
- **Impact:** Treating the earlier review as fully resolved would miss executable Git behaviors that bypass the advertised driver gate.
- **Recommended correction:** Keep the original review as historical evidence, explicitly mark which findings were fixed and which remain, and make the adversarial cases blocking assertions rather than narrative probes.
- **Required milestone:** M0 for harness controls; M2 for production policy and containment.

## What is independently supported

- On this fixture corpus, Pipeline A and B produced equal candidate/tree SHAs for the eight listed clean cases on Git 2.38.1 and 2.55.0.
- The tested basic file conflicts produced matching conflict step and ordinary path on both versions.
- A conventional B merge creates a checked-out worktree and therefore exercises checkout behavior absent from object-only construction.
- Worktrees are not a security boundary; repository config and executable Git features remain relevant.

## What remains unverified or is contradicted

- Universal equivalence, scale, macOS behavior, concurrent runs, and crash during a merge.
- Reliable refusal of custom merge drivers across config layers, worktree-specific paths, and attributes introduced by earlier candidate merges.
- Hook/filter containment, symlink-safe scratch paths, newline-safe conflict path parsing, and unexpected process execution.
- The Docker image's provenance and rebuild reproducibility.
- Whether the reported full-matrix cases were generated from exactly the same script revision and environment as the checked-in reports; the reports' `/tmp` outputs were not present as authoritative artifacts.

## References

- Experiment reports: `sp1-object-pipeline.md`, `sp1-worktree-pipeline.md`, `sp1-comparison.md`, `sp1-security-observations.md`, `sp1-recommendation.md` in `docs/experiments/`.
- Original red-team review: `gemini_red.md`.
- Readiness status: `docs/readiness/06-m0-readiness-checklist.md` and `docs/readiness/05-owner-decision-register.md`.
- Official Git references: [githooks](https://git-scm.com/docs/githooks), [gitattributes](https://git-scm.com/docs/gitattributes), [git-config](https://git-scm.com/docs/git-config), [git-merge](https://git-scm.com/docs/git-merge), [git-worktree](https://git-scm.com/docs/git-worktree).
