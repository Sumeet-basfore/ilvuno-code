# t3code — Technical Analysis

## Scope

| Item | Value |
|---|---|
| Repository | `pingdotgg/t3code` (MIT) |
| Commit examined | `da33a53d1896848eca237c6722d1447132f54eb3` (default branch `main`, verified with `git rev-parse HEAD`) |
| Release context | Latest stable `v0.1.10` (2026-11-17); latest nightly `v0.1.11-nightly.20261119.4941`. That nightly was built from `ff9efe89f6e7`, and `main` had moved to `dc7216c` by fetch time. The analysed SHA is therefore close to the nightly but not identical to it. |
| Date | 2026-11-19 |
| Method | Static reading of source, manifests, tests, and in-repo docs (`docs/internals`, `docs/orchestration-v2`, `docs/user`). Nothing was installed, built, or run. |
| History limitation | Depth-1 clone with no local history. Issues, PRs, releases, and repository stats come from the `gh` API (fetched 2026-11-19). |

Permalink base for all evidence: `https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/`

## Summary

- **One server, many clients.** A long-lived Node server (`t3`) owns provider processes, terminals, Git, files, and durable state. Web, Electron desktop, and React Native mobile are thin clients over a typed Effect RPC-over-WebSocket contract (T3-004, T3-005, T3-006, T3-008).
- **Provider adapter boundary.** All agents sit behind one `ProviderAdapterV2` service interface. It covers sessions, turns, steer, interrupt, rollback, fork, runtime requests, and a normalized event stream. Adapters are keyed by provider instance, not by driver (T3-009, T3-010, T3-011).
- **Native integrations.** Claude runs through `@anthropic-ai/claude-agent-sdk` `query()` against the user's own `claude` binary. Codex runs through `codex app-server`, which speaks JSON-RPC over stdio. Other providers use ACP, OpenCode servers, Pi RPC, or the Cursor SDK (T3-012, T3-013, T3-016).
- **Event-sourced orchestration.** A transactional SQLite (`node:sqlite`) store holds events, projections, idempotent command receipts, and an effect outbox. Side effects run in a separate worker after commit (T3-018, T3-019, T3-020, T3-024).
- **Multi-agent delegation exists, but is conversational.** Agents call an app-owned MCP server (`delegate_task`, `create_threads`, `t3_thread_launch`) to start child threads on any provider. A child can optionally get its own Git worktree. Results come back as text summaries (T3-028, T3-029, T3-030, T3-031).
- **No integration layer.** No code merges multiple agents' worktrees, detects cross-agent conflicts, or runs tests on a combined result. "Merge-back" moves conversation context only, never code (T3-048, T3-051).
- **Post-ship verification is delegated to the forge.** A PR watch wakes the agent on failed CI checks, conflicts, or rebase-needed. Commits are refused while conflict markers remain (T3-046, T3-047).
- **Remote-first.** Clients reach the server directly, over Tailscale, over SSH, or through T3 Connect. T3 Connect combines Clerk identity, a Cloudflare relay Worker, and per-environment `cloudflared` tunnels, with DPoP-bound, scoped sessions (T3-052, T3-054, T3-055, T3-056).
- **Security defaults lean permissive.** New threads default to **Full access**. The default pairing grant includes `orchestration:operate`, which the docs say is effectively full authority (T3-060, T3-056).

## Repository facts

| Fact | Value | Evidence |
|---|---|---|
| Stars / forks | 66,065 / 11,537 (API, 2026-11-19) | T3-069 |
| Open issues (API count, includes PRs) | 3,586 | T3-069 |
| Created / last push | 2026-02-08 / 2026-11-19T15:27Z | T3-069 |
| Primary language | TypeScript (about 212 MB of source by GitHub byte count). Also Kotlin, Swift, C, and Rust (`native/resource-monitor`) | T3-069 |
| Workspace | pnpm workspace with `apps/*` (server, web, desktop, mobile, marketing), about 32 `packages/*`, `infra/*` (relay, task-cache), `evals`, `scripts` | T3-001 |
| Core runtime library | Effect 4.1.3 (services, layers, schema, RPC, SQL) | T3-002 |
| Self-reported user count | "over 500,000 users" in `AGENTS.md` (marketing claim, not checked) | T3-070 |

## Architecture map

| Component | Path | Responsibility | Evidence |
|---|---|---|---|
| Server (`t3` CLI) | `apps/server` | Owns providers, orchestration, Git/VCS, terminals, MCP, auth, persistence; HTTP + WS RPC | T3-003, T3-005 |
| Contracts | `packages/contracts` | Effect Schema types for RPC, orchestration V2, git, auth, MCP; shared by all surfaces | T3-005 |
| Client runtime | `packages/client-runtime` | Connection supervisor, routes, auth renewal, thread/shell sync shared by web and mobile | T3-023 |
| Web | `apps/web` | React 19 + TanStack Router UI; also hosted at app.t3.codes | T3-004 |
| Desktop | `apps/desktop` | Electron shell; spawns and supervises a bundled server child; LAN/Tailscale exposure | T3-006, T3-007 |
| Mobile | `apps/mobile` | Expo / React Native client; push via T3 Connect | T3-008, T3-059 |
| Provider core | `packages/provider-core` | `ProviderAdapterV2` interface, process-group helpers, MCP session helpers | T3-009, T3-042 |
| Provider packages | `packages/provider-{acp,acp-registry,cursor,fx,grok,kiro,muse,opencode,pi}` | Per-provider drivers | T3-016 |
| Codex protocol | `packages/effect-codex-app-server` | Typed JSON-RPC client for `codex app-server` | T3-013 |
| Orchestration V2 | `apps/server/src/orchestration-v2` | Orchestrator, EventSink, EffectOutbox/Worker, projections, adapters, recovery | T3-018, T3-019 |
| VCS / Git | `apps/server/src/vcs`, `src/git`, `src/checkpointing` | Worktrees, commits, checkpoints (hidden refs), PR flows | T3-033, T3-038, T3-045 |
| Source-control hosts | `packages/source-control-*` | GitHub, GitLab, Forgejo, Bitbucket, Azure DevOps, GitCafe, Cursor Origin | T3-045 |
| MCP | `apps/server/src/mcp` | App-owned MCP endpoint for agents (orchestration, worktree, PR tools) | T3-028, T3-063 |
| Relay | `infra/relay` | Cloudflare Worker for T3 Connect: identity, links, tunnel allocation, webhook holding | T3-054 |

