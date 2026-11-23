# Provider contract: plain-language notes

Source: `/home/sumeet/personal-repo/projects/ilvuno-second/docs/provider-contract.md`

## What it covers

The contract says what an adapter (the piece that talks to an agent CLI) must do, and what the runner promises in return. Version 1 has two adapters:

- **`claude`**: native stream-json plus a control protocol.
- **`acp`**: the `agent-client-protocol` crate, pointed at any agent's ACP command.

Both were proven in `research/09-spike.md`.

**Design rule: zero first-party privilege.** If `claude` needs something the ACP adapter cannot also get, it goes into the shared capability struct or it does not exist.

## The boundary

The runner sends commands to the adapter. The adapter talks to the CLI process over stdio or HTTP and sends events back. The adapter is a translator, not a decision-maker. It never writes the database, never decides a round is done, and never retries a turn on its own. If it cannot answer a question, it emits `unknown` and the runner decides.

## Commands (runner to adapter)

- `spawn`: start the process. Reply `spawned {pid}` or `failed {reason}`.
- `session_open`: create a native session. Reply `session {native_id}`.
- `session_resume`: reattach. Reply `session {native_id}` or `resume_failed {reason}`.
- `prompt`: one user turn. Streams events and ends with `turn_finished`.
- `permission_reply`: answer a permission prompt. Ack only.
- `interrupt`: stop mid-turn. Ack, then `turn_finished{interrupted}`.
- `park` / `unpark`: free the process but keep the session id. Ack.
- `shutdown`: clean exit. Ack, then process exit.

**Rule:** every command carries a `request_id`. Sending the same id again must not produce a second turn.

## Events (adapter to runner)

There are eleven event kinds: `spawned`, `session`, `text_delta`, `thinking_delta`, `tool_call`, `tool_result`, `permission_request`, `usage`, `turn_finished`, `session_ended`, and `raw`.

Anything a CLI emits that does not fit goes into `raw` with the provider's original payload, so no information is lost.

`text_delta` is rendered and then discarded into the aggregate message. `tool_call` and `permission_request` are persisted, because they are evidence of what happened.

## Capabilities

Capabilities are declared, not probed ad hoc: resume, fork, rewind, interrupt, steer, approvals, structured output, usage, and the list of models. The runner greys out anything absent and never imitates a missing feature.

## Permission flow

- **Claude:** the client must send `control_request{"subtype":"initialize"}` first. Without it, every tool is silently denied. Permission requests arrive as `can_use_tool` and are answered with `allow` or `deny`.
- **ACP:** the agent calls `session/request_permission`. The client picks an outcome by option **kind** (prefer `allow_once` over `allow_always`), never by position.
- **ACP needs a mode that asks.** The default `agent` mode of `codex-acp` reviews its own risky actions, so the runner never sees a request and its policy is bypassed. Set `"mode": "workspace-write"` on the provider entry.
- Both wires map to one runner event, `permission_request {tool, input, options}`. The runner's policy decides. The adapter only translates.

## Prompts

Two prompt texts ship with the runner, versioned in code under `crates/runner/src/prompts/`. They are injected every turn, not once per session, because agents lose them. The journal records which version was injected into each task.

- A normal thread gets the **worker** text. The thread named `orchestrator` gets the **orchestrator** text.
- Worker preamble (`worker-preamble/4`) tells the worker: its thread, coordinator, task id, and round; its folder, which is the only place it may write; that it must read the board before starting and before reporting; the message policy; and that it must log intent before touching any file, ask when it needs a decision, report exactly once when finished, and not spawn agents or poll.
- Orchestrator preamble (`orchestrator-preamble/1`) tells the orchestrator to plan and delegate only. Every task needs a prompt and a scope, and a task without a scope is refused. Sources in a scope must be absolute paths. If two workers would touch the same file, the plan must change. The orchestrator cannot open the gate; the checks and the human decide. When a worker reports a failure, it must not mark the task done.

## Worker CLI surface and reads

- A worker may run only `ilvuno note` (kinds: intent, finding, question, answer; scope must stay inside the task's scope) and `ilvuno report`, both for its own task id.
- The orchestrator may also run `ilvuno task --thread <name>`.
- Everything else is refused by policy.
- Reads: a thread may read its own folder, the run folder (board and every thread's files), and any absolute path in its task's scope.
- Writes: only inside its own folder.
- Commands are injected with the runner binary's absolute path, because a Codex worker's login shell `PATH` may differ.

## Provider table for v1

| Instance | Wire | Approvals | Resume | Interrupt | Structured output |
|---|---|---|---|---|---|
| `claude-main` | stream-json + control | yes | yes (`--resume`) | yes | `--json-schema` |
| `codex-main` | `codex app-server` JSON-RPC | yes | yes (`thread/resume`) | yes | yes |
| `acp:*` | ACP over stdio | yes, only in `workspace-write` or `read-only` | depends on agent | yes | no |

The `claude-main` initialize handshake is mandatory. `acp:*` adapters vary in quality and are tier 3 unless tested. PTY scraping is not in v1: an agent with neither ACP nor a protocol does not ship.

## Error handling rules

| Class | Runner behaviour |
|---|---|
| `spawn_failed` | Mark the instance unavailable. Do not retry blindly. |
| `auth_required` | Pause the run, tell the human, keep branches. |
| `usage_limit` | Distinct from failure. Pause and offer an account switch. |
| `turn_failed` | One retry, then `dispatch.failed`. |
| `protocol` | Log `raw`, mark the session broken, do not reuse it. |
| `unknown` | Record `unknown`. Never silently resend. |

An adapter crash must never take the runner down. Adapters are child processes, and their death counts as `turn_failed{reason:"adapter_died"}`.

## Things a reader must not assume

- `/tmp` is not a boundary. It is writable in `workspace-write` mode by default.
- An agent that is not ACP or a protocol is not supported in v1.
- The adapter never makes runner decisions (retries, round completion, DB writes).
