# Provider contract: summary

**Topic.** What an adapter must implement and what the runner promises it. Two adapters exist in v1: `claude` (native stream-json plus control protocol) and `acp` (the `agent-client-protocol` crate, pointed at any agent's ACP command).

**Core rule.** Zero first-party privilege. If `claude` needs something the ACP adapter cannot also get, it goes into the shared capability struct or it does not exist.

**Boundary.** The adapter is a translator, not a decision-maker. It never writes the DB, never decides a round is done, and never retries a turn on its own. If it cannot answer, it emits `unknown` and the runner decides.

**Key terms.**
- **Commands (runner to adapter):** `spawn`, `session_open`, `session_resume`, `prompt`, `permission_reply`, `interrupt`, `park`/`unpark`, `shutdown`. Every command carries a `request_id`, and re-sending the same id must not produce a second turn.
- **Events (adapter to runner):** eleven kinds, including `text_delta`, `tool_call`, `permission_request`, `usage`, `turn_finished`, and `raw`. Anything unmodelled goes into `raw` so no information is lost.
- **Capabilities:** declared, not probed. The runner greys out what is absent and never imitates a missing feature.
- **Permissions:** Claude requires a `control_request{initialize}` before any turn, or every tool is silently denied. ACP answers by option kind, not position, and needs a mode that asks (`workspace-write` or `read-only`), not the default `agent` mode.
- **Prompts:** the worker and orchestrator texts are versioned in code and injected every turn, not once per session. The journal records the version injected into each task.
- **Error classes:** `spawn_failed`, `auth_required`, `usage_limit`, `turn_failed`, `protocol`, `unknown`. Adapter death is treated as `turn_failed` with reason `adapter_died`.

**Act on these.**
1. Send `control_request{initialize}` before any Claude turn.
2. Set `"mode": "workspace-write"` on ACP provider entries, or permission requests are bypassed.
3. Do not treat `/tmp` as a security boundary. It is writable in `workspace-write` mode by default.
4. Use absolute paths for runner commands in worker preambles, because a Codex login shell's `PATH` differs from the runner's.
5. Treat `acp:*` as tier-3 unless it has been tested.
6. Do not ship PTY scraping in v1. An agent without ACP or a protocol is labelled, not faked.
