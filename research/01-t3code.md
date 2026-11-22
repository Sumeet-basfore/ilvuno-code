# t3code (T3 Code) — source research

Repo: `github.com/pingdotgg/t3code` @ `7ddc81d6` (monorepo, MIT, © 2026 T3 Tools Inc).
Researched read-only at `/tmp/ilvuno-research/t3code`. Every claim below is from source or in-repo
docs; where only an in-repo doc asserts something, it is marked *(doc)*.

## 0. Executive summary (10 lines max)

1. t3code is an **MIT-licensed, local-first "control plane" for coding-agent CLIs** — a Node server
   that spawns Claude Code / Codex / Cursor / Grok / OpenCode / Antigravity / Kiro / Pi / Muse / fx /
   any ACP agent as subprocesses, plus web, Electron, iOS/Android and a `t3` CLI.
2. Architecture is **event-sourced CQRS**: typed WS commands → pure `Orchestrator` → events +
   projections + command receipts + a **durable effect outbox** committed in one SQLite transaction.
3. Domain model is **thread → run → execution-node tree**, with provider-native
   `ProviderSession/ProviderThread/ProviderTurn` handles as *refs*, not identity.
4. **11 built-in provider drivers** behind one `ProviderAdapterV2` contract; capability flags (not
   driver-name branching) decide forks, steering, rollback, subagents.
5. Cross-agent collaboration is real and shipped, but it is **tool-mediated, not ambient**: ~90 MCP
   tools, including `t3_thread_read/send/list/search/fork/transfer`, `delegate_task`,
   `task_status`, `t3_submit_output`, `t3_pending_request`, `schedule_task`, `t3_background_wait`.