## Findings by area

### 1. Architecture and technology stack

The stack is TypeScript end to end, built on Effect 4 (services, layers, schema, RPC, SQL) (T3-002). The server is a Node app published as `t3`. It depends on `@anthropic-ai/claude-agent-sdk`, `@cursor/sdk`, `@muse-code/sdk`, `node-pty`, and `playwright-core` (T3-003). Persistence uses `node:sqlite` (T3-020). The server code is large: `Orchestrator.ts` has 16,885 lines, `ClaudeAdapterV2.ts` 14,846, `CodexAdapterV2.ts` 9,012, and `GitVcsDriverCore.ts` 6,463 (T3-066). The orchestration layer is a deliberate "V2" rewrite around a provider-neutral execution graph (run → execution nodes → provider threads/turns), with invariants such as "app ids are primary" and "child completion never closes the parent run" (T3-078). Tests rely on replay: recorded provider transcripts are replayed against the real orchestrator, and only the provider transport is substituted (T3-072).

### 2. Frontend/backend separation

All execution stays in the environment that owns the workspace. Clients "must never substitute" their own filesystem or credentials (T3-004). The wire boundary is an Effect `RpcGroup` in `packages/contracts/src/rpc.ts`, served by `RpcServer.makeProtocolWithHttpEffectWebsocket` in `ws.ts` (T3-005). Subscriptions are per-thread and per-shell, so a client pays only for the data it views (T3-004, T3-022). The desktop app does not embed the server in-process. It spawns the server as a child process, waits for readiness, and restarts it with backoff (T3-006). Desktop listens on loopback by default and on `0.0.0.0` when LAN exposure is turned on (T3-007). Mobile is a separate React Native client that uses the shared `client-runtime` (T3-008, T3-023).

### 3. Agent provider adapter system

`ProviderAdapterV2` is an Effect `Context.Service` with three members: `getCapabilities`, `planSelectionTransition`, and `openSession`. `openSession` returns a scoped `ProviderAdapterV2SessionRuntime` (T3-009). The runtime exposes `ensureThread`, `resumeThread`, `startTurn`, `steerTurn`, `interruptTurn`, `respondToRuntimeRequest`, `readThreadSnapshot`, `rollbackThread`, `forkThread`, and an `events` stream. It also has optional members: `injectHistory`, `compactThread`, `stopTask`, `stopSubagent`, `unloadThread`, `mcpApps`, `awaitBackgroundTasks`, and others (T3-009). Events are a normalized schema union: `provider_session.updated`, `provider_thread.updated`, `provider_turn.updated`, `node.updated`, `subagent.updated`, and so on (T3-010). Missing capabilities are explicit and handled by policy. For example, ACP providers have no native steering, so steering falls back to interrupt-and-restart (T3-077). Routing is by **instance**, so two accounts on the same driver do not share state (T3-011).

### 4. Native Claude Code and Codex integrations

- **Claude:** the adapter calls the Agent SDK `query()` and passes `pathToClaudeCodeExecutable`. The pnpm overrides strip the SDK's bundled platform binaries, so the user's installed `claude` always runs (T3-012). Permission mapping turns T3 runtime modes into SDK `permissionMode` values. Full access maps to `bypassPermissions` with `allowDangerouslySkipPermissions`. A `canUseTool` callback is installed for approval-required, edit, and plan modes (T3-015). T3's MCP server is injected as an HTTP MCP server with a bearer token (T3-063). Query boot has a 5-minute deadline because the SDK leaves it unbounded (T3-017).
- **Codex:** the server spawns `codex app-server -c thread_unload_delay_secs=0` and talks to it through a typed JSON-RPC stdio client (T3-013). Runtime modes map to Codex `approvalPolicy`/`sandboxPolicy` pairs: `untrusted`+`readOnly`, `on-request`+`workspaceWrite`, auto-review, and granular+`dangerFullAccess` (T3-014). T3 opts out of Codex `turn/diff/updated` and derives diffs from its own checkpoints instead (T3-049).
- **PTY:** user terminals use `node-pty` (T3-075). Agents themselves are not driven through a PTY; they use SDK or JSON-RPC protocols (T3-012, T3-013).

### 5. Session persistence and reconnection

