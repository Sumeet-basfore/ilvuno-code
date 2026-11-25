# Prompt 09 · Paseo lessons

## Changes

- Added a cited Paseo comparison, two Claude replay fixtures and two credential-boundary tests.
- Added migration 0007 to drop the unused `messages` table. Updated the data model and recorded decision D28.

## Commits

- `d4aa29e` — Paseo comparison.
- `71c16a0` — Claude replay fixtures.
- `fc83d21` — credential-boundary tests.
- `8bd87e1` — drop the dead table.

## Verified

- Claude replay test: 2 passed.
- API and ACP tests: API 56 passed; ACP 19 passed, 1 ignored.
- Fresh database test: 1 passed, 13 filtered out.
- Workspace suite exited 0.
- Generated and read `research/15-paseo.html`; Git ignores it.
- Search found the old table creation only in 0001 and the new drop in 0007.

## Not verified

- The Claude fixtures are hand-written. No live provider run was made.
- The 25.1k star count is a dated review observation without an archived GitHub snapshot.
- `rustfmt` is not installed for toolchain 1.100.0. Clippy reported existing issues in `runner/export.rs` and `runner/ledger.rs`.

## Open

- `message_policy` remains advisory. Enforcement needs a separate product decision.
- The nine-provider count in the review summary conflicts with ten entries in the pinned provider docs. The comparison page records both.
- `feat/prompt-09-paseo-lessons` is pushed and awaits human review. It is not merged.
