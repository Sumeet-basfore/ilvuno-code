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

**ACP needs a mode that asks.** `codex-acp` defaults to its `agent` mode, which reviews its own risky
actions internally — the client never sees a request, so the runner's policy is bypassed. Set
`"mode": "workspace-write"` on the provider entry (passed to the adapter as `INITIAL_AGENT_MODE`).
Verified: a write outside the session cwd raises a permission request. In that mode the agent writes
freely inside its cwd, which is the thread's own folder, so the sandbox and our policy agree. `/tmp`
is writable in that mode by default — do not treat `/tmp` as a boundary.

Both map onto one runner event: `permission_request {tool, input, options}`. The runner's policy decides,
the adapter only translates.

## Prompts are part of the contract

Because a prompt is the only interface every agent understands. Two texts ship with the runner, versioned
in code (`crates/runner/src/prompts/`), and are injected every turn — not once per session, because agents
lose them. The journal records the version injected into each task (`prompt.injected`).

A thread gets the **worker** text, or the **orchestrator** text when it is the thread named `orchestrator`.
The goal's `threads` entry supplies the thread's standing job; the task supplies what to do now and the
scope. Both texts are the same text the code uses.

**Worker preamble** (`worker-preamble/5`, adapted from orca's `worker-dispatch-preamble`, the design that
survives uninstrumentable CLIs):

```text
You are a worker in Ilvuno, a multi-agent runner. Your thread is <thread>. Your coordinator is <coordinator>.
Your task id is <task_id>. Your round is <round_index>.
Work only inside your folder: <path>. You may read the run folder <run>.

Board: read `<run>/board.md` before you start and before you report.
Message policy for this run: <round|anytime|never>.

Rules:
- Before you touch a file, say so: `<bin> note --task <task_id> --kind intent --body "what you are about to do"`.
- Write only inside your folder. Never switch branches. Never merge.<checkout>
- When you need a decision you cannot make, ask and stop your turn:
  `<bin> note --task <task_id> --kind question --body "the question"`.
- When you finish: report exactly once with `<bin> report --task <task_id> --status ok|failed
  --summary "<3 sentences>" --files "<paths>"`.
- Do not spawn other agents. Do not poll for work. You will be woken.

=== TASK ===
<task text>
```

`<checkout>` is empty in folder mode. With a git worktree it becomes:
`- This is a real git checkout on branch <branch>. Do not switch branches. Every change you want to keep must be left in the working tree.`

**Orchestrator preamble** (`orchestrator-preamble/1`). The orchestrator plans and delegates through the
same `ilvuno task` command the user uses. It has no authority over the gate.

```text
You are the orchestrator in Ilvuno, a multi-agent runner. Your thread is <thread>.
Your task id is <task_id>. Your round is <round_index>.
Work only inside your folder: <path>. You may read the run folder <run>.

Board: read `<run>/board.md` before you plan and before you report.
Workers on this run: <workers>. The goal folder is <goal folder>.

You plan and delegate. You do not write the deliverable yourself.
Rules:
- Give a worker work with: `<bin> task --thread <worker> --prompt "what to do" --scope "a,b"`.
  Every task needs a prompt and a scope. A task without a scope is refused.
- The scope lists the only files and sources that task may touch. Give a source as an absolute path.
- If two workers would touch the same file, change the plan. Never let them race.
- Ask the human when you need a decision: `<bin> note --task <task_id> --kind question --body "the question"`.
- When your plan is made, report once: `<bin> report --task <task_id> --status ok --summary "<3 sentences>"`.
- You cannot open the gate. The checks decide, and the human decides.
- When a worker reports a failure, do not mark it done. Ask, or escalate to the human.
- Do not spawn other agents. Do not poll for work. You will be woken.

=== TASK ===
<task text>
```

**Worker CLI surface.** A worker may run exactly two commands: `ilvuno note` (`intent` | `finding` |
`question` | `answer`, scope must stay inside the task's scope) and `ilvuno report`, both for its own task id.
The orchestrator may also run `ilvuno task --thread <name> ...`. Everything else, including gate commands
and reads outside the permitted paths, is refused by the runner's policy. If the preamble asks for
something the policy forbids, that is a bug in one of them.

**Reads.** A thread may read its own folder, the run folder (which holds the board and every thread's
files), and any absolute path listed in its task's scope. It may write only inside its own folder.

**Who writes a task.** The task text the thread receives is its standing job, the task prompt, and the
scope. The runner adds the thread's standing job; it does not change the task's words. A task written by
the orchestrator records the orchestrator as its author. A task written by the user has no thread author.
A task the runner writes (a failed round waking the orchestrator) is marked `runner`.

**Environment.** Only the orchestrator's session carries `ILVUNO_TASK` (its own task id), which is how
`ilvuno task` knows who is calling. A user's `ilvuno task` has no such variable and writes to the newest run.

## Provider table for v1

| Instance | Wire | approvals | resume | interrupt | structured output | notes |
|---|---|---|---|---|---|---|
| `claude-main` | stream-json + control | yes | yes (`--resume`) | yes | `--json-schema` | initialize handshake mandatory |
| `codex-main` | `codex app-server` JSON-RPC | yes | yes (`thread/resume`) | yes | yes | types generated from the binary |
| `acp:*` | ACP over stdio | yes (`request_permission`), only in `workspace-write` or `read-only` | depends on agent | yes | no | adapter quality varies; tier-3 unless tested |

**Commands are injected with an absolute path.** The preamble's `<bin>` is the runner binary's full path,
because a Codex worker runs commands in a login shell whose `PATH` is not the runner's. The policy checks
the same absolute prefix.

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
