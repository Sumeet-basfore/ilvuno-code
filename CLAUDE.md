See `AGENTS.md` — it is the single source of project rules.

Critical, in case you read only this file:

- Rust: use `RUSTUP_TOOLCHAIN=1.100.0` (the `stable` toolchain on this machine is broken).
- No UI until the runner passes the v0 acceptance in `docs/prd.md` §7.
- Claude needs `control_request{initialize}` before any turn, and `--session-id` must be a UUID.
- Reports are HTML: `python3 tools/render-report.py --index <folder> "Title" "lede"`.
