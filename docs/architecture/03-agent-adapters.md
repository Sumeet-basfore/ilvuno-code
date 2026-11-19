# 03 — Agent Adapters

Status: **post-MVP design; one adapter ships in the MVP** (`command`, §7). The interface is
specified now because it constrains the core, and because a wrong boundary later costs a
rewrite (ADR-0010).

Evidence: Claude and Codex mechanisms below are documented surfaces read at pinned commits
(`T3-012`/`T3-013`, `ORC-015`/`ORC-018`, `CR-008`/`CR-009`). No PTY scraping, no OSC parsing,
no output-text inference is used anywhere in this design.

---

## 1. Why the core must not know about providers

The MVP's product claim is provider-independence (`INT-101`: "any producer"). If the engine
imported an adapter, three things would follow that we do not want:

1. A provider's capability gaps would become the product's capability gaps.
2. Provider version drift would break integration runs.
3. A user's agent choice would be constrained by our release cadence.

So the dependency rule is absolute: `engine` never imports `agent` (doc `01` §3). The engine
consumes two things only: **git refs** and **commands**. Adapters are an optional layer that
*creates* refs and *reports* what happened, post-MVP.

This also means an agent run that Ilvuno launches and an agent run the developer launched by
hand are the same kind of object downstream: an `AgentRun` with `produced_refs[]` observed via
git (§3.4 of doc `02`). That symmetry is why "import" and "launch" can share a model.

---

## 2. Documented integration surfaces (the three that exist)

| Surface | Who documents it | Fidelity | Used for |
|---|---|---|---|
| **Claude Agent SDK** (`@anthropic-ai/claude-agent-sdk`, `query()`) | Anthropic | High: typed messages, `canUseTool`, resume/fork | Claude adapter (post-MVP) |
| **Claude CLI stream-JSON** (`claude -p --input-format stream-json --output-format stream-json`) | Anthropic | High: same protocol as the SDK, no library | Environments without the SDK; observability |
| **Codex app-server** (`codex app-server`, JSON-RPC over stdio) | OpenAI | High: threads, turns, approvals, compaction | Codex adapter (post-MVP) |
| **ACP** (Agent Client Protocol) | ACP schema | Medium-high, per-agent dialects | Long tail: opencode, copilot, kimi, cursor, grok, … |
| PTY + escape sequences | nobody | Low, brittle | **Rejected** (ADR-0010) |

Prior art, and what we take from it:

| Source | Mechanism | Take | Reject |
|---|---|---|---|
| t3code | `ProviderAdapterV2` + normalized event union; providers keyed by instance (T3-009, T3-010, T3-011) | Instance-keyed registry; a normalized event union | 16k-line adapters |
| t3code | Claude via SDK `query()` with `pathToClaudeCodeExecutable` (T3-012) | Always run the **user's** binary | Bundling provider binaries |
| orca | Structured registry that rejects a declared capability without a method (ORC-012) | Capability honesty is enforced structurally | Running both PTY and structured paths |
| orca | TUI/PTY path for ~45 agents (ORC-011) | — | Status inference from screen output |
| cloudroom | Private `Adapter` trait: encode/receive/respond/capture/close (CR-007) | Narrow trait; protocol differences live in the adapter | Full-access-by-default execution (CR-008/009) |
| cloudroom | Native subagents disallowed, delegation routed to child sessions (CR-057) | Post-MVP consideration for delegation | — |

---

## 3. Authentication boundary (the rule that matters)

**Ilvuno is never a credential custodian.** It does not read, store, refresh, proxy, or
transmit provider credentials. It executes the user's own installed CLI/SDK, which
authenticates itself through whatever mechanism the provider supports locally.

That yields four rules:

| Rule | Consequence |
|---|---|
| `AUTH-1` No credential fields exist in any Ilvuno schema | A secret cannot leak through our artifacts because there is no slot for one (`INV-13`) |
| `AUTH-2` Never call a provider's HTTP API on the user's behalf | We cannot become an unlicensed reseller of someone's subscription, and cannot be blamed for quota consumption |
| `AUTH-3` A consumer subscription is **not** an API entitlement | An adapter must not assume a logged-in Claude/Codex CLI implies API access for any server-side feature; anything server-side is a separate, explicit decision (ADR-0015) |
| `AUTH-4` Auth state is probed, never inspected | `auth_state ∈ {ok, needs_login, unknown, unsupported_env}`, derived from the CLI's own behaviour (a typed failure), not by reading credential files |

