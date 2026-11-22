# Rust runner feasibility — can a Rust supervisor drive our target agent CLIs at TS parity?

Researched read-only, 2026-11-23. Every version, count and date below was fetched this session
(crates.io API, GitHub API, raw.githubusercontent.com, docs.rs, vendor docs pages). Ground truth for
"how the TS references drive each agent" is the bundled clones at `/tmp/ilvuno-research/{t3code,orca,cloudroom}`
and the sibling reports `01-t3code.md §4`, `02-orca.md §4`, `03-cloudroom.md §4`. Anything I could not
confirm from a primary source is listed in §8, not asserted.

## 0. Verdict (5 lines)

**Viable with one gap.** Parity is achievable for every target agent on the wire — the protocols are all
newline-framed JSON over a child process's stdio, or HTTP; none is SDK-locked. The gap is **schemas, not
capability**: the two vendor-native protocols we care most about (Claude Code's stream-json control channel,
Codex app-server) have no maintained Rust type crate, so we own hand-modelled types and their drift. That
gap is bounded and partly solvable — `codex app-server generate-json-schema` emits the schema from the
installed binary, and `agent-client-protocol` 5.5.1 (Apache-2.0, 2026-11-21) covers everything else.
Two shipped Rust apps already do this (cloudroom, agent-of-empires), so this is engineering, not research.

## 1. Protocol inventory

Column 4 is the load-bearing one: if the client today is a *plain subprocess protocol*, Rust is equal.

| Agent | Wire protocol | Client today | Language-agnostic? |
|---|---|---|---|
| **Claude Code** | `claude -p --input-format stream-json --output-format stream-json --verbose` + a JSON **control protocol** on the same stdio (`control_request`/`control_response`, subtypes incl. `can_use_tool`, `hook_callback`, `mcp_message`, `interrupt`) | `@anthropic-ai/claude-agent-sdk` TS/Py. t3code: `ClaudeProvider.ts` (958 L) passes `pathToClaudeCodeExecutable`; orca: `claude-stream-json-connection.ts` (340 L) overrides `spawnClaudeCodeProcess` | **Yes.** Wire is CLI flags + JSON. Types exist only as TS `SDKControl*`. Also reachable via ACP: `claude-agent-acp` 0.109.1 (registry) |
| **Codex** | `codex app-server --listen stdio://`, JSON-RPC 2.0 (`initialize`, `thread/start\|resume\|fork\|read\|list\|rollback`, `turn/start\|steer\|interrupt`, `model/list`, `review/start`) | t3code `packages/effect-codex-app-server` (Effect layers); cloudroom `codex.rs` (896 L Rust). Registry also ships `codex-acp` 2.9.0 (Apache-2.0) | **Yes**, and better: the binary emits its own schema (`generate-json-schema`, `generate-ts` — §3) |
| **Pi** | `pi --mode rpc` — JSONL commands / `response` / session events / extension-UI records | t3code `provider-pi/src/server/rpc.ts`; cloudroom `pi.rs` (900 L Rust) | **Yes, by design.** `packages/coding-agent/docs/rpc.md`: "Use it for language-independent integrations, process isolation, IDEs" |
| **OpenCode** | two lanes: `opencode acp` (ACP over stdio), or `opencode serve` HTTP + `@opencode-ai/sdk` | cloudroom `opencode.rs` (256 L, ACP); t3code `provider-opencode` (HTTP SDK, version-probes 1.x/2.x); orca dialect | **Yes** both ways (ACP; OpenAPI HTTP) |
| **Antigravity (`agy`)** | `agy acp --dangerously-skip-permissions --disable-telemetry` (ACP) | cloudroom `antigravity.rs` (323 L) reusing its ACP base; orca `antigravity-dialect.ts`. Google publishes `agy_acp_server-1.5.2-*.zip` on dl.google.com in the ACP registry | **Yes** (ACP). Proprietary binary |
| **Cursor** | three lanes: `cursor-agent -p --trust --force --output-format stream-json --stream-partial-output --resume <chat>`; `cursor-agent acp`; `@cursor/sdk` | cloudroom `cursor_print.rs` (909 L) for stream-json; orca `cursor-dialect.ts` + model catalog for ACP; t3code `@cursor/sdk` | **Yes** for the first two; `@cursor/sdk` is TS-locked — do not take it |
| **Gemini CLI** | `npx @google/gemini-cli@0.70.0 --acp` | registry entry (Apache-2.0); orca `gemini-dialect.ts` | **Yes** (ACP). ⚠ vendor banner on geminicli.com: replaced by Antigravity CLI for unpaid/Google One users on 2026-06-18 |
| **Generic ACP** | JSON-RPC 2.0, `initialize` → `session/new\|load\|resume` → `session/prompt`, `session/update` notifications, `session/request_permission` | `agent-client-protocol` 5.5.1 Rust SDK; TS via `@zed-industries/*` adapters; the ACP registry lists **46 agents** including every row above | **Yes.** The protocol is the product; `schema/v1` + `schema/v2` are machine-readable |