Durable state is SQLite projections plus an event log. The log keeps about **1 day** of history for reconnect replay and retry deduplication. Older events are pruned, so projections cannot be rebuilt from the log (T3-021). On resume, a client sends `afterSequence`. The server replays events if the gap is within count and byte bounds, and otherwise sends a fresh snapshot (T3-022). Command dispatch is idempotent through receipts: a duplicate command id returns the original result and does not re-run side effects (T3-024). Native provider sessions resume through adapter `resumeThread`. Adapters declare `canResumeAfterRestart` (T3-009). After a server restart, `ProviderRuntimeRecoveryService` retires outbox effects that belonged to lost provider processes and re-attaches persisted questions (T3-025). `RestartContinuation` then posts a continuation message so interrupted runs resume (T3-026).

### 6. Agent orchestration and delegation

The basic unit is one provider session per thread. Multi-agent work goes through T3's own MCP server. Any adapter-backed provider can call `delegate_task` to create a T3-owned child thread on any provider instance (Claude → Codex works, for example). It can also call `create_threads` (1–20 top-level threads), `t3_thread_launch`, `t3_thread_send`/`wait`/`interrupt`, `task_status`, and `task_cancel` (T3-028, T3-031). The child receives only the task prompt; the parent's history is not copied (T3-030). Completion is published back as a `subagent_result` context transfer. Finalization is idempotent and survives restarts (T3-030). A dispatch-mode limit stops a child from running with broader permissions than its caller (T3-028). A DAG scheduling feature request (#31774) was closed on 2026-11-06 with status "completed", but no DAG scheduler was found in source (T3-051).

### 7. Parallel task execution

Threads run concurrently. Each thread has its own provider session, and side effects run through an effect worker with default concurrency 4. A running effect blocks later effects on the **same** thread, which serializes work within a thread (T3-019). Writes to a workspace are serialized by a per-`cwd` lease semaphore. Restore and turn-start reservations exclude overlapping paths: parents, children, and symlink aliases (T3-037). Open issues report that batched `delegate_task` waits start children one at a time (#23175), and that mixed fan-out runs serially (#34706) (T3-032).

### 8. Git worktree lifecycle

- **Creation:** a launch can use one of three workspace strategies: `root`, `existing_worktree`, or `worktree` with a `baseRef` (T3-033). `createWorktree` refuses option-like ref names, supports sparse patterns and submodules, and rolls back on failure (T3-033). With `isolation:"worktree"`, a delegated child gets a worktree from the parent's `HEAD` commit. Uncommitted parent edits are excluded (T3-029). Project setup scripts can run when a worktree is created (`runOnWorktreeCreate`) and when a thread settles (`runOnSettle`) (T3-074).
- **Reuse:** threads can attach to an existing worktree, and several threads can share one checkout (T3-031). `ThreadWorktreeRestore` recreates a deleted worktree from its branch (T3-036). `WorktreeBranchSync` updates thread records when the branch changes inside a worktree (T3-073).
- **Cleanup:** storage cleanup applies rules for inactive days, settled days, on merge, on delete, and unchanged. "Merged" requires the merged PR's head SHA to equal the worktree head (T3-035). Removal refuses dirty worktrees unless forced, and restores `.git` if a partial delete fails (T3-034). Known gaps: a worktree shared by several threads is never cleaned (#17816), deleting a worktree discards uncommitted changes without warning (#37769), and renamed or detached branches are never cleaned (#17813) (T3-068).

### 9. Process supervision and cancellation

On Linux, agent processes are launched with `systemd-run --user --scope` inside a dedicated slice with `CPUWeight=10`, `IOWeight=10`, and `TasksMax=25%`. Scopes can be OOM-killed and are classified when that happens (T3-040). Ownership records hold the PID plus boot-id-qualified start time, so a restarted server can SIGKILL leftover processes without killing a reused PID (T3-041). Process-group signalling uses `kill(-pid)` with guards (T3-042). Cancellation is `run.interrupt` / `thread.stop`. Interrupts have typed timeout errors and a 2-second bounded wait (T3-043). `task_cancel` stops nested delegated tasks before the child thread (T3-044). The desktop app restarts a crashed server with backoff (T3-006). Outside Linux, the systemd/cgroup containment is not available; the code returns early on non-Linux platforms (T3-041).

### 10. Error handling and recovery

Errors are typed `Schema.TaggedError`s at the adapter boundary: open, resume, turn start, steer declined, interrupt timeout, protocol, session stopped, and so on. Only fixed reason literals reach user messages, so private configuration does not leak (T3-009). Recovery mechanisms:
- outbox retirement after a crash (T3-025);
- restart continuation (T3-026);
- usage-limit auto-resume when the reset time arrives (T3-027);
- workspace restoration (T3-036);
- desktop backend restart (T3-006).

A provider that cannot roll back its conversation must reject a revert before any file changes (T3-039). There is no general "repair" loop for agent output. The closest mechanism is the PR watch, which wakes the agent with CI failures (T3-047).

### 11. Remote execution

Execution always happens on the environment host. Direct LAN, Tailscale, SSH, and T3 Connect only change how a client reaches it (T3-052). A headless background service listens on all IPv4 interfaces but closes any connection whose peer is not loopback, private, or tailnet. The docs say explicitly that this is not a firewall (T3-053). T3 Connect uses Clerk identity and a relay Worker that allocates managed **cloudflared** tunnels. After bootstrap, app traffic flows over the tunnel hostname, and the relay does not proxy HTTP or WebSocket sessions (T3-054). Peer links let one environment's agents act in another environment through its `/mcp` (T3-058).

### 12. Cross-device continuity

There is no device-to-device session handoff. Every device connects to the same server and replays from its own cursor (T3-022, T3-023). The client-runtime supervisor owns retries: jittered exponential backoff capped at 5 minutes, plus liveness probes when the app returns to the foreground (T3-023). Pairing uses QR codes or links (`t3 pair`) that mint scoped sessions. DPoP-bound pairing sessions renew for up to 30 days (T3-056). T3 Connect bootstrap credentials are bound to the client's DPoP key, and per-device approval is optional and **off by default** (T3-055). Mobile push notifications require T3 Connect (T3-059). Guest links give read-only, HMAC-token access to a single thread (T3-065).

### 13. Git integration and change review

The server implements stacked git actions: `commit`, `push`, `create_pr`, `commit_push`, and `commit_push_pr` (T3-045). Per-turn diffs come from hidden checkpoint refs (`refs/t3/checkpoints/...`) and exclude files Git brought in through pull, merge, or rebase (T3-038, T3-049). `ReviewService` provides branch commit lists and diff previews (T3-050). PR state syncs across seven host types. A PR sync reactor re-reads links after an agent runs a merge command itself (T3-071).

### 14. Integration and verification workflows

**Not found in source:** nothing merges the code changes of multiple agents or worktrees, nothing detects conflicts between them before merge, and nothing runs builds or tests on a combined result. `thread.merge_back` transfers *conversation context* from a fork to its source thread. It refuses if the target has active or queued runs, and it touches no code (T3-048). Delegated tasks return a text summary. An open PR (#30507) states that `task_status` gives the parent "nothing host-recorded to check", for example which checkout or input was used (T3-030). What exists instead:
- refusal to commit with leftover conflict markers (T3-046);
- refusal of checkpoint restores in shared workspaces (T3-039);
- PR watch wakes on `checks-failed`, `conflicting`, `needs-rebase`, and `merge-queue-removed`, with a limit of 10 consecutive comment-only wakes (T3-047).

### 15. Permissions and security

- **Permission modes:** Supervised, Auto-accept edits, Auto, and Full access. **The initial default is Full access** (T3-060). Administrators can set a managed-policy ceiling (`maxRuntimeMode`) (T3-009).
- **Scoped RPC:** each RPC method is authorized against a required scope (T3-057). The docs say the default pairing grant includes `orchestration:operate` and `terminal:operate`, and that either scope "can mint a session with every scope" (T3-056).
- **Agent MCP credentials:** tokens are scoped to environment, thread, instance, and session. They expire and are not persisted (T3-063).
- **Concerns:**
  - Scheduled-task webhooks without a signature can be triggered by anyone with the URL (T3-064).
  - The relay holds mint-signing authority, so a compromised relay key is not harmless (T3-055).
  - Server secrets are files with 0700/0600 permissions. No at-rest encryption was observed in `ServerSecretStore` (T3-062, Inferred).
  - Prompt-level tool denial is acknowledged not to be a sandbox (T3-061).
  - OpenCode's project-wide "always" grants are mitigated by replying `once` (T3-061).

### 16. Data persistence and event handling

SQLite through `node:sqlite`, with 127 entries in the migrations directory (T3-020). `EventSink` commits events, projections, the command receipt, and outbox rows in **one transaction**. Subscribers are notified only after commit (T3-018). `EffectOutbox` rows move through pending → running → completed, failed, or cancelled, and claim predicates enforce per-thread ordering (T3-019). Raw provider frames go only to rotating diagnostic logs, capped at 64 KiB per payload (T3-016). Retained events must stay decodable across versions, so schema changes are constrained by stored history (T3-021).

### 17. Known engineering limitations

- **Very large modules.** Single files run to 9,000–17,000 lines (T3-066).
- **No TODO/FIXME markers.** None appear in non-test server or package source. This probably reflects a convention; debt is tracked in issues instead (T3-067).
- **Retention.** The 1-day event retention rules out a full rebuild of projections (T3-021).
- **Non-Linux supervision.** Process containment is weaker off Linux (T3-041).
- **Open issues (fetched 2026-11-19):**
  - #37769 (worktree delete loses uncommitted work);
  - #17816 and #17813 (cleanup gaps);
  - #22315 (diff counters collide across chats on one repo);
  - #34401 (parent Stop silently interrupts async children);
  - #30156 (awaited background task lost on Claude process exit);
  - #35995 (about 7-minute startup migrations on 0.1.10) (T3-068).

## Special focus: failure-mode coverage

| Failure mode | Handled? | How | Evidence | Confidence |
|---|---|---|---|---|
| Git conflicts | Partial | Commit refuses leftover conflict markers. The PR watch wakes the agent on `conflicting`/`needs-rebase`. Resolution is left to an agent or the user. No pre-merge cross-branch conflict detection. | T3-046, T3-047 | Confirmed |
| Cross-worktree integration | No | Child worktrees are created from the parent HEAD. Nothing merges them back; `merge_back` carries context only. | T3-029, T3-048, T3-051 | Confirmed (absence: Inferred) |
| Dependency conflicts | No | Not found in source. Setup scripts run per worktree but do not reconcile lockfiles across agents. | T3-074, T3-051 | Inferred |
| API/interface mismatches | No | Not found in source. No cross-change type-check or build gate. | T3-051 | Inferred |
| Stale branch state | Partial | `startFromOrigin` and base-ref checks run at launch. `WorktreeBranchSync` reconciles branch records. The PR watch reports `needs-rebase`. No automatic rebase. | T3-033, T3-073, T3-047 | Confirmed |
| Test failures | Partial (post-push only) | Forge CI results wake the agent through the PR watch. No local test run on a combined result. | T3-047 | Confirmed |
| Unverified agent completion claims | No | Delegated results are text summaries. PR #30507 proposes recording execution evidence because none exists. | T3-030 | Confirmed |
| Partial task execution | Partial | Run, attempt, and node states are explicit. `restartContinuation` resumes interrupted runs. Checkpoints allow rollback in isolated worktrees. | T3-026, T3-038, T3-039 | Confirmed |
| Interrupted integrations | N/A / Partial | No integration step exists. Outbox effects are retired or retried after a crash, and command receipts make retries idempotent. | T3-024, T3-025 | Confirmed |
| Failed repair attempts | No | No repair loop or attempt budget beyond the PR-watch wake cap (10 consecutive comment-only wakes) and usage-limit resume. | T3-047, T3-027 | Confirmed |

## Notable design decisions and trade-offs

1. **Event sourcing with short retention (T3-018, T3-021).** One transactional commit of intent, projections, and outbox gives idempotent retries and clean recovery. Pruning to about 1 day keeps the database small, but projections become authoritative and cannot be rebuilt from the log.
2. **App-owned ids, provider refs at the edge (T3-078).** This allows provider switching, forks, and portable context handoff. The cost is large adapters, because each provider's quirks must be normalized there (T3-066).
3. **Delegation through an MCP server the agent calls (T3-028).** The design is provider-agnostic and lets agents orchestrate other agents. The trade-off is that coordination is driven by the model, so the host does not enforce structure (DAGs, verification) (T3-051).
4. **Use the user's own CLIs and SDKs (T3-012, T3-013).** Users bring their own subscriptions and native behavior. In exchange, T3 inherits CLI version drift and needs provider-specific update and ownership logic (T3-016).
5. **Hidden-ref checkpoints (T3-038).** Rollback works without commits on the user's branch. Restore is refused in shared workspaces, which pushes users toward worktree isolation (T3-039).
6. **Relay as a broker, not a proxy (T3-054).** Traffic and latency stay off the relay. Trust in the relay's signing key remains, and device approval is opt-in (T3-055).

## Gaps and limitations (with evidence)

- No code-level integration of parallel agents, no combined verification, no conflict prediction (T3-048, T3-051).
- Completion of delegated work is not verified, and no execution evidence is recorded (T3-030).
- Several open worktree lifecycle bugs: shared worktrees, branch renames, data loss on delete (T3-068).
- Parallel delegation sometimes runs serially (T3-032).
- Defaults are permissive: Full access, and a broad default pairing grant (T3-056, T3-060).
- OS-level agent containment is Linux-only (T3-040, T3-041).

## Unverified items

- The "over 500,000 users" claim (T3-070). Only `AGENTS.md` asserts it.
- Whether issue #31774 ("DAG scheduling", closed as completed) shipped any feature. No implementation was found, and there were no comments (T3-051).
- At-rest encryption of server secrets beyond file permissions. Only `ServerSecretStore` was read (T3-062).
- Runtime behaviour. Nothing was executed, so every claim about behaviour comes from code or docs.
- Equivalence of the analysed SHA to the published nightly: they differ (see Scope).

## Appendix: Evidence table

Base: `https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/`

| ID | Claim (short) | Permalink | Symbol | Confidence |
|---|---|---|---|---|
| T3-001 | pnpm monorepo: apps/*, packages/*, infra/* | [pnpm-workspace.yaml#L1](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/pnpm-workspace.yaml#L1) | `packages` | Confirmed |
| T3-002 | Effect 4.1.3 core library | [pnpm-workspace.yaml#L71](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/pnpm-workspace.yaml#L71) | `catalog.effect` | Confirmed |
| T3-003 | Server pkg `t3`; Claude SDK, Cursor SDK, node-pty deps | [apps/server/package.json#L39](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/package.json#L39) | `dependencies` | Confirmed |
| T3-004 | Execution owned by environment; clients over authenticated RPC | [docs/internals/overview.md#L3](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/overview.md#L3) | — | Confirmed |
| T3-005 | Effect RPC over WebSocket server | [apps/server/src/ws.ts#L3653](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/ws.ts#L3653) | `RpcServer.makeProtocolWithHttpEffectWebsocket` | Confirmed |
| T3-006 | Desktop spawns server child, restart backoff | [apps/desktop/src/backend/DesktopBackendManager.ts#L1051](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/desktop/src/backend/DesktopBackendManager.ts#L1051) | `calculateRestartDelay` | Confirmed |
| T3-007 | Desktop binds 127.0.0.1 or 0.0.0.0 | [apps/desktop/src/backend/DesktopServerExposure.ts#L33](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/desktop/src/backend/DesktopServerExposure.ts#L33) | `DESKTOP_LAN_BIND_HOST` | Confirmed |
| T3-008 | Mobile RN app connects to any server | [AGENTS.md#L31](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/AGENTS.md#L31) | — | Confirmed |
| T3-009 | Adapter service + session runtime interface | [packages/provider-core/src/server/ProviderAdapter.ts#L700](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/packages/provider-core/src/server/ProviderAdapter.ts#L700) | `ProviderAdapterV2SessionRuntime`, `ProviderAdapterV2` (L815) | Confirmed |
| T3-010 | Normalized adapter event union | [packages/provider-core/src/server/ProviderAdapter.ts#L83](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/packages/provider-core/src/server/ProviderAdapter.ts#L83) | `ProviderAdapterV2Event` | Confirmed |
| T3-011 | Adapters routed by instance id | [apps/server/src/orchestration-v2/ProviderAdapterRegistry.ts#L57](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/ProviderAdapterRegistry.ts#L57) | `ProviderAdapterRegistryV2` | Confirmed |
| T3-012 | Claude via Agent SDK `query()` with user's binary | [apps/server/src/orchestration-v2/Adapters/ClaudeAdapterV2.ts#L1629](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/Adapters/ClaudeAdapterV2.ts#L1629) | `query`, `pathToClaudeCodeExecutable` (L1552); override note pnpm-workspace.yaml#L147 | Confirmed |
| T3-013 | Codex via `codex app-server` JSON-RPC stdio | [apps/server/src/provider/codexLaunchArgs.ts#L13](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/provider/codexLaunchArgs.ts#L13) | `codexAppServerArgs`; `makeChildProcessClient` (effect-codex-app-server/src/client.ts#L313) | Confirmed |
| T3-014 | Codex runtime-mode → approval/sandbox mapping | [apps/server/src/orchestration-v2/Adapters/CodexAdapterV2.ts#L739](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/Adapters/CodexAdapterV2.ts#L739) | `codexRuntimeModeTurnDefaults` | Confirmed |
| T3-015 | Claude permission-mode mapping, bypass in full access | [apps/server/src/orchestration-v2/Adapters/ClaudeAdapterV2.ts#L3357](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/Adapters/ClaudeAdapterV2.ts#L3357) | `permissionModeForClaudeRuntimePolicy`, `claudeRuntimeQueryPolicyForRuntimePolicy` (L3424) | Confirmed |
| T3-016 | Other providers (OpenCode, Pi, ACP, Antigravity) and diagnostics limits | [docs/internals/providers.md#L13](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/providers.md#L13) | — | Confirmed (doc) |
| T3-017 | Claude query boot 5-minute deadline | [apps/server/src/orchestration-v2/Adapters/ClaudeAdapterV2.ts#L364](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/Adapters/ClaudeAdapterV2.ts#L364) | `CLAUDE_BOOT_TIMEOUT` | Confirmed |
| T3-018 | EventSink: single transaction for events/projections/receipt/outbox | [apps/server/src/orchestration-v2/EventSink.ts#L344](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/EventSink.ts#L344) | `EventSinkV2`; overview.md#L50 | Confirmed |
| T3-019 | Effect worker concurrency 4; per-thread blocking | [apps/server/src/orchestration-v2/EffectWorker.ts#L1043](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/EffectWorker.ts#L1043) | `DEFAULT_EFFECT_WORKER_CONCURRENCY`; `EffectOutboxV2` claim predicate (EffectOutbox.ts#L341) | Confirmed |
| T3-020 | SQLite via node:sqlite; 127 migration entries | [apps/server/src/persistence/initializeV2Database.ts#L33](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/persistence/initializeV2Database.ts#L33) | `DatabaseSync` | Confirmed |
| T3-021 | ~1 day event retention; projections not rebuildable | [apps/server/src/orchestration-v2/ProjectionMaintenance.ts#L42](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/ProjectionMaintenance.ts#L42) | `RETAINED_EVENT_HISTORY`; overview.md#L61 | Confirmed |
| T3-022 | Resume: bounded replay else snapshot | [apps/server/src/orchestration-v2/ThreadStream.ts#L100](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/ThreadStream.ts#L100) | `decideThreadResume` | Confirmed |
| T3-023 | Client supervisor: jittered backoff ≤5 min, probes | [docs/internals/connection-runtime.md#L9](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/connection-runtime.md#L9) | `supervisor.ts` (client-runtime) | Confirmed (doc) |
| T3-024 | Idempotent command receipts | [docs/orchestration-v2/README.md#L108](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/orchestration-v2/README.md#L108) | `CommandReceiptStore` | Confirmed (doc) |
| T3-025 | Recovery retires lost-process effects, re-attaches questions | [apps/server/src/orchestration-v2/ProviderRuntimeRecoveryService.ts#L80](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/ProviderRuntimeRecoveryService.ts#L80) | `ProviderRuntimeRecoveryService` (retire at L1051) | Confirmed |
| T3-026 | Restart continuation of interrupted runs | [apps/server/src/orchestration-v2/RestartContinuation.ts#L236](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/RestartContinuation.ts#L236) | `continueRestartedRun` | Confirmed |
| T3-027 | Usage-limit auto-resume | [apps/server/src/orchestration-v2/UsageLimitRecoveryWorker.ts#L17](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/UsageLimitRecoveryWorker.ts#L17) | `limitRecoveryCommand` | Confirmed |
| T3-028 | Orchestrator MCP: cross-provider delegation; scoped credentials; mode limits | [docs/orchestration-v2/orchestrator-mcp-server.md#L1](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/orchestration-v2/orchestrator-mcp-server.md#L1) | `OrchestratorToolkit`; `DispatchModeLimit` | Confirmed |
| T3-029 | `isolation:"worktree"` from parent HEAD; uncommitted excluded | [apps/server/src/mcp/OrchestratorMcpService.ts#L2743](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/mcp/OrchestratorMcpService.ts#L2743) | schema `isolation` (contracts/orchestratorMcp.ts#L179) | Confirmed |
| T3-030 | Child gets only prompt; result is summary transfer; no execution evidence (PR #30507) | [docs/orchestration-v2/orchestrator-mcp-server.md#L21](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/orchestration-v2/orchestrator-mcp-server.md#L21) | `subagent_result` (L497); https://github.com/pingdotgg/t3code/pull/30507 | Confirmed |
| T3-031 | `create_threads` 1–20 threads sharing parent checkout | [docs/orchestration-v2/orchestrator-mcp-server.md#L325](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/orchestration-v2/orchestrator-mcp-server.md#L325) | `create_threads` | Confirmed (doc) |
| T3-032 | Parallel delegation runs serially in some cases | https://github.com/pingdotgg/t3code/issues/23175 , https://github.com/pingdotgg/t3code/issues/34706 (fetched 2026-11-19) | — | Confirmed (issue) |
| T3-033 | Worktree creation + launch workspace strategies | [apps/server/src/vcs/GitVcsDriverCore.ts#L5217](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/vcs/GitVcsDriverCore.ts#L5217) | `createWorktree`; `ThreadLaunchWorkspaceStrategy` (ThreadLaunchService.ts#L68) | Confirmed |
| T3-034 | Worktree removal safety and force | [apps/server/src/vcs/worktreeRemoval.ts#L215](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/vcs/worktreeRemoval.ts#L215) | `removeLinkedWorktree` | Confirmed |
| T3-035 | Cleanup rules; merged requires exact head SHA | [apps/server/src/storageCleanup.ts#L287](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/storageCleanup.ts#L287) | `storageCleanupPullRequestMerged`; `worktreeCleanupEnabled` (L117) | Confirmed |
| T3-036 | Missing worktrees restored from branch | [apps/server/src/workspace/restoreMissingWorktree.ts#L135](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/workspace/restoreMissingWorktree.ts#L135) | `makeRestoreMissingWorktree`, `ThreadWorktreeRestore` | Confirmed |
| T3-037 | Per-cwd lease + overlap reservations | [apps/server/src/workspace/workspaceLease.ts#L9](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/workspace/workspaceLease.ts#L9) | `withWorkspaceLease` | Confirmed |
| T3-038 | Checkpoints as hidden refs `refs/t3/checkpoints/` | [apps/server/src/storageCleanup.ts#L1756](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/storageCleanup.ts#L1756) | `CheckpointStore`; overview.md#L88 | Confirmed |
| T3-039 | Restore refused in shared workspace | [apps/server/src/orchestration-v2/CheckpointRestoreSafety.ts#L9](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/CheckpointRestoreSafety.ts#L9) | `isCheckpointRestoreIsolated` | Confirmed |
| T3-040 | Linux systemd scope/slice containment | [apps/server/src/process/agentScope.ts#L23](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/process/agentScope.ts#L23) | `AGENT_SLICE`, `systemd-run --user --scope` (L165) | Confirmed |
| T3-041 | Ownership records, SIGKILL orphans; Linux only | [apps/server/src/process/providerProcesses.ts#L62](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/process/providerProcesses.ts#L62) | `ProviderProcesses` (`platform !== "linux"` L46) | Confirmed |
| T3-042 | Process-group signalling | [packages/provider-core/src/server/processGroup.ts#L2](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/packages/provider-core/src/server/processGroup.ts#L2) | `signalProcessGroup` | Confirmed |
| T3-043 | Typed interrupt timeouts, 2 s bound | [apps/server/src/orchestration-v2/ProviderTurnControlService.ts#L339](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/ProviderTurnControlService.ts#L339) | `ProviderAdapterInterruptTimeoutError` | Confirmed |
| T3-044 | `task_cancel` stops nested tasks then child | [docs/orchestration-v2/orchestrator-mcp-server.md#L308](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/orchestration-v2/orchestrator-mcp-server.md#L308) | `task_cancel` | Confirmed (doc) |
| T3-045 | Stacked git actions commit/push/PR | [packages/contracts/src/git.ts#L18](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/packages/contracts/src/git.ts#L18) | `GitStackedAction` | Confirmed |
| T3-046 | Commit refused with conflict markers | [apps/server/src/vcs/GitVcsDriverCore.ts#L2960](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/vcs/GitVcsDriverCore.ts#L2960) | `GitVcsDriver.commit.conflictMarkers` | Confirmed |
| T3-047 | PR watch wakes on checks-failed/conflicting/needs-rebase; cap 10 | [apps/server/src/orchestration-v2/pullRequestWatch.ts#L9](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/pullRequestWatch.ts#L9) | `PullRequestWatchChange`, `PULL_REQUEST_WATCH_WAKE_LIMIT` | Confirmed |
| T3-048 | Merge-back = context transfer only; refuses active/queued | [apps/server/src/orchestration-v2/Orchestrator.ts#L6007](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/Orchestrator.ts#L6007) | `dispatchThreadMergeBack` | Confirmed |
| T3-049 | Diffs derived from checkpoints; Codex diff events opted out | [docs/internals/providers.md#L171](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/providers.md#L171) | — | Confirmed (doc) |
| T3-050 | Review service: branch commits, diff preview | [apps/server/src/review/ReviewService.ts#L31](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/review/ReviewService.ts#L31) | `ReviewService` | Confirmed |
| T3-051 | No multi-agent code integration / combined verification / DAG | Absence across `apps/server/src` greps; request https://github.com/pingdotgg/t3code/issues/31774 | — | Inferred (absence) |
| T3-052 | Remote routes don't change execution model | [docs/internals/remote.md#L1](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/remote.md#L1) | — | Confirmed (doc) |
| T3-053 | Private-network-only listener, "not a firewall" | [apps/server/src/startupAccess.ts#L68](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/startupAccess.ts#L68) | `isPrivateNetworkAddress`; remote.md#L59 | Confirmed |
| T3-054 | T3 Connect: Clerk + cloudflared tunnel; relay not a proxy | [docs/internals/t3-connect.md#L3](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/t3-connect.md#L3) | `RelayClient.layerCloudflared` (server.ts#L335) | Confirmed |
| T3-055 | Relay holds signing authority; device approval off by default | [docs/internals/t3-connect.md#L63](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/t3-connect.md#L63) | `requireConnectDeviceApproval` | Confirmed (doc) |
| T3-056 | Scoped sessions; broad default grant; DPoP 30-day renewal | [docs/internals/environment-auth.md#L8](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/environment-auth.md#L8) | — | Confirmed (doc) |
| T3-057 | Per-RPC-method required scopes | [apps/server/src/auth/RpcAuthorization.ts#L41](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/auth/RpcAuthorization.ts#L41) | `RPC_REQUIRED_SCOPES`, `requiredScopeForRpcMethod` | Confirmed |
| T3-058 | Peer links across environments via `/mcp` | [apps/server/src/peer/PeerLinks.ts#L93](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/peer/PeerLinks.ts#L93) | `PeerLinks`; remote.md#L70 | Confirmed |
| T3-059 | Mobile push requires T3 Connect | [docs/user/mobile-notifications.md#L13](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/user/mobile-notifications.md#L13) | — | Confirmed (doc) |
| T3-060 | Default permission mode is Full access | [docs/user/permission-modes.md#L10](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/user/permission-modes.md#L10) | — | Confirmed (doc) |
| T3-061 | OpenCode "always" grant mitigation; tool denial ≠ sandbox | [docs/internals/providers.md#L28](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/providers.md#L28) | — (also L77) | Confirmed (doc) |
| T3-062 | Secrets as 0700/0600 files; no encryption seen | [apps/server/src/auth/ServerSecretStore.ts#L162](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/auth/ServerSecretStore.ts#L162) | `ServerSecretStore` | Inferred |
| T3-063 | MCP endpoint on loopback; scoped expiring bearer tokens | [docs/orchestration-v2/orchestrator-mcp-server.md#L33](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/orchestration-v2/orchestrator-mcp-server.md#L33) | `McpSessionRegistry` | Confirmed (doc) |
| T3-064 | Unsigned webhooks triggerable by URL holders | [docs/orchestration-v2/orchestrator-mcp-server.md#L470](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/orchestration-v2/orchestrator-mcp-server.md#L470) | `schedule_task` | Confirmed (doc) |
| T3-065 | Guest links: HMAC read-only thread tokens | [docs/internals/environment-auth.md#L40](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/environment-auth.md#L40) | `GuestShareService` | Confirmed (doc) |
| T3-066 | Very large modules (Orchestrator 16,885 lines etc.) | [apps/server/src/orchestration-v2/Orchestrator.ts](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/Orchestrator.ts) | `wc -l` | Confirmed |
| T3-067 | No TODO/FIXME in non-test server/package source | grep over `apps/server/src`, `packages` (excluding tests, fixtures) | — | Confirmed |
| T3-068 | Open issues: worktree data loss/cleanup, diff collisions, child interrupts, lost bg task, slow migrations | https://github.com/pingdotgg/t3code/issues/37769 , /17816 , /17813 , /22315 , /34401 , /30156 , /35995 (fetched 2026-11-19) | — | Confirmed (issue) |
| T3-069 | Repo stats from GitHub API | https://api.github.com/repos/pingdotgg/t3code (fetched 2026-11-19) | — | Confirmed |
| T3-070 | "Over 500,000 users" | [AGENTS.md#L9](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/AGENTS.md#L9) | — | Unverified |
| T3-071 | Server-owned settlement; PR sync after agent merge | [docs/internals/overview.md#L93](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/overview.md#L93) | `PullRequestSyncReactor` | Confirmed (doc) |
| T3-072 | Replay-backed orchestration tests | [apps/server/src/orchestration-v2/testkit/ThreadMergeBack.integration.test.ts#L1](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/testkit/ThreadMergeBack.integration.test.ts#L1) | `runOrchestratorV2ProviderReplayScenario`; README.md#L42 | Confirmed |
| T3-073 | Worktree branch record reconciliation | [apps/server/src/orchestration-v2/WorktreeBranchSync.ts#L15](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/orchestration-v2/WorktreeBranchSync.ts#L15) | `makeReconcile` | Confirmed |
| T3-074 | Setup/settle scripts per worktree | [packages/shared/src/projectScripts.ts#L56](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/packages/shared/src/projectScripts.ts#L56) | `runOnWorktreeCreate`, `runOnSettle` | Confirmed |
| T3-075 | Terminals via node-pty | [apps/server/src/terminal/NodePtyAdapter.ts#L36](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/apps/server/src/terminal/NodePtyAdapter.ts#L36) | `NodePtyModuleLoader` | Confirmed |
| T3-076 | OpenCode 2 per-thread MCP entries, cross-thread deny rules | [docs/internals/providers.md#L22](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/internals/providers.md#L22) | — | Confirmed (doc) |
| T3-077 | ACP steering = cancel-and-restart policy | [docs/orchestration-v2/orchestrator-mcp-server.md#L120](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/orchestration-v2/orchestrator-mcp-server.md#L120) | — | Confirmed (doc) |
| T3-078 | V2 invariants: app ids primary; child never closes parent | [docs/orchestration-v2/README.md#L33](https://github.com/pingdotgg/t3code/blob/da33a53d1896848eca237c6722d1447132f54eb3/docs/orchestration-v2/README.md#L33) | — | Confirmed (doc) |
