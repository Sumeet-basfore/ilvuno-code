# 09 — Spike results

What was actually run, on this machine, with the versions below. Everything here is a measurement, not
an estimate. Date: 2026-11-23.

| | |
|---|---|
| OS | CachyOS, kernel 7.1.8, X11 |
| Rust | `1.100.0` (the `stable` toolchain is broken here — see §4) |
| Claude Code | 2.1.329 |
| Codex | `codex-cli 0.171.0` |
| ACP crate | `agent-client-protocol 5.5.1` (built in 37.5 s with tokio) |
| Adapters | `@agentclientprotocol/codex-acp@2.9.0`, `@agentclientprotocol/claude-agent-acp@0.109.1` |

---

## 1. Claude from Rust — permission round-trip: **pass**

No SDK. A plain Python process speaking JSONL on the CLI's stdin/stdout:

```
claude -p --input-format stream-json --output-format stream-json --verbose \
       --permission-prompt-tool stdio --permission-mode default --max-budget-usd 0.10
```

Sequence observed:

```
→ control_request initialize            → control_response success
system init
assistant thinking · assistant tool_use
← control_request  can_use_tool
    tool=Write  input={"file_path":"/tmp/ilvuno-spike/hello.txt","content":"ok"}
    → allow with updatedInput
result is_error=False turns=2 cost=$0.0132 duration=4141ms
```

The file was written with the exact content. **A permission prompt is a JSON object on a pipe.** That
was the one assumption that could have changed the architecture; it holds.

## 2. Claude — interrupt: **pass**

Sent `control_request{"subtype":"interrupt"}` six seconds into a long turn:

```
--- interrupt ---
→ control_response success (id=int-1)
result is_error=True turns=2 cost=$0.0118 duration=5988ms
```

Acknowledged and terminated. Same channel, one object.

**Two details that cost an hour and are now documented:** the session id must be a UUID (`--session-id`
rejects anything else, and the process exits before you can write to it), and the CLI silently
auto-denies every tool when the `initialize` control request is missing. There is no error — the turn
just completes having done nothing. The SDK sends `initialize` first; anything else must too.

## 3. ACP from Rust — **pass, first build**

`cargo add agent-client-protocol@5 --features process`, ~40 lines of client, pointed at
`npx -y @agentclientprotocol/codex-acp@2.9.0`. Observed in one run:

```
agent: @agentclientprotocol/codex-acp 2.9.0
session: 01a204cb-…
AgentMessageChunk × n            (streamed text)
ToolCall: exec_command "printf 'ok' > /tmp/ilvuno-spike/acp-ok.txt"
ToolCallUpdate: status Completed
UsageUpdate: used 27479 / size 258400
stop_reason: EndTurn
```

The file existed with the right contents afterwards. So: a Rust binary spawned a shipped agent
adapter over JSON-RPC, streamed its output, saw the tool call complete, and got a stop reason. One
dependency, no vendor SDK, 37 seconds to compile.

## 4. Codex schema generation — **pass, and needed**

`codex app-server generate-json-schema --out DIR` on 0.171.0 wrote **39 files**, including
`ClientRequest.json`, `ApplyPatchApprovalParams.json` and `ClientNotification.json`.

This confirms the decision to generate types from the installed binary instead of the published crate:
the crate is 0.63.0 (2025-12-11) against a 0.171.0 runtime.

## 5. Tauri shell — **not run**, deliberately

Needs `webkit2gtk-4.1` (not installed on this box) and a sudo install. It changes nothing about the
runner: the transcript wire is loopback HTTP either way, and the shell is the part we would replace
last. Deferring it is a decision, not an omission.

## 6. Environment issues found

| Issue | Impact | Workaround |
|---|---|---|
| `rustup` default toolchain `stable` is corrupt (`missing manifest`) | `cargo` fails on every project | `RUSTUP_TOOLCHAIN=1.100.0`, or reinstall `stable` |
| `webkit2gtk-4.1` absent | Tauri cannot run | install before the UI spike |
| Claude's `initialize` handshake is undocumented in `--help` | silent tool denial | always send it first |

## 7. One thing the spike revealed that is not about protocols

Codex's first message through ACP included:

> *"Exceeded skills context budget. All skill descriptions were removed and 3486 additional skills were
> not included."*

Three thousand four hundred and eighty-six skills. That is the local skill store being advertised into
every agent session. It eats context on every turn, and it is exactly the kind of ambient cost our ledger
and budget rules exist to make visible. Worth measuring before we ship anything: how many tokens per turn
does the environment inject before the user has said anything.

---

## What this changes

Nothing structural. D1 (Rust runner), D2 (ACP + native tiers) and D4 (goal run) all survive contact with
reality. Two protocol details are now known rather than assumed, and three environment problems are known
rather than latent.

## What it unlocks

The next artifact is not a UI. It is a headless runner plus a CLI that can execute one research goal
end to end:

```
ilvuno run --goal goal.json      # plan → dispatch → work → report → gate
```

The UI is a view onto that, and building it first would mean building it against imagined state.
