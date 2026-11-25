## Goal
Apply the owner-approved design system from the four supplied UI references to Ilvuno's desktop app.

## What changed
Commit `fe30749` (`feat/ui-design-system`). Added `docs/design-system.md`; updated the desktop theme, inspector navigation, Board focus, guided New run form, and command palette; saved ten real-window screenshots; appended the decision and implementation notes to `context/`.

## Verified
`RUSTUP_TOOLCHAIN=1.100.0 cargo build -p desktop --bin desktop` passed. `node --check crates/desktop/ui/app.js` and `git diff --check` passed. `cargo test --workspace` passed: 165 passed, 3 ignored, 0 failed. Read the built 1920×1080 window using an isolated copy of saved run `64239160-0caa-456a-b3cf-3011a0f64bb1`. Measured minimum text contrast: 4.59:1; control borders exceed 3:1. GPT‑6.1‑Terra at high effort reviewed the JavaScript changes and found no blocking issues.

## Could not verify
Open Review gate rendering: the real saved run is merged, so it has no open gate. The 960 px minimum-width layout was not directly checked.

## Still open
Check the window at 960 px and inspect an open gate from a real run when one is available.
