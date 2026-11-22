# Orca — reference implementation research

Repo: `github.com/stablyai/orca` @ `c91a3b2e` (v1.4.214), MIT, `Copyright (c) 2026 Lovecast Inc.`
Clone: `/tmp/ilvuno-research/orca` (read-only)

## 0. Executive summary

1. Orca is an Electron desktop IDE whose product is *many agent CLIs in one window*, organised around git worktrees; it is not a model host. It spawns the user's own `claude`/`codex`/`gemini`/… binaries and never substitutes its own weights.
2. Three integration tiers per agent: **PTY/TUI** (universal, heuristic prompt injection), **ACP JSON-RPC** (17 per-agent dialects), **native SDK** (Claude only, `@anthropic-ai/claude-agent-sdk`). 45 agent ids in total (`src/shared/agent-launch/catalog/tui-agent.ts`).
3. Concurrency is **worktree-per-worker**. Agents are not threads of one runtime; they are separate CLI processes in separate checkouts, coordinated by a **SQLite mailbox** (`orchestration.db`) with 9 message types.
4. The coordination primitive is **mail + wake**, not shared memory: `dispatch`, `worker_done`, `question`/`reply`, `escalation`, `merge_ready`, `handoff`, `decision_gate`, `heartbeat`, `status` (`create-core-tables-sql.ts:34`).
5. **The coordinator is itself an agent.** Orca deleted its programmatic scheduler (CLI `coordinator-start` throws `command_retired`); the DAG lives in the model's head and in `tasks`/`dispatch_contexts` rows it mutates through `orca orchestration …`.
6. Consequently the worker contract is **prompt-injected**, not an API: `buildDispatchPreamble()` writes task ID, dispatch ID, `check`/`ask`/`worker_done` recipes into the worker's first message (`worker-dispatch-preamble.ts:84`).
7. State: `orchestration.db` (schema v50, 20 migration modules) in Electron `userData`; agent state in `profile_state_documents`; all per-agent session history is read from *each CLI's own* on-disk store (AI Vault), never re-hosted.
8. Transport: one JSON-RPC runtime server (~850 methods) over NDJSON unix socket locally, WebSocket/relay for remote+phone+web; protocol v3 with capability negotiation; ~385 CLI commands are a thin client of it.
9. Cross-agent sharing today = shared worktree files + git + mail + `worker-read`/`agent read` transcripts + diff comments + the Crew relationship panel. **There is no shared scratchpad, no shared vector memory, no blackboard, no cross-agent context object.**
10. Cost of admission: 3.6M lines, 160 main-process modules, Electron 43, per-file size ratchets in CI. Reuse the *protocol shape* and the *hook/status model*; do not reuse the codebase.

## 1. What it is

Orca is a desktop IDE for "parallel agentic development": a sidebar of git worktrees (each an independent checkout that can run its own agent), a pane/tab system where any pane can be a terminal, an agent chat, a diff, a browser, an emulator, or a file viewer, and a coordination layer that lets one agent hire other agents in other worktrees. Its own README frames it as "Bots inside your agent IDE" — you bring the CLI subscription, Orca brings workspaces, review, git, remote hosts, and the orchestration mailbox (`README.md`).

Product surfaces in this repo:

| Surface | What it is | Evidence |
|---|---|---|
| Desktop | Electron 43 app, macOS/Windows/Linux, `pnpm dev`/`build:*` | `package.json`, `electron.vite.config.ts` |
| Web | Same React renderer served as a bundled web client by `orca serve`, reached by a pairing URL | `src/renderer/web-index.html`, `vite.web.config.ts`, `src/shared/web-client/bundled-web-client-manifest.ts`, `src/main/server/serve-readiness.ts:113` |
| Mobile | Expo/React Native app (own pnpm workspace, own lockfile) that pairs to a desktop over the relay | `mobile/package.json`, `mobile/app.config.js` |
| CLI | `orca` — 385 commands, a thin JSON-RPC client over the runtime socket | `src/cli/**`, `src/cli/dispatch/handler-group-manifest.ts` |
| Headless server | `orcad` — the runtime without the Electron UI, for "your agents keep running when your laptop sleeps" | `src/main/orcad/main.ts`, `src/main/daemon/**` |
| Cloud (open) | Relay cells, relay-edge, push gateway, fence broker, contracts — MIT, in-repo | `cloud/README.md`, `cloud/apps/*` |
| Cloud (closed) | hosted connector service (Slack/Linear/GitHub events), mobile release pipeline, some artifact sharing | `cloud/README.md`; `src/main/artifacts/artifact-cloud-*.ts` |

## 2. Stack & repo shape

Electron 43.8 + React 19.2 + Vite 8 + Tailwind 4 + Radix + shadcn primitives (`components.json`). Terminal is **xterm.js 6.1 beta + webgl addon** with `node-pty` on the host; Monaco for the editor. State: **zustand 5** slices in the renderer, SQLite (better-sqlite3-style sync API in `src/main/sqlite/sync-database.ts`) in main. Typescript 7, zod for every wire schema, oxlint/oxfmt instead of eslint/prettier. Tests: vitest + Playwright golden journeys (`tests/e2e/golden-journeys/`).

Rough size (`find … | wc -l`, excluding lockfiles/dist):

| Area | Files | LOC |
|---|---|---|
| `src/main` | 10,847 (+305 test) | 1,389,840 |
| `src/renderer` | 10,946 | 1,338,941 |
| `src/shared` | 3,898 | 367,095 |
| `src/cli` | 518 | 67,746 |
| `src/preload` | 235 | 20,167 |
| `src/packages/*` (workspace libs) | 35 | 3,449 |
| `native/` (Windows/Wayland addons) | 43 | 8,660 |
| `mobile/` | 3,103 | 353,834 |
| `cloud/` | 297 | 54,062 |
| **total** | ~30k ts/tsx | **~3.6M** |

Package layout (path → purpose):

| Path | Purpose |
|---|---|
| `src/main/<agent>/` × ~39 | Per-CLI adapters: `claude/`, `codex/`, `gemini/`, `pi/`, `opencode/`, `antigravity/`, `grok/`, `kimi/`, `hermes/`, `dsh/`, `zcode/`, `jcode/`, `muse/`, `amp/`, `droid/`, `goose/`, `copilot/`, `cursor/`, `cline/`, `qwen-code/`, `kiro/`, `kilo/`, `codebuddy/`, `continue/`, `devin/`, `auggie/`, `command-code/`, `mimo/`, `omp/`, `openclaude/`, `prime-agent`… plus `*-accounts/`, `*-usage/` |
| `src/main/acp/` | Agent Client Protocol client: JSON-RPC peer, per-agent dialects, session runtime (19,937 LOC) |
| `src/main/pty/`, `src/main/providers/` | PTY lifecycle, process trees, foreground inspection, shell readiness, execution-host routing |
| `src/main/runtime/` | The RPC server and its 32 sub-systems (103 method modules, 851 methods) |
| `src/main/runtime/orchestration/` | **The heart**: 492 files, 61,370 LOC (db, mailbox, crew, workers, coordinator, federation) |
| `src/main/workspaces/`, `src/main/git/worktrees/` | Worktree create/remove/sparse/move; 128 files in `workspaces` |
| `src/main/agent-hooks/` | Status hook server + 23 managed installers + OSC-7501 program-status parser |
| `src/main/browser/` | Embedded Chromium browser pane, CDP, design-mode element grab (81,166 LOC) |
| `src/main/native-chat/` | Chat UI's transcript readers/decoders per agent (reads the CLI's own files) |
| `src/main/ai-vault/` | Cross-agent history index + search + session handoff prompts |
| `src/main/bots/` | The "Bots" product: Slack/Linear/GitHub-resident agents on a git repo at `~/.orca/bots/<name>` |
| `src/main/orcad/`, `src/main/daemon/` | Headless server distribution and terminal daemon |
| `src/shared/` | Every wire contract, zod schema and domain type, used by all five clients |
| `src/cli/` | 385 commands, handler-group manifest for lazy loading |
| `skills/`, `skill-guides/`, `skill-stubs/` | 11 bundled skills shipped *into the agents'* skill dirs |
| `config/` | Build/test/lint scripts, ratchets, checks (heaviest non-source dir) |
| `cloud/` | Independent pnpm workspace: relay + push + contracts |
| `native/` | Windows notifications/registry/security addons, Wayland clipboard |