Two consequences worth stating plainly:

1. **ACP is the only place where "one adapter, N agents" is honest.** Registry-verified entry points
   exist for Claude (`claude-agent-acp`), Codex (`codex-acp`), Gemini (`--acp`), Cursor (`cursor-agent acp`),
   Antigravity (`agy_acp_server`), OpenCode (`opencode acp`), Pi (`pi-acp`), Grok (`grok agent stdio`), Kiro.
2. **But ACP is a wrapper for Claude and Codex, not their native surface.** Every adapter is a third
   process (Node, or a prebuilt bundle that contains Node + the CLI) between us and the agent — extra
   failure domain, extra cold start, and a lossy translation of the richest protocol (Claude's
   `can_use_tool`/`hook_callback`/`mcp_message`). Native clients are ~900–1800 lines each per the
   cloudroom precedent; adapters are free but capped in fidelity.

## 2. The Claude gap — what the SDK has that raw stream-json does not

I read the SDK source and the CLI reference this session. The honest finding: **the SDK is a transport,
not a capability.** Its own code says so.

`claude-agent-sdk-python`, `src/claude_agent_sdk/types.py:2384-2406` (`_configure_can_use_tool`), docstring:

> "checks it is not combined with `permission_prompt_tool_name`, emits the shadowing advisory, and returns
> a copy with `permission_prompt_tool_name="stdio"` **so the CLI sends permission requests over the control
> protocol**."

`src/claude_agent_sdk/_internal/transport/subprocess_cli.py:523,742` — the whole "SDK" is:

```
claude --output-format stream-json --verbose … --input-format stream-json
```

with flags the CLI reference documents (`--include-partial-messages`, `--replay-user-messages`,
`--include-hook-events`, `--resume=`, `--session-id=`, `--fork-session`, `--resume-session-at=`,
`--permission-mode`, `--permission-prompt-tool`, `--mcp-config`, `--setting-sources`, `--json-schema`).

Control-request subtypes **the SDK sends** (from `_internal/query.py`): `initialize`, `interrupt`,
`set_permission_mode`, `set_model`, `get_mcp_status`, `mcp_reconnect`, `mcp_toggle`, `get_context_usage`,
`rewind_files`, `stop_task`. Subtypes **the CLI sends**: `can_use_tool`, `hook_callback`, `mcp_message`.
Every one is `{"type":"control_request","request_id":…,"request":{…}}` on stdin, answered by
`{"type":"control_response","response":{"subtype":"success","request_id":…,"response":…}}` on stdout.

Primary-source confirmation that the wire (not just the SDK) is documented: the official Agent SDK
TypeScript reference describes `request_id` as "The `control_request` envelope's request_id. A
`control_response` your application sends outside the SDK … must echo this value", and the `initialize`
control response carrying a `pending_permission_requests` array of complete `control_request` messages for
idempotent redelivery after a transport gap. That is a protocol spec in prose.

**What a Rust client can achieve — per capability, verified:**

