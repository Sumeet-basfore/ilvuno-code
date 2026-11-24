See `AGENTS.md` — it is the single source of project rules.

Critical, in case you read only this file:

- Rust: use `RUSTUP_TOOLCHAIN=1.100.0` (the `stable` toolchain on this machine is broken).
- Every prompt from the owner assumes poteto mode: read `.agents/skills/poteto-mode/SKILL.md` **before** the
  task, then do it in that style.
- A subagent runs on `gpt-6-luna` at `reasoningEffort: "xhigh"` and on no other model.
- Tests: **one** `cargo test --workspace` when a change is finished — not after every edit. While working,
  `cargo build`, and only the single test file you are touching when you need a signal.
- The window holds no engine code; it reads the local API and nothing else.
- Claude needs `control_request{initialize}` before any turn, and `--session-id` must be a UUID.
- Reports are HTML: `python3 tools/render-report.py --index <folder> "Title" "lede"`.