6. Every agent session gets a **shared injected system-prompt contract** (how to delegate, how to
   follow a delegate's `next`, never poll) — `provider-core/src/server/orchestrationInstructions.ts`.
7. Threads share a filesystem when they share a checkout (project root by default, or a task folder
   holding per-repo worktrees); worktree children get the parent *commit*, never dirty state.
8. There is **no shared memory, no message bus, no cross-thread board**: a thread learns about
   another only by an explicit tool call, a citation/link, or a user-inserted context record.
9. State is one SQLite file (`statev2.sqlite`), 55 tables, ~137 migrations, event log pruned to ~1
   day — **projections are the source of truth**, so history is not replayable from events alone.
10. Costs: ~2.2M lines of TS/TSX tracked (≈1.29M excluding tests), 5 apps + 33 packages, Effect
    everywhere, Node 26 + pnpm 12 + `vite-plus`. It is a product, not a starter kit.

---

## 1. What it is — one paragraph + product surfaces

T3 Code is a **server-owns-everything** agent GUI. One `t3` server process owns providers, PTYs, Git,
checkpoints, and the SQLite state for one machine ("environment"); web / desktop / mobile are thin,
reconnectable controllers over authenticated RPC (`docs/internals/overview.md:1-11`). The selling
point is *bring-your-own-subscription*: it never resells tokens, it drives CLIs you are already
logged into (`README.md:13-30`). It is not a framework or SDK — there is no plugin API for
third-party UI; extensibility is MCP tools, provider packages, and repo-local `t3.json` scripts.

| Surface | Where | Notes |
|---|---|---|
| Server | `apps/server` (package name `t3`, `bin: ./dist/bin.mjs`) | Node 26, WS+HTTP, event-sourced orchestration, provider subprocesses |
| Web | `apps/web` (React 19 + Vite) | Served by the server at `/`; also hosted at `app.t3.codes` |
| Desktop | `apps/desktop` (Electron) | Wraps the web bundle **and bundles a server**; can act as host for remote clients |
| Mobile | `apps/mobile` (Expo / React Native, iOS+Android) | Store apps; connects to any server |
| CLI | `apps/server/src/binCli.ts`, `bin.ts`, `binMain.ts` | `t3` = start server + open web; also `pair`, `acp-mcp-call`, build-exe |
| Marketing | `apps/marketing` | Static site, 1.6k LOC, not part of runtime |

Remote is first-class, not bolted on: LAN, Tailscale, SSH, and "T3 Connect" (a Cloudflare relay in
`infra/relay` used only for link/credential management and webhook forwarding — app traffic goes over
the tunnel, not through the relay: `docs/internals/t3-connect.md:1-10`).

---

## 2. Stack & repo shape

**Languages.** TypeScript everywhere (service, web, mobile, desktop main+renderer). Rust for
`native/resource-monitor` and `native/libghostty-vt` (terminal), Swift for `native/apple-speech` and
macOS SnapShot capture, Kotlin/Swift for RN native modules.

**Tooling.** pnpm 12.14 workspaces + `vite-plus` (`vp`) as the task runner (`package.json:5-75`).
`engines.node: 26.13.1`. Typechecking is `effect-tsgo tsrs` (Rust TS7 checker w/ Effect diagnostics) —
there is no `typescript` dependency (`AGENTS.md:141`). Lint is oxlint with a **custom plugin**
`oxlint-plugin-t3code` (rules like `no-test-timeout-or-unreasoned-skip`,
`no-wall-clock-in-tests`, `no-unreasoned-state-sync`).

**Size (measured, excluding the vendored `.repos/` tree of 24,488 files).**

| Area | files (ts/tsx/rs/swift/kt) | total lines | prod lines (excl `*.test.*`, `*.testkit.*`, `/test/`) |
|---|---|---|---|
| `apps/server` | 1,566 | 753,424 | 322,441 |
| `apps/web` | 2,123 | 491,598 | 355,256 |
| `apps/mobile` | 1,603 | 262,178 | 196,166 |
| `apps/desktop` | 291 | 86,687 | 47,993 |
| `packages/*` | 1,442 | 557,890 | 364,840 |
| `apps/marketing` + `native` | 41 | 6,520 | 6,329 |
| **total** | **~7,066** | **~2.16M** | **~1.29M** |

Other counts: **137 migration files**, **55 SQLite tables** (27 `orchestration_v2_*`, 20 of those
projection read models — see §6), **33 packages**, **5 apps**, **10 locales** in
`apps/web/src/i18n/translations`, **105 tracked docs pages**, **90 MCP tools**, **11 provider
drivers**, **6,492 test files**.

**Package layout (table).**

| Path | Purpose |
|---|---|
| `packages/contracts` | The wire. Effect/Schema types for RPC, commands, events, domain entities (112 files; `orchestrationV2.ts` alone is 4,814 lines). No runtime logic. |
| `packages/shared` | Runtime utils used by every surface, subpath exports, no barrel (parsers, citation format, drainable worker). |
| `packages/client-runtime` | Everything shared by web+mobile: connection supervisor, RPC clients, state/atoms, thread merge logic. |
| `packages/provider-core` | The adapter *contract*: `ProviderAdapter.ts` (898 lines), `ModelCatalog`, `ProviderHost`, MCP session/injection, orchestration instructions. |
| `packages/provider-<name>` | One package per non-native driver (`acp`, `acp-registry`, `cursor`, `fx`, `grok`, `kiro`, `muse`, `opencode`, `pi`), each exporting `./client`, `./server`, `./settings`, `./testing`. |
| `packages/effect-codex-app-server`, `packages/effect-acp` | Protocol bindings (Codex JSON-RPC app-server; ACP) as Effect layers. |
| `packages/source-control-*` (11) | GitHub / GitLab / Bitbucket / Azure DevOps / Forgejo / gitcafe / cursor-origin / tanstack + core/definitions/testing. PR create/review/merge per forge. |
| `packages/secret-provider-*` | Secrets: onepassword + core/definitions. |
| `packages/ssh`, `packages/tailscale` | Remote reachability from the client side. |
| `packages/issue-tracker-linear` | Linear issue link/search. |
| `native/` | Rust `resource-monitor`, `libghostty-vt`; Swift `apple-speech`, `browser-secret`, Hyprland/KDE snapshot helpers. |
| `infra/relay` | Cloudflare Worker: T3 Connect links, tunnel allocation, webhook forwarder. |
| `evals/` | Model-behaviour evals for prompt/tool-description changes (`evals/README.md`). |
| `.repos/` | Vendored read-only reference libs (Effect etc.) — 24k files, not part of the build. |

---

## 3. Core domain model

The whole model lives in `packages/contracts/src/orchestrationV2.ts`. Terms also defined in
`docs/internals/glossary.md`.

```
Environment (one server, owns fs+creds+state)
└── Project (dir on that env, may track many git repos)
    └── AppThread (durable conversation; provider-agnostic)
        └── Run (ordinal, counted user-visible turn)          :793
            ├── RunAttempt (initial | steering_restart | retry | provider_recovery)
            └── ExecutionNode tree (root_turn → tool_call | reasoning | plan | approval_request
                | user_input_request | subagent | …)          :877
        ProviderSession (process/account handle)               :1076
        ProviderThread  (native conversation handle)          :1220
        TurnItem[] (26 kinds, the visible timeline)            :1849
        Checkpoint[] + CheckpointScope[]  (hidden git refs)    :1592
        ContextTransfer[] / ContextHandoff[] (lineage + transfer)  :248
```

| Entity | Definition | Key fields |
|---|---|---|
| `OrchestrationV2AppThread` | `:475` | `projectId`, `providerInstanceId`, `modelSelection`, `runtimeMode`, `interactionMode`, `branch`, `worktreePath`, `activeProviderThreadId`, `lineage{parentThreadId,relationshipToParent,rootThreadId,sideChat}`, `forkedFrom`, `delegatedFrom`, `pullRequests[]`, `issues[]`, `snoozedUntil`, `settledAt`, `limitRecovery`, `guestShare` |
| `OrchestrationV2Run` | `:793` | `ordinal`, `providerInstanceId`, `providerThreadId`, `activeAttemptId`, `status`, `queuePosition`, `queueHeld`, `checkpointId`, `contextHandoffId`, `restartContinuationOfRunId`, `delegatedTaskId`, `freshContext`, `outputSchema`, `output` |
| `RunStatus` | `:692` | `preparing \| queued \| starting \| running \| waiting \| completed \| interrupted \| failed \| cancelled \| rolled_back` |
| `OrchestrationV2ExecutionNode` | `:877` | `parentNodeId`, `rootNodeId`, `kind` (11 values incl. `hook` — unused in code), `status`, `countsForRun`, `providerTurnId`, `nativeItemRef`, `runtimeRequestId`, `checkpointScopeId` |
| `OrchestrationV2ProviderSession` | `:1076` | `driver`, `providerInstanceId`, `status`, `cwd`, `model`, `capabilities`, `extensionStatuses` |
| `OrchestrationV2Subagent` | `:1003` | `origin: "provider_native" \| "app_owned"`, `childThreadId`, `remoteChild`, `prompt/title/model`, `completionWake`, `completionDelivery`, `workflow` |
| `OrchestrationV2TurnItem` | `:1849` | 26 tags: `user_message, assistant_message, reasoning, proposed_plan, todo_list, command_execution, file_change, file_search, web_search, approval_request, user_input_request, checkpoint, notification, system_notice, error, compaction, handoff, fork, run_interrupt_request/result, subagent, dynamic_tool` |
| `OrchestrationV2ContextTransfer` | `:248` | `type` (`fork\|provider_handoff\|merge_back\|subagent_spawn\|subagent_result`), `sourcePoint`, `basePoint`, `status` (`pending\|resolved_native\|resolved_portable\|failed\|consumed\|superseded`) |
| `ModelSelection` | `packages/contracts/src/modelSelection.ts:22` | `{instanceId, model, options?}` — **instance**, not provider kind; options are provider-defined (`effort`, `speed`…) |
| `RuntimeMode` | `providerPolicy.ts:25` | `approval-required \| auto-accept-edits \| auto \| full-access`, ranked; `DEFAULT = full-access` |
| `ProviderInteractionMode` | `providerPolicy.ts:45` | `default \| plan` (separate from permissions) |

**How they relate.** A thread always has exactly one *selected* provider instance + model, but can
accumulate many `ProviderThread`s over its life (provider switches, forks). A run targets one
`ProviderThread` and one `providerTurn`; root completion is the only thing that completes a
user-visible run — child turns never do (`docs/orchestration-v2/README.md:30-40`). Two threads may
share a `ProviderSession` only when the driver declares `supportsMultipleProviderThreadsPerSession`
— today only Codex (`CodexAdapterV2.ts:298`; enforced in `ProviderSessionManager.ts:3179-3184`).

---

## 4. Agent / provider integration

### 4.1 The abstraction

- **Driver** = integration (`codex`, `claude`, `cursor`, `grok`, `fx`, `opencode`, `antigravity`,
  `kiro`, `pi`, `muse`, `acpRegistry`). **Instance** = one configured account/binary
  (`ProviderInstanceId`), so two accounts on one driver never share session/catalog state
  (`docs/internals/providers.md:8-12`).
- Registration: `apps/server/src/provider/builtInDrivers.ts:33` (`BUILT_IN_DRIVERS`), plus a
  per-driver compatibility policy in `provider/model-manifest.json`.
- The contract is `ProviderAdapterV2` (`packages/provider-core/src/server/ProviderAdapter.ts:877`):
  - service surface: `getCapabilities()`, `planSelectionTransition()`, `openSession()` → `SessionRuntime`
  - `SessionRuntime` (`:760`) methods: `ensureThread`, `resumeThread`, `injectHistory`,
    `startTurn`, `resumeTurn`, `compactThread`, `steerTurn`, `applyModelSelection`,
    `applyThreadTitle`, `stopTask`, `awaitBackgroundTasks`, `interruptTurn`, `stopSubagent`,
    `unloadThread`, `respondToRuntimeRequest`, `readThreadSnapshot`, `rollbackThread`, `forkThread`,
    `uploadFeedback`, `mcpApps`, `mcpSignIn`, plus probes `hasPendingBackgroundWork`,
    `backgroundTasksEndWithProcess`, `getModelContextWindow`.
- Inputs are app-shaped: `ProviderAdapterV2TurnInput` (`:584`) carries `appThread`, `runId`,
  `rootNodeId`, `message` (text + attachments + provenance), `modelSelection`, `runtimePolicy`
  (`cwd`, `approvalPolicy`, `sandboxPolicy`, `workspace.folders/extraRoots`), `appContext`.

### 4.2 Streaming event protocol

Adapters emit **one normalized union**, `ProviderAdapterV2Event` (`:100-236`), 18 variants over 17
types (`turn.terminal` has a success form and a `failed` form carrying `ProviderFailure`):
`app_thread.created`, `provider_session.updated`, `provider_thread.updated`,
`thread_model_selection.updated`, `provider_turn.updated`, `node.updated`, `subagent.updated`,
`message.updated`, `turn_item.updated`, `steer.updated`, `background_work.updated`,
`background_work.lost`, `command_output.delta` (ephemeral, not persisted),
`prompt_suggestion.updated` (ephemeral), `runtime_request.updated`, `plan.updated`,
`turn.terminal` (two variants: success-family and `failed` w/ `ProviderFailure` + optional retry).

Design rules visible in the types: app ids everywhere, provider ids only as `providerThreadId` /
`providerTurnId` refs; `turn.terminal` carries `threadDisposition: "reusable"|"broken"` (the session
cannot be reused after a broken turn); `retainedRuntimeRequestIds` says which approvals/questions
outlive the turn. Raw provider frames go to **rotating diagnostic logs only**, never the DB
(`docs/internals/providers.md:190-200`).

### 4.3 How each CLI is actually driven

| Driver | Transport | Spawn / attach | Multi-thread per session | Notable |
|---|---|---|---|---|
| `codex` | Codex **app-server** JSON-RPC over stdio (`packages/effect-codex-app-server`) | `spawner.spawn(...)` in `CodexAdapterV2.ts:1913` | **yes** | richest protocol: `thread/fork` w/ `lastTurnId`, `thread/revert`, native subagents, `/goal` multi-turn runs |
| `claude` | `@anthropic-ai/claude-agent-sdk` in-process; SDK spawns the CLI itself | `spawnClaudeProcessGroup()` `ClaudeAdapterV2.ts:425` | no | tracks descendant PIDs; `/compact`; hooks/goal state read from transcript |
| `cursor` | `@cursor/sdk` in-process | SDK `Agent.create`/`resume` | no | MCP servers passed per-`send` |
| `pi` | **`pi --mode rpc`** JSON-RPC over stdio | `ChildProcess.make(... detached)` `provider-pi/src/server/rpc.ts:221` | no | T3 injects an extension file for MCP; keeps Pi's own extension/package loader |
| `opencode` | `@opencode-ai/sdk` over its HTTP server + events | 1.x: **one managed server per thread**; 2.x: **one server per instance**, per-thread MCP entry | no | version-probed 1.x/2.x adapters |
| `grok`, `kiro`, `fx` | **ACP** (stdio JSON-RPC) via `packages/provider-acp` | shared ACP adapter + small flavor | no | Grok adds xAI extensions; steering = interrupt-and-restart |
| `antigravity` | ACP, own driver | managed installer + per-instance profile | no | isolates Google accounts per profile; own realm/leases |
| `muse` | `@muse-code/sdk` | in-process | no | — |
| `acpRegistry` | ACP, generic | resolves agent from official ACP Registry, installs binary (checksums) or `npx`/`uvx` globally, launches it | no | zero per-agent code; capability-gated degradation |

### 4.4 Session resume / continue

- Three ids: `providerSessionId` (process/session), `providerThreadId` (native conversation),
  `providerTurnId` (native turn). Persisted in projections and re-bound on restart
  (`orchestration_v2_projection_provider_session_bindings`).
- `SessionRuntime.canResumeAfterRestart` gates whether conversations survive a server restart;
  `resumeThread({ importedNativeSession })` fails loudly when the native session was the only copy
  of imported history.
- `pendingRequestsSurviveRestart` is `false` for both Codex and Claude
  (`CodexAdapterV2.ts:305`, `ClaudeAdapterV2.ts:575`) — old approvals are marked `not_resumable`
  rather than pretending they are live.
- Long-lived threads get **idle compaction** (`IdleCompactionService.ts`, `IDLE_COMPACTION_TEXT =
  "/compact"` at `orchestrationV2.ts:597`) and manual `/compact`; drivers without native compaction
  raise `ProviderCompactionUnsupportedError`.

### 4.5 Permissions, models, auth

- Runtime modes are *provider-enforced only*; T3 refuses to imitate a missing mode, and an
  unoffered stored mode silently runs as `approval-required` (`RuntimePolicy.ts`,
  `docs/internals/adding-a-provider.md:30-34`).
- Model catalog per instance (`ModelCatalog.ts`), and `ModelSelection.options` are advertised by the
  live session — writing an option the session has not advertised is treated as skipped, not failed.
- Auth = the user's existing CLI login. Per-instance sign-in flows live in
  `apps/server/src/provider/*Auth*.ts` (Codex ChatGPT device/OAuth + a cross-machine
  "primary handoff" that transfers tokens once, `CodexChatGptHandoff.ts`). T3 stores API keys only
  via secret providers (`onepassword`) or env vars, never in the repo.
- The **agent-facing** credential is separate: each provider session gets a bearer token scoped to
  (environment, parent thread, provider instance, session) for the local MCP endpoint
  (`mcp/McpSessionRegistry.ts`, `docs/orchestration-v2/orchestrator-mcp-server.md:38-58`).

---

## 5. Concurrency & orchestration

- **Dispatch modes** on a message (`orchestrationV2.ts:3569-3577`): `start_immediately`,
  `queue_after_active`, `steer_active{targetRunId}`, `restart_active{targetRunId}`.
- **Per-thread serialization**: one active run; others queue with `queuePosition`/`queueHeld`.
  Delivery order is a merge of "delegated completion wakes" and user messages so neither starves the
  other (`QueuedRunOrder.ts:17-42`). Stop interrupts the turn, *holds* queued turns, ends PR watches.
- **Parallelism comes from threads, not from one thread's runs.** N threads per project, each own
  provider session (except Codex), each own cwd/worktree. Server work is Effect fibers with a
  bounded `EffectWorker` draining the outbox.
- **Worktrees.** `workspaceStrategy` on launch: `root` (default), `worktree{baseRef,
  startFromOrigin}`, `existing_worktree` (`orchestrationV2.ts:756+`). For a project tracking several
  Git repos, a worktree thread gets a **task folder**: one directory holding the project folder plus
  a worktree of each chosen repo at the same relative paths, all on the thread's branch
  (`docs/internals/overview.md:86-99`, `TaskFolders.ts`, table `task_folders`).
- **Subagents** are two shapes: `provider_native` (Codex/Pi native children, observed; Claude cannot
  fork from a subagent thread) and `app_owned` (T3 creates a child *thread* + run via
  `delegate_task`). App-owned children are ordinary threads with `lineage.relationshipToParent =
  "subagent"` and can be messaged; native children cannot (`isProviderNativeSubagentThread`,
  `orchestrationV2.ts:584-594`).
- **Cancellation** is layered: `thread.stop` (user Stop) → `run.interrupt` → `task_cancel` (MCP,
  cascades to every child, cancels the child's own queued turn, disposes parent delivery) →
  adapter `interruptTurn` / `stopSubagent`.
- **"Next"/delegate contract.** `delegate_task` returns `next` (`OrchestratorMcpRunningTaskNext`,
  `orchestratorMcp.ts:256`), and the injected system prompt makes it binding: *"Follow a running
  delegate's `next`. Don't poll it (`task_status`, `sleep`, watchers, Monitor, ScheduleWakeup)"*
  (`orchestrationInstructions.ts:12`). The same file forbids starting subagents from inside a
  subagent, and forbids `t3_thread_interrupt` on a `childThreadId` (use `task_cancel`).
- **Background waits.** A completed turn can still be *waiting*: `blockedOnThreadWait`,
  `pendingBackgroundTasks` (`orchestrationV2.ts` thread shell fields), `t3_background_wait`,
  `NativeChildWorkQuery.ts`, and `hasPendingBackgroundWork` gates session release so a dev server or
  subagent keeps the thread in "Waiting" instead of being killed.
- **Settlement** (auto-settle on PR merge / inactivity) is server-owned
  (`ThreadSettlementService.ts`) and the PR sync reactor re-reads links when a run that ran
  `gh pr merge` ends — so a merge performed by an agent, which sends no webhook, is still noticed
  (`PullRequestSyncReactor.ts`, `docs/internals/overview.md:70-80`).

---

## 6. State & persistence

- **One SQLite file**: `$T3CODE_HOME/userdata/statev2.sqlite` (`apps/server/src/config.ts:149-151`),
  default `~/.t3`. A git worktree defaults to its own gitignored `.t3` so dev cannot touch live state
  (`AGENTS.md:100-104`). Attachment bytes live outside the workspace (`attachmentStore.ts`).
- **Write path** (the thing to copy): `command → Orchestrator decides events (no I/O) → EventSink
  commits {events, projection rows, command receipt, outbox effects} in one transaction → publish
  after commit → EffectWorker runs effects → results come back as commands`
  (`EventSink.ts:107`, `:299-320`, `EffectWorker.ts:899-930`).
- **Tables** (55): `orchestration_v2_events`, `orchestration_v2_command_receipts`,
  `orchestration_v2_effect_outbox`, and 20 `orchestration_v2_projection_*` tables (`threads, runs,
  run_attempts, nodes, turn_items, messages, provider_sessions, provider_threads, provider_turns,
  runtime_requests, subagents, checkpoints, checkpoint_scopes, context_transfers, context_handoffs,
  plans, thread_item_counts, turn_item_counts, metadata, …`) plus `orchestration_v2_turn_item_positions`,
  `orchestration_v2_thread_launch_workflows`, `orchestration_v2_legacy_imports`, and
  non-orchestration tables (`scheduled_tasks`, `scheduled_task_webhook_*`, `auth_sessions`,
  `peer_environment_links`, `task_folders`, `pull_request_*`, `checkpoint_diff_blobs`,
  `mcp_app_model_context`). 12 legacy V1 tables (`projection_*` ×10 plus `orchestration_events`,
  `orchestration_command_receipts`) remain for the importer.
- **Source of truth = projections, not the event log.** Events are retained ~1 day
  (`ProjectionMaintenance.ts:42 RETAINED_EVENT_HISTORY = Duration.days(1)`); the maintenance service
  prunes everything the projections cover, keeping only each stream's high-water mark,
  `thread.created`, every `running` run update, and prompt anchors. Consequence stated plainly in
  the doc: *"projections cannot be rebuilt from the log"* (`docs/internals/overview.md:47-56`). A
  client whose cursor is below the pruned mark resumes from a snapshot.
- **Idempotency**: `clientRequestId`/`commandId` → durable receipt; retries return the same result,
  including across restarts, even after the events it produced were pruned.
- **Transcript storage**: normalized `turn_items` + `messages` rows, each item has
  `ordinal`, `runId`, `nodeId`, `messageId`, `type`, and a JSON payload. FTS table
  `orchestration_v2_message_search` backs `t3_thread_search` (`messageSearchIndex.ts`).
- **Checkpoints are not in the DB.** Hidden Git refs via `checkpointing/CheckpointStore.ts`; one
  root scope per thread plus one sparse `repository` scope per tracked Git folder a turn touches; a
  `checkpoint.captured` record marks it. Diffs come from checkpoints, which is why Codex opts out of
  `turn/diff/updated` (`docs/internals/providers.md:200`).

---

## 7. Transport / IPC

- **Protocol**: Effect RPC (`Rpc.make` / `RpcGroup.make`) over a WebSocket at `/ws`, plus plain HTTP
  for uploads, previews, OAuth callbacks and `/mcp`. In dev, Vite proxies `/api`, `/ws`, `/oauth`
  (`AGENTS.md:77`). `packages/contracts/src/rpc.ts` is 2,621 lines and holds the single
  `WsRpcGroup` at `rpc.ts:2373` with **247 typed RPCs**, named by 120 constants in
  `WS_METHODS` (`rpc.ts:469-712`).
- **Server dispatch**: `apps/server/src/ws.ts` (3,868 lines) authenticates the socket at upgrade,
  then per-method decodes input, calls one service method, maps errors. Authorization is per-method,
  not per-socket: `CLIENT_GUARDED_RPC_SCOPES` in contracts + `permissionAtom` in the client
  (`AGENTS.md:199`).
- **Sync model**: snapshot + cursor streams, not a global store dump. `ThreadStream.ts` (per-thread),
  `ShellStream.ts` (sidebar shells), `snapshotSequence` so a client knows whether it missed pruned
  events; `ThreadLiveEventCoalescer.ts` bounds chatty live updates; `LiveStreamBudget.ts` and a
  wire-level body-dedup marker (`DedupedBodyMarker`, `orchestrationV2.ts:1010`) exist purely to cut
  bytes on the socket.
- **Client side**: one connection owner per environment in `packages/client-runtime/src/connection/`
  (supervisor w/ jittered backoff capped at 5 min, route walk, 500 ms socket probes —
  `docs/internals/connection-runtime.md`). Web/desktop/mobile all reuse it.
- **Desktop IPC**: Electron main exposes a preload/IPC surface typed in `packages/contracts/src/ipc.ts`
  (1,450 lines); the renderer is the same web app.
- **Remote**: direct, Tailscale, SSH, or T3 Connect (relay). Identity is per-environment and stable;
  routes are ordered hints, and the driver walks them (`docs/internals/remote.md:1-45`).

---

## 8. Frontend architecture

- **Framework**: React 19 + Vite, no Next.js. Routing: `@tanstack/react-router` with generated
  `routeTree.gen.ts`; route files mirror screens (`_chat.$environmentId.$threadId.tsx`,
  `_chat.settings.*`, `_chat.pull-requests`, `_chat.issues`, `_chat.new`, `approve-agent`).
- **State**: `@effect/atom-react` atoms in `packages/client-runtime/src/state/*` (the bulk of shared
  state: `entities`, `connections`, `threads-sync`, `composerDispatch`, `commandPermissions`,
  `threadHistoryController/merge`, `boundedThreadSnapshotHttp`), plus `zustand` for a few local web
  stores (`composerDraftStore`, `commandPaletteStore`, `closedViewStore`) and
  `use-sync-external-store` glue. Mutations go through `createEnvironmentRpcCommand` so every write
  carries a `commandId`, optimistic state, and a permission check.
- **Streaming markdown**: bespoke pipeline — `ChatMarkdown.tsx` + `MarkdownCodeBlock.tsx` with
  Shiki-based incremental highlighting (`apps/web/src/lib/incrementalHighlighting.ts`), KaTeX,
  DOMPurify, `hast-util-to-jsx-runtime`, `entities`, plus `@pierre/diffs` and `@pierre/trees` for
  diff/tree views.
- **Virtualization**: `@legendapp/list` in both web and mobile chat timelines; per-thread
  `ThreadLiveEventCoalescer` + bounded history paging (`threadHistoryPaging.ts`) keep rows cheap.
- **Design system**: Tailwind v4 + `@base-ui/react` primitives with a local
  `components/ui` layer whose rule is "variants, never restyle with className" (enforced by
  `shadcn/no-restyle`, `AGENTS.md:201-204`). Icons `lucide-react`. Themes are user-editable and
  agent-editable (`t3_theme_read/set` MCP tools).
- **Composer**: Tiptap with a custom inline node per context kind
  (`ComposerPromptEditorTiptap.tsx`), sending canonical `[label](t3-context://v1/<kind>/<id>)` links
  plus `message.context.records` (see §10).
- **i18n**: `gt-react` + 10 JSON locale files; `vp run i18n:check` gates untranslated keys.
- **Mobile**: Expo, `@react-navigation/native-stack` (not the web router), `uniwind` for styling,
  `react-native-enriched-markdown`, its own diff/terminal/highlight native modules.

---

## 9. Extensibility

| Mechanism | How | Evidence |
|---|---|---|
| **MCP tools (agent-facing)** | 90 tools in 15 toolkits (`orchestrator, thread, project, environment, device, preview, pullRequests, issues, message, attachment, html, theme, worktree, toolSearch, core`) served at `http://127.0.0.1:<port>/mcp`, injected into every provider session with a per-session token | `apps/server/src/mcp/toolkits/*/tools.ts`, `McpHttpServer.ts` |
| **MCP apps (UI in chat)** | Provider can render MCP-UI resources; adapters expose `mcpApps.listTools/callTool/readResource`, plus `mcpSignIn` for OAuth'd MCP servers; model context persisted in `mcp_app_model_context` | `ProviderAdapter.ts:716-758`, `apps/server/src/mcpApps` |
| **Skills** | Composer `skill` context record + `providerSkills.ts` in client-runtime; skills come from the provider's own loader (Pi/Claude), T3 only references them | `contracts/composerContext.ts:207` |
| **Project scripts** | `t3.json` at the repo root declares actions with `runOnWorktreeCreate`, `runOnSettle`, `stopOnSettle`, plus workspace/submodule/sparse-checkout defaults. This repo's own `t3.json` uses it to seed worktrees | `contracts/src/t3ProjectFile.ts:38-55`, `docs/user/project-settings.md:190-208` |
| **Scheduled tasks** | interval / fixed-time / webhook triggers; webhook signature, filters, replay/coalesce dedupe, held-while-offline via relay; `bindToCurrentThread` posts into an existing thread | `apps/server/src/scheduledTasks/*` (8.5k LOC), table `scheduled_tasks` |
| **Custom providers** | New `packages/provider-<x>` + one line in `builtInDrivers.ts` + client definition + model-manifest policy. ACP agents need **no code** — add them as an `acpRegistry` instance | `docs/internals/adding-a-provider.md:118-140` |
| **Source control** | 11 packages, one per forge, each naming its CLI commands (e.g. `changeRequestCloseCommand`) so merge detection stays generic | `packages/source-control-*` |
| **Issue trackers** | Linear package + `issue_tracker_bindings`; threads carry `issues[]` | `packages/issue-tracker-linear`, `ThreadIssueService.ts` |
| **Secrets** | Secret providers (1Password) + `request_secret` MCP tool so an agent can ask the human for a value without it entering the transcript | `packages/secret-provider-*`, `secrets/` |
| **Preview automation** | Agent drives a real browser tab via 13 `preview_*` tools (snapshot/act/evaluate/record) and can publish HTML results inline (`html_preview`+`html_render`) | `toolkits/preview`, `mcp/PreviewAutomationBroker.ts` |
| **Devices** | Simulator/emulator panel + `device_*` tools backed by `expo-device-hub` and `agent-device` | `device/`, `docs/internals/devices.md` |

There is **no** third-party plugin API for the UI, no extension host, no user-defined slash commands
(commands are a fixed provider-level set), and no sandbox for MCP servers beyond process env.

---

## 10. Shared context / cross-agent communication (CRITICAL)

### 10.1 What exists today

1. **Shared filesystem, by default.** Thread cwd = project root unless a worktree was requested;
   `ProviderAdapterV2RuntimePolicy` passes `cwd` plus `workspace.folders`/`extraRoots` so multi-repo
   projects give the agent several writable roots (`ProviderAdapter.ts:64-76`). Two threads in the
   same checkout genuinely see each other's files. Worktree threads do **not**: a child gets the
   parent's *commit*; "Uncommitted parent edits are not copied"
   (`docs/orchestration-v2/orchestrator-mcp-server.md:262-266`).
2. **Cross-thread reads.** `t3_thread_read` returns messages, plans, or `activity` (tools, reasoning,
   checkpoints, handoffs, runtime requests) for **any thread in the environment**, paginated by
   `beforePosition`/`afterPosition`, ~30k chars per call
   (`toolkits/thread/tools.ts`, `docs/orchestration-v2/orchestrator-mcp-server.md:395-430`).
3. **Cross-thread discovery.** `t3_thread_search` (title/branch/PR/project + FTS over message text,
   returning snippets and links) and `t3_thread_list` (project-scoped, filterable by status/lineage).
4. **Cross-thread messaging.** `t3_thread_send` with `auto|queue|steer|restart`, bounded by the
   caller's own runtime/interaction mode (a thread may not grant another more privilege than it has).
   `t3_thread_update/organize`, `t3_queue` (list/edit/cancel/promote-to-steer).
5. **Delegation.** `delegate_task` creates a **child thread + run** on a chosen provider instance,
   inheriting modes (never escalating), optionally `isolation: "worktree"`, and returns
   `taskId`/`childThreadId`/`link`/`next`. Child gets **only the task prompt** — parent history is
   never copied (`orchestrator-mcp-server.md:14-18`).
6. **Structured results, not prose parsing.** `outputSchema` on send/launch + `t3_submit_output`
   lets a parent receive `output` as validated JSON on the run (`orchestratorMcp.ts`, §"Tool Surface").
7. **Lineage + context transfer.** `ContextTransfer` records `fork | provider_handoff | merge_back |
   subagent_spawn | subagent_result`, with a `sourcePoint` (run/checkpoint/turn/item/native ref) and
   lazy resolution: `native_fork` when the same provider supports it, else a materialized
   `ContextHandoff`. Both are auditable rows, not hidden prompt hacks
   (`docs/orchestration-v2/thread-lineage-and-context-transfer.md:96-160`).
   `t3_thread_transfer` exposes `fork`/`merge_back`/`status` to agents
   (`toolkits/thread/tools.ts:167-192`).
8. **Portable handoff content.** `ContextHandoffServiceV2` summarizes user/assistant messages, command
   executions, file changes and checkpoints, trimming each text item to 240 chars; provider switches
   use full history or a delta since that provider last participated; merge-back uses the fork delta.
   It explicitly does **not** reconstruct native session state, tool state or approvals
   (`docs/internals/context-handoffs.md`).
9. **A shared behavioural contract in every session.** `orchestrationInstructions.ts:3-30` is
   injected into Codex / Claude / Cursor / OpenCode / Pi / ACP sessions (ACP gets it wrapped in
   `<t3_code_instructions>` on each prompt change; wired per adapter in `CodexDeveloperInstructions.ts`,
   `ClaudeAdapterV2.ts`, `provider-cursor`, `provider-opencode`, `provider-pi/mcpExtensionSource.ts`,
   `provider-acp/src/server/adapter.ts`). It defines when to delegate vs. use a native
   subagent, that a delegate's `next` must be followed without polling, that a subagent may not
   spawn subagents, how to name threads (`link`, verbatim), and how to ask a human for a secret.
10. **User-authored cross-thread context.** The composer has a `thread` context record
    (`environmentId`, `threadId`, `title`) and an `environment` record, sent as
    `[title](t3-context://v1/thread/<id>)` + a `<t3_context version="1">` envelope appended to the
    provider prompt (`contracts/composerContext.ts:213-232`,
    `docs/internals/composer-context-references.md:60-72`). So a user can point thread B at thread A
    without A knowing.
11. **Citations & links.** Assistant answers carry `AssistantCitation` links that navigate to the
    source thread/message and expand to quoted text before dispatch; `t3-thread://v2/<env>/<thread>`
    links are the canonical way agents reference a thread (`docs/internals/assistant-citations.md`,
    `client-runtime/src/linkedThread.ts`).
12. **Cross-environment.** Peer links (`peer_environment_links`, `apps/server/src/peer/*`,
    `t3_environment_link`) let an agent on machine A delegate to / read threads on machine B; results
    name the owning environment (`t3_environment_links`).
13. **Recurring shared context.** `schedule_task(bindToCurrentThread)` posts runs into an existing
    thread on a schedule or webhook, so a "watcher" agent can keep contributing to the same thread.
14. **Indirect shared state.** PRs and issues are linked to threads (`pullRequests[]`, `issues[]`) and
    kept fresh by `PullRequestSyncReactor`; checkpoints let any thread diff/rewind the worktree;
    `t3_worktree_list/status/handoff` expose the checkout map.

### 10.2 What does NOT exist

- **No shared memory store.** No vector DB, no embedding index, no cross-thread knowledge base. The
  only cross-thread index is SQLite FTS over message text (`orchestration_v2_message_search`).
- **No message bus / pub-sub / broadcast.** No way for one agent to announce something to "all
  threads". Communication is point-to-point via tools, or parent→child via `delegate_task`.
- **No @mentions between agents.** @-mentions in the composer are *files* (`mention` record) and
  `skill`s; a thread mention exists but is a user gesture, not an agent-to-agent address.
- **No shared task board / DAG across threads.** `todo_list` is a per-run turn item; workflows are
  provider-native artifacts displayed as a `subagent.workflow` tree. `delegate_task` creates a flat
  parent→child relation, not a DAG with dependencies.
- **No automatic context propagation.** Children see only their prompt; siblings see nothing.
  Another thread's transcript is invisible until someone calls `t3_thread_read`/`t3_thread_search`.
- **No agent-to-agent approval authority.** A parent can read a child's pending question and answer
  it (`t3_pending_request`), but approvals are explicitly **not** routed
  (`orchestrator-mcp-server.md:496-500`), and the parent's `pendingApproval` "grants no approval
  authority".
- **No merge arbitration.** `merge_back` transfers a *context summary*, not code; nothing reconciles
  two worktrees' conflicting edits.
- **`ExecutionNode.kind: "hook"` is declared but unused** (`orchestrationV2.ts:893`; no producer in
  non-test code). There is no user-facing hook system for agents.
- **No cross-project scope.** Thread tools are project-scoped for list/search and
  environment-scoped for mutation; there is no "workspace of projects" object.
- **No shared terminal session across threads.** PTYs are per-thread; `terminal` context records
  copy an excerpt, not a live handle.

---

## 11. Notable engineering decisions

- **Pure decision core + durable outbox.** Command handling cannot do I/O; effects are rows committed
  in the same transaction, claimed later (`EventSink.ts:107`, `EffectWorker.ts:899`). This is why
  "accepted" ≠ "done" everywhere in the API and why a lost provider process can be retired rather
  than replayed.
- **App ids primary, provider ids as refs.** Enforced in the event union itself
  (`ProviderAdapter.ts:100-236`); forks/handoffs are app records (`ContextTransfer`), and a
  provider-native fork is only a *resolution strategy*.
- **Capabilities over driver names.** `OrchestrationV2ProviderCapabilities` (`orchestrationV2.ts:406`)
  with 11 sub-structs; missing capability → policy fallback (interrupt-restart steering, portable
  context forks), never an `if (driver === "codex")` in orchestration.
- **Normalized events, never cross-translated.** "Provider events are never rewritten to look like
  another provider event" (`docs/orchestration-v2/README.md:31`) — each adapter owns its own
  semantics.
- **Projections are the truth; the log is a replay buffer.** Deliberate: prune at 1 day, keep
  high-water marks, resume from snapshot. Simple to operate, hostile to event-replay purists.
- **Checkpoints as hidden Git refs**, sparse per repository touched, never commits on the user's
  branch; rollback must coordinate provider conversation state or refuse before touching files
  (`docs/internals/overview.md:60-85`).
- **One adapter package per provider, with a hard boundary**: server imports only
  `@t3tools/provider-core/server/*`, contracts and shared — never `apps/server`
  (`docs/internals/adding-a-provider.md:130-136`).
- **Replay-fixture testing instead of mocked frames.** Adapters are tested by replaying *recorded
  real transcripts* through the real orchestrator (`record-*-replay-fixture.ts` scripts,
  `docs/orchestration-v2/testing-strategy.md`). "A test that drives the adapter with hand-written
  frames is a guess about the protocol."
- **Instruct the model, don't just expose tools.** ~90 tools plus a carefully worded injected
  contract, with `evals/mcp-tool-use` to measure whether models actually use them correctly.
- **Performance as a stated invariant.** Wire-level body dedup, live-event coalescing, bounded
  stream budgets, "a dropped frame is a bug" (`AGENTS.md:22-27`), and a per-file performance test
  suite (`test:perf:v2-wire`).

---

## 12. What to reuse for our product (ranked)

1. **The whole orchestration shape**: `command → pure decide → events+projections+receipts+outbox in
   one txn → worker runs effects`. `apps/server/src/orchestration-v2/{Orchestrator,EventSink,
   EffectWorker,ProjectionStore}.ts` + `docs/internals/overview.md`. This is the single most
   valuable idea here: it is what makes multi-agent UI honest about "queued vs running vs waiting".
2. **`ProviderAdapterV2` as the adapter contract** (`packages/provider-core/src/server/
   ProviderAdapter.ts`) — including the *shape* of the event union and the capability structs. Even
   if we write our own, copying the 14 normalized event kinds and the 11 capability groups saves
   months of discovering them per provider.
3. **Capability-driven policy instead of driver branching** (`RuntimePolicy.ts`,
   `ProviderSessionTransitionPolicy.ts`, `docs/orchestration-v2/provider-capability-system.md`).
4. **`ContextTransfer` + `ContextHandoff` for cross-agent handoff** (`ContextHandoffService.ts`,
   `docs/orchestration-v2/thread-lineage-and-context-transfer.md`). Lazy resolution
   (native fork if same provider, else a bounded textual handoff) is exactly the primitive a
   multi-CLI orchestrator needs, and it is auditable.
5. **The agent-facing MCP tool surface** (`apps/server/src/mcp/toolkits/orchestrator|thread/`). Our
   "threads can see and talk to each other" is 90% this: read/list/search/send/fork/merge/delegate/
   wait/submit_output/pending_request. Copy the tool *descriptions* too — they are tuned.
6. **The injected collaboration contract** (`provider-core/src/server/orchestrationInstructions.ts`).
   Directly reusable text; it encodes the failure modes (polling, nested subagents, privilege
   escalation) that otherwise show up as model misbehaviour.
7. **`t3_thread_read`'s bounded, paginated timeline read** + `outputSchema`/`t3_submit_output` for
   machine-readable handoffs — far better than parsing a child's prose.
8. **Checkpoint-per-run via hidden Git refs** (`checkpointing/CheckpointStore.ts`) — gives "what did
   this agent actually change" and per-run rewind without polluting git history.
9. **Snapshot + cursor streaming with a pruned log** (`ThreadStream.ts`, `ShellStream.ts`,
   `snapshotSequence`). Cheaper and more robust than replaying full history to every client.
10. **Replay-fixture provider testing** (`apps/server/scripts/record-*-replay-fixture.ts`,
    `docs/orchestration-v2/testing-strategy.md`) — the only way to test 11 protocols without 11 CI
    logins.

---

## 13. What to avoid / gotchas

- **License**: MIT for the repo (© T3 Tools Inc), so reuse is permitted with attribution.
  `third-party-licenses.config.json` + `third-party-licenses/spdx` exist because some vendored deps
  are not MIT — check before shipping a derived bundle. The **hosted** pieces are not in this repo's
  MIT grant in practice: `app.t3.codes`, the Clerk-backed relay config, App Store builds, and
  product analytics (see §15).
- **Sheer weight**: 1.29M prod LOC, Effect + `effect-tsgo` as a hard toolchain dependency, Node 26
  and pnpm 12 pins, 137 hand-written migrations, a custom oxlint plugin, and a 3,868-line `ws.ts`.
  Copying patterns is realistic; forking the repo is a career move.
- **Effect is load-bearing**, not decorative: `Context.Service` DI, `Stream`, `PubSub`,
  `Deferred`, `TestClock`. Half-adopting it produces the worst of both worlds.
- **Single-writer SQLite + one event log per environment** assumes one machine owns the data. It does
  not scale horizontally, and the 1-day pruned log means "replay from zero" is not a recovery option.
- **The event-sourced paths are subtle**: `ProjectionMaintenance` pruning, `snapshotSequence`
  staleness, `resume` re-checking the prune mark after replay, `thread.created` retention for the
  legacy importer. Easy to break by "cleaning up" retention.
- **Provider sessions are the scarce resource.** All drivers except Codex refuse to attach two app
  threads to one session (`ProviderSessionManager.ts:3179`), so "N threads" ≈ "N subprocesses" —
  memory/CPU cost is real, and `ProviderRuntimeRecoveryService` exists because they die.
- **`apps/mobile` has 25+ native modules**; `vp run mobile:install` is required before Metro. Not a
  light "add a mobile client" story.
- **Unstable-looking internals**: 105 docs pages, nightly builds, frequent adapter rewrites
  (V1→V2 orchestration, `legacy/` importer, OpenCode 1.x/2.x dual adapters). Pin a commit.
- **Known sharp edges documented in-repo**: "Stop sent before the turn started" can be ignored by
  providers; "default" model is a sentinel and must never be forwarded; usage limits must be
  classified as `usage_limit` not generic failure; settings written before the provider advertises
  them silently do nothing (`docs/internals/adding-a-provider.md:88-116`).
- **Maintainer policy**: features need an approved discussion before a PR; one problem per PR
  (`CONTRIBUTING.md`, `AGENTS.md:207-213`). Upstreaming our ideas is not a realistic plan.

---

## 14. Top files to read

| Path | Why |
|---|---|
| `docs/internals/overview.md` | The architecture constraints in 100 lines: ownership, durable intent, checkpoints, settlement. |
| `docs/orchestration-v2/README.md` | V2 model + 10 invariants (app ids, child turns don't close parents, capability policy). |
| `docs/orchestration-v2/orchestrator-mcp-server.md` | 634 lines; the actual cross-agent protocol: delegation, wait semantics, `next`, pending questions, policy ceiling. |
| `docs/orchestration-v2/thread-lineage-and-context-transfer.md` | Fork/handoff/merge-back/subagent as one primitive; lazy resolution; Codex native-fork boundaries. |
| `packages/contracts/src/orchestrationV2.ts` | The domain: AppThread (475), Run (793), ExecutionNode (877), Subagent (1003), ProviderSession (1076), Checkpoint (1592), TurnItem (1849), ContextTransfer (248), command/event unions (3500+). |
| `packages/provider-core/src/server/ProviderAdapter.ts` | The adapter contract and the 14-event normalization union. |
| `apps/server/src/orchestration-v2/Orchestrator.ts` | 17k lines of pure command→event decisions; read `RunExecutionService`/`ThreadManagementService` first, then grep commands. |
| `apps/server/src/orchestration-v2/EventSink.ts` | How events + projections + receipts + outbox land in one transaction. |
| `apps/server/src/orchestration-v2/EffectWorker.ts` | Outbox claim/settle loop, cancellation re-checks, drainable worker. |
| `apps/server/src/orchestration-v2/ThreadManagementService.ts` | The shared boundary behind both WS and MCP: send modes, wait polling, interrupt selection. |
| `apps/server/src/mcp/toolkits/thread/tools.ts` | The agent-facing thread API, incl. `t3_thread_transfer` and `t3_thread_search`. |
| `apps/server/src/mcp/toolkits/orchestrator/tools.ts` | `delegate_task`, `task_status`, `task_cancel`, `create_threads`, scheduled tasks. |
| `packages/provider-core/src/server/orchestrationInstructions.ts` | The exact text that makes agents collaborate instead of polling. |
| `apps/server/src/orchestration-v2/Adapters/CodexAdapterV2.ts` | Reference adapter: capabilities, native fork/rollback, `/goal` multi-turn runs. |
| `packages/provider-acp/src/server/adapter.ts` | 10k-line shared ACP adapter — how 4 drivers share one protocol. |
| `apps/server/src/orchestration-v2/{RunExecutionService,ThreadSettlementService,ProviderSessionManager}.ts` | Run lifecycle, waiting/settlement, session reuse rules. |
| `apps/server/src/persistence/Migrations/OrchestrationV2/*.ts` | Real schema (Foundation, Subagents, RecoveryIndexes…) without reading 137 files. |
| `apps/server/src/ws.ts` + `packages/contracts/src/rpc.ts` | The transport contract and 247 methods. |
| `packages/client-runtime/src/connection/supervisor.ts` | Reconnect/probe policy — the part every remote agent UI gets wrong. |
| `docs/internals/providers.md` | 200 lines of provider-specific traps that took them multiple adapters to learn. |

---

## 15. Open questions

- **T3 Connect / relay**: `infra/relay` is in-repo, but running it needs Cloudflare account config,
  Clerk keys, Durable Objects and tunnel provisioning (`docs/operations/connect-setup.md`). Whether
  the *hosted* `app.t3.codes` flow depends on anything not in this tree is not answerable from code.
- **Clerk-based cloud identity**: web/desktop/mobile import `@clerk/*` and the desktop integrates
  `@clerk/electron`; the hosted deployment's Clerk instance, JWT templates and allowlists are not in
  the repo.
- **App Store / Play distribution**: EAS config, signing, and the shipped native binaries are not
  reproducible here (no `eas.json` secrets, no store metadata pipeline beyond screenshot scripts).
- **Product analytics / telemetry sinks**: `apps/server/src/telemetry` + `docs/user/telemetry.md`
  name the events but not the destination credentials; whether anything is sent in a self-hosted
  build depends on settings not visible in code.
- **Guest links** (`guestShare`, `GuestShareDecider.ts`): the sharing service's public origin and any
  hosted validation are unclear from source.
- **Model catalogs**: the per-model metadata (context windows, pricing, traits) is partly in
  `provider/model-manifest.json` and partly fetched live from each provider; there is no single
  offline table we could vendor.
- **Which providers actually pass their replay suites today** — the fixtures and testkits are in
  `orchestration-v2/testkit/fixtures`, but CI results are not in the repo, and several adapters
  (Grok, fx, Kiro, Muse, OpenCode 2.x) look newer/less covered than Codex/Claude.
- **`hook` ExecutionNode kind**: declared in contracts and mentioned in the V2 design doc, no
  producer in non-test code. Either an unimplemented plan or dead surface — worth asking upstream if
  we intend to model provider hooks.
- **Real-world latency/cost of the MCP-mediated coordination** (e.g. `delegate_task` + `next` wake
  chains): the docs describe the contract precisely but publish no measurements; we would have to
  measure it ourselves.
