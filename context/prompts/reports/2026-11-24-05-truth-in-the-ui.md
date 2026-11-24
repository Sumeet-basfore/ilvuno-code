## Goal
Make run status, duration and token use truthful in the window, including after a runner crash.

## What changed
- `d3e1f68` sets end times for stopped runs, recovers abandoned runs at API startup, records budget refusal, and shows per-round token counts.
- Added real-window screenshots: [`paused with lost-turn reason`](../../../crates/desktop/shots/2026-11-24-truth-ui-paused.png) and [`budget and per-round tokens`](../../../crates/desktop/shots/2026-11-24-truth-ui-budget.png).
- Updated the project state, timeline and decisions.

## Verified
- Pinned-toolchain `cargo build` passed; `node --check crates/desktop/ui/app.js` passed.
- `cargo test --workspace`: **163 passed, 3 ignored, 0 failed**.
- Killed the real runner mid-turn and restarted it. Startup recovered the run as `human_paused` with reason `runner stopped; the turn was lost`.
- A real run with a 1,000-token cap used **61,760 tokens** in its first turn. The next task was refused and remained unattempted. The recorded cost was **$0.01607189**.
- Read both screenshots. The paused view shows the lost-turn reason and round token count. The budget view shows the exhausted state, round token count and start-new-run guidance.

## Could not verify
- `cargo fmt --all` could not run: the Rustup proxy reports `unknown proxy name: T3-Code-x86_64`, and the installed pinned toolchain has no `rustfmt`. Build and tests used the installed Rust 1.100.0 binaries directly.

## Still open
- Prompt 04 remains reviewed and passing at the human gate. It has not been accepted or merged. Prompt 05 does not change that gate decision.
