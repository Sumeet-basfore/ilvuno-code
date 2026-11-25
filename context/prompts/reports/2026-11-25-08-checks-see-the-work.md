# Prompt 08 · checks see the work

## Goal

Run each round's checks against the worktree gate acceptance will merge.

## Changed

- Commit `9a95210` adds a temporary integration worktree built from the run base and the checked-out thread branches.
- Round checks use the same active branches and order as gate acceptance. The runner removes the temporary tree after checks.
- Conflicts and tree-build errors block the round. Folder-mode runs keep their current check location.
- The red regression test stays unchanged. Added tests cover two writers, an unstarted thread, conflicts, a failed check fixed in the next round, and git tree cleanup.
- Decision D25 records the integration tree and its branch-name suffix.

## Verified

- `RUSTUP_TOOLCHAIN=1.100.0 cargo test -p runner --test loop_fake`: **13 passed**.
- `RUSTUP_TOOLCHAIN=1.100.0 cargo test -p runner --lib git::tests::integration_tree`: **2 passed**.
- One `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` run: **172 passed, 3 ignored, 0 failed**.
- `git diff --check` passed. The Rust review found no issues. The comment audit clarified one `TreeBuild::Ready` comment; no comments were deleted or restored.

## Limits

- A merge conflict leaves no valid combined tree. The readiness record contains only `threads_do_not_merge`; configured checks are skipped instead of being run on the project folder or a partial tree.
- The round tree follows gate acceptance's active-checkout filter and first-dispatch order. It does not use the prompt's name order, because that would not match the accept path.
- Formatting was not checked. The pinned toolchain has no `cargo-fmt` component, and it was not installed.

## Open

The branch push is pending. No functional issue remains open.
