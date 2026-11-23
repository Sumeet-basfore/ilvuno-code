## Goal
Complete prompt 03: show recorded tool activity under each conversation task.

## What changed
- `6648207` adds the filtered journal route, safe activity fold, SSE task boundaries, and API tests.
- Updated `docs/api.md`; added the real-window shots `tool-activity.png` and `tool-activity-open.png`.
- Updated project context and marked prompt 03 done.

## Verified
- `cargo test --workspace`: **158 passed, 3 ignored**.
- `cargo build --workspace`, `node --check crates/desktop/ui/app.js`, and `git diff --check` passed.
- Existing real run `000e3613-b821-4e1c-8863-e871967234d4` showed **7 tools, 2 denied**.
- Read [collapsed screenshot](../../../crates/desktop/shots/tool-activity.png) and [open screenshot](../../../crates/desktop/shots/tool-activity-open.png). Enter opened the fold; the conversation stayed in place.

## Could not verify
- No new live provider call was started. The screenshots use the existing real run; live SSE rows were checked with the scripted provider.
- Tool result text is not stored. The window can show the first line of a recorded `dispatch.failed` reason or summary, but it cannot show tool output or a tool-level error.

## Still open
- Prompt 03 is done. Prompt 04, the first dogfood change, remains next.