`unsupported_env` matters: a headless CI container usually has no interactive login, so an
adapter that requires one must refuse with a typed error rather than retry or silently
degrade. This is the same honesty rule as `unverified` verification (doc `07` §2).

Anti-patterns we will not implement, each observed in the survey: exporting a subscription
token into the child environment where tool subprocesses inherit it (CR-038); copying one
machine's login files to another host (CR-042); storing keys as plaintext when secure storage
is unavailable (ORC-074).

---

## 4. Adapter interface

Deliberately small. Anything not here is a provider quirk that stays inside the adapter.

```go
type Adapter interface {
    // Identity and honesty
    Info() ProviderInfo                  // id, display name, protocol version
    Capabilities(ctx) (Capabilities, error) // declared AND implemented (ORC-012 rule)

    // Lifecycle
    Start(ctx, StartRequest) (Session, error)
    Close(ctx, SessionRef) error

    // Interaction
    Send(ctx, SessionRef, TurnInput) (TurnRef, error)
    Steer(ctx, TurnRef, text string) error        // requires Capabilities.Steer
    Interrupt(ctx, TurnRef) error                 // requires Capabilities.Interrupt
    RespondToPermission(ctx, PermissionRequest, Decision) error

    // Durability (optional per capability)
    Resume(ctx, ResumeRequest) (Session, error)   // requires Capabilities.ResumeAfterRestart
    Fork(ctx, SessionRef, ForkRequest) (Session, error)

    // Observation
    Events(SessionRef) <-chan Event               // normalized union, §5
}
```

```go
type Capabilities struct {
    Steer, Interrupt, Fork, Rollback, Compact bool
    ResumeAfterRestart                         bool
    TypedApprovals                             bool   // vs. implicit/auto
    StructuredEvents                           bool   // false ⇒ adapter refuses to run
    NativeSubagents                            bool
    MaxConcurrentTurns                         int    // 1 unless provider supports parallel
}
```