| Capability | Achievable? | How |
|---|---|---|
| Structured events | ✅ | `--output-format stream-json` lines; `--include-partial-messages` for deltas |
| Tool-call events | ✅ | assistant `tool_use` blocks + `tool_result`; partials carry `content_block_*` |
| Permission prompts | ✅ | `--permission-prompt-tool stdio` → `can_use_tool` control requests; answer `allow/deny` + `updatedInput` |
| Interrupts | ✅ | `control_request{subtype:"interrupt", cancel_queued}`; `--replay-user-messages` gives stdin acks so we know what landed |
| Session resume | ✅ | `--resume <id>`; `--fork-session`; `--resume-session-at <uuid>`; own ids via `--session-id` |
| Compaction | ⚠️ | No control request exists. cloudroom sends `/compact` as a user message (`claude.rs:499-500`) and watches `system/compact_boundary` + `compact_metadata.post_tokens`. Implementation evidence only — see §8 |
| Hooks (PreToolUse etc.) | ✅ | hooks are declared in the `initialize` control request; CLI then calls back with `hook_callback`. cloudroom answers them from Rust (`claude.rs:932,1699`) |
| Client-served MCP tools | ✅ | `--mcp-config '{"mcpServers":{"x":{"type":"sdk"}}}'` + `mcp_message` control requests. cloudroom implements a `delegate` tool this way (`claude.rs:770-880`) |
| File checkpoint / rewind | ✅ | `rewind_files` control request; CLI needs env `CLAUDE_CODE_ENABLE_SDK_FILE_CHECKPOINTING=true` (Py SDK `subprocess_cli.py:818-820`) |

**Genuinely SDK-only (and therefore work we take on):**
- **Types.** `SDKControl*`, `SDKMessage` unions exist only in TS (and mirrored in Py). The CLI reference
  page does not document `control_request` at all — I grepped it: 0 occurrences, vs 8 in the TS SDK
  reference. So the protocol is documented *as an SDK artefact*, not as a CLI contract.
- **In-process MCP tool ergonomics** (`createSdkMcpServer`, `tool()`): the wire is `mcp_message`, the
  ergonomics are TS.
- **Session-store adapters** (S3/Redis/Postgres — `examples/session_stores/*` in the Py SDK): client-side
  code over CLI session files. Reimplement if we want it.
- **CLI resolution / pinning / missing-binary guardrails.** We must find and version-check `claude` ourselves.

Verdict on §2: **no capability is impossible.** The cost is a hand-written model of ~10 control-request
subtypes and ~15 message types, plus the discipline to answer `hook_callback` and `can_use_tool` or the
turn stalls.

## 3. What exists in Rust already

### Protocols / SDKs (libraries we could depend on)

| Crate | Version | Released | Licence | Downloads (recent) | What it is |
|---|---|---|---|---|---|
| `agent-client-protocol` | 5.5.1 | 2026-11-21 | Apache-2.0 | 6.59M (2.69M) | **Full ACP client+agent+proxy SDK.** Features: `process` (spawns + supervises the child: process-group SIGTERM→SIGKILL, 64 KiB stderr capture, 1–2 s grace), `stdio`, `schemars`, `unstable_*` (session fork, subagents, plan ops, token usage). `AcpAgent::from_str("npx …")`; `Client.v1()…connect_with(agent, …)`. Also `mcp_server::*` for serving MCP **over ACP** |
| `agent-client-protocol-schema` | 1.21.2 | 2026-11-21 | Apache-2.0 | 7.28M (3.14M) | The schema types alone — use for codegen/vendoring |
| `agent-client-protocol-derive` | 5.5.1 | 2026-11-21 | Apache-2.0 | 2.63M | Dispatch macros |
| `agent-client-protocol-http` | 5.5.1 | 2026-11-21 | Apache-2.0 | 229k | HTTP/WS transport (if the runner is ever remote) |
| `codex-app-server-protocol` | **0.63.0** | **2025-12-11** | Apache-2.0 | 2.1k | Codex app-server types. ⚠ **single version, ~11 months and ~108 codex releases stale** (codex is at `rust-v0.171.0`, 2026-11-20). Do not depend; use `generate-json-schema` instead |
| `github-copilot-sdk` | 1.0.35 | 2026-11-19 | MIT | 278k | Precedent only: a vendor shipping a *Rust* SDK for its coding-agent CLI over JSON-RPC |
| `sacp` | 11.0.0 | 2026-03-16 | MIT/Apache-2.0 | 326k | Symposium's ACP extensions; stale, only if we want their RPC/HTTP layer |

### Apps to learn from (both Apache-2.0/MIT, both shipping today)

