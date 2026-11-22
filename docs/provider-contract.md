# Provider contract

What an adapter must implement, and what the runner promises it. Two adapters exist in v1: `claude`
(native stream-json + control protocol) and `acp` (the `agent-client-protocol` crate, pointed at any
agent's ACP command). Both were proven in `research/09-spike.md`.

Design rule, from cloudroom: **zero first-party privilege.** If `claude` needs something the ACP adapter
cannot also get, it goes in the shared capability struct or it does not exist.

---

## The boundary

```
runner ──command──▶ adapter ──wire──▶ CLI process
       ◀──event──── adapter ◀──────  (stdio or HTTP)
```

The adapter is a translator, not a decision-maker. It never writes the DB, never decides a round is done,
never retries a turn on its own. If it cannot answer a question, it emits `unknown` — the runner decides.

## Commands (runner → adapter)

| Command | Meaning | Required response |
|---|---|---|
| `spawn(instance, cwd, env)` | start the process | `spawned {pid}` or `failed {reason}` |
| `session_open(thread)` | create a native session | `session {native_id}` |
| `session_resume(thread, native_id)` | reattach | `session {native_id}` or `resume_failed {reason}` |
| `prompt(turn_id, text, attachments)` | one user turn | stream of events, ending in `turn_finished` |
| `permission_reply(request_id, allow, updated_input, reason)` | answer a prompt | ack; the wire handles the rest |
| `interrupt(turn_id)` | stop mid-turn | ack, then `turn_finished{interrupted}` |
| `park(thread)` / `unpark(thread)` | free the process, keep the session id | ack |
| `shutdown` | clean exit | ack, then process exit |

Idempotency: every command carries a `request_id`. Re-sending the same id must not produce a second turn.

## Events (adapter → runner)

Eleven kinds. Anything a CLI emits that does not fit goes into `raw` with the provider's original payload,
so we never lose information we did not model yet.

| Event | Payload |
|---|---|
| `spawned` | pid, version, agent name |
| `session` | native id, model catalog, capabilities |
| `text_delta` | text |
| `thinking_delta` | text |
| `tool_call` | id, name, input, kind, status |
| `tool_result` | id, output (truncated), status |
| `permission_request` | request id, tool, input, options |
| `usage` | tokens in/out, context used/size, cost if known |
| `turn_finished` | stop reason, interrupted?, error? |
| `session_ended` | reason |
| `raw` | provider payload, for diagnostics only |

`text_delta` is rendered and discarded into the aggregate message; `tool_call` and `permission_request`
are persisted because they are evidence of what happened.

## Capabilities

Declared, not probed ad hoc. The runner greys out what is absent; it never imitates a missing feature.

```rust
struct Capabilities {
    resume: bool,        // session survives a restart
    fork: bool,          // branch a conversation
    rewind: bool,        // revert files with the conversation
    interrupt: bool,
    steer: bool,         // inject a message into a running turn
    approvals: bool,     // tool permission round-trip
    structured_output: bool, // JSON-schema constrained
    usage: bool,         // token/cost reporting
    models: Vec<Model>,  // id + reasoning levels + context window
}
```

## Permission flow — the two wires

**Claude.** `claude -p --input-format stream-json --output-format stream-json --verbose
--permission-prompt-tool stdio`. The client **must** send
`control_request{"subtype":"initialize"}` first; without it every tool is silently denied. Then
`control_request{"subtype":"can_use_tool"}` arrives with `{tool_name, input}` and is answered with
`control_response{behavior:"allow"|"deny", updatedInput}`. `interrupt` uses the same channel. Verified.

**ACP.** The agent calls `session/request_permission`; the client answers with a `RequestPermissionOutcome`
selected by option **kind** (`allow_once` preferred over `allow_always`), never by option position.
Verified against `codex-acp 2.9.0`.

Both map onto one runner event: `permission_request {tool, input, options}`. The runner's policy decides,
the adapter only translates.

## Prompts are part of the contract

Because a prompt is the only interface every agent understands. Two texts ship with the runner, versioned
in code, and are injected every turn — not once per session, because agents lose them.

**Worker preamble** (adapted from orca's `worker-dispatch-preamble`, the design that survives
uninstrumentable CLIs):

```
You are a worker in Ilvuno, a multi-agent runner. Your coordinator is thread <id>.
Your task id is <task_id>. Your round is <round_index>.
Your branch is <branch>. Work only inside your worktree: <path>.

Board: read `<run>/board.md` before you start and before you report.
Message policy for this run: <round|anytime|never>.

Rules:
- Announce your intent on the board before you touch a file.
- Write only inside your worktree. Never switch branches. Never merge.
- When you need a decision you cannot make: write a `question` to the board and stop your turn.
- When you finish: report exactly once with `ilvuno report --task <task_id> --status ok|failed
  --summary "<3 sentences>" --files "<paths>"`.
- Do not spawn other agents. Do not poll for work. You will be woken.

=== TASK ===
<task text>
```

**Orchestrator contract** (adapted from t3's, which is the version that works):

```
You plan and delegate. You do not write code or prose deliverables yourself.
Before dispatching, write one `intent` per worker naming the files or sources it will own.
Dispatch only as many workers as the goal needs; prefer 2–4.
Each dispatch must name a scope. A dispatch without a scope is refused by the runner.
When a worker reports a failure, do not mark it done. Ask, or escalate to the human.
If two workers will touch the same file, change the plan — never let them race.
When all dispatches are reported, produce the round verdict: pass, pass_with_notes, or block.
Follow the run's budget. If you cannot make progress, stop and say so.
```

Both texts live in `crates/runner/src/prompts/` with a version constant; the journal records which
version was injected into which thread.

## Provider table for v1

| Instance | Wire | approvals | resume | interrupt | structured output | notes |
|---|---|---|---|---|---|---|
| `claude-main` | stream-json + control | yes | yes (`--resume`) | yes | `--json-schema` | initialize handshake mandatory |
| `codex-main` | `codex app-server` JSON-RPC | yes | yes (`thread/resume`) | yes | yes | types generated from the binary |
| `acp:*` | ACP over stdio | yes (`request_permission`) | depends on agent | yes | no | adapter quality varies; tier-3 unless tested |

Not in v1: PTY scraping. If an agent has neither ACP nor a protocol, it does not ship — labelled, not faked.

## Error taxonomy

| Class | Example | Runner behaviour |
|---|---|---|
| `spawn_failed` | binary missing, bad args | mark instance unavailable, do not retry blindly |
| `auth_required` | logged out, token expired | pause the run, tell the human, keep branches |
| `usage_limit` | subscription exhausted | distinct from failure; pause, offer account switch |
| `turn_failed` | non-zero exit mid-turn | one retry, then `dispatch.failed` |
| `protocol` | unparseable frame | log `raw`, mark session broken, do not reuse |
| `unknown` | no confirmation either way | record `unknown`; never silently resend |

A crash in the adapter must never take the runner with it: adapters are child processes, and the runner
treats their death as `turn_failed{reason:"adapter_died"}`.
