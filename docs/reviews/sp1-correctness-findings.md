# SP-1 Git Correctness Findings

This document separates behavior reproduced in the isolated audit from behavior asserted by SP-1 reports but not established by their executable checks. Tested Git versions: installed 2.55.0 and cached Git 2.38.1 image. All cases used disposable repositories.

## Findings

### C-1 — Pinned source SHAs are consumed, but not bound to the result record

- **Severity:** Medium
- **Evidence:** Fixture construction stores `BASE` and the ordered `SRCS` as object IDs in `.git/sp1-meta.env`, and both loops consume those IDs, which is a sound pinning pattern for this harness. However, the JSON result does not include the ordered pins, and the matrix does not compare them to expected fixture IDs. It therefore does not bind a candidate SHA to the inputs that produced it.
- **Reproduction:** In a copied harness fixture, change `SRCS` in `.git/sp1-meta.env` to a different valid commit ID while retaining the same fixture label, then run the matrix case. The pipeline consumes the changed pin; the matrix has no expected-pin assertion that rejects the changed case.
- **Impact:** Reviewers cannot independently establish from each result row which exact source IDs and order produced its candidate. A fixture can change while retaining its label.
- **Recommended correction:** Emit the base SHA and ordered source SHAs in each result record, assert them against the fixture manifest, and add a moved-ref test proving the stored object IDs remain authoritative.
- **Required milestone:** M0 harness; M2 engine contract.

### C-2 — Already-integrated handling is only tested in one simple ancestry shape

- **Severity:** Medium
- **Evidence:** F5 reports skipping an already-integrated branch without an empty merge commit. The fixture does not establish behavior for a source ancestor of base, a source that became integrated through an earlier candidate step, or duplicate SHAs appearing at different positions in the pinned input list.
- **Reproduction:** Extend isolated fixtures with (a) source ancestor of base, (b) second source already included by first merge, and (c) repeated exact SHA; assert skip reason, step numbering, and no empty commit.
- **Impact:** Incorrect skip rules can create empty commits or silently alter intended merge ordering.
- **Recommended correction:** Define integration membership as ancestry of the current candidate commit, retain pinned order, and encode the skip reason and original input index in output. Add the three cases above as assertions.
- **Required milestone:** M0 fixture coverage; M2 implementation.

### C-3 — Unrelated-history classification relies on a brittle stderr match

- **Severity:** High
- **Evidence:** The comparison report itself notes B classifies unrelated history only after matching stderr. A returns exit 128; B returns a different stage/exit. The audit reproduced that hook failure can also return 1 with no unmerged paths, yet the script classifies it as conflict. Stderr text is locale/version dependent.
- **Reproduction:** Run F6 and record step/exit; then configure `pre-merge-commit` to exit 42 on a clean merge. B reported `conflict`, step 1, and an empty conflict path list.
- **Impact:** Distinct failures (unrelated histories, hook rejection, merge strategy/program errors) can be conflated, producing false conflict decisions and misleading recovery instructions.
- **Recommended correction:** Use exit status plus machine-readable repository postconditions: unmerged index stages/paths, `MERGE_HEAD`, and merge state. Treat every unmatched failure as a typed operational error. Do not parse human stderr as the sole classifier.
- **Required milestone:** M0 harness assertions; M2 production error model.

### C-4 — Conflict path handling is not safe for newline-containing filenames

- **Severity:** High
- **Evidence:** A conflict on `line<LF>break.txt` was one path in Git's NUL-delimited `merge-tree -z` output, but SP-1 transforms NUL to newline and line-parses it into two entries. B's human-readable `git diff` output quoted the path and the script did not decode it; the JSON emitter also does not escape control characters.
- **Reproduction:** Create a temporary repository with a filename containing LF, make conflicting edits on two branches, and run both pipeline variants. Compare emitted paths with `git ls-files -u -z`.
- **Impact:** Conflict reports can contain wrong path counts or invalid JSON, undermining reproducible evidence and approval review.
- **Recommended correction:** Preserve NUL-delimited bytes through parsing, encode paths as JSON strings with proper escaping (or base64 raw path bytes plus a display field), and compare against the unmerged index using `-z`.
- **Required milestone:** M0 report schema/harness; M2 user-facing conflict reporting.

### C-5 — `merge.ff=only` is overridden by explicit `--no-ff`; this is policy, not config fidelity

- **Severity:** Medium
- **Evidence:** With `merge.ff=only`, SP-1's explicit `git merge --no-ff` succeeded and created a two-parent merge commit for a fast-forwardable source on host Git 2.55.0. This confirms the command-level no-ff policy, not user-config-driven behavior.
- **Reproduction:** Configure `merge.ff=only`, create a fast-forwardable source in a disposable repo, and run `git merge --no-ff <sha>`.
- **Impact:** Results are deterministic only if Ilvuno intentionally owns and documents merge flags. User config is not the effective policy where explicit options override it.
- **Recommended correction:** Declare the candidate merge policy, pass all relevant options explicitly, sanitize config, and record strategy/options in evidence. Add tests showing which user settings are ignored or honored.
- **Required milestone:** M0 reference behavior; M2 product/engine contract.

### C-6 — Merge strategy and attributes are not fully pinned by the experiment