| Project | What it proves | Facts |
|---|---|---|
| **cloudroom** (`github.com/davidondrej/cloudroom`, Apache-2.0, core 0.1.119) | **Native Rust clients for every harness**, no ACP for the two vendors | `core/src/runtime`: `claude.rs` 1780 L, `codex.rs` 896, `pi.rs` 900, `cursor_print.rs` 909, `antigravity.rs` 323, `opencode.rs` 256, `process.rs` 433, `terminal.rs` 731 — **13.7k lines total**, deps `serde`/`serde_json`/`tokio`/`axum`/`reqwest`, **no ACP crate** (hand-rolled `initialize`/`session/new` in `cursor.rs`). `publish = false` → vendor or read |
| **agent-of-empires** (`agent-of-empires/agent-of-empires`, MIT, v1.25.0 2026-11-18, 4.7k★) | **Rust runner + ACP + pinned adapters** is a shipped product shape | `agent-client-protocol = "5"` with `unstable_end_turn_token_usage, unstable_plan_operations, unstable_session_fork, unstable_subagents`; `ratatui 0.30`, `tokio 1.52`, `rusqlite 0.41`, `axum 0.8` (ws), `notify 8.2`; `src/acp/` ≈103 files (includes tests); wraps Claude/Codex/Pi with **pinned** `@agentclientprotocol/claude-agent-acp@0.103.0`, `@agentclientprotocol/codex-acp@2.5.0`, `pi-acp@0.0.34`; releases darwin amd64/arm64 + linux amd64/arm64 (**no Windows**) |
| `1jehuang/jcode` (21.9k★, MIT), `mikeyobrien/ralph-orchestrator` (3.2k★, MIT) | Rust agent harness / loop orchestrator as a category exists | not adapters — reference for structure only |

### Runner plumbing picks (all verified on crates.io)

| Need | Crate | Version | Released | Licence | Note |
|---|---|---|---|---|---|
| Runtime | `tokio` | 1.54.1 | 2026-10-21 | MIT | `process`, `io-util`, `sync`, `time`, `signal` — that is all we need |
| Persistence | `rusqlite` | 0.43.0 | 2026-11-15 | MIT | rust 1.89. Pick this: one writer, WAL, no async pool we don't need |
| Persistence (alt) | `sqlx` | 0.9.1 | 2026-11-18 | MIT/Apache-2.0 | **rust 1.94** — a toolchain floor for no benefit at our scale |
| PTY (fallback lane) | `portable-pty` | 0.9.0 | 2025-02-11 | MIT | 31M dl, last release Feb 2025 — stable, not dead |
| FS watching | `notify` | 9.0.0 | 2026-11-21 | CC0-1.0 | For worktree change detection |
| TUI (if any) | `ratatui` | 0.30.2 | 2026-06-19 | MIT | AOE uses it; we probably don't |

Codex types are **not** a crate problem, they are a codegen problem: `codex app-server generate-json-schema
--out DIR [--experimental]` and `generate-ts --out DIR [--prettier BIN]` are real subcommands in
`codex-rs/cli/src/main.rs:731-754`, and `codex-app-server-protocol` exports `generate_ts`, `generate_json`,
`generate_types` (`src/lib.rs`, `src/precomputed_exports.rs`). So the plan for Codex is: run the installed
binary's schema generator at build/CI time, commit the schema, diff it on version bump. That eliminates the
one drift risk we cannot otherwise close.

## 4. Work estimate per protocol client

Order of magnitude for a competent Rust developer who reads the TS reference and cloudroom's Rust port.
"Parity" = turns, streaming text, tool calls, permissions, interrupt, resume, usage, plus error surfacing.