Structural rule (from orca's registry, ORC-012): a capability set is validated against the
implementation at construction; declaring `Steer` without an implementation is a startup
error. That is the cheapest possible defence against capability drift across provider
versions.

### 4.1 Capability matrix (expected, to be probed at runtime)

| Capability | Claude (SDK/stream-json) | Codex (app-server) | ACP agents |
|---|---|---|---|
| Structured events | yes | yes | yes |
| Typed approvals | yes (`canUseTool`) | yes (`*/requestApproval`) | yes (`session/request_permission`) |
| Steer mid-turn | yes | yes (`turn/steer`) | varies per agent |
| Interrupt | yes | yes (`turn/interrupt`) | yes |
| Resume after restart | yes (`resume`, `resumeSessionAt`) | yes (`thread/resume`) | varies |
| Fork | yes (`forkSession`) | yes (`thread/fork`) | varies |
| Compact | yes | yes (`thread/compact/start`) | varies |
| Native subagents | yes (must be considered for delegation) | yes | varies |

Evidence: T3-009/T3-012/T3-015, T3-013/T3-014, ORC-015/ORC-018/ORC-087, CR-008/CR-009,
CR-057. Capabilities are **probed**, not assumed; the matrix is a starting expectation and a
test fixture, not a contract.

---

## 5. Normalized event stream

One union, mirroring the shape that both prior systems converged on (T3-010, CR-007). Events
are additive-only across versions (ADR-0014).

```
Event =
  | session.created     { session_id, provider, model? }
  | session.updated     { session_id, auth_state, capabilities }
  | turn.started        { turn_id, input_digest }
  | turn.delta          { turn_id, channel: text|thinking|tool, text }   // partial
  | tool.requested      { turn_id, tool, args_redacted, permission_state }
  | tool.completed      { turn_id, tool, ok, result_digest }
  | permission.request  { request_id, kind: command|patch|permission, detail }
  | permission.decided  { request_id, decision, by }
  | turn.completed      { turn_id, status: ok|error|cancelled, usage? }
  | session.compacted   { session_id }
  | session.exited      { session_id, reason: ok|crashed|killed|unknown }
  | error               { code, message, retryable }
```

Three rules that make this stream safe to consume:

1. **No provider-specific payloads in `Event`.** A provider's raw frame may be retained in a
   *diagnostic* log (rotating, size-capped) but never in evidence (doc `09` §5).
2. **`turn.delta` is presentation, never truth.** Deltas are not parsed for status.
3. **Only `turn.completed` and git observations produce state.** A completed turn still
   proves nothing about integration (`INV-7`).

---

## 6. Process control, cancellation, resume

### 6.1 Spawn

```
spawn(adapter):
  - resolve the user's binary (never bundle): claude | codex | <acp cmd>
  - env = allowlist ⊕ adapter-specific additions; scrub ambient provider tokens we did not
    create (ORC-079 pattern)
  - stdio = pipes; no PTY (structured protocols only)
  - process group = new group (Setpgid) so cancellation reaches descendants
  - record OwnershipRecord{pid, pgid, start_time_ns, boot_id} before returning
```

Ownership records are what make orphan cleanup safe: after a crash, a restart can decide
whether a PID is still ours by comparing `(pid, start_time, boot_id)` rather than trusting a
bare PID (T3-041 pattern; Linux-only caveat handled by capability probe).

### 6.2 Cancellation

```
Interrupt(turn):
  - send protocol interrupt if Capabilities.Interrupt
  - wait ≤ 2s for turn.completed{cancelled}         (T3-043's bound)
  - else send SIGTERM to the process group; wait ≤ 5s
  - else SIGKILL the group
  - record which rung was used in the AgentRun (evidence of how it ended)
```

Steering degrades explicitly: if a provider lacks `Steer`, the adapter either rejects the
call (default) or performs a typed cancel-and-restart, and **records that it did** (T3-077's
policy, made visible rather than silent).

### 6.3 Resume / reconnection

Two different things, often conflated:

| Concern | What it means | Rule |
|---|---|---|
| Client reconnection | the observer re-attaches to a live session | replay events from `seq`; snapshot if the gap exceeds bounds (T3-022) |
| Provider resume | a *new* process continues an old provider conversation | only if `ResumeAfterRestart`; validated by the provider (T3-009 `canResumeAfterRestart`, CR-016 identity check) |
| Task continuity | the *task* survives without any session | the Passport; never a transcript (`TSK-112`) |

An adapter that cannot resume returns `ErrResumeUnsupported`; the engine then reports
`session_not_resumable` and the task continues without it. That is a first-class outcome, not
an error state to hide.

### 6.4 Error normalization

| Code | Cause | Retryable | User action |
|---|---|---|---|
| `provider_not_found` | binary missing | no | install / configure path |
| `provider_auth_required` | needs login | no | run the CLI's login |
| `provider_unsupported_env` | no interactive auth possible (CI) | no | use a supported auth mode or skip |
| `provider_protocol_mismatch` | handshake/version rejected | no | upgrade |
| `provider_capability_missing` | call requires an absent capability | no | change approach |
| `provider_turn_failed` | provider reported an error | maybe | see message |
| `provider_exited` | process died | maybe | see evidence |
| `provider_timeout` | boot or turn exceeded deadline | maybe | raise timeout |
| `provider_cancelled` | interrupt succeeded | no | — |

Every code maps to an exit-code class so callers never parse strings (doc `12` §6).

---

## 7. The MVP adapter: `command`

The only adapter in the MVP, and it is intentionally dumb. It exists because repair needs to
invoke *something* the developer chose (`INT-502`), without Ilvuno embedding an agent.

```
CommandAdapter
  Config: { argv[] | shell string, timeout, cwd: "candidate", env_allow[], stdin: "brief" }
  Start:  spawn with the repair brief on stdin, cwd = candidate worktree
  Stream: none (line-buffered passthrough to the run log, redacted)
  Capabilities: { Steer:false, Interrupt:true(SIGTERM→KILL), ResumeAfterRestart:false,
                  Fork:false, TypedApprovals:false, StructuredEvents:false }
  Outcome: exit code + duration + produced refs observed via git
```

Consequences, stated plainly:

- We cannot show what the tool "thought". We only know exit code and what git says changed.
  That is consistent with `INV-7` and with the product's refusal to trust claims.
- Because `StructuredEvents:false`, the command adapter is **not** eligible for future
  dispatch use without a real adapter. It is a repair executor, not an agent runtime.
- Approvals are impossible here: `TypedApprovals:false` means the command adapter must be run
  with whatever permission posture the user configured in their tool. Ilvuno's contribution is
  isolation (worktree) + evidence, not permission mediation. This is the honest boundary and
  it is why full-access-by-default (T3-060) is not inherited: there is no default, there is a
  user-configured command.

### 7.1 Why not just use the Claude CLI in the MVP

Tempting, and rejected:

1. It would make the first release depend on one provider's version cadence and auth.
2. It invites "Ilvuno runs agents", which pulls the roadmap into dispatch — the commoditized
   segment (product `05` §2.1).
3. `command` already serves the one MVP need (repair), with zero provider surface.

Revisit condition: when a real dispatch requirement appears **and** a metric justifies it
(doc `ADR-0010`).

---

## 8. Security of the adapter layer

| Threat | Control |
|---|---|
| Prompt injection via repository content reaches a tool that then executes | Adapters do not add capability; the provider's own permission layer stays authoritative. Ilvuno never auto-approves (`TypedApprovals` means *ask*, not *allow*) |
| HTTP MCP endpoints that an agent can be tricked into calling | No agent-facing MCP server in MVP; if added later, scoped expiring tokens per task (T3-063 prior art) |
| Unscoped webhook triggering agent work | No webhooks in MVP; later, signed only (T3-064 anti-pattern) |
| Agent modifies the passport to forge approval | Detectable: digest chain + approval records outside the repo (§3.14 of doc `02`; doc `09` §4.5) |
| Agent reads another worktree's files | Worktrees are sibling directories; an agent with shell access can read them. Documented limitation (doc `09` §7), not a claimed sandbox |
| Token in child env | Allowlist; never copy provider env vars we did not set (`AUTH-1`) |

The last-but-one row is the one to be honest about: **worktree isolation is not a security
boundary against a shell-capable agent**. It is a correctness boundary (no interference) and
a blast-radius reducer. Real isolation is a container/VM question that the product does not
claim (ADR-0008, doc `09` §7).

---

## 9. Failure scenarios specific to adapters

| Scenario | Expected behaviour |
|---|---|
| Provider binary present, not logged in | `provider_auth_required`; no retry; the run is `failed` with a clear action |
| Provider upgraded, protocol changed | `provider_protocol_mismatch` at handshake; adapter refuses; evidence records versions |
| Provider supports steering but the agent ignores it | `turn.completed{status: cancelled}` is required; a silent no-op becomes a timeout error, not a guess |
| Session resumed on another machine | adapter returns `ErrResumeUnsupported`; task continues from Passport (doc `10` §5) |
| Two turns started concurrently on a single-turn provider | adapter serializes or returns `ErrBusy`; the engine never relies on provider concurrency |
| Provider emits a huge line (>16 MiB) | adapter bounds frames and fails the turn with `provider_protocol_mismatch` (CR-036's bound) |
| Process killed externally | `session.exited{reason: unknown}`; `AgentRun.state = lost`; never `completed` |
| Crash of Ilvuno itself mid-turn | on restart: `uncertain`; requires `--decide` (doc `08` §4) |

---

## 10. Open questions

| ID | Question | Recommendation |
|---|---|---|
| `AA-1` | Does the adapter layer belong in the core repo or a plugins directory? | In-repo behind a strict import rule; a separate repo would fragment the build |
| `AA-2` | Normalize `usage` (tokens/cost) across providers? | No in MVP; if added, treat as advisory metadata, never as evidence |
| `AA-3` | Should we support ACP first (breadth) or Claude+Codex (depth)? | Depth: two well-supported adapters beat nine thin ones, given the product is integration not dispatch |
| `AA-4` | How do we keep the capability matrix honest as providers drift? | Nightly conformance job against recorded fixtures + a `doctor` probe; drift becomes a test failure, not a user report |