- **Severity:** High
- **Evidence:** `merge.default=spy` selected an external driver when no explicit merge attribute was present; earlier source commits can add attributes that affect later merge steps. A `merge.upper.driver=true` configuration on host Git 2.55 returned success but kept the “ours” content, losing the later source's change. The `drivers-text` variant also behaved differently between Git versions and approaches: Git 2.38 A reported worktree-add failure with two refs left, while B classified the conflict. The variant neutralized the driver to `text`, which is not a safe universal neutralization proof.
- **Reproduction:** Use the SG-3/SG-4 fixtures for custom drivers. For content loss, set `merge.upper.driver=true`, create conflicting edits, and inspect resulting tree content rather than only return code. Re-run `drivers-text` on both versions.
- **Impact:** Exit code and driver name alone do not prove semantic correctness. Configured strategies/drivers can produce successful but lossy candidates or divergent failure behavior.
- **Recommended correction:** Pin the merge strategy and options in a sanitized environment; refuse custom drivers and unsupported attributes at every step; assert resulting trees against independently specified expected trees. Never use a generic external driver as a neutralizer without proving semantics on every supported Git version.
- **Required milestone:** M0 harness fixtures; M2 production merge policy.

### C-7 — Commit metadata reproducibility is demonstrated only for the controlled sample

- **Severity:** Medium
- **Evidence:** Candidate SHAs matched across tested clean cases and versions using the fixture's fixed committer envelope/message. The reports do not establish behavior with varying author identities, encoding, timezone, locale, environment dates, or configured signing/hooks.
- **Reproduction:** Repeat the same pinned fixture under different `GIT_AUTHOR_*`, `GIT_COMMITTER_*`, locale, and signing config; compare candidate IDs and recorded metadata.
- **Impact:** A SHA reproducibility claim is valid only when all metadata inputs are controlled and part of the run record.
- **Recommended correction:** Define author/committer/message/date/signing policy explicitly, set deterministic values in the subprocess environment, and emit a normalized metadata record with input SHAs and Git version.
- **Required milestone:** M0 reproducibility contract; M2 engine evidence.

### C-8 — Candidate SHA equality is bounded to the tested corpus

- **Severity:** Medium
- **Evidence:** Independent comparison confirms A/B and cross-version equality for the eight named clean cases in this run. It does not cover LFS, submodules, filters, case-only renames, large rename sets, replace refs, grafts, shallow repositories, alternates, or other merge options. The reported conflict OIDs were not backed by durable raw output included in the repository.
- **Reproduction:** Expand the isolated matrix with each listed edge case and compare candidate tree/commit IDs and normalized reports. Preserve raw outputs and expected results as repository review artifacts without adding them to production paths.
- **Impact:** The SP-1 result cannot justify a universal equivalence claim or guarantee later versions behave identically.
- **Recommended correction:** Narrow the claim to the tested cases; maintain a versioned edge corpus and independent expected trees. State supported repository/configuration constraints.
- **Required milestone:** M0 narrow baseline corpus; M2 support matrix before release.

### C-9 — Cleanup and conflict retention behavior differs from the recommendation

- **Severity:** High
- **Evidence:** P3 cleanup removed worktree admin metadata but left an orphan directory on both versions. For F3/F4, B runs `git merge --abort`, removing markers, stages, and `MERGE_HEAD`; the recommendation's H-3 says keep the worktree by default. A stop-after-1 run leaves its worktree path and blocks an immediate rerun. Pipeline A can publish refs before materialization; a Git 2.38 `drivers-text` failure left two refs.
- **Reproduction:** Run P3; inspect `worktree list --porcelain` and filesystem. Run B on F3/F4 and inspect index/merge state after result. Run stop-after-1 twice. Run A with `drivers-text` on Git 2.38 and inspect `refs/ilvuno`.
- **Impact:** Recovery and debugging behavior are not what the recommendation claims. Orphan directories, stale refs, and lost conflict state create ambiguous ownership and rerun hazards.
- **Recommended correction:** Decide conflict retention explicitly and implement it; verify both filesystem and Git metadata during cleanup; journal the outcome; make retries idempotent; delete only refs/paths proven owned by the run. Add kill/crash-at-each-step tests before production.
- **Required milestone:** M0 harness cleanup for its own temporary artifacts; M2 run lifecycle and recovery.

### C-10 — `.ilvuno` ignore behavior is a prerequisite, not an intrinsic property of B

- **Severity:** Medium
- **Evidence:** SP-1 itself reports both A and B make the main worktree appear dirty without the `.ilvuno/` ignore rule; with it, status is clean. The readiness checklist still has unresolved owner decisions, and B's scratch directory can also be redirected by a symlink (SG-8).
- **Reproduction:** In a temporary repo run the candidate worktree under `.ilvuno/` with and without the ignore rule and inspect status; repeat with `.ilvuno` symlinked outside the root.
- **Impact:** “Main untouched” and “clean worktree” depend on an explicit storage contract and path integrity.
- **Recommended correction:** Make scratch location and ignore behavior part of the approved design. Assert main worktree state using a complete safety fingerprint, and reject symlinked scratch components.
- **Required milestone:** M0 owner decision and harness assertion; M2 production behavior.

## Supported and unsupported correctness claims

**Supported narrowly:** equal candidate/tree IDs for eight reported clean cases across A/B and Git 2.38.1/2.55.0; basic content-conflict step/path agreement; explicit `--no-ff` produced a merge commit in the tested fast-forward case.

**Not established:** universal pipeline equivalence; complete already-integrated semantics; robust conflict/fatal distinction; arbitrary path reporting; full repository-state preservation; cross-version behavior for custom drivers/filters; cleanup completeness; crash recovery; or user-config fidelity.