| Client | First working turn | Production parity | Riskiest part |
|---|---|---|---|
| **ACP (generic)** | **1 day** — `AcpAgent::from_str` + `Client.v1()` + `session/prompt` is ~40 lines (the SDK's own `yolo_one_shot_client.rs`) | **2–3 weeks** for AOE-grade (leaseless-agent watchdogs, frame ordering, idempotency keys, per-agent dialects, model/config option plumbing) | Every adapter is a different implementation of the same spec; the spec says `session/update` but adapters differ in tool-call shape and stop reasons |
| **Claude stream-json** | 2–3 days | **1.5–2 weeks** (cloudroom: 1780 L, but ~40% is plan/usage/credit/fast-mode bookkeeping we may not want) | Control-request bookkeeping: answering `can_use_tool`/`hook_callback` correctly, mid-turn interrupt state, and knowing when a turn is really over (`result` vs `control_response` interleaving) |
| **Codex app-server** | 2 days (schema gives honest types) | **3–5 days** | `experimentalApi` gating (methods vanish without it), approval replay/matching, `thread/fork` with `lastTurnId`, and version drift between our vendored schema and the user's binary |
| **Pi RPC** | 1 day | **2–3 days** (cloudroom 900 L incl. version gate, extension injection, thinking levels) | JSONL framing (no `readline`: U+2028/U+2029 split frames — t3code and Pi's own docs both say so), session-file ownership, injecting our MCP shim as a Pi extension |
| **Cursor** | 1 day (ACP) / 2–3 days (print mode) | **4–6 days** via ACP only; print-mode lane open-ended | Two lane divergence; `--resume` chat ids live in `.cursor/chats/<md5>/<id>`; sibling implementations reported a copy-the-SQLite-store bug that filled a 50 GB disk (#606 per the cloudroom report) |
| **Antigravity** | 4 hours (ACP) | **1–2 days** (cloudroom 323 L) | `session/resume` vs `session/new` capability checks, Google/Gemini-key auth paths, plan restrictions; packaging (Google-distributed binary or `agy` install) |
| **OpenCode** | 4 hours (ACP) | **1–2 days** | v1/v2 divergence, per-session question tool and permission ceiling (only `full`) |
| **Gemini CLI** | 4 hours (`--acp`) | **0.5–1 day** | Vendor deprecation risk (see §1) |

Aggregate: a runner that drives **all eight at parity through ACP adapters + native Claude/Codex** is
**6–9 engineer-weeks** for one experienced Rust dev, of which ~2 weeks is the supervisor/state machine
(not protocol). Compare: t3code spent, by its own size, ~2.2M lines across 5 apps — but its protocol
drivers are a small fraction of that; the cost centre there is UI, CQRS and 137 migrations.

## 5. Risks specific to Rust (what we pay that the TS references did not)

1. **No official types for the two richest protocols.** Claude: none in any language except TS/Py SDK
   source. Codex: a crate exists but is 0.63.0 against a 0.171.0 binary. Mitigation: vendor Codex schema via
   `generate-json-schema`; hand-model Claude from the TS reference and pin the CLI version we support
   (`claude --version` gate, like cloudroom's `PI_MINIMUM` and `claude_version.rs`).
2. **Silent drift, no compile-time signal.** If Claude adds a `control_request` subtype we don't know, we
   must fail *loudly* — unhandled control requests must answer `deny` + log the frame, never hang. This is
   the single most likely production bug class.
3. **Adapter licences and runtime weight.** `claude-acp` is listed **proprietary** in the ACP registry while
   its repo is Apache-2.0 — resolve before shipping. `codex-acp`/`pi-acp`/`claude-agent-acp` are Node
   processes: `npx` cold start (seconds), a second runtime to detect/update, and a second place for errors.
   AOE pins exact versions and ships a `doctor --fix`; we would need the same.
4. **Debugging a subprocess from a compiled binary.** No `console.log`, no dynamic import shims. We need
   desperately boring tooling: per-thread JSONL frame logs (redacted), child stderr capture (the ACP crate
   caps it at 64 KiB — enough for a Node stack trace), and a frame inspector. ACP gives us one for free:
   `agent-client-protocol-conductor` (a proxy/snoop CLI in the SDK repo) can sit between us and an agent.
   cloudroom's approach is worth copying: raw provider frames go to rotating logs, never to the DB.
5. **Cross-compilation and packaging.** AOE ships 4 targets and skips Windows. cloudroom refuses to run its
   service on non-Unix (its `Cargo.toml` gates the whole service on `cfg(unix)`). If Windows is in scope,
   budget real time: process groups, job objects, path handling, and signals are all different.
6. **Crash reporting.** No JS `Error.captureStackTrace` + source maps. `sentry-rust`/`tracing` work but
   a panic in a supervisor thread is a *thread*, not a process — set `panic = "abort"` deliberately or
   isolate per-thread tasks; otherwise a poisoned mutex hides a dead agent.
7. **Iteration speed.** Rust rebuilds (2–30 s) vs `tsx watch`. Mitigate with workspace layout + `cargo
   check` in a loop + keeping protocol code in one small crate that builds in <5 s.
8. **Toolchain floors.** `sqlx 0.9.1` wants rust 1.94 while `rusqlite 0.43` and `agent-client-protocol`
   want 1.88/1.89. Choosing `sqlx` raises the floor for everyone (CI, contributors, packaging images).

## 6. Recommended architecture

- **One `runner` binary, one process per (thread, agent).** No in-process agent SDK. Supervisor owns:
  spawn/env/cwd, process group kill, stdin writer, stdout frame reader, turn state machine, persistence.
  Per-thread `tokio` task pair (`writer`, `reader`) + one supervisor task; a killed thread cleans the
  process group (`kill(-pgid)` or the ACP crate's `AcpAgent` shutdown ladder).
- **Two adapter traits, one canonical event enum.**
  `trait NativeAgent { fn capabilities() -> Caps; async fn start_turn(); async fn steer(); async fn
  interrupt(); async fn resume(); }` for Claude/Codex; `trait AcpAgent` re-exported from the crate for the
  six ACP agents. Both emit the *same* `ThreadEvent` enum — model it on t3code's `ProviderAdapterV2Event`
  (18 variants, `turn.terminal` with `threadDisposition: reusable|broken`), trimmed to what we render.
  Capability flags, never driver-name branching: `supports_fork`, `supports_compaction`, `supports_steer`,
  `permission_modes`, `reasoning_levels`.
- **Persistence: `rusqlite` + WAL, one writer task, projections as source of truth.** Tables: `thread`,
  `run`, `run_attempt`, `turn_item`, `provider_session`, `permission_request`, `frame_log_meta`. Do not
  event-source the raw frames into SQLite — append them to per-thread JSONL files (gzip on rotation), keep
  SQLite for state.
- **Message types.** Envelope `{thread_id, run_id, seq, ts, payload}`; payload is `ThreadEvent`. All ids
  are app ids; provider ids (`provider_thread_id`, `provider_turn_id`, session id) are refs on the side.
  Every control response carries the request id back — one shared helper, no ad-hoc writes.
- **Swappable adapters = one file per agent + one registry entry** `{id, binary, args, protocol_kind,
  version_gate, caps}`. Adding an agent should be data + one of two trait impls. Pin adapter package
  versions in that registry and expose a `doctor` subcommand that verifies each one, like AOE's.
- **Shared context between threads.** Do it *outside* the agents: a runner-owned MCP server (stdio, and
  MCP-over-ACP where supported) injected into every spawn (`--mcp-config` for Claude, `--mcp-servers`/Pi
  extension, ACP `mcpServers` in `session/new`). That is exactly how t3code does it (~90 MCP tools) and it
  keeps the feature agent-agnostic.

## 7. One-day spike

Goal: de-risk the biggest unknown — *does a Rust process drive a real agent, end to end, with a permission
round-trip and a clean interrupt, and what does it feel like to debug?* Do it twice: once ACP, once native
Claude. Ship a written answer, not a product.

```bash
# 0. Setup (no repo writes)
mkdir -p /tmp/ilvuno-spike && cd /tmp/ilvuno-spike
cargo new acp-spike && cd acp-spike
cargo add agent-client-protocol@5 --features process
cargo add tokio --features rt-multi-thread,macros,process,io-util
cargo add anyhow

# 1. Copy the SDK's own example as the skeleton (verified to exist this session)
#    https://raw.githubusercontent.com/agentclientprotocol/rust-sdk/main/src/agent-client-protocol/examples/yolo_one_shot_client.rs
#    Swap: AcpAgent::from_str("<command below>")

# ACP leg — Codex through its adapter (Apache-2.0, npm):
cargo run -- --command "npx -y @agentclientprotocol/codex-acp@2.9.0" \
  "Create /tmp/ilvuno-spike/out/hello.txt containing 'ok'. Report the tool call you made."

# ACP leg — Claude through its adapter (prebuilt binary from the registry entry, or npm):
cargo run -- --command "npx -y @agentclientprotocol/claude-agent-acp@0.109.1" \
  "Create /tmp/ilvuno-spike/out/hello.txt containing 'ok'."

# 2. Native leg — Claude stream-json, no SDK, no crate
claude -p --input-format stream-json --output-format stream-json --verbose \
  --include-partial-messages --replay-user-messages \
  --permission-prompt-tool stdio --session-id spike-$(date +%s) --no-chrome 2>err.log
#   then paste one user message line on stdin:
#   {"type":"user","message":{"role":"user","content":"Create /tmp/ilvuno-spike/out/hello2.txt containing 'ok'"}}
#   capture: does a control_request{can_use_tool} arrive, and does answering
#   {"type":"control_response","response":{"subtype":"success","request_id":"<id>",
#    "response":{"behavior":"allow","updatedInput":{...}}}} let the turn finish?
#   then send {"type":"control_request","request_id":"9","request":{"subtype":"interrupt","cancel_queued":true}}
#   and confirm a control_response + result.

# 3. Native leg — Codex app-server, and the drift answer
codex app-server generate-json-schema --out /tmp/ilvuno-spike/codex-schema
ls /tmp/ilvuno-spike/codex-schema                      # schema for THIS installed version
codex app-server generate-json-schema --out /tmp/ilvuno-spike/codex-schema-x --experimental
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"spike","version":"0"},"capabilities":{"experimentalApi":true}}}' \
  | codex app-server --listen stdio:// | head -3      # expect an initialize result, not a crash
```

**Success criteria (all must hold, else the answer is "not viable as scoped"):**

1. ACP leg: `initialize` returns `agent_info`; `session/new` returns a session id; `session/update`
   notifications stream text; the agent's file write actually appears in `/tmp/ilvuno-spike/out/`.
2. ACP leg: a `session/request_permission` arrives and is answered by `PermissionOptionKind` (never by
   option position) — and the turn completes after `allow`.
3. Native Claude leg: a `control_request` with `subtype:"can_use_tool"` is observed and answered; the turn
   ends with `result`; an `interrupt` control request produces a prompt `control_response` and a turn
   termination within 5 s.
4. Native Codex leg: `initialize` response captured; `generate-json-schema` produced a non-empty bundle for
   the installed version; diffing that bundle against a second install (or after `codex upgrade`) is a
   mechanical step, not a rewrite.
5. **Debug leg:** we can reconstruct a failed session from logs alone — child stderr + our frame log spans
   the whole turn — without re-running.
6. Timing recorded: cold start per adapter (expect 1–5 s for `npx`), first token, turn end.

If criterion 3 or 6 fails, the gap is Claude-native — then the decision is "ACP-only for Claude" and the
Claude effort in §4 collapses to 1 day.

## 8. What I could not verify

- **`/compact` as a prompt in `-p` mode.** Only implementation evidence (cloudroom `claude.rs:499-500`
  sends `{"type":"user","content":"/compact"}`). No CLI reference or SDK doc states it. `--auto-compact`,
  `autoCompactWindow` and `compact_boundary` messages are documented; the manual than one is not.
- **`agent-client-protocol-conductor` CLI flags.** The binary exists in the SDK repo (`src/agent-client-protocol-conductor`)
  with `snoop.rs`/`trace.rs`, but I did not read its arg surface — do not paste it into docs as fact yet.
- **Cursor ACP docs body.** `cursor.com/docs/cli/acp` returns 200 and the title is "ACP | Cursor Docs", but the
  rendered body did not extract. `cursor-agent acp` is confirmed only via the ACP registry's launch args.
- **Gemini CLI `--acp` still working.** Registry (v0.70.0) says yes; `geminicli.com/docs/cli/acp` 404s and the
  site banner says Gemini CLI was replaced by Antigravity CLI on 2026-06-18 for unpaid/Google One users.
- **`claude-agent-acp` licence.** Registry says `proprietary`; the GitHub repo is Apache-2.0. Unresolved.
- **Parity of `codex-acp` 2.9.0 vs native app-server.** I did not read its source; assumed lossy for
  approvals/experimental methods.
- **`codex-app-server-protocol` 0.63.0 vs codex 0.171.0 API shape.** I read `src/lib.rs` on `main` (current)
  and the crate metadata (0.63.0, single version). I did not diff them.
- **AOE's per-area line counts.** Estimated from GitHub blob sizes ÷ 38 (includes tests). cloudroom numbers
  are exact `wc -l`.
- **crates.io download counts as popularity.** `agent-client-protocol` at 6.6M vs 248 GitHub stars is
  implausible organically (bot/CI inflation is common); treat as "shipped", not "loved".
- **Whether Pi RPC mode is covered by a stability promise.** `rpc.md` documents it as the
  language-independent interface but states no compatibility commitment; AOE/cloudroom both gate on
  `pi --version`.
- **Windows viability for every lane.** No Rust precedent found that ships Windows (AOE: 4 unix targets;
  cloudroom: `cfg(unix)` service). Not investigated beyond that.
- **Real cold-start numbers for the ACP adapters.** §7 criterion 6 is designed to measure them; nothing
  measured here.