## 3. Core domain model

Entities, with the definitions:

- **Project / Repo / Worktree.** `Worktree` is the central aggregate — it is *"Orca's tracked view of a repo checkout, its metadata, terminals, browser tabs, and UI state"* (`skill-guides/orca-cli.md`). Type: `src/shared/worktree/types.ts:94` — `id` is `` `${repoId}::${path}` ``, plus `identity`, `projectId`, `hostId`, `runtimeOwnerEnvironmentId`, `createdWithAgent`, `linkedPR/linkedIssue/linkedLinearIssue/linkedWorkItem`, `diffComments`, `sparseDirectories`, `pushTarget`, provenance (`cliProvenance`, `automationProvenance`). It additionally carries branch/head/PR via `GitWorktreeInfo`.
- **Execution host.** `local | ssh:<id> | runtime:<id>` (`src/shared/execution-host/execution-host.ts:6`). *Every* feature must work on all three; a worktree records which host owns it. `runtime:` hosts are `orcad` instances and ephemeral VMs (`src/main/ephemeral-vm/`).
- **Terminal / PTY.** `term_<uuid>` handle, pane key, process incarnation; the terminal handle *is* the mailbox address for a terminal agent (`src/shared/orchestration/mailbox/orchestration-party-identity.ts`).
- **Agent / agent session.** `TuiAgent` = 45-value union of CLI ids (`src/shared/agent-launch/catalog/tui-agent.ts`). A running agent has an `orca_session_id:<uuid>` when it is a *chat* (structured), or only a terminal handle when it is a TUI. Chats keep their id across `/clear` (`skill-guides/orca-cli.md`).
- **Run.** The coordination scope: `runs(id, objective, coordinator_handle, coordinator_orca_session_id, consumer_generation, legacy)` (`create-core-tables-sql.ts:5`). Every message, task and dispatch carries `run_id`; a legacy run id `run_legacy_local` exists for pre-Runs data, and `run_unbound` for mail from a terminal in no run (`orchestration-rpc-contract.ts:16`).
- **Task.** A unit of work with a spec and a dependency list: `tasks(id, run_id, parent_id, spec, status ∈ pending|ready|dispatched|completed|failed|blocked, deps, result)` (`create-graph-tables-sql.ts:152`).
- **Dispatch.** One *attempt* at a Task by one worker: `dispatch_contexts(id, task_id, assignee_handle, assignee_pane_key, assignee_orca_session_id, creator_*, depth, consumer_generation, retry_of_dispatch_id, follow_up_of_dispatch_id, status ∈ pending|dispatched|completed|failed|circuit_broken)` (`:178`). Depth is the nesting level (root coordinator = 0, its workers = 1).
- **Worker.** Not a table — a projection over Dispatch + terminal + agent status (`src/main/runtime/orchestration/crew/crew-read-model.ts`, `.../workers/`). A worker has stage, liveness verdict, attention categories, auto-release deadline.
- **Message.** The mailbox row (`create-core-tables-sql.ts:23`), with 9 types and 3 priorities, plus `thread_id` for question threads and pointer columns for delivery.
- **Decision gate.** A blocking question attached to a task with options: `decision_gates(id, task_id, question, options, status ∈ pending|resolved|timeout)` (`:234`).
- **Crew.** The *relationship graph* a party sees: `{ managedBy, managing, talkingWith }` rows, each row `{ partyKey, role ∈ manager|worker|peer, agentKind, status, task, progress, latest, reply, pendingQuestion, approvals, openTarget }` (`src/shared/orchestration/orchestration-crew.ts:17`).
- **Bot.** A long-lived agent with a git-backed home: `~/.orca/bots/<name>/` containing `SOUL.md` and `MEMORY.md` as *files the agent edits itself* (`src/main/bots/bot-home.ts:92`, `src/shared/bot-soul.ts`). Bot type: `src/shared/bot-types.ts:259`.
- **Automation.** Scheduled or event-triggered prompt: schedule, GitHub events (22 kinds incl. `ci_failed`, `pr_conflicted`, `deploy_finished`), Slack events, Google events (`src/shared/automation-event-triggers.ts`).
- **Browser.** A real Chromium tab inside the app with CDP, cookies, site settings, extensions, design-mode element picking (`src/main/browser/`, 81k LOC).
- **Artifact.** A shareable file with a cloud share record (`src/main/artifacts/`).

Relations: `Project → Repo → Worktree(s) → Pane(s)/Terminal(s) → Agent(s)`; `Run → Task(s) → Dispatch(es) → Worker`; `Message` links any two parties and optionally a `task_id`/`dispatch_id`; `DecisionGate` links a message to a task. Worktrees are *execution isolation*; Runs are *coordination scope*. They are orthogonal — a Run can span worktrees and hosts.

```
Project ──▶ Repo ──▶ Worktree (id = repoId::path, hostId) ──▶ Pane ──▶ Terminal(term_…)
                                                              └──▶ ok ──▶ Agent
                                                                          ├─ chat  → orca_session_id:<id>
                                                                          └─ tui   → term_ handle only
Run (objective, coordinator_handle|
      coordinator_orca_session_id)
 ├─ Task(spec, deps[], status)          ◀── parent_id self-reference (DAG)
 │    └─ Dispatch (attempt: assignee, depth, retry_of, follow_up_of) ──▶ Worker projection
 │           ├─ DecisionGate(question, options)
 │           └─ Message(type=dispatch … type=worker_done)
 └─ Message(any two parties, thread_id, priority, payload)
        └─ Delivery(consumer_generation) ──▶ veto fence on re-pointed panes
Bot ──▶ git repo at ~/.orca/bots/<name> (SOUL.md, MEMORY.md) + Items + Automations
Automation ── schedule | github/slack/google event ──▶ prompt run (optionally as a Bot)
```

The two identity axes matter for our design: **an agent is addressed either by a terminal handle (PTY, no id until it reports) or by an `orca_session_id` (structured chat)** — and the mailbox has to serve both (`orchestration-party.ts`, `mailbox/structured-pointer/`). Any design we build must pick which of those two it is, per thread.

