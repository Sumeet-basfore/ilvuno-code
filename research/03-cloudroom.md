# Cloudroom — source research report

Repo: `github.com/davidondrej/cloudroom` @ clone `2025-11-22`, Apache-2.0 (BB's MIT notice in `NOTICE`).
Cloned at `/tmp/ilvuno-research/cloudroom`. Read-only pass; nothing built, nothing modified.

---

## 0. Executive summary

1. Cloudroom is a **desktop app + Rust "core" service** that runs *other vendors' agent CLIs* (`claude`, `codex`, `pi`, `cursor-agent`, `opencode`, `droid`, `kimi`, `agy`, `openclaw`, `a0`, …) as subprocesses, one per "session", and streams their native output into one timeline UI.
2. Its bet is **not** to reimplement the agent loop: "The harness keeps its agent loop, tools, and login. Cloudroom core keeps the sandbox, the process, the queue, the recorded history, and recovery" (`README.md:28`).
3. Two *independent* runtimes implement the same provider semantics: TypeScript `gui/packages/agent-runtime` for **Local** threads (inside a host daemon on your Mac), and Rust `core/src/runtime/*` for **Cloud** threads (inside a Linux sandbox). Keeping them in step is an explicit, tested cost (`gui/packages/provider-parity`).
4. Provider = plugin. Three first-party CLI drivers (`claude-code`, `codex`, `pi`) plus one generic **ACP** dialect bridge that declares 14 agents (`gui/plugins/provider-acp/src/known-agents.ts:38-442`). Cloud side knows 13 harness kinds (`core/src/runtime/mod.rs:47-66`).
5. Every Cloud session is its own sandbox with its own git branch; **Teleport** freezes a live session, moves its native history, and resumes it on the other machine.
6. **Cross-agent communication exists and is unusually complete**: parent→child spawn (`room-cli thread spawn`, own sandbox + code copy), `tell`/`stop`/`archive`/`output`/`log`, cross-sandbox relay through the vendor's website, `--shared-sandbox` children in the same folder, sibling broadcast, `thread fork`, plus a Tasks tracker whose comments resume the worker thread.
7. What does **not** exist: no shared memory or context store between threads, no transcript injection by @-mention (a thread mention is a text label), no `next` pointer, no Cloud→Local parent messaging, and children never see a parent's later edits (they must go through Git).
8. Persistence is split: the app's SQLite (40 tables, Drizzle) is the source of truth for *Local*; the Cloud core uses a **JSONL journal + snapshot + gzip segments**, no SQL, and syncs to object storage over 5 HTTP calls.
9. `web/` — the website that owns accounts, sandboxes, relays, and org sharing — is **not in this repo**. Many referenced modules (`web/lib/mobile.ts`, `web/lib/org.ts`, `web/lib/sandbox-logins.ts`) are absent. The open repo is the client + core, not the control plane.
10. Engineering culture is visible in the code comments: ADR numbers, issue numbers, and stated trade-offs everywhere. But the ADR corpus itself is mostly closed (`core/adr/` holds exactly one file).

---

## 1. What it is — one paragraph + product surfaces

Cloudroom is a "run any coding agent harness, anywhere, from one app" product. A **project** points at a repo; a **thread** is one conversation with one agent on one **provider**; an **environment** is the folder on a machine where that thread runs. The same thread record can be Local (agent process on your laptop, host daemon) or Cloud (agent process in a Linux sandbox on a VPS, with the Rust core sitting next to it), and can be **teleported** between them mid-life. The app is an Electron desktop shell around a React SPA, backed by a local Node server (SQLite) that fans work out to host daemons; Cloud threads bypass that entirely and the app talks straight to the sandbox's HTTP API. Around this core it ships a plugin ecosystem (40 plugin dirs in `gui/plugins/`) that supplies providers, a task tracker, scheduled loops, workflows, snippets, browser and computer use, org sharing and a team board.

Surfaces, evidenced:

| Surface | Where | Evidence |
|---|---|---|
| **Desktop app** (Electron, macOS arm64 / Linux x64 / Win11) | `gui/apps/desktop` | `gui/apps/desktop/package.json` (`run-electron-builder`, `--mac --arm64`) |
| **Web app / PWA** (served by the same local server; also reachable remotely through a tunnel) | `gui/apps/app` | `gui/apps/app/package.json` (`generate:pwa-icons`), `gui/plugins/connect/PLUGIN_OVERVIEW.md` |
| **Local** execution | host daemon on the user's machine | `gui/apps/host-daemon`, `gui/docs/system-overview.md` |
| **Cloud** execution | Rust core + agent in a sandbox | `core/src/lib.rs:9-33`, `core/docs/architecture.png` |
| **CLI** (`room-cli`, also `title`) for humans *and* agents | `gui/apps/cli` (`bin: room-cli`), plus a Rust `room-cli` inside the sandbox | `gui/apps/cli/package.json`; `core/src/mac/room-cli/SKILL.md` |
| **Phone** | `cloudroom.dev/m`, PWA added to home screen | `core/src/mac/room-cli/SKILL.md` ("Phone: the user opens cloudroom.dev/m…") |
| **Self-hosted server** | Ubuntu 24.04 VM, `install.sh`, HTTP API on `127.0.0.1:9840` | `core/docs/setup.md:1-20`, `core/docs/api.md:3` |
| **Tunnel/relay** (browser or phone → your Mac, no inbound ports) | Cloudflare Worker `cloudroom-connect` at `cloudroom.run` | `gui/apps/connect/wrangler.jsonc`, `gui/apps/connect/src/tunnel-do.ts` |

---

## 2. Stack & repo shape

**Languages.** Rust 49k LOC (`core/src`, 73 files) + TypeScript/TSX ~434k LOC (4,869 files, 1,615 test files) in `gui/`. Repo on disk ≈ 85 MB excluding `.git`.

**Split.** Three top-level dirs and nothing else:

| Path | Purpose |
|---|---|
| `core/` | The Rust service. One crate, one binary (`cloudroom`). Owns sandboxes, process supervision, durable queue, journal/replay, SSE, auth, disk guard, terminals, previews, computer use, GPU rental. `core/src/lib.rs:1-33` |
| `gui/` | pnpm + Turborepo monorepo: the Electron desktop app, the React SPA, the local Node server, the host daemon, the CLI, the CF tunnel worker, and ~35 workspace packages. `gui/pnpm-workspace.yaml`, `gui/turbo.json` |
| `skills/` | Nine built-in `SKILL.md` dirs given to every agent, plus `manifest.json`. Deliberately tiny and separate from plugin skills. |

**Packages (selected, `gui/packages/`):**

| Path | Purpose |
|---|---|
| `agent-runtime/` | Local provider runtime: spawns a bridge process per provider, speaks JSON-RPC, owns turn lifecycle (2.8k LOC `runtime.ts`) |
| `provider-bridge-protocol/` | The wire vocabulary: `thread/delta` kinds, delta assembler, conformance scenarios, parity harness (15.5k LOC) |
| `provider-bridge-acp/` | ACP dialect bridge: one implementation for Cursor/opencode/omp/Grok/Hermes/OpenClaw/Devin/Droid/Kilo/Cline/Kimi/Antigravity/Agent-Zero |
| `provider-parity/` | Replays recorded provider cells across two bridge builds and diffs the assembled timelines |
| `domain/` | Every zod schema: thread, environment, provider, events, plugins, keybindings (~12k LOC) |
| `db/` | Drizzle schema + queries on SQLite (`better-sqlite3`), 40 tables |
| `server-contract/`, `host-daemon-contract/` | The two contract boundaries. `gui/docs/system-overview.md` states implementation packages never import across them |
| `hono-typed-routes/`, `sdk/` | Typed route descriptors + the client SDK (`sdk.threads.send`, realtime client, node/browser transports) |
| `plugin-sdk/` | The plugin surface (`BbPluginApi`, 2.2k+ LOC of backend contract) |
| `thread-view/` | Pure event→timeline projection (58 files), shared by app, CLI and tests |
| `tunnel-contract/`, `tunnel-client/` | Binary frame protocol (8 frame types) + Mac-side client |
| `environment-provider-host/`, `host-workspace/`, `host-watcher/` | Workspace provisioning, file watching |
| `templates/` | Prompt templates as versioned markdown (system messages, guide pages, helper prompts) |

**Tooling.** `cargo` (Rust 1.89+), `pnpm@10.34.9` + Turborepo, Node 22.19+, Vite 8, `bun check` for typecheck, Vitest, oxlint, Electron builder. The app targets PWA too (`gui/apps/app/package.json`).

**Notable deps** (both a strength and a weight): `better-sqlite3`, `hono`, `drizzle-orm`, `better-auth`, `ws`, `zod@4`, `@earendil-works/pi-ai`, React 19 + `react-router@7` + `jotai` + `@tanstack/react-query`, TipTap, `mermaid`, `ghostty-web` (terminal), `@pierre/diffs`, `sharp`, Radix.

**License.** Apache-2.0 (`LICENSE`), with BB's MIT notice preserved in `NOTICE` — `gui/` is a fork of [BB](https://github.com/get-bb/bb) (`README.md:38`).

---

## 3. Core domain model

Two models coexist: the **app model** (SQLite, shared by Local and Cloud threads) and the **core model** (one Rust `Session` per process). The app model is the richer one.

### App model (`gui/packages/db/src/schema.ts`, 40 tables)

- **Project** (`projects`) — a repo, plus `projectSources` (local path per host, or git). `gui/docs/system-overview.md`.
- **Environment** (`environments`) — binds a workspace dir to a host: `path`, `isWorktree`, `branchName`, `baseBranch`, `status`, `lifecycle.phase` (`active|parked|retiring|teardown|destroyed`), `workspaceProvisionType` (`unmanaged|managed-worktree|personal`). `gui/packages/domain/src/environment.ts`.
- **Thread** (`threads`) — the unit of work. Fields that matter for us: `executionTarget` (`local|cloud`), `environmentId`, `providerId`, `modelOverride`, `reasoningLevelOverride`, `parentThreadId`, `sourceThreadId`, `originKind`, `originPluginId`, `visibility` (`visible|hidden`), `purpose` (`{text, goal, percent, sourceSeq}`), `status`. `schema.ts:553-647`, `domain/thread.ts:433+`.
- **Provider** — *not* a table: registered at runtime by plugins. `providerModelCatalogs`, `provider-parity`, `install`/`usage` health are cached per host/provider.
- **Message / turn** — not a table either. Everything is an **event**: `events` (`threadId`, `sequence`, `type`, `itemId`, `itemKind`, `parentToolCallId`, `data`, `scopeKind=turn|thread`). `schema.ts:787-869`. Append-only, unique `(threadId, sequence)`.
- **Queued message** (`queuedThreadMessages`) — a first-class row: `initiator` (`user|agent|system`), `senderThreadId`, `waitingOn`, `failureReason`, `sendAt`, `hardQueue`, `pausedByStop`, `editable`. `domain/thread.ts:337-400` is the richest documentation of this in the repo.
- **Cloud binding** (`cloudroom_threads`) — per-thread mirror of the sandbox side: `coreUrl`, `startRequestId`, `sessionId`, `cursor`, `nativeId`, `turnId`, `queuePaused`, `lastWorkspace`, `lastGitStatus`. `schema.ts:649-667`.
- **Cloud command outbox** (`cloudroom_commands`) — durable commands (`prompt|stop|resume|sleep|edit|cancel|reorder|steer|compact|goal|rewind|attach|title|teleport|side_chat|model|unarchive_session`). `schema.ts:668-677`.
- **Pending interaction** (`pendingInteractions`) — approvals and agent questions, with lifecycle + resolution. `domain/pending-interactions.ts` (791 lines).
- **Thread tabs / sections / search segments / conversation outlines / dynamic-context file states / custom instructions hash** — UI + context bookkeeping. `schema.ts:692-786`.
- **No artifact or canvas entity.** Visuals are files + directives (see §10).

### Core model (`core/src/session/mod.rs:117+`)

```rust
pub struct Session {
  session_id, harness: runtime::Kind, capabilities, native_cursor,
  provider, native_id, native_path, native_offset, state: SessionState,
  current_request, current_turn, last_sequence,
  receipts: BTreeMap<String, Receipt>,   // request_id → accepted|delivered|completed|…
  queue: Vec<String>,                    // ordered prompts accepted while busy
  prompts: BTreeMap<String, PromptState>,
  storage_paused, queue_paused, compacting, checkpoint, rewind_request,
  parent_session, archived, pinned,
  reported, child_states, started, child_starts, late_reports,
  workspace, reasoning, title, last_app_prompt, thread_reports, continuing, continued, …
}
```

`SessionState` is a 17-variant machine (`core/src/session/state.rs:13-33`): `pending, waiting_for_files(legacy), starting, resuming, starting_turn, running, interrupting, closing, idle, sleeping, suspended, rewinding, closed, failed, process_lost, saved_history_only, Other`. Serialized as the same strings as v0.1.10 so old journals replay (`state.rs:20-22`). Legacy strings are kept readable rather than migrated.

**Relations, plainly:** Project 1—N Environment 1—N Thread; Thread N—1 Provider (by id); Thread 1—N Event; Thread 1—N QueuedMessage; Thread 1—N child Thread (`parentThreadId`); Cloud Thread 1—1 `cloudroom_threads` row 1—1 sandbox `Session`. A `Session` is not a thread: one thread can have many sessions over its life (harness switch, message edit, fork — `continuing`/`continued` in the struct).

---

## 4. Agent / provider integration

### 4.1 The shape: provider = plugin, bridge = process, delta = vocabulary

`gui/docs/provider-plugin-api.md` states the design goal explicitly:

> "everything a provider touches is owned by its plugin … with the smallest possible provider-agnostic core", and four principles including **"Zero first-party privilege. First-party providers use only the public API."**

Pipeline (`gui/docs/provider-plugin-api.md`, "Layers"):

```
host agent → bridge (plugin) → thread/delta (core vocabulary) → delta assembler (core)
           → ThreadEvent (core) → persistence → timeline projection → renderers
```

Local side, concretely: the server resolves a `ProviderRegistration` (`gui/apps/server/src/services/providers/provider-registry.ts:22+`) containing `info`, `serverCapabilities`, `bridgeOptions`, `extensionKinds`, `visibility`, `nativeSkillRoots`, `nativeCommandRoots`, `envPassthrough`, and a `deriveProviderOptions` function. `agent-runtime` then spawns the plugin's **bridge artifact** with a `bridgeLaunch` (`{pluginId, dataDir, source:{kind:"artifact", digest, artifactPath}, capabilities, providerOptions, envPassthrough}` — `gui/packages/agent-runtime/src/types.ts`).

The bridge speaks JSON-RPC over stdio with a typed command union (`gui/packages/agent-runtime/src/provider-adapter.ts:37-143`):

```
skills/configure · model/list · provider/health · provider/usage
provider/installation/status · provider/installation/run
thread/start · thread/resume · thread/fork · turn/start · turn/steer
thread/stop · thread/discard · thread/goal/{set,clear} · thread/name/set
thread/archive · thread/unarchive
```

Cloud side does **not** use plugin bridges: `core/src/runtime/*.rs` speaks each CLI's native protocol directly — `claude.rs` (stream-json, 75k), `codex.rs` (app-server, 36k), `pi.rs` (RPC mode, 33k), `cursor_print.rs` (a hand-rolled ACP subset, 33k), `opencode.rs`, `openclaw.rs`, `droid.rs`, `kimi.rs`, `antigravity.rs`, `agent_zero.rs`, `fx.rs`, `hermes.rs`, `devin.rs`.

### 4.2 Streaming event protocol

Delta kinds, from `gui/packages/provider-bridge-protocol/src/thread-delta.ts` (27 `kind:` literals): `input.accepted`, `input.provider`, `turn.open`, `turn.boundary`, `item.open`, `item.close`, `item.progress`, `item.textDelta`, `item.textClose`, `item.outputDelta`, `command.outputSnapshot`, `usage`, `contextWindow`, `context.compacted`, `context.cleared`, `turn.diff`, `thread.started`, `thread.identity`, `thread.name`, `extension.state`, `provider.rateLimits`, `provider.error`, `provider.modelFallback`, `provider.warning`, `unhandled`, `session.ended`, `session.reset`.

Persisted `ThreadEvent` types = provider events + 13 system events (`domain/thread-events.ts:24-38`: `client/thread/start`, `client/turn/requested`, `client/turn/rejected`, `client/turn/start`, `system/error`, `system/manager/user_message`, `system/thread/interrupted`, `system/operation`, `system/interaction/lifecycle`, `system/permissionGrant/lifecycle`, `system/userQuestion/lifecycle`, `system/thread-provisioning`, `system/provider-turn-watchdog`). Item kinds include `command`, `fileChange`, `tool`, `compaction`, `agentMessage`, `reasoning`, `plan`, `webSearch`, `webFetch`, `imageView`, `imageGeneration`, `backgroundTask`, `fileRead`, `search`, `delegation`, `planSteps`, `extension` (`thread-delta.ts:58-160`).

Core's own event kinds are coarser and *explicitly not portable*: `core/docs/api.md` says "Output records are not yet harness-neutral. Kinds are shared, but `data` differs by harness … Check `session.harness` before parsing `data`." Kinds: `text_delta, thinking_delta, item_started, item_completed, tool_delta, tool_snapshot, native_event, native_record` + lifecycle records (`receipt, state, harness, workspace, native_identity, launch_reasoning, checkpoint, usage, usage_limited, child, child_result, child_thread, rewind*, teleport, secret_request, prompt_warning, model_fallback`) + disk-safety records.

### 4.3 Model catalogs and per-model options

`availableModelSchema` (`domain/provider-types.ts:17-30`): `{id, model, displayName, routeProviderId?, description, supportedReasoningEfforts[], defaultReasoningEffort, isDefault, serviceTiers?}`. Reasoning is validated **per model**, and `serviceTiers` exist only where known. Core does the same over HTTP: `GET /v1/capabilities` exposes `harnesses[].models[].reasoning_levels` and `service_tiers` (`fast`, `ultrafast`), and "a null catalog means discovery failed, not unsupported models" (`core/docs/harnesses.md`, API and compatibility) — the null-vs-empty distinction is enforced with a 60-second cache.

Reasoning levels are a shared enum: `none, low, medium, high, xhigh, ultracode, max, ultra` (`domain/shared-types.ts:4-12`). Providers map them (`low→minimal`, `max→xhigh` for OpenCode; `none→off` for Pi/OpenClaw) and **fail rather than silently downgrade**.

### 4.4 Session resume

Three different strategies, all adapter-owned:
- **Claude**: exact-identity resume + conversation forks; rewind forks history without restoring files.
- **Pi**: exact identity + workspace + model/provider + supported version; "never creates a missing file or silently migrates/replaces a conversation" (`core/docs/harnesses.md`, Queue/completion/recovery).
- **ACP agents**: `session/load` or `session/resume` by native id (OpenCode, Droid, Kimi, Agent Zero, OpenClaw via `sessions.create` + `session/resume` by key).
- **Cursor**: one `cursor-agent -p --output-format stream-json --resume CHAT` per prompt; chats live in `.cursor/chats/<md5>/<id>`, and a past bug copied the whole SQLite store each tool call and "filled a 50 GB disk in days (#606)".

Recovery contract: `GET /v1/sessions/{id}/recovery` → `ready|empty|missing|unavailable`; `startup_error` ∈ `startup_timeout|empty_session|missing_history|resume_failed`; failed resumes become `process_lost` with **no restart loop and no fresh-conversation fallback** (`core/docs/session-lifecycle.md`).

### 4.5 Permission / runtime modes

One shared enum: `accept-edits | auto | full` (`domain/shared-types.ts:20`), plus `clampPermissionModeToCeiling` (`:32`) so a provider ceiling (e.g. OpenCode only allows `full`) wins. Providers declare `permissionModes` in `providerCapabilitiesSchema` (`provider-types.ts:36-46`) alongside `supportsThreadArchive`, `supportsThreadRename`, `supportsServiceTier`, `supportsNativeUserQuestion`, `supportsFork`, `supportsSessionRewind`, `supportsManualCompaction`, `modelCatalogScope`.

Cloud is blunter: agents run without approval prompts at all (`README.md` Security), and several harnesses start in `auto-high`/`auto` full-access by ADR 0045 (`harnesses.md`, Droid/Kimi sections). A regex `Command Guard` prevents root/home deletion, disk wipes, hosted-repo deletion and fork bombs — and it is honestly labelled: "This is regex-based accident prevention, not a sandbox" (`harnesses.md`, Command Guard; shared source `core/src/runtime/command-guard.mjs`).

### 4.6 Auth / subscription handling — what is actually wrapped

Accounts are handled *on the execution machine*, never in the GUI: "Tokens never pass through the GUI or core API" (`harnesses.md`, Native subscription login). Per provider:

| Provider | Wrapped subscription | Login mechanism |
|---|---|---|
| Claude Code | Claude plans | native browser sign-in, one-time code forwarded to the CLI; `POST /v1/accounts/claude/{login,cancel,complete}` |
| Codex | ChatGPT plans **or** OpenAI API key | device-code flow; `…/codex/import` accepts a 64 KiB file-backed `auth.json`, mode 0600, only when absent |
| Pi | whatever Pi is configured with | Mac sync helper copies `~/.pi/agent/auth.json` providers the VM lacks; `!command` and `$VAR` resolved on the Mac |
| Cursor | Cursor subscription | browser login on the VM with `NO_OPEN_BROWSER=1`; API keys via `--list-models` verification |
| Kimi | Moonshot | copies `config.toml` + every regional `credentials/*.json`; **refresh-token rotation can sign the Mac out** (ADR 20261107-081737) |
| Antigravity | Google | link+code or Gemini API key, encrypted per user |
| OpenClaw | — | copies sign-ins/API keys; **refuses Claude plans** (ADR 0265) |
| Droid | Factory | `FACTORY_API_KEY` only; "a browser login can't run in a sandbox" |
| Agent Zero | user's own server | `AGENT_ZERO_HOST`, else `~/.agent-zero/.env` |

There is a real **usage-limit recovery** path: a turn that dies on a subscription limit is recorded `usage_limited`; after a successful account switch the core restarts each quota-stopped session and sends `keep working` (`harnesses.md`, "Continue after a usage limit").

### 4.7 Provider support table

Local (`gui/plugins/provider-acp/src/known-agents.ts`) and Cloud (`core/src/runtime/mod.rs:47-66`) ids:

| Agent | Local provider id | Cloud harness id | Cloud adapter | Notes |
|---|---|---|---|---|
| Claude Code | `claude-code` | `claude-code` | `claude.rs` | richest adapter: skills, subagent suppression, fast mode off |
| Codex | `codex` | `codex` | `codex.rs` | `model/list` catalog, `ultrafast`, goals |
| Pi | `pi` | `pi` | `pi.rs` | `--mode rpc`, bundled context extension, provider selection |
| Cursor | `acp-cursor` | `cursor` | `cursor_print.rs` | print mode, `--disable-project-configs`, no guard |
| opencode | `acp-opencode` | `opencode` | `opencode.rs` | `full` only; `OPENCODE_CONFIG_CONTENT` disables question tool |
| Antigravity (agy) | `acp-antigravity` | `antigravity` | `antigravity.rs` | always visible in composer; plan restrictions documented upstream |
| Grok Build | `acp-grok` | — | — | local only |
| omp | `acp-omp` | — | — | local only |
| Hermes Agent | `acp-hermes-agent` | `hermes` | `hermes.rs` | — |
| OpenClaw | `acp-openclaw` | `openclaw` | `openclaw.rs` | shared Gateway, ~15 s cold start, 1.3 GB |
| Devin | `acp-devin` | `devin` | `devin.rs` | — |
| Droid | `acp-droid` | `droid` | `droid.rs` | — |
| Kimi Code | `acp-kimi` | `kimi` | `kimi.rs` | — |
| Kilo / Cline | `acp-kilo`, `acp-cline` | — | — | local only |
| Agent Zero | `acp-agent-zero` | `agent-zero` | `agent_zero.rs` | steer disabled; `stop_frees` |
| fx | — | `fx` | `fx.rs` | Cloud only |
| custom ACP | `Custom agents` JSON (`{id, displayName, command}`) | — | — | user-supplied ACP server on stdio |

Count: **14 ACP-declared local providers + 3 first-party CLI drivers** on Local; **13 harness kinds** in Cloud.

---

## 5. Concurrency & orchestration

**Parallel threads.** Trivially parallel: each Cloud thread = its own sandbox + branch; each Local thread = its own provider process and usually its own worktree (`gui/docs/worktrees.md`: managed worktrees at `<DATA_DIR>/plugins/environment-git-worktree/host-data/worktrees/<thread-id>/<repo>`). Startup concurrency is capped: "At most half the available logical CPUs (minimum one) initialize concurrently; this does not cap running sessions" (`session-lifecycle.md`).

**Subagents / delegates — the parent/child contract.** This is the heart of the design.

- A child is a *real thread in its own sandbox* by default: `room-cli thread spawn --provider … --prompt TEXT` copies the parent's code ("commits and uncommitted changes, without gitignored files or dependencies; nested Git repos arrive empty or without history") to its own branch (`core/src/mac/room-cli/SKILL.md`).
- `--shared-sandbox` runs it in the *same* sandbox and folder instead. The doc adds a blunt warning: shared children share the sandbox's memory, so "give a big fan-out its own sandboxes".
- The API is the same route the agent uses: `POST /v1/sessions` with `parent_session` + `prompt` → session `cr_child_{request_id}`, same folder, inherited guard and system prompt (`core/docs/api.md`, Sessions). Capability: `child_threads`.
- Cross-sandbox children (`room-cli thread spawn` from a Cloud thread without `--shared-sandbox`) go through the **website**: the core records a `child_start`, a worker retries `POST {api}/child` until the website says `started`; the spawned child gets `parent_thread`, and each finished turn is pushed to the website's `relay` as a `report` until taken (410 drops it for good) — `core/docs/api.md`, Sessions; `core/src/session/relay.rs:1-6`.
- **Parent notice loop:** "Each time a child turn ends, Core queues the parent a `notice_…` prompt with the child's reply, even while no app is connected." The parent records a `child_thread` with `{id, harness, title}`. In the app, `gui/apps/server/src/services/threads/child-thread-notifications.ts` batches these into a system message with `childReport`, interruption reason, undelivered-message warnings, and signed-out holds.
- **Detach:** `POST /v1/sessions/{id}/detach` makes a child top-level so the parent stops hearing from it (`session-lifecycle.md`).

**The wait/cancel contract.** Explicit and honest about its limits:
- `room-cli thread wait --children` blocks until no child is working, up to 100 s, **exit 2 means run it again** (`core/src/mac/room-cli/SKILL.md`).
- "Cloud threads have no `thread queue`. … They can't confirm whether your message is waiting, received, or lost. Unchanged output alone is no reason to resend." Same file. That is a deliberate refusal to fake delivery guarantees.
- Stop/archive are downward-only and can be refused; `archive` cancels unsettled starts.
- Cloud core receipts outbox: `unknown` on shutdown, `unknown_after_restart` on crash — **uncertain work is never resent** (`core/docs/api.md`, Conventions).

**`next` pointer.** There is none. I grepped for it: `next` appears only in `core/src/gpu/mod.rs:128` (a GPU API reply) and test fixtures. The direction-of-travel is the opposite: the *parent* holds `child_states` (last known state per child) and the child reports upward. If your reference model expects "another thread's `next`", Cloudroom does not have it.

**Scheduling / loops.** `core/src/session/automation.rs` ("Automations are called Loops in the app", ADR 0212/0259). Core only proxies the command to the website, which "computes the next run, so Core never parses cron". Triggers: cron+timezone, `--at`/`--in`, or `--webhook` secret URL. Modes: `--same-thread` re-prompt, `--new-thread` fresh sandbox, `--script` first (60 s, no output = skip, 3 failures pauses), and `--agent A,B,C` rotates agents across runs skipping unavailable ones. `--parent` names a manager thread each run starts under. `room-cli wake 5m "TEXT"` is a one-shot timer. Loop runs report via `room-cli loop report --outcome done|acted|needs-you|failed --confidence …`.

**Worktrees.** Local-only and plugin-owned: built-in **Worktree** plugin, `.worktreeinclude`, setup script on first create, teardown script before removal, auto-park on archive and restore on unarchive (`gui/docs/worktrees.md`, `schema.ts` `environments.lifecycle.phase` = `parked`).

**Background work.** `domain/background-task.ts`: task types `local_workflow | local_bash | local_agent | local_subagent`, statuses `pending|running|paused|completed|failed|killed|stopped`. `threadActivityState` counts `activeWorkflowCount`, `activeBackgroundAgentCount`, `activeBackgroundCommandCount`, `activePlanModeCount`, `activeGoalCount` and gates sleep/archive.

---

## 6. State & persistence

**Local (app).** SQLite via `better-sqlite3` + Drizzle, 40 tables, WAL-ish single-writer; migrations in `gui/packages/db/drizzle`. Server described as "Stateless itself; the DB is the source of truth" (`gui/docs/system-overview.md`). Notifications: a `DbNotifier` pushes change events onto the WS hub (`gui/packages/db/src/notifier.ts`). Event rows are append-only with a per-thread `sequence` and a partial-index for every hot read path (12 indexes on `events` alone, `schema.ts:799-868`) — that index list is effectively a list of the product's query shapes.

**Cloud (core).** No database. A **JSONL journal per session**, fsynced before subscribers are notified:
- `snapshot.json` = replayed state at a journal position, written after startup, on drain/shutdown, and **every 20,000 records**; startup restores it and replays only newer records. A missing/older snapshot means full replay. `state.rs:5,105-118`; `session-lifecycle.md`.
- Old records fold into gzip segments `<first>-<last>.jsonl.gz` (≤1,024 records, ≤8 MiB) because "one file per record took ~4.6 KiB of disk each (#280)". Compaction is off (`COMPACT_HISTORY`) until a reader ships, because older cores can't open a compacted journal.
- Records >1 MB come back without a body.
- **Write-failure model is unusually careful:** a failed rename before rename or ENOSPC/EDQUOT changes nothing and only that write fails; *any other* failed rename or a failed directory fsync "has an uncertain outcome", flips readiness false, and blocks new mutations until restart. Never reset the guard blindly.
- History has a 32 MiB `reserve` file (twice the largest harness record) that a space-refused write deletes and retries into.

**What is source of truth, and where.** Local: SQLite. Cloud: the sandbox journal, mirrored to object storage by the website (`CLOUDROOM_API_URL`, five calls, stand-in `core/tests/fake_api.py`). Self-hosted: `CLOUDROOM_LOCAL_HISTORY=1` and the journal *is* the history — "No database is needed" (`core/docs/setup.md:96-100`). The core "refuses to start without one of the two settings, so a managed machine never silently keeps history only on its disk". That's the key architectural decision: **history is never only on an ephemeral box**.

**Resume semantics.** `sleeping` after 30 min idle (history on disk, process gone); `suspended` on graceful shutdown; restart relaunches only sessions with queued work or activity in the last 30 min "so a reboot does not refill memory". A prompt, teleport update, or pause-resume wakes the same native conversation, then delivers the work.

**Disk safety as a first-class subsystem** (`core/docs/storage.md`, 72 dense lines): 5 GB warn, 2 GB pause new work and freeze managed agents, 2.5 GB resume, kill the single process group that wrote the most (≥32 MB) if still paused after 30 s, and an absolute floor of 250 MB on a 20 GB box so history can always be written. Writes counted from `/proc/<pid>/io` over process lifetime, read with the agent's own file identity via `CAP_SETUID/CAP_SETGID`. It ends with "This is polling-based protection, not a kernel guarantee."

---

## 7. Transport / IPC

Four distinct protocols, each with a contract package:

| Hop | Protocol | Where |
|---|---|---|
| App ↔ local server | HTTP (Hono, typed routes) + **WebSocket** for change notifications and terminals | `gui/apps/server/src/server.ts:799` (`/ws`), `:812` (`/ws/terminals/:id`), `gui/packages/hono-typed-routes` |
| Server ↔ host daemon | WebSocket RPC, daemon dials out; "Routes work to hosts over the active daemon WebSocket" | `gui/docs/system-overview.md`, `gui/apps/server/src/ws/daemon-protocol.ts`, `gui/packages/host-daemon-contract` |
| App ↔ Cloud core | Plain HTTP + **SSE** (`/v1/sessions/{id}/stream?after=N`, event `id` = sequence so `Last-Event-ID` resumes exactly) | `core/docs/api.md:3`, History and live events |
| Browser/phone ↔ your Mac | Binary tunnel frames over one WebSocket, relayed by a Cloudflare Durable Object | `gui/packages/tunnel-contract/src/index.ts`, `gui/apps/connect/src/tunnel-do.ts` |

**Detail worth stealing — the WS client protocol is minimal.** `gui/apps/server/src/ws/client-protocol.ts` handles exactly three messages: `subscribe`, `unsubscribe`, `ping`. Everything else is server→client `changed` events keyed by a `RealtimeSubscriptionTarget`. That is: **no request/response over the socket at all**; RPC is HTTP, push is WS. The SDK mirrors it (`gui/packages/sdk/src/realtime-client.ts`, with 1 s → 30 s exponential reconnect).

**Tunnel.** 8 frame types (`openHttp`, `bodyChunk`, `bodyEnd`, `respHead`, `openWs`, `wsOpenAck`, `wsData`, `closeStream`), 1 MiB max chunk, protocol v2 with a 4 s heartbeat (12 s staleness). The DO holds the Mac socket, holds new requests for `GAP_HOLD_MS = 8_000` after a drop instead of failing them, and pings the Mac every 2 s while its data is arriving so a big download can't starve the heartbeat. `gui/apps/connect/wrangler.jsonc` binds D1 (`cloudroom-connect`) and a `TunnelDO` class, routes `cloudroom.run` + `*.cloudroom.run`.

**Terminals** are their own framing on the same upgrade: kind byte + big-endian u32 length, kinds `0 output, 1 exit, 2 hello, 3 ping` from the core, `0 input, 1 resize, 3 ping` from the app; reconnect with `since` replays up to 1 MiB, and `410` means the shell died with the core (`core/docs/api.md`, Workspaces/terminals).

**API conventions worth copying.** Every command carries a `request_id` (1–64 chars); retrying returns the same receipt and never repeats work; reusing an ID for *different* content is `request_conflict`. `202` means saved, not finished. Receipts move `accepted → delivered → {completed|interrupted|failed|unknown|unknown_after_restart}`. Session IDs are derived: `cr_` + start request id. Errors are `{error, code}` with the instruction "Show `error` to people; branch on `code`" and an explicit split between permanent 409s and temporary ones (`storage_blocked`, `service_stopping`, `model_catalog_unavailable`, `*_auth_unavailable`).

---

## 8. Frontend architecture

**Framework.** React 19 + Vite 8, `react-router@7` (data router, `AppRoutes` in `gui/apps/app/src/App.tsx:288+`), Tailwind 4 + Radix + `class-variance-authority`, `react-compiler` via Babel plugin, Ladle for stories, oxlint.

**State.** Four layers, cleanly separated:
1. `@tanstack/react-query` for server state (`gui/apps/app/src/test/queryClientTestHarness.tsx`),
2. `jotai` + `jotai-family` for UI/atoms (`use-async-atom-value.ts`, `ui-preferences-sync.ts`),
3. the **SDK realtime client** for invalidation (`sdk.realtime` subscribing to `thread:changed`, `project:changed`, `environment:changed`, `host:changed`, `system:changed`),
4. `sessionStorage`-ish per-pane routing state (`SplitWorkspaceRoute`, `splitPaneCommands`).

**Routing.** A single catch-all `SplitWorkspaceRoute` owns the workspace, with explicit routes only for settings/plugins/skills/machines. Notably, ~20 legacy paths are kept as redirect components (`LegacyToolsPathRedirect`, `LegacyAutomationCollectionRedirect`, …) — the product renamed Automations→Loops and Tools→Plugins and paid for it in redirect shims rather than breaking links.

**Key screens / panels.**
- `views/thread-detail/ThreadDetailView.tsx` (3,355 lines) — header, timeline, composer, secondary panel, `SubagentBar` for children.
- `views/RootComposeView.tsx` + `RootComposeCompactHome` — the new-thread composer (project/environment/provider/model/reasoning/permission pickers, initial prompt).
- `views/SettingsView.tsx` — the plugin/skill/machine settings surface.
- Secondary panel: `components/secondary-panel/` (files, git diff, terminal, plugins, tasks, PRs).
- Split panes with per-pane thread context (`SplitThreadArea`, `PaneContext`, `PaneMaximizeButton`).

**Chat rendering.** Deliberately non-agentic: a pure projection package. `gui/packages/thread-view` (58 files) turns `ThreadEvent[]` into a `ThreadTimeline` of rows — `build-thread-timeline.ts` alone is 1,726 lines, with separate projections for assistant streaming, tool activity, background tasks, delegations, plan steps, compactions, goals, provisioning, web activity, and "noise" events. Nothing renders from raw events; the app, the CLI and tests all read the same projection. **This is the single most reusable idea in the frontend.**

Markdown is `react-markdown` + `remark-gfm/math/directive` + `rehype-sanitize`, and **plugins extend messages via leaf directives** resolved through `plugin-slot-resolvers.ts` (`::inline-vis{file=…}`, `::task{key=…}`), so a plugin can render inside a message without core knowing about it.

**Design system.** `gui/packages/core-ui` (tokens) + `shared-ui` + Radix primitives; themes are plugins (`theme-preview/skills/theme-authoring`, `appTheme` table, `domain/code-themes/*` with Nord/Dracula/Gruvbox/Catppuccin/Solarized light+dark variants); keybindings are a typed schema (`domain/app-keybindings.ts`, 328 lines) with user overrides (`uiPreferences`).

---

## 9. Extensibility

**A plugin is a package with a `bb` manifest.** `domain/plugin-manifest.ts` requires `{name, description, branding{icon|logo}, server}` and allows `app`, `host`, `skills[]`, `themes[]`; `engines.bb` / `engines.bbPluginSdk` gate compatibility. 40 plugin dirs ship in-tree; `gui/plugins/bb-official.json` is the marketplace listing with categories and screenshots.

**The plugin API is the biggest surface in the repo.** `gui/packages/plugin-sdk/src/backend-contract.ts` exposes `BbPluginApi` with: `settings`, `storage` (namespaced KV + per-plugin DB), `http` (routes under `/api/v1/plugins/<id>/http/*`), `rpc` (typed contracts via `defineRpcContract`), `realtime` (push to frontends), `background` (services + cron), `cli` (an agent-facing `room-cli` subcommand — `plugin-cli-proxy.ts`), `agents` (per-turn context contributions), `providers` (provider registration), `ui` (host-rendered contributions), `events`, `experimental_hooks`, `experimental_environments`, `experimental_machines`, `experimental_serverAccess`, `experimental_aiServices`, `experimental_telemetry`, plus `sdk` ("The full BB SDK, bound to this server over loopback"). Prefixing the unstable ones with `experimental_` and keeping `docs/api_to_audit.md` is a convention worth copying verbatim.

**Skills.** Format (see `skills/*/SKILL.md`): YAML frontmatter `name`, `description`, optional `disable-model-invocation: true` (manual-only, e.g. `decisions`, `file-tree`, `setup-help`, `adr-verbatim`), optional `triggers: [user, model]`, then markdown. `skills/manifest.json` lists 9 built-ins; plugins contribute 20 more (`find gui/plugins -name SKILL.md`), including one per provider (`claude-code-provider`, `codex-provider`, `pi-provider`, `acp-provider`). There is also a full `skill-creator` and `plugin-authoring` skill.
- Loading is **literal text injection, not execution**: core reads selected user/project `SKILL.md` bodies on the execution machine under the agent's permissions and prepends them, deduping repeats, warning on missing (`prompt_warning`) and failing on ambiguous/oversized (`core/docs/harnesses.md`, Selected skills). A skill whose tag *starts* the message and carries Claude Code settings is passed through so Claude runs it natively.
- Skills also come from each agent's own roots: `nativeSkillRoots` per provider in `known-agents.ts` (`~/.claude/skills`, `~/.codex/skills`, `.agents/skills`, `.cursor/skills`, `.gemini/config/skills`, ancestor-rooted `.agents/skills` for projects).
- A registry install path exists against `skills.sh` (`gui/apps/server/src/routes/skills-registry.ts`, `services/skills/registry-*.ts`).

**MCP.** Two directions. Inbound: MCP servers are configured and **synced** to Cloud (`mcp_sync` feature; `GET /v1/settings`, `POST /v1/sync`). Outbound: the ACP bridge exposes *its own* MCP server to the agent (`tool-proxy-mcp.ts`, server name `bb-bridge`) so plugin-declared `dynamicTool`s reach the agent as MCP tools over a local socket with a token — that is how `AskUserQuestion` and `bb_workflow_run` become callable.

**Hooks.** Codex/Claude/Pi hooks load normally; core injects **only** the command guard, and "never edits personal hooks or trusts unrelated Codex hooks" (`harnesses.md`, Command Guard). `experimental_hooks` on the plugin side is a *dispatch checkpoint* (let go / queue with a reason / refuse), not a prompt hook.

**Scripts.** Two kinds: loop `--script` (bash/sh/node/python3, 60 s, stdout/stderr/exit recorded, composable into the prompt) and environment `setup`/`teardown` scripts on worktree creation.

**Snippets.** Account-synced text with a CLI (`room-cli snippets …`, `--json`), Raycast JSON import/export, and org-scoped publishing (`gui/plugins/snippets`, `core/src/session/snippets.rs` — core proxies because "Core keeps the hosting credential").

**Publishing to an org.** `gui/plugins/shared-projects` (repo shares, invites, per-account project) and `gui/plugins/organization` (org-wide `instructions`, `repo_rules` per repo, `member_view: none|shared|all`, and a **team board** of teammates' thread cards with `read`/`fork`/`takeOver` actions plus a `handoff` payload with messages). Both contracts explicitly note the answers come from `web/lib/project-shares.ts` / `web/lib/org.ts` — i.e. the server side is closed.

---

## 10. Shared context / cross-agent communication (CRITICAL)

Everything the repo does **today** to let separate agent sessions see each other's work, ranked by how much it actually shares:

1. **Same folder, different processes (`--shared-sandbox`).** Children can run in the parent's sandbox and folder. Then "share work" is literally the filesystem. Still not shared memory or context: it shares a disk.
2. **Git as the channel (default).** A spawned child gets its own sandbox, its own branch, and a copy of commits + uncommitted changes. The SKILL.md is explicit about the consequence: *"It doesn't see your later edits, so share work through Git."* Uncommitted work is the practical limit, not a bug.
3. **`tell` — cross-thread messaging.** `room-cli tell CHILD_ID TEXT`, `tell --parent TEXT` (message the thread that started you), `tell ID TEXT` also reaches every thread under that parent, labelled `[message from thread:ID]`. `tell --mode queue` always queues (request id suffix `_queue`). Delivery semantics are stated honestly: a failed steer is queued instead; an unconfirmed delivery returns 503 so the caller can retry; a `tell` from another thread is *refused* when the target's disk is full ("its sender, parent or child, hears that the disk is full", `core/docs/storage.md`).
4. **`output` / `log` — reading another thread.** `room-cli thread output CHILD_ID` (latest output) and `log CHILD_ID` (whole chat), `thread list`, `thread show`. That is the *only* transcript-sharing primitive, and it is one-directional (parent → child) by construction.
5. **Turn reports (push, not pull).** Every finished child turn queues a `notice_…` prompt on the parent with the child's reply — "even while no app is connected". Batched in the app with interruption reasons, undelivered-message counts, and signed-out holds. `POST /v1/sessions/{id}/detach` opts a child out.
6. **Sibling / hierarchy broadcast.** A `tell` aimed at a parent also reaches all threads under it, so siblings can hear each other *through the parent*, labelled by sender. This is the closest thing to a shared bus in the product.
7. **`thread fork`.** Copy the conversation + code into a new sandbox at the last finished turn. Used with `--provider`/`--model` to hand a conversation to a *different agent*. Only the user can fork to the Mac; agents can only fork to Cloud.
8. **Side chat.** `gui/plugins/side-chat`: a fresh agent in the same folder, no copied conversation, quoted text as its only context, plus **"Send to main thread"** which queues a reply into the original thread. Deliberately context-free, and it auto-archives unused side chats after 24 h.
9. **Tasks as shared state.** `gui/plugins/tasks` is the strongest *human-visible* cross-agent surface: a task has description, subtasks, attachments, comments, and attached worker threads. `delegate` builds a seed prompt containing description, project, subtasks, attachment ids with a fetch command, recent comments, preset instructions and a **report-back contract**; the worker is a normal thread. `--notify` on a comment routes it to the thread that authored the latest agent reply and **resumes it if idle** (`gui/plugins/tasks/steer/index.ts`: `sdk.threads.send({mode:"steer-if-active"})`). Agents can also `tasks attach`/`detach` themselves.
10. **Workflows as a fan-out DSL.** `gui/plugins/workflows`: a JS file in a sandbox with "no file, shell, network, or clock access" that calls `agent(...)` to start real worker threads, pipelines stages, parallel barriers, JSON-Schema-constrained results, cached calls so a resumed run replays them. Worker threads are hidden from the sidebar.
11. **Artifacts = files + directives, not records.** `::inline-vis{file="charts/out.html" height="480"}` renders a workspace file (or a thread-storage file with `source="thread-storage"`) inline in the message, sandboxed in an iframe with an opaque origin, 5 MiB cap. `::task{key="PROD-1"}` renders a live task card. Both are *pointers to files/records*, deliberately not a binary artifact store.
12. **Per-thread file/naming conventions.** `ROOM_THREAD_ID`, `ROOM_PROJECT_ID`, `ROOM_THREAD_STORAGE` are injected into the shell (`gui/packages/agent-runtime/src/thread-shell-environment.ts:51`; `core/src/mac/room-cli/SKILL.md` warns they "may be empty" for shared-sandbox children and older threads). That is the only ambient identity other threads can key off.
13. **Org-level shared context.** Org `instructions` + `repo_rules` (per repo) are injected into every member's threads; the team board shows teammates' cards (title, purpose, percent, PR state) but **"Never messages"** (`gui/plugins/organization/contract.ts`).

**What does NOT exist — state plainly:**

- **No shared memory or context store.** No per-project knowledge base, no message bus, no "agent A's findings" object. `custom-instructions` is user-authored rules; `threadDynamicContextFileStates` tracks whether listed context files are present/too-large/non-utf8 — it does not share them.
- **No `next` pointer.** Verified by grep; `child_states` + reports are the actual mechanism.
- **@-mentioning a thread does not inject its transcript.** `promptMentionResourceSchema` allows `kind: "thread"` and the server resolves it to `{threadId, projectId, label}` only (`gui/apps/server/src/routes/threads/base.ts:342-361`), and the composer serializes a mention to plain text `@<label>` (`prompt-mention-clipboard.ts:70`). It is a link, not context.
- **No Cloud→Local parent messaging.** `relay.rs` states it: *"your parent moved to the user's Mac, and Cloud children can't message a Local parent yet."* The app can forward a completed-turn reply while it is open; nothing more.
- **No cross-thread file locking or conflict resolution.** Two threads on the same folder can collide; the product's answer is worktrees.
- **No agent-initiated thread discovery.** `thread list` shows your children and (with `--include-archived`) their archived siblings; the SKILL.md instructs "Never inspect, message, or launch other threads unless the user asks."
- **No shared terminal or shared screen across threads.** Computer use is per-sandbox (`core/src/computer_use/SKILL.md`: Xvfb 1440×900 + Cua Driver).

---

## 11. Notable engineering decisions

1. **Two runtimes, one vocabulary, a parity harness to keep them honest.** Local `agent-runtime` and Cloud `core/src/runtime` are independent implementations; `gui/packages/provider-parity` replays recorded cells through two bridge builds and diffs assembled timelines, with an allowlist for known divergences (`provider-parity/src/index.ts`, `cli.ts`). Expensive, but it lets each side use its natural language.
2. **Receipts, not acknowledgements.** Every mutation is `request_id` + `accepted→delivered→terminal`, with `unknown`/`unknown_after_restart` for genuinely uncertain work and an explicit rule that uncertain work is **never resent** (`core/docs/api.md`, `session-lifecycle.md`). This is the honest version of at-least-once.
3. **Domain strings are the API.** `SessionState` serializes as its own name and keeps retired values readable rather than migrating (`core/src/session/state.rs:20-22`, `Other(String)`). `ThreadEventType` is a refinement over strings so unknown future types don't crash old clients (`provider-event.ts:917-926`).
4. **Native history is adapter-owned; core never parses it.** "Core records the ACP stream only" (Cursor/Droid/Kimi); Claude disables fast mode, structured questions and hidden native subagents so its stream is the only truth. Each adapter owns resume identity checks (`harnesses.md`).
5. **A single small semantic vocabulary + declarative presentation.** "Core understands a small semantic vocabulary. Everything else is an extension kind with mandatory declarative presentation." "Presentation comes from the bridge, never from core tables." "Every client renders everything without plugin code." (`gui/docs/provider-plugin-api.md`, Principles.)
6. **Zero first-party privilege for providers.** First-party providers use only the public plugin API — "Every special case is a public primitive or is deleted" (same file). That is why `provider-claude-code` has a `PLUGIN_OVERVIEW.md` instead of a core branch.
7. **The dispatch checkpoint, not a prompt hook.** `experimental_hooks` intercepts a message *on its way to a provider* and can queue it with a reason or refuse it (`plugin-sdk/src/backend-contract.ts:2223-2229`). This is how `provider-retry` implements retry/queueing without touching core.
8. **Disk safety is a subsystem with published numbers.** 5 GB / 2 GB / 2.5 GB / 250 MB, a 32 MiB reserve file, per-process-group I/O accounting, and a stated failure mode ("polling-based protection, not a kernel guarantee"). `core/docs/storage.md`.
9. **Cheap sandbox hygiene in one file.** One `agent_env` in `core/src/runtime/linux.rs` gives every harness `BASH_ENV`/`CLAUDE_ENV_FILE` so `oom_score_adj 500` applies to every descendant command, plus `OMP_NUM_THREADS`/`PYTHON_CPU_COUNT`, a Node preload trimming `os.cpus()`, and a `sitecustomize` shim — because `nproc` and `os.cpu_count()` report host CPUs in a container. Six lines of problem, one file of fix.
10. **Loop scheduling lives in the website, not the core.** "Core passes each command to the website … the website's every-minute cron wakes the sandbox … so Core never parses cron" (`core/src/session/automation.rs:1-6`). Deliberate: cron correctness and wake-ups are a control-plane concern, and loops must work while the Mac is off.
11. **One WS protocol for push, HTTP for RPC.** The client socket accepts only `subscribe`/`unsubscribe`/`ping` (`gui/apps/server/src/ws/client-protocol.ts`). No RPC-over-WS, no dual write path, no reconnect state machine beyond subscriptions.
12. **The tunnel tolerates a Mac that blinks.** 8 s gap-hold on a dropped socket, 2 s keep-alive probes during large uploads, 12 s heartbeat staleness (`gui/apps/connect/src/tunnel-do.ts:32-43`). These constants are the product.

---

## 12. What to reuse for our product

Ranked by value per line of code we'd have to write ourselves.

1. **Provider-as-plugin with a declarative capability manifest** — `gui/packages/domain/src/provider-types.ts:123` (`providerInfoSchema`) + `gui/docs/provider-plugin-api.md`. Why: it is exactly our problem (many CLIs, one UI) and it has already made the hard calls — capabilities are *declared*, never probed ad hoc; `modelCatalogScope` is `host|workspace`; `fork: none|tip|…` captures a real difference between agents. Copy the shape, not the code.
2. **The `thread/delta` → `ThreadEvent` → timeline projection split** — `gui/packages/provider-bridge-protocol/src/thread-delta.ts` + `gui/packages/thread-view/`. Why: 27 delta kinds cover every agent we care about, and the projection layer means the UI never learns a provider name. Our chat/timeline work is the single largest UI cost, and this is a working answer.
3. **The dynamic-tool MCP proxy** — `gui/packages/provider-bridge-acp/src/bridge/tool-proxy-mcp.ts` + `plugin-sdk` `agents.dynamicTool` contributions. Why: it gives every agent, including ones we don't control, a uniform way to call back into *our* host (ask the user, run a workflow, attach a task) without patching the agent. Env-token + local socket, per thread. This is the cheapest possible "cross-agent tool bus".
4. **The request_id/receipt contract** — `core/docs/api.md` Conventions + Errors. Why: with N agents and M machines, retries are inevitable; `unknown` as a first-class terminal state (rather than pretending success or blindly retrying) is the single most valuable reliability idea here.
5. **Parent/child thread contract with `tell` + report notices** — `core/src/session/children.rs:4`, `core/src/session/relay.rs:1-6`, `core/src/mac/room-cli/SKILL.md`. Why: it is the closest existing design to "Claude writes, Codex reviews" and it has already discovered the sharp edges — children don't see later edits, shared sandboxes share memory, `wait` needs a 100 s ceiling with exit-2 retry.
6. **The session state machine + sleep/resume ladder** — `core/src/session/state.rs:13-33`, `core/docs/session-lifecycle.md`. Why: 17 states that distinguish `sleeping` / `suspended` / `process_lost` / `saved_history_only` are what make "many long-lived threads" affordable. If we invent our own we will end up with `running|idle|error` and no way to recover.
7. **History as a journal + snapshot + segment compaction** — `core/src/session/state.rs:5,105-118`, `core/docs/session-lifecycle.md` (Storage and replay). Why: turn logs are the biggest and least predictable dataset we'll own; "snapshot every 20k records, replay only newer, gzip into ≤1,024-record segments, 32 MiB reserve" is a complete, tested policy.
8. **Cross-sandbox relay to a control plane instead of sandbox-to-sandbox dialing** — `core/src/session/relay.rs`, `core/docs/api.md` (`parent_thread`, `/relay`, `report` until 410). Why: it makes agent-to-agent messages work with the user's laptop off, and it gives the control plane the authorization check. WebRTC or direct dialing is a dead end for this product shape.
9. **Task tracker as the shared-state substrate** — `gui/plugins/tasks` (`delegate/index.ts` seed prompt, `steer/index.ts` `steer-if-active`, `skills/tasks/SKILL.md`). Why: it is the only cross-agent surface in the repo that humans read and agents write, and the seed prompt + report-back contract is a compact, copyable prompt design.
10. **Workflow DSL for fan-out** — `gui/plugins/workflows/PLUGIN_OVERVIEW.md`. Why: `agent()` + pipeline + schema-constrained output + cached successful calls is a much better orchestration primitive than chaining `tell`s, and it survives restarts by design.
11. **Disk/space guard with published thresholds** — `core/docs/storage.md`. Why: multi-agent on one box fills disks in days (they burned a 50 GB disk in `#606`). The reserve-file trick alone is worth stealing.
12. **Tunnel-with-gap-hold** — `gui/packages/tunnel-contract/src/index.ts` + `gui/apps/connect/src/tunnel-do.ts:32-43`. Why: if we ever want "open my local project from a browser", this is a working, small, self-hostable relay with the right constants.

---

## 13. What to avoid / gotchas

**Licensing.**
- Apache-2.0 for the repo, MIT notice for BB preserved in `NOTICE`. Apache-2.0 is permissive but includes a patent grant + NOTICE-retention obligation: if we copy files we must carry both `LICENSE` and `NOTICE`, and mark modified files.
- `gui/` is a *fork*. Every `@cloudroom/*` package name is a rename of `@bb/*`, and fixture transcripts still contain `@bb/domain` imports (`gui/plugins/provider-claude-code/src/__fixtures__/transcripts/*.ndjson`). Expect stale references if we copy wholesale.
- 40 plugins are in-tree but `gui/plugins/bb-official.json` points at `getbb.app` marketplace URLs. Marketplace metadata is not ours to republish.

**Closed / missing pieces** (do not plan around them):
- **`web/` does not exist in this repo**, yet the code depends on it heavily: `web/lib/mobile.ts`, `web/lib/sandbox-logins.ts`, `web/lib/org.ts`, `web/lib/org-board.ts`, `web/lib/project-shares.ts`, `web/lib/threadCore`. The account system (better-auth tables exist locally, but the *account/Cloud billing/subscription* service), sandbox provisioning, Cloud Credits, `/child` and `/relay` endpoints, and loop cron all live there.
- **ADR corpus is closed.** `core/adr/` contains exactly one file (`0001-accept-privileged-core-service.md`), while the code cites ADR 0045, 0120, 0133, 0184, 0203, 0211, 0212, 0234, 0237, 0259, 0265, 0268, 0298, 0319 and dated ones like `20261118-201443`. The reasoning behind most decisions is *not* in the repo.
- **Managed Cloud requires a hosting backend.** Core "refuses to start" without `CLOUDROOM_LOCAL_HISTORY=1` or `CLOUDROOM_API_URL`; self-hosting is supported but then you own the VM, the agent accounts, and the logins.
- **Teleport and cross-sandbox children require the website.** `room-cli thread spawn` without `--shared-sandbox` "repeats `POST {api}/child` until the website answers" — no fallback.
- **`gui/plugins/organization` and `shared-projects` are client-only.** Empty without the website.

**Heavyweight / sharp edges.**
- **The dependency surface is enormous.** 4,869 TS files, 1,615 test files, React 19 + Vite 8 + TS 7 alias + `better-sqlite3` native build + `sharp` + Electron + Ladle + oxlint + Turborepo. A fresh clone build needs macOS arm64, Node 22.19+, `pnpm@10.34.9` exactly, Python 3.11+, Xcode CLT (`gui/README.md`).
- **Two runtimes is a permanent tax.** Any behavior you add (a new event kind, a new permission rule) must be implemented twice, plus a parity allowlist entry, plus Rust and TS tests. The repo admits this by building `provider-parity`.
- **`ThreadDetailView.tsx` is 3,355 lines** and `build-thread-timeline.ts` is 1,726. Copying the frontend means inheriting those.
- **Fixed 32 KiB prompt / 64 KiB JSON limits**, 10 MiB images, 100 MiB files, 8 MiB upload parts (clients use 4 MiB "since some sandbox proxies drop requests over about 8 MB"). If our prompts are bigger, the API contract changes.
- **The Cloud event payloads are not portable.** "Output records are not yet harness-neutral … `data` differs by harness." Any cross-harness feature (e.g. a unified diff view) has to go through the *app* model, not the core.
- **Several harnesses are previews.** Cursor: "Manual compaction, rewind, independent subagent controls and context-only notices are not advertised." OpenClaw: ~15 s cold start, 1.3 GB, and "a session whose model resolves to a Claude plan is deleted and refused". Agent Zero: "a cancel only pauses Agent Zero… so follow-ups queue instead of steering".
- **Credential rotation conflicts are real and unsolved.** "Kimi rotates refresh tokens, so the Mac and a sandbox may sign each other out (ADR 20261107-081737)." Same class of risk for Codex ("cross-machine refresh conflicts still require live verification").
- **Security posture is "trusted user, single-tenant VM".** "Agents run without approval prompts and share their account's files. Do not give that account sudo or access to a Docker socket." Command Guard is regex, "not a sandbox". Container mode without cgroups additionally relies on `agent_sudo: true` — i.e. the container *is* the boundary.
- **`gui/README.md` says the sync and preview helpers in `apps/server/src/assets/` are copies from core — change them in core.** A duplication trap that will silently diverge.
- **`COMPACT_HISTORY` is off** because older cores cannot read compacted journals — a self-inflicted forward-compatibility constraint we should not repeat (version the reader, then the writer).

---

## 14. Top files to read

| Path | Why |
|---|---|
| `README.md` | 4 KB of positioning; "what's in this repo" tells you the whole shape |
| `core/docs/harnesses.md` | 209 lines, the single densest document in the repo: all 13 adapters, auth, capability differences, recovery, command guard |
| `core/docs/api.md` | The full HTTP contract including request-id/receipt semantics and every error code |
| `core/docs/session-lifecycle.md` | Close / sleep / suspend / resume / recover, and the replay+snapshot storage model |
| `core/src/session/mod.rs:117-260` | The `Session` struct — the durable state of one running agent |
| `core/src/session/state.rs:1-120` | The 17-state machine and snapshot/segment policy |
| `core/src/session/children.rs` | Parent→child spawn, notices, `tell_target` — the delegate contract |
| `core/src/session/relay.rs:1-60` | Cross-sandbox messaging through the website, with its exact failure strings |
| `core/src/mac/room-cli/SKILL.md` | The agent-facing orchestration manual: spawn / wait / tell / fork / loop / mac / gpu |
| `gui/docs/provider-plugin-api.md` | The four principles that keep the provider layer provider-agnostic, plus the pipeline diagram |
| `gui/packages/domain/src/provider-types.ts` | `providerInfoSchema`, `availableModelSchema`, capabilities, recovery kinds |
| `gui/packages/agent-runtime/src/provider-adapter.ts` | The `AdapterCommand` union — what a provider must be able to do |
| `gui/packages/provider-bridge-protocol/src/thread-delta.ts` | The 27 delta kinds and the item taxonomy |
| `gui/packages/db/src/schema.ts` | 40 tables; read `threads`, `events`, `queuedThreadMessages`, `cloudroom_threads` in that order |
| `gui/apps/server/src/services/threads/child-thread-notifications.ts` | How child turns become parent prompts, in the app model |
| `gui/plugins/tasks/delegate/index.ts` | `buildSeedPrompt` — the best compact example of a delegation contract |
| `gui/plugins/tasks/steer/index.ts` | 56 lines: `steer-if-active` as a cross-agent notification primitive |
| `gui/docs/system-overview.md` | The four-process map and the two contract boundaries, in one page |
| `gui/docs/worktrees.md` | Managed worktree lifecycle, `.worktreeinclude`, setup/teardown, parking |
| `core/docs/storage.md` | The disk policy with real numbers (5 GB / 2 GB / 2.5 GB, 32 MiB reserve, per-cgroup I/O accounting) |

---

## 15. Open questions

Not answerable from this repo; each needs an account, a hosted service, or a human.

1. **What exactly does `web/` do?** Referenced modules imply: accounts/sessions, sandbox provisioning and wake, Cloud Credits, `/child` spawn accounting, `/relay` delivery, org shares, `/api/v1/cloudroom/threads-changed` signal, and loop cron. The repo contains no code or schema for any of it.
2. **How are sandboxes provisioned?** `core/install/` has `configure.py`, `cloudroom.service`, `agent-home.py`, `previews.py`; `gui/apps/vps` is a 3-file Docker image that only runs a tunnel client. No image build, no orchestrator, no per-sandbox limits that I could find. `core/docs/setup.md` describes *one* VM for *one* trusted user.
3. **How do Cloud Credits / billing / plan gates work?** `room-cli usage`, `credits_usage`, "Scans need a paid plan", `provider-usage` meters — all point at a closed service.
4. **Does the relay's authorization hold up?** `relay.rs` claims "The website checks the link in its own records". Unverifiable here; the reverse flow (any sandbox calling `POST /v1/sessions/{id}/relay`) is authenticated only by the core's own token, so the trust model depends entirely on the website.
5. **What is the actual concurrency/scale ceiling?** No load numbers, no per-host thread limits beyond "half the CPUs for startup", no queue depth. `MACS_PER_ACCOUNT`, `SESSION_TTL_MS` exist in the tunnel D1 schema (`app/connect/src/worker.ts` imports them from `store.ts`) but the values are policy, not measurement.
6. **How much of `provider-parity` actually passes today?** The harness and allowlist exist; the allowlist contents (`provider-parity` `ALLOWLIST_PATH`) tell you how far apart Local and Cloud really are — that file is the honest answer to "how good is the parity claim".
7. **Does the "team board" `takeOver` actually transfer a running thread across accounts?** The contract has `handoff` with messages + repo + branch + harness/model/reasoning, but the implementation is in `web/lib/org-board.ts`.
8. **What is the real-world failure rate of resume?** `recovery` returns `ready|empty|missing|unavailable` and `harnesses.md` says "File availability is not proof that the native handshake will succeed." No metrics are in the repo (`core/src/observability/` is local diagnostics, not telemetry).
9. **Is teleport safe under load?** `session/teleport.rs` is 37 KB with a phase machine, but `core/docs` only describes the user-visible steps; the file-consistency story (uncommitted work, running tools) is not documented.
10. **Why is only one ADR in the repo?** Either the rest are deliberately private or the export is partial. Either way, the reasoning for most of the architecture is not auditable from source.