## 4. Agent / provider integration

**No single driver abstraction.** There are three separate mechanisms, and which one applies is per-agent and per-launch-mode:

**(a) PTY/TUI — the default and the fallback for everything.** Orca launches the CLI as a child in a PTY and then *types into it*. The "driver" is a **config record of quirks**, not a class: `TuiAgentConfig` (`src/shared/agent-launch/catalog/tui-agent-config-types.ts`) with fields like `promptInjectionMode`, `draftPromptFlag`, `draftPromptEnvVar`, `pasteNeedsTypedRequest`, `preflightTrust`, `draftPasteReadySignal`, `submitRetryDelayMs`, `submitLineSettleMsPerLine`, `windowsShiftEnterEncoding`, `ctrlEnterEncoding`. 45 records, 362 lines, comments cite PR/issue numbers for each workaround (`tui-agent-config.ts:16`).

Prompt injection modes: `argv` (positional), `flag-prompt`, `flag-prompt-interactive`, `flag-interactive`, `hermes-query`, `stdin-after-start`. Readiness is detected by **regex on rendered output** — `'claude-composer-prompt'`, `'codex-composer-prompt'`, `'antigravity-composer-prompt'`, `'render-cursor-after-bracketed-paste'`, etc. (`tui-agent-config-types.ts:3`). Example: Claude gets `--prefill` for drafts because paste-after-ready raced (PR #926 in comment); Antigravity gets `submitLineSettleMsPerLine: 45` because `agy` expands multiline pastes over seconds (`tui-agent-config.ts:168-178`).

**(b) ACP (Agent Client Protocol) — structured chat.** Real JSON-RPC over the child's stdio: `src/main/acp/transport/acp-json-rpc-peer.ts`, 1-MB+ line framing, retry, write queue, request settlement. Per-agent dialect modules normalise each implementation's deviations: 17 per-agent dialects over a shared `acp` base — `antigravity, auggie, cline, codebuddy, copilot, cursor, devin, droid, gemini, goose, grok, hermes, kimi, kiro, omp, opencode, qwen-code` (`src/main/acp/acp-dialects/`). Dialects own session/update mapping, model catalogs (`cursor-model-catalog.ts`, `grok-model-catalog.ts`), questions (`qwen-code-questions.ts`), subagents/sessions. ACP agents are *downloaded* by Orca from the official registry (`src/main/acp/registry/acp-registry-catalog.ts`, `cdn.agentclientprotocol.com/registry/v1/latest/registry.json`) with per-target sha256.

**(c) Native SDK — Claude only.** `@anthropic-ai/claude-agent-sdk` behind a stream-json connection, with the user's own CLI forced in via `pathToClaudeCodeExecutable` and the SDK's bundled CLIs deliberately excluded from the install (`src/main/claude/claude-stream-json-connection.ts`, `pnpm-workspace.yaml` `ignoredOptionalDependencies`). Control requests, `canUseTool` permission callbacks, user-message queue, child-tree reaping for proven exit. Codex has an equivalent but different lane: `codex app-server` (`src/shared/agent-session/lifecycle/provider-handles/agent-session-provider-handle-encoding.ts:16`).

**Status reporting — two channels, hooks win.** `src/main/agent-hooks/README.md` documents this precisely: (1) *managed integrations* — per-agent hook/plugin/extension that POSTs to a loopback hook server, 23 installers (`managed-agent-hook-registry.ts:45`); (2) **OSC 7501 "Program Status Protocol"**, an open terminal-escape protocol where the agent writes its own state into the PTY stream and Orca answers a `?` probe. Precedence, 2-minute staleness rules and the hybrid facet model are in that README; the live support matrix (which agent sends what) is maintained there with version numbers.

**Chat vs terminal surfaces.** Every agent can run as a TUI; agents with a structured lane can run as a chat. The user's default is per-agent (`globalSettings.agentDefaultSurface`, `nativeChatProviders` — `src/shared/native-chat/settings/native-chat-providers.ts`). **Chat mode does not re-implement the agent.** It starts the same binary and then *reads the CLI's own session file*: `src/main/native-chat/transcript-reader.ts` dispatches per agent to `decodeClaudeChatTranscriptLine`, `decodeCodexTranscriptLine`, `decodeGrokTranscriptLine`, `decodeOmpTranscriptLine`, with OpenCode/Hermes read from their SQLite stores. A chat can be moved to a terminal and back "without losing context" (README) because both sides point at the same provider session id.

**Resume / identity.** A session's resume cursor is a typed provider handle: `{provider, agent, nativeId, resumeCursor}` with namespaces `claude-sdk` (cursor = transcript leaf UUID) and `codex-app-server` (`agent-session-provider-handle-encoding.ts:9-30`).

**Auth / subscription.** Orca never holds model keys for the CLIs; it runs the user's logged-in CLI. It does add account plumbing *around* that: `src/main/claude-accounts/`, `codex-accounts/`, `grok-accounts/`, `managed-data-accounts/` let the user switch accounts without logging out; `src/main/claude-usage/`, `codex-usage/`, `muse-usage/` scrape usage/limits; `src/main/ai-vault/` and `*`-accounts keep per-account `home`/`CLAUDE_CONFIG_DIR` isolation. Orca's own keys exist only for its own extras (speech, MiniMax cookies, ZCode plan key) and are stored under `~/.orca` or the OS secret service (`src/main/macos-keychain/`, `src/main/linux-secret-service/`).

### Provider support table

| Agent | PTY/TUI | ACP chat | Native SDK | Hooks | Account switch | Usage |
|---|---|---|---|---|---|---|
| Claude Code | ✅ `--prefill`, composer marker | ✗ (SDK instead) | ✅ claude-agent-sdk | ✅ | ✅ | ✅ |
| Claude Agent Teams | ✅ via `orca claude-teams` wrapper (not Win/WSL) | ✗ | ✗ | ✅ | ✅ | ✅ |
| Codex | ✅ | ✗ (app-server) | ✅ `codex app-server` | ✅ | ✅ | ✅ |
| Gemini CLI | ✅ | ✅ dialect | ✗ | ✅ | — | — |
| Antigravity (`agy`) | ✅ path-scoped trust | ✅ dialect | ✗ | ✅ | — | — |
| Pi / oh-my-pi | ✅ `ORCA_PI_PREFILL` env | ✅ omp dialect | ✗ | extension | — | — |
| OpenCode / OpenCode2 | ✅ composer gate | ✅ dialect (+child sessions) | ✗ | plugin | — | ✅ |
| Grok | ✅ | ✅ dialect + subagents/goals | ✗ | ✅ | ✅ | — |
| Cursor | ✅ | ✅ + model catalog | ✗ | ✅ | ✅ | — |
| Qwen Code | ✅ | ✅ + questions | ✗ | ✅ | — | — |
| Kimi, Kiro, Cline, CodeBuddy, Auggie, Cursor, Droid, Goose, Copilot, Devin, Hermes | ✅ | mixed (see dialects) | ✗ | ✅ | — | — |
| Muse, ZCode, DSH, jcode, Command Code, MiMo, Amp, Continue, Kilo, OpenClaude | ✅ | — | ✗ | ✅ (most) | — | ✅ muse |
| DeepSeek Harness | ✅ (`dsh-tui` profile) | — | ✗ | ✅ | — | — |
| Anything else | ✅ generic `stdin-after-start` / `argv` | ✗ | ✗ | ✗ | — | — |

Verdict: **the long tail is handled by heuristics, not adapters.** Orca's real moat here is 45 hand-tuned PTY quirk records plus 23 hook installers plus OSC-7501 fallback — ~2 years of chasing CLI regressions.

## 5. Concurrency & orchestration

**Isolation model.** One worker → one worktree (default), or `--worktree current` to share the parent's checkout, or `--worktree new-child` to nest a branch off the parent's. Worktree creation is Orca's (`orca worktree create`), and the skill guide forbids agents from making their own checkouts: *"Make checkouts yourself; let worker-start create a worker's workspace"* (`skill-guides/orchestration.md`). Creation goes through a locked, resumable transaction (`src/main/git/worktrees/creation/worktree-preparation-*.ts`, `worktree-config-write.ts` explains the `config.lock` race).

**The parent/child contract is prompt text, not an API.** `buildDispatchPreamble()` (`worker-dispatch-preamble.ts:84`) prepends to the worker's first message:

```
You are working inside Orca, a multi-agent IDE. You are a dispatched worker.
Your coordinator's Orca session ID is: orca_session_id:<id>   (or: "receives mail you send with no recipient")
Your task ID is: <taskId>
# read mail:  orca orchestration check --terminal <handle>
# ask, then end your turn (Orca wakes you):  orca orchestration ask --from <h> --question "..."
# blocked:  orca orchestration send --type escalation --subject "Blocked: …"
# finish exactly once:  orca orchestration send --type worker_done … --outcome succeeded
=== TASK ===
<spec>
```

`worker_done` settlement is authoritative: it "settles its Task; never mark a reported failure completed" (`skill-guides/orchestration.md`), enforced by `requireWorkerDoneSettlement` and `dispatch_contexts.status`. A dispatch id fences completion across retries of one task (`PreambleParams.dispatchId` comment).

**Nesting is bounded.** `nestedWorkerMaxDepth` default **4** (root = 0), enforced server-side with refusal code `nested_worker_depth_exceeded` (`src/shared/orchestration/workers/nested-worker-depth.ts:9`), and the setting is *renderer-only — omitted from the SettingsUpdate RPC so workers cannot raise their own cap*.

**Supervision is the agent's job.** There is a legacy in-process `Coordinator` class with `pollIntervalMs=2000`, `maxConcurrent=4`, phases `decomposing → dispatching → monitoring → merging → done` (`src/main/runtime/orchestration/coordinator/coordinator.ts:33`). Its RPC entry (`orchestration.run`) and CLI (`orchestration coordinator-start/stop`) are **retired** — the CLI throws `command_retired` and the RPC fence rejects it with `orchestration_migration_required` (`src/main/runtime/rpc/orchestration/compatibility/orchestration-contract-fence.ts:19`). The live model: the coordinating *agent* loops — `worker-start` several, **end its turn**, get woken by mail, act, repeat. `skill-guides/orchestration.md` is explicit: *"Never poll or run watch scripts… Orca wakes you."*

**Wake mechanics.** Because a TUI agent cannot be sent a message, Orca *types a note* into it: "you have mail, run `orca orchestration check --terminal <h>`". Delivery is deferred until the agent's live status shows it takes input (idle/working); otherwise the send receipt carries a `recipient_delivery_held` warning (`skill-guides/orca-cli.md`, "Messaging agents"). For chats the message is injected directly as a chat message with a sender header. Worst case (message too large for a chat message) the note names `check` and the agent reads from the mailbox. Implementation: `OrchestrationMailboxNotificationCoordinator` (`.../mailbox/notifications/mailbox-notification-coordinator.ts:31`) plus a sibling "structured pointer" lane for session-addressed workers with no PTY to type into.

**Cancellation / lifecycle.** `worker-stop`, `worker-abandon`, `worker-release`, `worker-retain`, `worker-keep-open`, `worker-auto-close`; each has a guarded state (`stop_unknown`, `start_unknown`, `abandoned`) because "a live PTY can contain a dead agent" (`worker-observation-handlers.ts` comment). Auto-release: workers close on their own unless kept open (`src/shared/orchestration/workers/worker-auto-close.ts`). `skill-guides/orchestration.md` states the safety rule: *"Silence, a timeout, or `unverifiable` liveness alone never proves exit or authorizes autonomous recovery."*

**Federation.** Runs can span machines: `federated_dispatches`, `remote_dispatch_attachments` with 9 states (`starting|ready|start_unknown|failed|succeeded|stopping|stop_unknown|stopped|abandoned`), sequence-numbered relay items both directions (`create-graph-tables-sql.ts:26-44`). `docs/reference/ssh-execution-boundary.md` is cited in-schema for why five states exist.

**Concurrency guidance for the model** (not enforced): start ~4 workers at once from one shell command, then end the turn (`skill-guides/orchestration.md`).

**Worker lifecycle, as the server models it** (from `workers/launch`, `workers/lifecycle`, `workers/reports`, `workers/approvals`):

1. `worker-start` → admission (`orchestration-admission.ts`: depth cap, caller identity, target host servable) → worktree prepare/attach → CLI spawn → preamble typed in → `dispatch_contexts.status = dispatched`, `worker_terminal_resources` minted.
2. Readiness observation: composer/idle marker, trust prompt, `agent_readiness` timeout if the binary is missing.
3. Working: mail arrives → held until the agent takes input → pointer typed or chat message injected → agent runs `check` → `ask`/`reply`, `escalation`, or `heartbeat`.
4. Settlement: `worker_done` with `--outcome` → `lifecycle-reconciliation.ts` settles Task + Dispatch; a second `worker_done` is refused as duplicate.
5. Post-settlement: auto-release timer, or kept open for the user, or a `send` re-opens it as a **follow-up** Dispatch (`follow_up_of_dispatch_id`) so the same worker continues the same Task.
6. Exit without `worker_done` → `escalation` with `stopReason` (`exited` | `lost`), and `worker-show` reports the independent agent-liveness verdict plus a `nextAction` argv.

**Approvals are a first-class lane.** A worker's permission prompt is not left in its PTY: it is booked (`terminal-worker-approval-book.ts`), surfaced to the coordinator as a `CrewRow.approval`, and answered via `orchestration approve|deny` (`WorkerApprovalPayloadSchema`, `worker-approval-time-left.ts`). This is how a *terminal* worker asks a *remote chat* coordinator for permission.

## 6. State & persistence

Two SQLite databases + a JSON profile layer:

| Store | Location | Contents |
|---|---|---|
| `orchestration.db` | `<userData>/orchestration.db` (`src/main/runtime/orchestration/db/orchestration-db-file.ts:3`) | runs, messages, deliveries, mutation receipts, tasks, dispatch_contexts, decision_gates, attempt observation facts, federation tables, worker terminal resources, question threads |
| profile state DB | `<userData>/` (`src/main/persistence/profile-state/database/`) | `profile_state_documents` (per-domain JSON documents + parts): projects, worktrees, terminals topology, automations runs, settings, window sessions |
| `~/.orca/` | home | `agent-hooks/` scripts + ownership, `bots/` (git repos with SOUL.md/MEMORY.md), `bot-tools.json`, credential stores, claude-agent-teams shim |
| per-repo `.orca/` | worktree | `issue-command` (local), setup runner scripts (`workspace-hooks/folder-setup-runner-files.ts`) |
| per-agent stores | wherever each CLI puts them | Orca **reads** these (AI Vault, native chat) and does not migrate them |

`orchestration.db` is tuned, versioned and hardened: `busy_timeout=5000`, `auto_vacuum=INCREMENTAL`, `journal_mode=WAL`, `synchronous=NORMAL`, `temp_store=MEMORY` (because a full temp dir aborted transactions, #38123 — `orchestration-db.ts:60-72`), plus file permissions hardening (`database-file-permissions.ts`) and **50 numbered migrations** (`db/schema/migrate-v50.ts`) with explicit version-skew handling (`orchestration-schema-version-skew.ts`). Deletes are *recorded* (`removed_orchestration_work`) so a reset can be audited.

**Resume semantics.** A chat resumes by provider handle (Claude: session id + leaf UUID; Codex: thread id). A terminal agent resumes by re-running its recorded launch recipe (`SleepingAgentLaunchConfig` in `src/shared/agent-session/lifecycle/agent-session-resume.ts`). `orchestration_worker` rows keep the "sleeping" state so a sleeping agent still lists (`agent list` shows it as done while asleep).

**Cloud sync.** Orca syncs *nothing* of the workspace state to the cloud by default. The relay forwards encrypted RPC frames between a phone/web client and the host; the desktop holds the state. Optional: artifacts sharing (`src/main/artifacts/artifact-cloud-service.ts`), telemetry (`posthog-node`), and bots' connector events (hosted service, closed). Relay is X25519 + E2EE with per-device keys (`src/shared/remote-runtime/e2ee/`, `src/main/runtime/rpc/server/runtime-rpc-device-key-registration.ts`).

## 7. Transport / IPC

**Three transports, one RPC surface.**

1. **Electron main↔renderer**: typed `ipcMain`/`contextBridge` (`src/preload/api/`, `src/main/ipc/`), 1,126-param generated catalog.
2. **Local runtime socket**: `unix` socket (POSIX) / named pipe (Windows), **NDJSON**, 1 MB frames, `maxConnections=32`, mode `0600`, 30s idle timeout suspended during dispatch, 10s keepalives (`src/main/runtime/rpc/transports/unix-socket-transport.ts:1`). This is what the CLI uses.
3. **Remote / mobile / web**: WebSocket or the cloud relay (`relay-transport.ts`, `cloud/apps/relay`), plus E2EE device keys and a mobile method allowlist (`runtime-rpc-mobile-method-allowlist.ts`). `orcad` is the same server without Electron.

**Protocol discipline.** `RUNTIME_PROTOCOL_VERSION = 3`, `MIN_COMPATIBLE_RUNTIME_{CLIENT,SERVER}_VERSION = 2`; the comment above it states the rule: add optional fields and new methods freely; raise minimums only to refuse unsupported peers; **never require equal app versions** (`src/shared/remote-runtime/protocol/protocol-version.ts`). Capabilities are a flat string array negotiated per client class — `NATIVE_REMOTE_RUNTIME_CLIENT_CAPABILITIES`, `MOBILE_RUNTIME_CAPABILITIES`, `WEB_RUNTIME_CLIENT_CAPABILITIES`, etc. (`orchestration-runtime-capabilities.ts`). Orchestration adds a second fence: every mutation must carry `orchestrationContractVersion === 1`, else `orchestration_migration_required` with a machine-readable reason (`client_contract_missing|unsupported|runtime_capability_missing|command_retired`).

**Mutation idempotency.** Every orchestration mutation takes a request id; the server writes `mutation_receipts(caller_fingerprint, request_id, method, payload_hash, state, receipt)` keyed on the caller's identity, so a lost reply can be replayed without double effects. The CLI surfaces this as "reuse any request ID the error names" and `orchestration request-show`.

**Streaming.** Long outputs are paginated/subscribed, not pushed whole: crew rows arrive as `begin / rows / delta / complete / end / summary` frames (`orchestration-crew.ts:53`), worker transcripts page with a cursor (`OrchestrationWorkerTranscriptPage`), terminals have binary routing for output (`runtime-rpc-binary-routing.ts`). Long-poll rather than polling for idle clients (`runtime-rpc-long-poll.ts`).

**Sync model.** The renderer holds a projection (`src/renderer/src/runtime/`) fed by subscriptions plus a revisioned session graph; the "crew read model" is a server-side cache with a 30s grace period after the last subscriber, per-party relevance to wake only affected panes (`crew/crew-read-model.ts`), and delta frames.

## 8. Frontend architecture

React 19 + Vite, no router — a workspace/pane layout model drives everything (`src/renderer/src/store/workspace-layout`, `components/tab-group/`, `components/pane-resize/`). State: **zustand** with cold/hot slices (`store/slices/*`: 30 slices — agent-inventory, agent-status, browser, diff-comments, editor, settings, tabs, worktrees, wsl, …), plus a store-observer installer layer and a transition log for debugging (`store/record-transitions`, `store/active-workspace-change-log.ts`).

Key panels/screens: sidebar (worktrees with smart tiers, PR/issue links, dirty counts), tab groups with splits where each pane is `terminal | native-chat | diff | browser | emulator | file | pull-request | task-page`, right sidebar (review/checks/files), `activity`, `dashboard`, `settings` (with an Orchestration pane), `Crew` panel, `bots` (Items/Chat/Activity), `skills` browser, `native-chat` timeline with tool-call rows.

Terminal rendering: `@xterm/xterm` 6.1 beta + `addon-webgl` + `addon-serialize` + `addon-search` + `addon-ligatures` + `addon-image` + `addon-unicode11` + `@xterm/headless` for transcript reconstruction. Pane input has an ownership model (`RuntimeTerminalDriverController`: `idle | desktop | mobile` with an input floor) so a phone and the desktop cannot fight over keystrokes (`src/main/runtime/terminal/driver/runtime-terminal-driver-controller.ts`).

Design system: `docs/STYLEGUIDE.md` — monochrome, "color is reserved for state", tokens only from `src/renderer/src/assets/main.css`, shadcn primitives, Geist Sans / `var(--font-mono)`, 14/13/12/11px type ramp. Enforced by `pnpm lint:design-system`. i18n: 11 locales, i18next, an `en-runtime-required.json` overlay for host-side strings.

Mobile: Expo 58 + React Native + TanStack Query + the same xterm packages inside a webview; pairing via relay; iPad/Android tablets supported; owns its own pnpm workspace and lockfile.

## 9. Extensibility

- **Skills.** Orca ships 11 bundled skills (`skills/*/SKILL.md`: `orca-cli`, `orchestration`, `orca-jira`, `orca-linear`, `jira-tickets`, `linear-tickets`, `orca-computer-use`, `orca-emulator`, `orca-emulator-android`, `orca-per-workspace-env`, `orca-plugin-authoring`) and **installs them into each agent's own skills directory** — 42 providers mapped to their `~/.<agent>/skills` or `.agents/skills` paths (`src/shared/skills/installation/skill-install-providers.ts`). The shipped `SKILL.md` is deliberately a **stub**: its content is "run `ORCA skills get <name>`" so the *binary* serves the version-matched guide, including references loaded on demand. This is the neatest trick in the repo for keeping agent instructions in sync with a shipped app.
- **skill-guides/ + skill-stubs/.** Authoring source for the above: `skill-guides/orchestration.md` (158 lines) is the real protocol doc; `skill-stubs/` holds the short forms, plus `_shared/cli-resolution.md`.
- **Hooks.** 23 managed installers that write native hook config into each agent's settings; a loopback hook server receives events; plus the OSC-7501 terminal protocol for agents without hooks (`src/main/agent-hooks/`).
- **MCP.** Orca is an MCP *consumer*: it imports/clones MCP server config from other agents' files (`src/main/configuration-import/`, `src/shared/mcp/configuration`), with UI to toggle servers. It is not an MCP server itself.
- **Plugins.** A full plugin platform: manifest `orca-plugin.json`, four execution modes (`data`, `sandboxed-ui`, `trusted-worker`, `isolated-frame`), declarative TSX UI, workers, storage atoms, settings, events, themes, syntaxes, status-bar items, panels; catalog + install + integrity checks (`src/main/plugins/`, `docs/reference/plugin-*.md`, `examples/plugins/*`). 27 `plugin.*` RPC methods.
- **Automations.** Schedule/event-driven prompts across any repo, with run history and per-run message capture (`src/main/automations/`).
- **Embedded browser.** Full Chromium tab: CDP, cookies, site settings, extensions, downloads, WebAuthn, element grab for Design Mode, and an agent-facing CLI (`orca browser …`, 73 `browser.*` RPC methods). Plus `computer` (desktop GUI via CLI) and `emulator` (iOS Simulator / Android emulator: `tap`, `type`, `ax`, `logcat`).
- **Workspace hooks.** Per-repo `orca.yaml` (tracked) and `.orca/issue-command` (local) provide the "issue → branch → prompt" recipe per project (`src/main/workspace-hooks/`).

## 10. Shared context / cross-agent communication (CRITICAL)

What exists **today**, verified in code:

1. **Shared filesystem + git.** Default is isolation (one worktree per worker); sharing is opt-in via `--worktree current`. Two agents in the same worktree see each other's edits immediately and share `git status`. There is **no file locking, no conflict arbitration, no "who is editing this file" signal** between agents — the guidance is social ("use a new worktree when sharing would conflict").
2. **The mailbox.** A real message bus with types, threads, priorities, delivery state, and idempotent mutation receipts (§6). Addressing: terminal handle, `orca_session_id:<id>`, `run:<runId>`, `dispatch:<id>`, or a group address (`@…` — lifecycle types refuse groups: `rejectLifecycleGroupRecipient`).
3. **`ask` / `reply`.** A question thread with an answer, 30-min-per-wait cap with resume, `--wait` for terminal-side blocking. A worker ends its turn after asking; Orca wakes it with the answer — this is the mechanism that makes "one agent asks another" work across *turn-based and terminal-based* agents without RPC.
4. **`worker_done` + settlement.** A structured completion report (3 sentences, `--outcome succeeded|failed`, optional `--files-modified`, `--report-path`) that mutates server state. Reported failure can never be recorded as success.
5. **Decision gates.** A task can block on a gate with options; resolved by a human or a coordinator via `gate-resolve` (`decision_gates` table, `coordinator-decision-gates.ts`).
6. **Worker observation.** `worker-read --dispatch` returns the worker's transcript (or terminal screen) as structured output; `worker-show` returns stage, liveness verdict, attention categories, waiting-on-human detection, auto-release deadline. So *agent B can read agent A's actual output*, not just its messages. `agent read`, `terminal read`, `terminal wait` do the same for non-worker agents.
7. **Crew panel.** A live per-party graph of `managedBy / managing / talkingWith`, showing status, current task, latest message, pending question, pending approvals, and an `openTarget` you can jump to. This is the only *visual* cross-agent surface, and it is scoped to the party you are looking at.
8. **Diff comments → agent.** Review notes are persisted on `WorktreeMeta` with `sentAt` ("handed to an agent"), formatted deterministically (`File: … / Lines: … / User comment: "…"`) so they can be pasted into any agent's prompt (`src/shared/file-review/diff-comments-format.ts`).
9. **PR/check feedback → agent.** Draft commit messages and PR bodies from the diff; send failing checks or PR review comments back to an agent (README; `src/main/github/`).
10. **AI Vault.** Indexes *every* agent's own on-disk session history across accounts/hosts, searchable, with a **continuation prompt** that can hand one agent's transcript to another (`src/main/ai-vault/session-handoff-prompt.ts`, `src/shared/agent-session/conversation-context/agent-session-continuation.ts` — supports `focused` and `full` modes with a fenced, bounded transcript).
11. **Handoff (one-way).** `orca worktree create --agent X --prompt "…"` or `orchestration send --to …` with no Task/Run. Explicitly *not* orchestration: "`task-create` is also forbidden because it records coordinator-owned tracking state" (`skill-guides/orca-cli.md`).
12. **Bots' shared home.** A bot's `SOUL.md`/`MEMORY.md` are plain files it edits, and a bot's Items have comments that can carry picked skills and attachments — so *human and bot* share a durable written surface. Not agent↔agent generic.
13. **Artifacts.** Shareable files with cloud records; also in-repo `.orca/artifacts` slots for orcad.

What does **NOT** exist (searched: `shared context`, `scratchpad`, `blackboard`, project-wide memory, agent-to-agent state):

- **No shared scratchpad / blackboard / project-level context object.** Two agents in the same project have no structured way to see "what the others have decided" except by reading files or asking.
- **No shared vector memory or embeddings across agents.** AI Vault search is over *historical transcripts*, per-host, opt-in, and does not feed the running agents' context.
- **No automatic context propagation.** A new worker gets a hand-written spec, not "the project state so far". Nothing summarises the other workers' progress into its prompt; the coordinator must do that in prose.
- **No peer discovery.** An agent can only address parties it was told about (its coordinator, or a handle it looked up via `terminal list` / `agent list`). There is no registry an agent can query for "who else is working on this project".
- **No file-level coordination** — no locks, no leases, no conflict detection between agents.
- **No message replay/history beyond the mailbox** — the mailbox keeps rows, but there is no "conversation summary" surface for an agent joining late (it can `inbox`/`check`, but it must read raw messages).
- **No cross-project memory.** Bots have MEMORY.md; ordinary agents do not.
- **`@group` broadcast exists but is thin** — an address form, not a topic/subscription model; lifecycle types are refused on groups.

Net: Orca's cross-agent context is **mail + files + human-mediated prose**. That is deliberate (it survives any agent's context loss and any agent's model), but it puts the burden of shared understanding on the coordinator agent's writing.

### 10.1 Sharing mechanism matrix

| Mechanism | Scope | Durable | Readable by an agent without a human | Notes |
|---|---|---|---|---|
| Same worktree files + git | per-worktree | ✅ | ✅ (its own tools) | no locks, no ownership signal |
| Parent worktree (`--worktree current`) | explicit | ✅ | ✅ | default is isolation |
| Mailbox message | per-party | ✅ (`messages`) | ✅ via `check`/`inbox` | the only push mechanism |
| `ask`/`reply` thread | per-party | ✅ (`question_threads`) | ✅ | blocks or resumes |
| `worker_done` report | Run | ✅ (`tasks.result`) | ✅ via `task-list` | single writer (worker) |
| Decision gate | Task | ✅ (`decision_gates`) | ✅ via `gate-list` | human-resolvable |
| Worker transcript (`worker-read`) | Dispatch | ✅ (provider session) | ✅ | reads the *other* agent's output |
| Diff comments | Worktree | ✅ (`WorktreeMeta`) | paste-only | `sentAt` records handoff |
| AI Vault | host, all agents | ✅ (index) | ✗ (UI/CLI only) | history, not live context |
| Crew panel | per-party | derived | ✗ visual only | `managedBy/managing/talkingWith` |
| Bot `SOUL.md`/`MEMORY.md` | per-bot | ✅ (git) | ✅ | bots only |
| Artifacts | cloud | ✅ | needs a link | file sharing |
| Broadcast `@group` | Run | ✅ | ✅ | address form, not pub/sub |

Everything in the "✅ readable" rows is *pull*: an agent only sees another agent's work if it was told a handle and ran a command. There is no subscription an agent can hold, and no event it can receive while it is thinking.

## 11. Notable engineering decisions

- **The coordinator is a prompt, not a scheduler.** `orchestration.run` and `coordinator-start/stop` are retired and rejected at a migration fence, even though the `Coordinator` class remains (`coordinator/coordinator.ts`, `gates.ts:17`, `orchestration-contract-fence.ts:19`). Coordination moved into the agent's loop; the server only persists Tasks/Dispatch/mail.
- **The worker contract is text injected into the worker's prompt** (`worker-dispatch-preamble.ts:84`), including the exact `check`/`ask`/`worker_done` command lines and a "do not use AskUserQuestion, the coordinator cannot see it" warning. This is how a *terminal* agent is made to behave like a well-behaved RPC client.
- **One config record per CLI quirk instead of an abstraction.** 45 `TuiAgentConfig` records with regex readiness markers and per-OS key encodings; comments cite the PR that discovered each (`tui-agent-config.ts`). Brutal, effective.
- **Read the provider's own session store instead of re-hosting it.** Chat mode = same binary + decode `~/.claude/projects/**/*.jsonl` etc. (`native-chat/transcript-reader.ts`). Zero migration cost, automatic feature parity with the CLI's own history.
- **Two-channel status with precedence, one of them an open terminal protocol.** Hooks first, OSC-7501 fallback, 2-minute staleness handoff, hybrid facet for progress/blocked (`agent-hooks/README.md`).
- **Version-matched agent docs served by the binary.** Shipped `SKILL.md` files are stubs that shell out to `orca skills get <name>` (`skills/orchestration/SKILL.md`), so agent instructions never drift from the installed app.
- **Mutations carry caller identity + request id + payload hash** in `mutation_receipts`, and the CLI teaches agents to reuse the request id (`orchestration-rpc-contract.ts`, `cli/runtime/connection/client-error-recovery.ts`).
- **Capability negotiation, not version equality.** Flat capability strings per client class; protocol floor 2, ceiling 3 (`protocol-version.ts`, `orchestration-runtime-capabilities.ts`).
- **The nested-worker depth cap is server-side and excluded from the settings RPC** so a worker cannot raise its own recursion limit (`nested-worker-depth.ts`).
- **Recorded deletions and fenced states.** Deleted orchestration work is captured for audit; workers get `start_unknown`/`stop_unknown` rather than a boolean, because "a live PTY can contain a dead agent".
- **Ratchet-style quality gates** instead of cleanup projects: `check:max-lines-ratchet`, `check:ts-nocheck-ratchet`, `check:host-list-render-ratchet`, `lint:design-system`, `audit:dead-code`, plus a `check:changed` that runs only the tests importing your change (`AGENTS.md`).

## 12. What to reuse for our product

Ranked by value to "one project, many agent threads that see each other's work":

1. **The mailbox schema + message taxonomy.** Nine types with a `thread_id`, priorities, delivery contracts, and receipts is exactly the primitive set we need; it maps 1:1 to "these threads can message each other". Reuse: `db/schema/create-core-tables-sql.ts:23-60` (messages/deliveries/run_coordinator_handles), `shared/orchestration/mailbox/*`.
2. **The "note-into-a-TUI" wake strategy.** The insight that a terminal agent can only be woken by typing a *pointer* at it ("run `check` to read your mail") is the single most important unblocker for supporting CLIs we cannot instrument. Reuse: `mailbox/notifications/mailbox-notification-coordinator.ts`, `skill-guides/orca-cli.md` §"Messaging agents".
3. **The prompt-injected worker preamble.** Copy the shape (identity lines, task id, exact command recipes, "finish exactly once", "do not use your own question UI"). Reuse: `workers/launch/worker-dispatch-preamble.ts:84-140`.
4. **Two-channel agent status with precedence.** Hooks + an out-of-band terminal protocol, with a staleness rule. Reuse: `agent-hooks/README.md` (the design, incl. the OSC-7501 spec link) and `server/program-status/hook-precedence.ts`.
5. **Per-CLI quirk table.** If we drive TUIs, we will need exactly this: readiness regex, submit retries, per-OS key encodings, trust pre-writes. Reuse: `shared/agent-launch/catalog/tui-agent-config-types.ts` as the schema (do not copy 45 entries wholesale — copy the fields we need and add per-agent as we hit bugs).
6. **`worker_done` settlement semantics.** Reported failure cannot become success; a dispatch id fences completion across retries. Reuse: `workers/reports/*`, `workers/orchestration-worker-settlement.ts`, `create-graph-tables-sql.ts:178-230`.
7. **Transcript-as-source-of-truth for chat.** Reading each CLI's own JSONL/SQLite session store removes an entire class of "our copy is wrong" bugs and gives free history/search. Reuse: `native-chat/transcript-reader.ts`, `transcript-decoding/*`, `ai-vault/` for the cross-agent index.
8. **Version-matched instructions served by the app.** Stub skill → `skills get <name>` → real guide from the running binary. Reuse: `skills/*/SKILL.md` + `skill-guides/*.md` + `src/main/skills/bundled/*`.
9. **Mutation idempotency (`request_id` + payload hash + caller fingerprint).** Cheap, prevents double-dispatch when an agent's turn dies mid-call. Reuse: `mutation_receipts` table + `cli/runtime/connection/client-error-recovery.ts`.
10. **Crew read model with per-party deltas.** If we show a "who's working for whom" view, do it server-side with relevance tracking rather than rebuilding the graph in the client. Reuse: `crew/crew-read-model.ts`, `shared/orchestration/orchestration-crew.ts`.
11. **Decision gates.** A blocking question with options attached to a task is a better primitive than "a chat message that might get lost" for human-in-the-loop.
12. **Depth cap as a fence, not a warning.** `nested_worker_depth_exceeded` with the setting hidden from the RPC. Reuse: `shared/orchestration/workers/nested-worker-depth.ts`.

## 13. What to avoid / gotchas

- **License.** MIT (root `LICENSE`, "Copyright (c) 2026 Lovecast Inc."). `cloud/` is explicitly covered by the same root MIT (`cloud/README.md`). Third-party notices: `docs/site/THIRD_PARTY_NOTICES.md`. But: the *hosted* services are not in-repo — the connector service (Slack/Linear/GitHub events), the mobile release pipeline, and parts of artifact sharing are closed. Don't plan on self-hosting those.
- **Weight.** 3.6M LOC, 160 main-process dirs, Electron 43, xterm v6 **beta** packages pinned, TS 7, Vite 8, `node-pty` native compilation, Windows/Wayland native addons. Any "just reuse this module" plan collapses under transitive coupling: `src/shared/` alone is 367k LOC of contracts and the orchestration code imports ~everything.
- **The PTY-heuristic treadmill.** `tui-agent-config.ts` is a museum of regressions with PR numbers. Every CLI release can break paste readiness, submit, or trust prompts. Supporting 45 TUIs is a permanent maintenance tax — and it is *the* reason Orca's agent list looks impressive and its bug surface is enormous.
- **Dead and semi-dead code.** `orchestration.run`/`runStop` are retired at the fence yet still registered (`gates.ts`); `Coordinator` is legacy; `coordinator-start/stop` CLI throw. Don't model our design on those files without reading the migration fence first.
- **`opencode2`, `pi` vs `omp` vs `prime-agent`, `claude-agent-teams`** — near-duplicate agent ids that exist for historical reasons; id proliferation is a real cost in the schema (every hook installer, skill provider, dialect, usage module needs an entry).
- **Schema churn.** `orchestration.db` is at v50 with 20 hand-written migration modules and several "this runs before migrate so it must compile against a pre-v42 table" comments. That is the cost of shipping a coordination DB inside a desktop app with downgrade risk.
- **Docs drift.** The repo's own prose (README/skills) is *ahead* of some code paths and behind others; the skill guides are the most accurate documents in the repo and they are hand-written. Trust code over docs; trust `skill-guides/` over `docs/`.
- **No shared-context primitive** means the coordinator agent's prompt writing is the bottleneck. If we copy Orca's model, we inherit the failure mode "the coordinator forgot to tell the worker X".
- **`--keep-open` / auto-release defaults differ by launch mode** (kept-open vs new-workspace best-effort), and a failure to keep open only fails the start when explicitly requested (`worker-launch-handler.ts`). Small thing, easy to model wrong.
- **Windows cmd.exe cannot send orchestration mail bodies** — `orca.cmd` refuses `send`/`reply`; and on Windows you must use `orca.exe`. Genuine platform sharp edge baked into the docs.

## 14. Top files to read

1. `skill-guides/orchestration.md` — the whole coordination protocol as the agents are taught it. Start here.
2. `skill-guides/orca-cli.md` — handoff vs orchestration, agent addressing, mail delivery semantics, worktree selectors.
3. `src/main/runtime/orchestration/db/schema/create-core-tables-sql.ts` — runs/messages/deliveries/receipts + the 9 message types.
4. `src/main/runtime/orchestration/db/schema/create-graph-tables-sql.ts` — tasks, dispatch_contexts, decision_gates, federation, coordinator_runs.
5. `src/main/runtime/orchestration/workers/launch/worker-dispatch-preamble.ts` — the injected worker contract.
6. `src/main/runtime/orchestration/mailbox/notifications/mailbox-notification-coordinator.ts` — how mail becomes a typed note or a chat message.
7. `src/main/runtime/orchestration/crew/crew-read-model.ts` + `src/shared/orchestration/orchestration-crew.ts` — the relationship graph and its delta stream.
8. `src/shared/agent-launch/catalog/tui-agent-config-types.ts` + `tui-agent-config.ts` — the per-CLI quirk schema and 45 examples.
9. `src/main/agent-hooks/README.md` — hooks + OSC 7501 precedence model, with a live support matrix. Best-written doc in the repo.
10. `src/main/acp/transport/acp-json-rpc-peer.ts` + `src/main/acp/acp-dialects/acp-dialect.ts` — the structured-agent lane.
11. `src/main/claude/claude-stream-json-connection.ts` — the only native-SDK lane; shows the "force the user's binary" trick.
12. `src/main/native-chat/transcript-reader.ts` — transcript-as-truth, per-agent decoders.
13. `src/main/ai-vault/session-handoff-prompt.ts` + `src/shared/agent-session/conversation-context/agent-session-continuation.ts` — handing one agent's history to another.
14. `src/shared/remote-runtime/protocol/protocol-version.ts` + `orchestration-runtime-capabilities.ts` — the compatibility doctrine.
15. `src/main/runtime/rpc/transports/unix-socket-transport.ts` — the local IPC frame: NDJSON, 1 MB, 32 conns, 0600.
16. `src/main/runtime/rpc/orchestration/compatibility/orchestration-contract-fence.ts` — request-id/contract fencing for mutations.
17. `src/shared/worktree/types.ts` — the Worktree aggregate (what an app-side "workspace" really needs).
18. `src/main/runtime/orchestration/db/orchestration-db.ts` — pragmas, migrations, triggers, listener wiring; the whole persistence posture in one file.
19. `src/shared/orchestration/workers/nested-worker-depth.ts` — the small, complete example of a server-side safety fence.
20. `src/main/bots/bot-home.ts` + `src/shared/bot-soul.ts` — the "agent memory is files in a git repo" pattern.

## 15. Open questions

Cannot be answered from this repo:

1. **The hosted connector service.** Slack/Linear/GitHub event fan-out to signed-in Orcas runs on a service "not in this repository" (`cloud/README.md`). How bots receive events, how retries/dedup work, and what the GitHub App scopes are, is unknown.
2. **Mobile release / store pipeline.** `mobile/fastlane/` exists but the push gateway deployment and `stablyai/orca-cloud` are private.
3. **Artifact sharing backend.** `artifact-cloud-service.ts` talks to a service whose API is only visible as client calls; retention and ACLs are opaque.
4. **Orca accounts.** `src/main/managed-data-accounts/`, `src/main/claude-accounts/`, and the `accounts.*` RPCs (47 methods) reference a "signed-in Orca" for managed models/data. Whether that ever becomes a first-class model gateway is not decidable from code.
5. **Telemetry volume/shape.** `posthog-node` is a dependency and `src/shared/telemetry/schemas/*` exists, but what is actually sent, and the opt-out UX, need the hosted project or the telemetry docs.
6. **Actual measured reliability of the TUI heuristics.** No published failure rates per agent; the config comments imply frequent breakage but there's no dashboard in-repo.
7. **Why `claude` has an SDK lane while Codex uses `codex app-server` and every other agent is PTY-only** — the decision record isn't in `docs/`; only PR references inside comments.
8. **Whether the legacy `Coordinator` class is scheduled for deletion** or kept as a compatibility path. Both the class and its retired RPC entry remain.
9. **The OSC-7501 spec's governance.** The spec link points at `superlogical.com/rex/docs/build/program-status`, not a standards body; whether third-party agents will implement it is speculation.
10. **Multi-user/team semantics.** Nothing in the schema models a second human on the same host; `mutation_caller_identities`, `bot-access-*` and pairing cover devices and bots, not teammates.
