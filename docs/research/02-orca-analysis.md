# orca — Technical Analysis

| Field | Value |
|---|---|
| Repository | `stablyai/orca` (https://github.com/stablyai/orca) |
| Commit examined | `e348473086c7a064b8abd0a6a70a914269bc11df` (default branch `main`) |
| Release at research time | v1.4.264, published 2026-11-19 (`gh release list`). Note: `package.json` at this SHA still says `1.4.214` (ORC-084), so the release version is set outside the manifest. |
| Date | 2026-11-19 |
| License | MIT (GitHub API `license.spdx_id`) |
| Method | Static reading only. Nothing was installed, built or run. |
| History limitation | The clone is depth-1, so there is no local git history. History, issues, PRs and releases come from the `gh` API, fetched 2026-11-19. |
| Evidence base | Permalink base: `https://github.com/stablyai/orca/blob/e348473086c7a064b8abd0a6a70a914269bc11df/` |

**How "Confirmed" was established.** About 35 citations were re-read and line-checked directly by the author against the SHA. The rest were read by delegated read-only explorers working from the same clone. Every code citation tagged Confirmed was read in source by one of these. Directory-name-only observations are tagged Inferred.

---

## Summary

- **Agents are integrated through two separate stacks.**
  - **Terminal (TUI) stack:** about 45 agent CLIs run inside node-pty terminals. Orca learns their status from hooks it injects (Claude `settings.json`, Codex `hooks.json`) and from OSC 7501 escape sequences (ORC-011, ORC-016, ORC-088).
  - **Structured "native chat" stack:** a typed adapter registry. Claude goes through `@anthropic-ai/claude-agent-sdk` `query()`, Codex through `codex app-server` JSON-RPC, Pi through JSONL-RPC, and 9 more agents through ACP (ORC-012 to ORC-015, ORC-018).
- **Terminals outlive the app.** A detached PTY daemon, versioned at protocol v60, owns them. A headless emulator serves snapshots and a durable scrollback log replays history, so the app reattaches to running agents after a restart or crash (ORC-005, ORC-006, ORC-056, ORC-057).
- **One runtime, several clients.**
  - "Remote runtime" means `orcad`, the Orca runtime served headless without Electron. Desktop, CLI and mobile all reach it through one custom RPC envelope (ORC-008, ORC-009).
  - Over SSH, orcad is deployed to the remote host, bound to 127.0.0.1 there, and reached through an SSH port-forward (ORC-063).
- **Mobile continuity is a thin client of the same runtime.**
  - Pairing uses an `orca://pair` QR offer (ORC-068).
  - Transport is either a direct `ws://` connection on port 6768 or a cloud relay that splices two outbound sockets. Application-layer E2EE (NaCl/HKDF) protects it; there is no TLS (ORC-064 to ORC-069).
  - The phone can call only methods on an allowlist (ORC-070).
- **There is a multi-agent orchestrator, and it does not integrate.**
  - A SQLite-backed `Coordinator` dispatches pre-created tasks to worker terminals: at most 4 at once, a circuit breaker at 3 failures, and a mailbox protocol (ORC-031, ORC-034, ORC-038).
  - The `merge_ready` message and the `merging` phase exist but are never acted on (ORC-041).
  - Worker completion is self-reported, `filesModified` included (ORC-042).
- **Worktree lifecycle is the most mature part.**
  - Before creating a worktree it fetches the base. It rolls back failed creates.
  - Setup and archive hooks are gated by trust.
  - Removal checks for dirty state, removes the worktree, deletes the branch only after proving it is merged, and is journaled so an interrupted removal can resume (ORC-044 to ORC-049).
- **Git review is single-worktree and PR-centric.** It covers staging and commit, push with `--force-with-lease`, and rebase onto base. Conflicts are detected per worktree, with an "AI resolve conflicts" action. PRs are created and merged with `gh`, and merge is guarded by `--match-head-commit` (ORC-051 to ORC-053).
- **Security model.**
  - Good: every local server binds loopback and uses a per-run token, and secrets go to safeStorage or AES-GCM sealing.
  - Gaps:
    - token checks are not constant-time;
    - SSH host keys are accepted on first contact without a prompt;
    - direct mobile sockets still accept the legacy v1 E2EE;
    - API keys fall back to plaintext when safeStorage is unavailable;
    - the hook token is exported into every PTY's environment.
  - Evidence: ORC-020, ORC-067, ORC-072 to ORC-074, ORC-021.

---

## Repository facts

| Fact | Value | Source |
|---|---|---|
| Primary language | TypeScript, about 158 MB of the bytes counted by the GitHub languages API. Also JavaScript, Swift, Kotlin, Python, PowerShell, Rust, C/C++ for native add-ons and mobile. | `gh api repos/stablyai/orca/languages` |
| Files at SHA | 37,954 files (excluding `.git`). About 35k of them are `.ts`/`.tsx`. | `find` on clone |
| Created / last push | 2026-03-17 / 2026-11-19T15:41Z | GitHub API |
| Stars / forks / open issues | 129,579 / 8,234 / 8,548. GitHub's `open_issues_count` includes open PRs. | GitHub API, fetched 2026-11-19 |
| Release cadence | v1.4.261 to v1.4.264 shipped 2026-11-16 to 2026-11-19, about one per day. There is a separate Android mobile pre-release track (`android-v0.0.67`). | `gh release list` |
| Workspaces | pnpm root, with `src/packages/*` (`@orca/process-host`) and Windows native add-ons. `mobile/` and `cloud/` are independent workspaces with their own lockfiles (ORC-002). | `pnpm-workspace.yaml` |
| Node / package manager | Node 24, pnpm 12 | `package.json#L416-L419` |
| Key deps (ORC-001) | electron 43.8.10, electron-vite 6, react ~19.2, zustand 5, tailwindcss 4, @xterm/xterm 6.1 beta, monaco-editor 0.55, node-pty 1.1 (patched), ws 8.22 (patched), ssh2 1.17, zod 4.6, tweetnacl 1.0, @anthropic-ai/claude-agent-sdk 0.3.319, vitest 5, typescript 7 | `package.json#L229-L390` |
| Mobile | Expo ~58, react-native 0.88.1 | `mobile/package.json#L37,#L70` |
| Cloud | `cloud/apps/{relay, relay-edge, relay-ingress, relay-fence-broker, push}` and `cloud/packages/{relay-contract, push-contract, postgres-schema, cloud-database, connector-contract}` | directory listing |
| Code TODO/FIXME | 14 matching lines in non-test `src/` | `grep` on clone |

---

## Architecture map

| Component | Path | Responsibility | Evidence |
|---|---|---|---|
| Electron main process | `src/main/` (about 150 subsystems) | Hosts `OrcaRuntimeService`, the IPC handlers and the RPC server | ORC-008 |
| Renderer | `src/renderer/` | Sandboxed React UI: xterm panes, Monaco diff, source control, chat | ORC-003, ORC-055 |
| Preload | `src/preload/index.ts` | Single `contextBridge` `api` surface | ORC-004 |
| PTY daemon | `src/main/daemon/` | Detached Node process that owns PTYs, snapshots and durable history. Has its own wire protocol. | ORC-005 to ORC-007, ORC-056 to ORC-058 |
| orcad (headless runtime) | `src/main/orcad/`, `src/cli/runtime/serve/` | The same runtime without Electron. Started by `orca serve`, and deployed to SSH hosts. | ORC-009, ORC-010, ORC-063 |
| Runtime RPC | `src/main/runtime/rpc/` | Auth-token envelope over a Unix socket or pipe, WebSocket, long-poll or relay | ORC-008, ORC-064 |
| TUI agent catalog | `src/shared/agent-launch/catalog/` | About 45 CLI agent definitions: detect and launch commands, prompt injection, resume argv | ORC-011, ORC-022 |
| Structured agent registry | `src/main/native-chat/agent-session-wire/providers/`, `src/main/runtime/agent-session/installation/` | Adapter interface and registry for SDK, app-server, ACP and RPC agents | ORC-012, ORC-013 |
| Claude / Codex / ACP adapters | `src/main/claude/`, `src/main/codex/`, `src/main/acp/` | Provider-specific transports, hooks, permissions and resume | ORC-014 to ORC-019, ORC-087 |
| Agent hook server | `src/main/agent-hooks/`, `src/shared/agent-hook-listener/` | Loopback HTTP receiver for hook events, token-authenticated | ORC-020, ORC-021, ORC-088 |
| Orchestration | `src/main/runtime/orchestration/` | Coordinator, tasks and dispatches in `orchestration.db`, mailbox | ORC-031 to ORC-043 |
| Git / worktrees | `src/main/git/`, `src/main/runtime/workspaces/` | Worktree create, remove, archive and drift probe; staging, push, rebase, conflicts | ORC-044 to ORC-052 |
| GitHub | `src/main/github/` | PR create, merge, review comments, PR/CI watch | ORC-053, ORC-054 |
| Process host | `src/packages/process-host/` | Spawn, cancel and process-group tree-kill | ORC-059 |
| Persistence | `src/main/sqlite/`, `src/main/persistence/` | `node:sqlite` `DatabaseSync` databases and atomic 0600 JSON files | ORC-076, ORC-077, ORC-025, ORC-037 |
| Mobile | `mobile/` | Expo client that pairs with a runtime | ORC-068, ORC-071 |
| Cloud relay / push | `cloud/` | Splicing relay that sees only E2EE frames, plus an APNs/FCM push gateway | ORC-069 |

---

## 1. Architecture and technology stack

Orca is an Electron 43 desktop app with a React 19, zustand and Tailwind renderer, built with electron-vite (ORC-001). Its distinguishing property is that **the runtime has been pulled out of Electron.**

- `orcad` is "the Orca runtime served without Electron" (ORC-009). Its launcher checks the server bundle's SHA-256 before loading it.
- `orca serve` starts orcad from the CLI (ORC-010).
- Long-lived terminal state lives in a separate detached daemon (ORC-005), so neither the UI nor the runtime host owns agent processes directly.

The repo enforces process boundaries with ratchets. `package.json` has scripts such as `check:daemon-protocol-crossing`, `check:runtime-electron-ratchet` and `check:legacy-relay-fence-ratchet` (`package.json#L58-L76`), which suggests these boundaries are treated as invariants (Inferred: the scripts' contents were not read).

## 2. Frontend/backend separation

There are four process tiers.

1. **Renderer.** Sandboxed: `sandbox: true`, context isolation, webview guests hardened (ORC-003).
2. **Preload.** A single `api` bridge (ORC-004).
3. **Main or orcad runtime.** The Electron main process registers a large IPC surface; the explorer counted about 908 `ipcMain.handle` channels (Inferred count).
4. **PTY daemon.**

The tiers talk over three protocol families:

- **Electron IPC:** namespaced channels such as `pty:data` and `pty:ackData`, with explicit acknowledgement for backpressure.
- **Daemon protocol** (ORC-006, ORC-007):
  - Handshake: a hello carrying version, token, client id and a `control`/`stream` role.
  - Framing: NDJSON, or a negotiated `binary-v1` frame of a 1-byte kind plus a u32 length.
  - Requests: `createOrAttach`, `write`, `resize`, `kill`, `signal`, `detach`, `getSnapshot`.
  - Versioning: a protocol version gate per feature.
- **Runtime RPC** (ORC-008): a custom envelope `{id, authToken, method, params}` that returns `{ok, _meta.runtimeId}`, at `RUNTIME_PROTOCOL_VERSION = 3` with minimum compatible version 2. Despite the `jsonl-rpc/` folder name, it is not JSON-RPC. Only the legacy SSH relay uses true JSON-RPC 2.0.

The CLI, the mobile app and remote desktops are all clients of this RPC. Schemas are written in Zod.

## 3. Agent provider adapter system

**TUI path (ORC-011).**
- `TuiAgent` is a union of about 45 agent ids.
- `TuiAgentConfig` gives each one `detectCmd`, `launchCmd`, `expectedProcess` and `promptInjectionMode` (`argv | flag-prompt | stdin-after-start`), plus a draft-prompt flag and a preflight-trust flag.
- Adding an agent means adding a data entry, not code.
- Resume is a separate pure function per agent, `getAgentResumeArgv` (ORC-022).
- Permission bypass flags are a static map (ORC-027).
- `claude-agent-teams` is a catalog entry that launches `orca claude-teams` (ORC-089).

**Structured path (ORC-012, ORC-013).**
- Each adapter implements `StructuredAgentSessionAdapter`: `acquire`, `dispatch`, `rewind`, `compact`, `forkForRewind`, `manageMcp` and permission controls.
- `StructuredAgentRegistry` rejects duplicate agents. It also rejects any capability an adapter declares without implementing the method for it, so the capability set and the methods cannot drift apart.
- `STRUCTURED_AGENT_RUNTIME_CATALOG` lists `pi`, `codex`, `claude` and then `ACP_LAUNCH_SPECS`.
- The ACP specs (ORC-014) cover grok, opencode, opencode2, omp, antigravity, copilot, kimi, cursor and hermes. Each is a command plus args, with a per-agent "dialect" for quirks. The ACP types are generated from schema v1.21.0.

## 4. Native Claude Code and Codex integrations

**Claude, structured (ORC-015, ORC-024, ORC-079).**
- Calls `query()` from `@anthropic-ai/claude-agent-sdk`. `pathToClaudeCodeExecutable` points at the user's own CLI; the SDK's bundled binaries are excluded in `pnpm-workspace.yaml`.
- Passes a custom `spawnClaudeCodeProcess` so Orca's process host owns the child.
- Uses `canUseTool` for approvals and `includePartialMessages`, with the `claude_code` preset system prompt.
- Orca derives the session id itself: `stableUuid('orca-claude:'+sessionId)`. It passes `resume` if a transcript exists, otherwise `sessionId`.
- Rewind uses `resume`, `resumeSessionAt` and `forkSession`.
- Ambient credentials are scrubbed from the environment before spawn.

**Claude, terminal (ORC-016, ORC-017).**
- `applyManagedHooks` merges Orca command hooks into `~/.claude/settings.json`. It removes stale Orca entries and keeps the user's own hooks.
- The hooked events are SessionStart, UserPromptSubmit, Stop, StopFailure, SubagentStart and SubagentStop, Pre/PostToolUse, PermissionRequest and PostCompact. SessionEnd is gated by CLI version.
- Transcript JSONL under `<accountHome>/projects` is read to catch up on missed events.

**Codex (ORC-018, ORC-019, ORC-028).**
- Structured chat spawns `codex app-server` and speaks JSON-RPC over stdio: `initialize`, `thread/start`, `thread/resume` with `excludeTurns`, `thread/fork`.
- Server-initiated approval requests: `item/commandExecution/requestApproval`, `item/fileChange/requestApproval` and `item/permissions/requestApproval`.
- Terminal Codex uses a PTY plus `hooks.json`. Orca's own hooks are switched off for app-server launches with `-c` overrides.
- `codex exec` and Codex `notify`: **Not found in source** (only test fakes match).

**Status channel (ORC-088).** Managed hooks POST to a loopback server, and OSC 7501 sequences parsed from PTY output fill the gaps. Hooks take precedence.

## 5. Session persistence and reconnection

There are three layers.

**(a) Terminal survival.**
- The PTY daemon is detached and unref'd (ORC-005), and its socket path is versioned (`daemon-v60.sock`, ORC-058).
- On reconnect, `createOrAttach` returns `{isNew, snapshot}` from a headless xterm emulator (ORC-056).
- History is persisted to an append-only `OCKL` log that tolerates a torn tail: 5 MB log, 200 MB checkpoint (ORC-057).
- RPC subscriptions are replayed after reconnect and tagged `_replayedAfterReconnect` (ORC-078).

**(b) Agent resume.**
- Hook payloads carry the provider session id and transcript path. These are stored as `AgentProviderSessionMetadata`.
- Sleeping-agent records are created on worktree sleep, on quit, or live, and are validated against `getAgentResumeArgv`. On restart the agent is relaunched with, for example, `claude --resume <id>` or `codex resume <id>` (ORC-022, ORC-023).
- Structured sessions keep a `providerHandleChain` in `agent-session-domain.db` and a journal in `agent-session-journal.db`, with a JSON recovery capsule for restart offers (ORC-025).

**(c) Known failure modes, from issues fetched 2026-11-19.**
- Agents relaunched with `--resume` while the originals were still running in older daemons (#37909, ORC-091).
- A restored agent that fails at startup loses its resume record (#42289, ORC-092).
- Native chat sessions do not survive an Orca restart the way PTYs do (#42611, open feature request, ORC-093).

## 6. Agent orchestration and delegation

**Coordinator (ORC-031 to ORC-035, ORC-082).**
- `Coordinator` in `src/main/runtime/orchestration/coordinator/coordinator.ts` is started over RPC `orchestration.run`, and only one can be active at a time.
- It **does not decompose specs.** Tasks must already exist (created with `orchestration.taskCreate`), and in practice an agent creates them through the `orca` CLI, guided by the bundled `orchestration` skill.
- Every 2 s it ticks through four steps: process mail, open decision gates, dispatch ready tasks, check convergence.
- To dispatch, it creates a dispatch context and types a preamble into a worker terminal. The preamble tells the worker to report back with `orca orchestration send --type worker_done`.

**Data model (ORC-034).**
- `TaskStatus` is pending, ready, dispatched, completed, failed or blocked.
- `DispatchContextRow` adds failure count, retry-of, depth and `circuit_broken`.
- Mail types are status, dispatch, worker_done, merge_ready, escalation, handoff, decision_gate, question and heartbeat.

**One agent per terminal.**
- **All coordinator-managed workers live in the coordinator's single worktree** (ORC-033).
- Per-task worktrees come only from the separate `orchestration.workerStart` RPC with `--worktree new-top-level|new-child` (ORC-036).
- Issue #42415 reports that `worker-start` silently defaults to the caller's worktree, including a shared main checkout (ORC-090).
- Nested worker depth defaults to 4 (ORC-081).
- Mailbox routing and federation across machines (`orchestration/mailbox/*`) were seen only as directory names (Unverified).
- Bots and automations are separate delegation surfaces. Automations can use `new_per_run` workspaces and a precheck command (ORC-080).

## 7. Parallel task execution

- Parallelism comes from running many terminals or chats at once across worktrees. Each agent is an independent process.
- The coordinator caps dispatched tasks at `MAX_CONCURRENT_DEFAULT = 4`, overridable per run, and creates at most one new terminal per tick (ORC-031, ORC-033).
- Worktree deletion is capped at 2 concurrent.
- **Not found in source:**
  - a global cap on `workerStart` workers;
  - a guard against overlapping automation runs (explorer report; Inferred).

## 8. Git worktree lifecycle

**Creation (ORC-044, ORC-045).**
- `performAddWorktree` runs `git worktree add [--no-checkout] --no-track -b <branch> <path> <base>`. When reusing a branch it runs `git worktree add <path> <branch>`.
- It records `branch.<b>.base` and sets `push.autoSetupRemote`.
- The base is resolved as: explicit request, then the repo's `worktreeBaseRef`, then the default base. The remote-tracking ref is fetched first, and creation fails if that fetch fails, unless a local fallback was explicitly allowed.
- Branch names get a git-username prefix by default, are validated with `git check-ref-format`, and get a suffix loop on collisions.

**Hooks (ORC-046).**
- `orca.yaml` `scripts.setup`/`archive` run with a 120 s timeout.
- A trust policy (`ask`, `run-by-default`, `skip-by-default`) refuses creation if the policy is `ask` and the user has not decided. Setup content must match the approved trust.
- Issue #36414 reports that setup hooks do not reliably gate agent startup (ORC-094).

**Removal (ORC-047 to ORC-049).**
- Steps: archive hook (blocking unless waived), unlock check, `status --porcelain --untracked-files=all` dirty check (refuses unless forced), stop PTYs, `git worktree remove`.
- Branch deletion uses `git branch -d`, and force-deletes only after proving the branch has no unmerged changes, which covers squash merges. Otherwise the branch is preserved.
- Removals are serialized per repo and journaled so an interrupted removal can resume after a restart.
- Failed sparse creates roll back with a forced worktree removal and branch deletion.

**Reuse.** "Open existing branch" mode, a pool of prepared checkouts, and archive/restore.

**Folder workspaces have no isolation.** They share `repo.path` (ORC-050), and issue #44715 reports cleanup removing one repo's worktree while another repo in the same folder workspace was still in use (ORC-095).

## 9. Process supervision and cancellation

- `@orca/process-host` owns spawn and cancellation. On POSIX it kills the process group (`-pid`, detached children) after a 2 s grace; on Windows it uses `taskkill` (ORC-059).
- **Daemon crash loop:** at most 5 respawns in 60 s, then `DaemonCrashLoopError` (ORC-060).
- **Orphan prevention:** a death-watch pipe that closes even on SIGKILL, systemd cgroup scopes on Linux, and a macOS login-session watch (ORC-061).
- **Main-thread hang:** watched by a worker-thread watchdog. orcad also installs a fatal-signal guard (ORC-062).
- **Structured providers:** have group-orphan guards and an output-drain-on-exit step in `src/main/provider-process/` (Inferred from file names).
- **Cancelling a Claude permission prompt:** aborting settles the pending `canUseTool` with `null` (ORC-026).

## 10. Error handling and recovery

Recovery is designed per subsystem rather than through a generic retry layer:

- daemon respawn with a budget (ORC-060);
- torn-tail log recovery (ORC-057);
- removal journal and resume (ORC-049);
- rollback of failed worktree creates (ORC-044);
- dispatch circuit breaker after 3 failures (ORC-038);
- stale-base refusal that keeps the task `ready` and retries on the next tick (ORC-039);
- escalation triage in the coordinator.

When the task DAG cannot complete, convergence only logs "Stuck" and attempts no repair (ORC-043).

The repo also contains an evaluation harness that runs real agent CLIs as bots, serially and with `retry: 0` (ORC-085). It is a test asset, not a runtime feature.

## 11. Remote execution

**SSH (ORC-063).**
- Uses `ssh2`, or the system `ssh` binary.
- Orca deploys the orcad bundle plus a Node runtime to the host and runs `nohup ... --json --bind 127.0.0.1 --port <n>`.
- The desktop reaches it with `forwardOut` port-forwarding, falling back to stdio when sshd blocks forwarding.
- A legacy `relay.js` remains only for terminals created before 1.4.225.

**Auth (ORC-065, ORC-077).**
- An E2EE handshake, then a sealed device-token authentication.
- Local clients such as the CLI use an owner `authToken` read from `orca-runtime.json` (mode 0600).
- `orcad` refuses public source IPs unless `--allow-public` is given.
- Issue #24198 asks for remote Codex app-server connections, which implies structured Codex is currently local to the runtime that spawns it (Inferred, ORC-096).

## 12. Cross-device session continuity

The continuity mechanism is **"every device is a client of one runtime"**; no state is replicated between devices.

**Pairing (ORC-068).** An `orca://pair?code=<base64url JSON>` offer carries the endpoint, a device token, the host public key and optional relay details.

**Transport (ORC-064, ORC-069).**
- Direct: a WebSocket on port 6768. It binds loopback until a network device is paired, then rebinds to `0.0.0.0`.
- Relay: phone and desktop each open an outbound WebSocket and the cloud relay splices them, seeing only E2EE frames. Push notifications go through an APNs/FCM gateway.

**Scope (ORC-070).** The phone may call only methods on the `MOBILE_RPC_METHOD_ROUTES` allowlist: terminal, git, github, files, worktree, session, orchestration, nativeChat and others.

**Credentials (ORC-071).** Stored in the iOS keychain as `WHEN_UNLOCKED_THIS_DEVICE_ONLY`.

**Known gaps (ORC-097).** Blank Claude view after reconnecting to a headless `orca serve` host (#36705), and iOS disconnecting when backgrounded over Tailscale (#13354).

## 13. Git integration and change review

**Local git (ORC-051, ORC-052, ORC-055).**
- Stage, unstage and bulk-stage; `git commit -m`.
- Push with `--force-with-lease --set-upstream`.
- "Pull rebase from base" fetches into a temporary ref, then rebases with `--fork-point` or fast-forwards.
- Cherry-pick, revert and sequencer continue/abort.
- Conflict state is detected from MERGE_HEAD, rebase dirs, CHERRY_PICK_HEAD and REVERT_HEAD, with a conflict-review editor surface.
- An "AI resolve conflicts" action launches an agent with a generated prompt.
- Review UI: a combined diff viewer, and line-level `DiffComment`s that can be sent to an agent.

**GitHub (ORC-053, ORC-054).**
- PRs are created with `gh pr create`.
- `mergePR` defaults to squash and passes `--match-head-commit`.
- Merge is blocked when review is required, changes were requested, a merge queue applies, or `merge-tree` finds conflicts against base.
- PR watch checks the agent's "pushed" and "PR opened" claims against GitHub.
- Issue #25346 notes that finishing an in-progress rebase or merge with conflicts in-app is still incomplete (ORC-098).

## 14. Integration and verification workflows

**Not found in source.** Orca has no step that combines several agents' outputs. Specifically:

- **No merging of multiple worktrees or branches into an integration branch.** `merge_ready` is ignored, and `merging` is declared but never assigned (ORC-041).
- **No test or check run on a combined result.** The only "checks" are GitHub CI observation (ORC-054) and automation prechecks (ORC-080).
- **No cross-worktree conflict prediction.** `merge-tree` is used only between a PR's head and its base (ORC-053).
- **No verification of agent completion claims beyond identity.** `worker_done` is checked for authority (sender equals assignee, ids match), but its outcome and `filesModified` are recorded as `provenance: 'worker_report'` without checking them (ORC-042). Related open issues: #42148, "Reject an empty worker_done body" (ORC-099), and #18869, unverified `--from` sender attribution (ORC-100).
- **No automatic repair loop.**

The one integration-adjacent guard is base drift: a dispatch is refused when the worktree is more than 20 commits behind base (ORC-039, ORC-040). Issue #23823 asks for documentation of an orchestration flow "with acceptance and merge gates" (ORC-101), which suggests users expect this layer and do not find it.

## 15. Permissions and security

**Approval model (ORC-026 to ORC-030).**
- Five modes: `own`, `ask`, `accept-edits`, `auto`, `full-access`.
- Claude: mapped to SDK permission modes, with `full-access` passing `--dangerously-skip-permissions`.
- Codex: `approvalPolicy` and sandbox (`read-only`, `workspace-write`, `danger-full-access`), plus `approvalsReviewer: auto_review`.
- ACP: under full access it automatically picks the first `allow_once` option.
- TUI agents: a static "YOLO" flag map.
- Terminal Claude: a PermissionRequest hook is held open for up to 122 s for a UI decision. The blocking PreToolUse path fails open after 8 s (ORC-029).
- **Sandboxing is delegated entirely to each agent's own sandbox.** Orca adds no OS sandbox around agent processes (Inferred: none found).

**Credentials (ORC-074, ORC-075).**
- Electron `safeStorage`. Headless orcad uses AES-256-GCM sealing with a host-bound key file.
- When safeStorage is unavailable, API keys fall back to plaintext files (0600).

**Concerns observed (all Confirmed in code).**
- Runtime and daemon token checks use `===`/`!==`, not a constant-time comparison (ORC-072).
- SSH TOFU: unknown hosts become `accept-and-remember` with no prompt ("Prompt is reserved for future UI") (ORC-073).
- `requireV2` E2EE is enforced only on relay connections. Direct LAN sockets accept the v1 handshake, which has no forward secrecy (ORC-067).
- No TLS, since endpoints are `ws://` (ORC-066), and no Origin check on the runtime WebSocket was found.
- `ORCA_AGENT_HOOK_TOKEN` is exported into every pane's environment (ORC-021), so any process in the pane, including the agent, can post forged status events.
- Orca writes to the user's global `~/.claude/settings.json` (ORC-016).

## 16. Data persistence and event handling

**Engine (ORC-076).** Node's built-in `node:sqlite` `DatabaseSync`, called synchronously through a wrapper.

**Databases (ORC-025, ORC-037).**
- `profile-state.db`: a document store.
- `agent-session-domain.db` and `agent-session-journal.db`: `journal_rows`, `journal_sessions`.
- `orchestration.db`: `tasks`, `dispatch_contexts`, `peer_mail_*`, `coordinator_runs`.
- `last-status.db`: agent status.

**JSON files (ORC-077).** Runtime bootstrap, device and E2EE key files, written atomically at mode 0600.

**Terminal output** is an append-only binary log (ORC-057).

**Events.** Hooks feed a typed status inference. The agent-session journal is a row journal per session (Inferred: event schema columns not read). RPC subscriptions can be replayed after reconnect (ORC-078).

- **Not found in source:** a unified event-sourced log across all subsystems, or encryption of the SQLite databases.
- Issue PR #44007 compacts old orchestration databases (ORC-102).

## 17. Known engineering limitations

- **Orchestration** stops at dispatch and has no integration stage (ORC-041). Workers share the coordinator's worktree (ORC-033, #42415).
- **Completion claims** are trusted (ORC-042, #42148).
- **Resume races and lost records** (#37909, #42289). Native chat sessions are not restart-durable (#42611).
- **Mobile reconnect** reliability (#36705, #13354).
- **Security**: the gaps under §15.
- **Size**: about 38k files and roughly daily releases. Many static ratchets suggest the code base needs heavy guardrails to stay coherent (Inferred).
- **Tests**: the repo's `AGENTS.md` tells contributors "Do not add tests by default" (ORC-083). That is a deliberate policy, and it lowers regression coverage expectations.

---

## Special focus: failure-mode coverage

| Failure mode | Handled? | How | Evidence | Confidence |
|---|---|---|---|---|
| Git conflicts | Partial | Detected per worktree (MERGE_HEAD, rebase state). Conflict-review UI and "AI resolve conflicts". PR merge blocked if `merge-tree` reports conflicts vs base. No resolution across agents. | ORC-052, ORC-053 | Confirmed |
| Cross-worktree integration | No | No code merges two agents' worktrees or branches. `merge_ready` is ignored and the `merging` phase is never set. Integration happens implicitly as sequential PRs to the base. | ORC-041 | Confirmed |
| Dependency conflicts (lockfiles, packages) | No | Nothing found that compares or reconciles dependency changes across worktrees | none found | Inferred (absence) |
| API/interface mismatches between agents' changes | No | No combined typecheck or build. Only per-PR GitHub CI is observed. | ORC-054 | Inferred (absence) |
| Stale branch state | Yes, partial | Fetches base before create. The drift probe refuses dispatch when the worktree is more than 20 commits behind (overridable), but is inert without a coordinator worktree and fails open if the probe fails. Rebase from base. `--force-with-lease`, `--match-head-commit`. | ORC-039, ORC-040, ORC-045, ORC-051, ORC-053 | Confirmed |
| Test failures | Partial | GitHub check status watched, and checks can be rerun. Merge gated on required checks via GitHub. Orca never runs tests locally. | ORC-053, ORC-054 | Confirmed |
| Unverified agent completion claims | Partial | `worker_done` sender and ids are checked. Outcome and `filesModified` are self-reported. PR-opened and push claims are verified against GitHub. | ORC-042, ORC-054 | Confirmed |
| Partial task execution | Partial | Task and dispatch status in SQLite. Failed dispatch, then circuit-broken after 3 failures. Convergence logs "Stuck". No checkpoint of partial file changes. | ORC-034, ORC-038, ORC-043 | Confirmed |
| Interrupted integrations | N/A / partial | No integration step exists. Interrupted worktree removals resume from a journal. In-progress rebase/merge can be aborted, but finishing in-app is incomplete (#25346). | ORC-049, ORC-052, ORC-098 | Confirmed |
| Failed repair attempts | No | No repair loop. Escalations surface to the coordinator agent or user. "AI resolve conflicts" is one-shot and user-initiated. | ORC-043, ORC-052 | Confirmed |

---

## Notable design decisions and trade-offs

1. **The PTY is the universal adapter, with structured adapters alongside it.** The TUI catalog makes about 45 agents work with little code and keeps the native UX, but status has to be inferred from hooks and OSC. The structured registry gives typed approvals, rewind and fork, but only for about 12 agents. Running both doubles the surface area (ORC-011, ORC-012).
2. **The daemon owns processes.** Agents survive UI crashes and updates, at the cost of a versioned protocol (v60), socket-ownership proofs, crash-loop budgets, cgroup scopes and resume-duplication bugs (ORC-005, ORC-006, ORC-060, #37909).
3. **Headless runtime and thin clients.** The same runtime serves desktop, CLI, SSH-remote and mobile, so continuity is simply re-attaching. Nothing works offline on mobile, and the runtime host must stay up (ORC-009, ORC-068).
4. **E2EE instead of TLS.** This works through the relay and on LANs without certificates, but it means maintaining a custom crypto protocol, and the legacy v1 handshake is still accepted on direct connections (ORC-065, ORC-067).
5. **Orchestration is driven by agents through a CLI and skill, not by a planner.** Orca does no task decomposition or merging; the coordinator agent does both. This is flexible, but correctness rests on the agents' self-reports (ORC-032, ORC-082, ORC-042).
6. **Orca writes agent configuration.** Injecting hooks into the user's global `~/.claude/settings.json` and Codex hooks gives rich status, but it edits global state that the user owns (ORC-016).
7. **The git safety posture is conservative.** Branch deletion requires proof the branch is merged, removal requires a clean worktree, and pushes and merges are guarded by lease and head-SHA checks (ORC-047, ORC-048, ORC-053).

## Gaps and limitations

- Integration Intelligence (combined merge, combined verification, cross-worktree conflict prediction, repair loop): absent (ORC-041, ORC-042, ORC-043).
- Coordinator workers share one worktree, and `workerStart` defaults to the caller's worktree (ORC-033, ORC-090).
- Security: timing-unsafe token comparison, silent SSH TOFU, v1 E2EE downgrade on direct connections, plaintext API-key fallback, hook token in the PTY environment (ORC-021, ORC-067, ORC-072 to ORC-074).
- Resume and restart: duplicate relaunches, lost resume records, non-durable native chat (ORC-091 to ORC-093).
- Mobile reconnect reliability (ORC-097).
- Folder workspaces provide no isolation (ORC-050, ORC-095).

## Unverified items

- How orchestration mailbox routing and cross-machine "federation" peer mail work: only directory names were seen.
- The status-inference rules (`server-status-inference.ts`, `server-claude-status-rules.ts`): not read.
- The exact column-level event schema of `agent-session-journal.db`: tables confirmed, columns not read.
- The about 908 IPC channel count: an explorer's count, not independently recounted.
- The contents of the ratchet scripts that enforce process boundaries: only the script names were seen.
- Cloud relay internals beyond `cloud/README.md` and the splice state-machine state names. Server-side relay and push code was not audited.
- Whether `allow-stale-base` and other spec directives are documented to users.
- Stars, forks and issue counts are API snapshots from 2026-11-19 and can change.

---

## Appendix: EVIDENCE TABLE

Base `B` = `https://github.com/stablyai/orca/blob/e348473086c7a064b8abd0a6a70a914269bc11df/`. Issue and PR links were fetched 2026-11-19. Only the titles and state of issues were read; their bodies were not.

| ID | Claim (short) | Permalink | Symbol | Confidence |
|---|---|---|---|---|
| ORC-001 | Tech stack: Electron 43, React 19, zustand, node-pty, ws, ssh2, zod, tweetnacl, Claude Agent SDK 0.3.319 | B`package.json#L229-L390` | dependencies | Confirmed |
| ORC-002 | pnpm workspaces; mobile excluded | B`pnpm-workspace.yaml#L1-L8` | packages | Confirmed |
| ORC-003 | Renderer sandbox: true, preload | B`src/main/window/createMainWindow.ts#L163-L165` | createMainWindow | Confirmed |
| ORC-004 | Single contextBridge `api` | B`src/preload/index.ts#L218` | exposeInMainWorld | Confirmed |
| ORC-005 | PTY daemon spawned detached as Node | B`src/main/daemon/launch/daemon-launched-child-spawn.ts#L99-L110` | daemon spawn | Confirmed |
| ORC-006 | Daemon PROTOCOL_VERSION = 60 with feature gates | B`src/main/daemon/protocol/daemon-protocol-version.ts#L21` | PROTOCOL_VERSION | Confirmed |
| ORC-007 | Daemon hello (token, role) and binary-v1 framing | B`src/main/daemon/protocol/daemon-hello-protocol.ts#L1-L9` | HelloMessage | Confirmed |
| ORC-008 | Runtime RPC envelope with authToken; protocol v3, min 2 | B`src/main/runtime/rpc/core.ts#L103-L118`; B`src/shared/remote-runtime/protocol/protocol-version.ts#L53-L55` | RpcRequest, RUNTIME_PROTOCOL_VERSION | Confirmed |
| ORC-009 | orcad = runtime without Electron | B`src/main/orcad/serve-startup/orcad-command-arguments.ts#L9` | ORCAD_USAGE | Confirmed |
| ORC-010 | `orca serve` launches orcad | B`src/cli/runtime/serve/startup/serve-orcad-launch.ts#L1` | serve-orcad-launch | Confirmed |
| ORC-011 | TUI agent catalog (about 45 ids, launch config) | B`src/shared/agent-launch/catalog/tui-agent.ts#L1-L46`; B`src/shared/agent-launch/catalog/tui-agent-config-types.ts#L26-L81` | TuiAgent, TuiAgentConfig | Confirmed |
| ORC-012 | Structured adapter interface and registry | B`src/main/native-chat/agent-session-wire/providers/structured-agent-session-adapter.ts#L183`; B`src/main/native-chat/agent-session-wire/providers/structured-agent-registry.ts#L27` | StructuredAgentSessionAdapter, StructuredAgentRegistry | Confirmed |
| ORC-013 | Structured catalog: pi, codex, claude, ACP | B`src/main/runtime/agent-session/installation/structured-agent-runtime-metadata.ts#L153` | STRUCTURED_AGENT_RUNTIME_CATALOG | Confirmed |
| ORC-014 | ACP launch specs (9 agents) | B`src/main/acp/launch/acp-launch-registry.ts#L160` | ACP_LAUNCH_SPECS | Confirmed |
| ORC-015 | Claude via Agent SDK query() with user's CLI path | B`src/main/claude/claude-stream-json-connection.ts#L95-L123` | openClaudeStreamJsonConnection | Confirmed |
| ORC-016 | Managed hooks merged into ~/.claude/settings.json | B`src/main/claude/hooks/hook-settings.ts#L224-L262` | applyManagedHooks | Confirmed |
| ORC-017 | Claude hooked events list | B`src/main/claude/hooks/claude-managed-hook-events.ts#L13-L66` | CLAUDE_EVENTS | Confirmed |
| ORC-018 | Codex via `codex app-server` JSON-RPC | B`src/main/codex/structured/launch/codex-structured-launch-resolution.ts#L196-L203` | launch args | Confirmed |
| ORC-019 | Codex thread/start and thread/resume | B`src/main/codex/structured/threads/codex-structured-thread-open.ts#L99-L124` | thread open | Confirmed |
| ORC-020 | Hook server on 127.0.0.1, random token, 403 on mismatch | B`src/shared/agent-hook-listener/transport/agent-hook-startup.ts#L78`; B`src/main/agent-hooks/server/server-lifecycle.ts#L52-L71` | listenForAgentHooks | Confirmed |
| ORC-021 | Hook port and token injected into PTY env | B`src/main/agent-hooks/server/server-runtime-env.ts#L14-L27` | runtime env | Confirmed |
| ORC-022 | Per-agent resume argv (claude --resume, codex resume) | B`src/shared/agent-launch/startup/agent-resume-argv.ts#L9-L87` | getAgentResumeArgv | Confirmed |
| ORC-023 | Sleeping-agent records validated for restore | B`src/shared/workspace-session/workspace-session-sleeping-agents.ts#L106-L113` | sleeping agents | Confirmed |
| ORC-024 | Deterministic Claude session id; resume vs sessionId | B`src/main/claude/structured/lifecycle/claude-structured-session-id.ts#L4-L12` | claudeSessionIdForOrcaSession | Confirmed |
| ORC-025 | agent-session journal/domain SQLite DBs | B`src/main/native-chat/agent-session-journal/database/journal-host-database.ts#L16-L23` | journal host db | Confirmed |
| ORC-026 | Permission modes; Claude canUseTool callbacks | B`src/shared/agent-launch/permissions/agent-chat-permission-mode.ts#L13-L15`; B`src/main/claude/structured/permissions/claude-structured-inbound-control.ts#L40-L100` | AgentChatPermissionChoice, buildClaudePermissionCallbacks | Confirmed |
| ORC-027 | YOLO bypass flags per TUI agent | B`src/shared/agent-launch/permissions/tui-agent-permissions.ts#L12-L44` | YOLO_TUI_AGENT_ARGS | Confirmed |
| ORC-028 | Codex approval/sandbox policy mapping | B`src/shared/agent-launch/permissions/codex-permission-policy.ts#L21-L60` | codex permission policy | Confirmed |
| ORC-029 | Claude PermissionRequest hook held up to 122 s | B`src/main/agent-hooks/server/claude-permission-hook-hold.ts#L5-L30` | permission hook hold | Confirmed |
| ORC-030 | ACP full-access auto-selects allow_once | B`src/main/acp/session/acp-structured-acquire.ts#L110-L125` | acp acquire | Confirmed |
| ORC-031 | Coordinator class, phases, MAX_CONCURRENT=4, 2 s tick | B`src/main/runtime/orchestration/coordinator/coordinator.ts#L25-L37` | Coordinator | Confirmed |
| ORC-032 | Coordinator requires pre-created tasks | B`src/main/runtime/orchestration/coordinator/coordinator.ts#L145-L156` | Coordinator.run | Confirmed |
| ORC-033 | Workers listed and created in the coordinator's single worktree | B`src/main/runtime/orchestration/coordinator/coordinator.ts#L236-L257` | listAvailableWorkerTerminals | Confirmed |
| ORC-034 | Task/dispatch/mail data model | B`src/main/runtime/orchestration/db/orchestration-records.ts#L3-L330` | TaskStatus, DispatchContextRow, MESSAGE_TYPES | Confirmed |
| ORC-035 | Dispatch preamble asks for worker_done via CLI | B`src/main/runtime/orchestration/workers/launch/worker-dispatch-preamble.ts#L120` | preamble | Confirmed |
| ORC-036 | workerStart new-top-level/new-child worktree | B`src/main/runtime/rpc/methods/orchestration/worker/start/placement/worker-worktree-creation.ts#L29-L82` | worker worktree creation | Confirmed |
| ORC-037 | orchestration.db file and tables | B`src/main/runtime/orchestration/db/orchestration-db-file.ts#L5` | orchestration db path | Confirmed |
| ORC-038 | Circuit breaker after 3 failures | B`src/main/runtime/orchestration/db/dispatch-context/dispatch-circuit-breaker.ts#L1` | DISPATCH_CIRCUIT_BREAK_FAILURES | Confirmed |
| ORC-039 | Stale-base dispatch refusal (>20 behind; inert without worktree) | B`src/main/runtime/orchestration/coordinator/dispatch/coordinator-task-dispatch.ts#L70-L86` | DISPATCH_STALE_THRESHOLD | Confirmed |
| ORC-040 | Drift probe: fetch, then rev-list left-right | B`src/main/runtime/workspaces/metadata/runtime-worktree-drift-probe.ts#L15-L69` | probeRuntimeWorktreeDrift | Confirmed |
| ORC-041 | merge_ready ignored; `merging` phase never set | B`src/main/runtime/orchestration/coordinator/coordinator.ts#L185-L191`; B`src/main/runtime/orchestration/workers/lifecycle/lifecycle-reconciliation.ts#L120-L130` | tick / reconcile | Confirmed |
| ORC-042 | worker_done outcome and filesModified self-reported | B`src/main/runtime/orchestration/workers/lifecycle/lifecycle-reconciliation.ts#L279-L305` | reconcileWorkerDoneMessage | Confirmed |
| ORC-043 | DAG convergence logs "Stuck", no repair | B`src/main/runtime/orchestration/coordinator/coordinator-dag-convergence.ts#L19-L28` | convergence | Confirmed |
| ORC-044 | git worktree add args, rollback | B`src/main/git/worktrees/creation/worktree-add.ts#L229-L271` | performAddWorktree | Confirmed |
| ORC-045 | Fetch base before create; fail if fetch fails | B`src/main/runtime/workspaces/creation/runtime-local-worktree-create-base.ts#L20-L83` | resolveRuntimeLocalWorktreeCreateBase | Confirmed |
| ORC-046 | orca.yaml setup/archive hooks, 120 s timeout, trust | B`src/main/workspace-hooks/hooks.ts#L26` | hooks | Confirmed |
| ORC-047 | Removal flow and dirty check | B`src/main/git/worktrees/removal/worktree-removal-preflight.ts#L17-L83` | assertWorktreeCleanForRemoval | Confirmed |
| ORC-048 | Safe branch delete; force only if proven merged | B`src/main/git/worktrees/removal/worktree-branch-removal.ts#L56-L129` | branch removal | Confirmed |
| ORC-049 | Interrupted removal journal and resume | B`src/main/workspaces/removal/worktree-removal-records.ts#L1-L29` | removal records | Confirmed |
| ORC-050 | Folder workspace shares repo.path | B`src/main/runtime/workspaces/catalog/runtime-folder-workspace.ts#L43-L45` | folder workspace | Confirmed |
| ORC-051 | Push with lease; pull-rebase from base | B`src/main/git/remotes/remote.ts#L27-L50` | gitPush | Confirmed |
| ORC-052 | Conflict detection and AI resolve conflicts | B`src/main/git/source-control/mutations/git-conflict-operation.ts#L14-L73` | detectConflictOperation | Confirmed |
| ORC-053 | mergePR squash default, --match-head-commit | B`src/main/github/client/merge/merge-pr.ts#L21-L116` | mergePR | Confirmed |
| ORC-054 | PR/CI watch verifies push and PR-opened claims | B`src/main/github/pr-watch/pr-pending-actions.ts#L68-L80` | pr pending actions | Confirmed |
| ORC-055 | Diff comments sendable to agents | B`src/shared/file-review/diff-comment-types.ts#L1-L40` | DiffComment | Confirmed |
| ORC-056 | node-pty in daemon; headless emulator snapshots | B`src/main/daemon/pty-subprocess/native-pty-spawn.ts#L17-L84` | native pty spawn | Confirmed |
| ORC-057 | Durable OCKL history log with torn-tail recovery | B`src/main/daemon/history/storage/terminal-history-log.ts#L3-L23` | terminal history log | Confirmed |
| ORC-058 | Versioned daemon socket path | B`src/main/daemon/launch/daemon-spawner.ts#L122-L132` | getDaemonSocketPath | Confirmed |
| ORC-059 | process-host group/tree kill | B`src/packages/process-host/README.md#L3` | @orca/process-host | Confirmed |
| ORC-060 | Daemon respawn 5 per 60 s | B`src/main/daemon/launch/daemon-respawn-throttle.ts#L13-L14` | respawn throttle | Confirmed |
| ORC-061 | Death-watch pipe / cgroup scopes | B`src/main/daemon/process-scopes/daemon-scope-death-watch.ts#L5-L31` | death watch | Confirmed |
| ORC-062 | Main-thread hang watchdog | B`src/main/hang-watchdog/main-thread-hang-watchdog.ts#L1-L31` | watchdog | Confirmed |
| ORC-063 | orcad deployed over SSH, bound 127.0.0.1, port-forwarded | B`src/main/managed-server/install/orcad-remote-launch.ts#L135-L136`; B`src/main/ssh/port-forwarding/ssh2-port-forward-provider.ts#L32` | remote launch | Confirmed |
| ORC-064 | WS port 6768, loopback until a network device is paired | B`src/main/runtime/rpc/server/runtime-rpc-pairing-types.ts#L21-L25`; B`src/main/runtime/rpc/server/runtime-rpc-lifecycle.ts#L202-L208` | DEFAULT_WS_PORT | Confirmed |
| ORC-065 | E2EE handshake, then sealed device-token auth | B`src/shared/remote-runtime/remote-runtime-client-handshake.ts#L6-L13` | client handshake | Confirmed |
| ORC-066 | ws:// endpoints (no TLS) | B`src/main/runtime/rpc/server/runtime-rpc-pairing-types.ts#L28-L30` | formatWsEndpoint | Confirmed |
| ORC-067 | requireV2 only for relay; v1 accepted direct | B`src/main/runtime/rpc/mobile/mobile-socket-wiring.ts#L197` | requireV2 | Confirmed |
| ORC-068 | orca://pair offer schema | B`src/shared/remote-runtime/pairing/mobile-relay-pairing-offer.ts#L45-L103` | PairingOfferSchema | Confirmed |
| ORC-069 | Cloud relay splices outbound sockets; push gateway | B`cloud/README.md#L3-L47` | — | Confirmed |
| ORC-070 | Mobile RPC method allowlist | B`src/main/runtime/rpc/server/runtime-rpc-mobile-method-routing.ts#L285` | MOBILE_RPC_METHOD_ROUTES | Confirmed |
| ORC-071 | Mobile keychain WHEN_UNLOCKED_THIS_DEVICE_ONLY | B`mobile/src/host-credentials/pairing-keychain.ts#L5-L7` | pairing keychain | Confirmed |
| ORC-072 | Non-constant-time token comparison | B`src/main/runtime/rpc/server/runtime-rpc-request-admission.ts#L154`; B`src/main/daemon/server/daemon-client-connections.ts#L138` | request admission | Confirmed |
| ORC-073 | SSH unknown host accepted and remembered without prompt | B`src/main/ssh/host-keys/ssh-host-key-decision.ts#L6-L195` | HostKeyAction | Confirmed |
| ORC-074 | API key plaintext fallback | B`src/main/credentials/encrypted-api-key-file-store.ts#L135-L142` | api key store | Confirmed |
| ORC-075 | safeStorage store; orcad AES-GCM sealing | B`src/main/host/electron-secret-store.ts#L1-L54`; B`src/main/orcad/credentials/orcad-secret-sealing.ts#L12-L26` | secret stores | Confirmed |
| ORC-076 | node:sqlite DatabaseSync | B`src/main/sqlite/sync-database.ts#L2` | sync database | Confirmed |
| ORC-077 | orca-runtime.json authToken, 0600 atomic files | B`src/shared/remote-runtime/bootstrap/runtime-bootstrap.ts#L42`; B`src/shared/secure-storage/secure-file.ts#L177-L179` | runtime bootstrap | Confirmed |
| ORC-078 | Subscription replay after reconnect | B`src/shared/remote-runtime/rpc/runtime-subscription-replay.ts#L4` | _replayedAfterReconnect | Confirmed |
| ORC-079 | Claude SDK env scrubbed of ambient credentials | B`src/main/claude/claude-stream-json-connection.ts#L104-L109` | openClaudeStreamJsonConnection | Confirmed |
| ORC-080 | Automations: existing/new_per_run, precheck | B`src/shared/automations/automation-definition.ts#L19-L31` | automation definition | Confirmed |
| ORC-081 | Nested worker depth 4 | B`src/shared/orchestration/workers/nested-worker-depth.ts#L9` | nested depth | Confirmed |
| ORC-082 | Orchestration skill: agents drive the orca CLI | B`skills/orchestration/SKILL.md#L1-L17` | orchestration skill | Confirmed |
| ORC-083 | AGENTS.md "Do not add tests by default" | B`AGENTS.md` | Repo Rules | Confirmed |
| ORC-084 | package.json version 1.4.214 vs release 1.4.264 | B`package.json#L3` | version | Confirmed |
| ORC-085 | Bot-orchestration eval harness, retry 0 | B`config/vitest.bot-orchestration-eval.config.ts#L3-L17` | eval config | Confirmed |
| ORC-087 | ACP session/new, load, resume | B`src/main/acp/protocol/acp-session-setup.ts#L59-L72` | session setup | Confirmed |
| ORC-088 | Status via hooks plus OSC 7501 | B`src/main/agent-hooks/README.md#L3-L27` | — | Confirmed |
| ORC-089 | claude-agent-teams launches `orca claude-teams` | B`src/shared/agent-launch/catalog/tui-agent-config.ts#L29-L50` | TUI_AGENT_CONFIG_SOURCE | Confirmed |
| ORC-090 | Issue #42415: worker-start defaults to caller's worktree | https://github.com/stablyai/orca/issues/42415 | — | Confirmed (title) |
| ORC-091 | Issue #37909: duplicate resumed agents across daemons | https://github.com/stablyai/orca/issues/37909 | — | Confirmed (title) |
| ORC-092 | Issue #42289: restored agent loses resume record | https://github.com/stablyai/orca/issues/42289 | — | Confirmed (title) |
| ORC-093 | Issue #42611: native chat not restart-durable | https://github.com/stablyai/orca/issues/42611 | — | Confirmed (title) |
| ORC-094 | Issue #36414: setup hooks don't reliably gate startup | https://github.com/stablyai/orca/issues/36414 | — | Confirmed (title) |
| ORC-095 | Issue #44715: cleanup removes in-use worktree in folder workspace | https://github.com/stablyai/orca/issues/44715 | — | Confirmed (title) |
| ORC-096 | Issue #24198: request for remote Codex app-server | https://github.com/stablyai/orca/issues/24198 | — | Confirmed (title) |
| ORC-097 | Issues #36705, #13354: mobile reconnect problems | https://github.com/stablyai/orca/issues/36705 ; https://github.com/stablyai/orca/issues/13354 | — | Confirmed (title) |
| ORC-098 | Issue #25346: finish conflicted rebase/merge in-app | https://github.com/stablyai/orca/issues/25346 | — | Confirmed (title) |
| ORC-099 | Issue #42148: reject empty worker_done body | https://github.com/stablyai/orca/issues/42148 | — | Confirmed (title) |
| ORC-100 | Issue #18869: unverified --from sender attribution | https://github.com/stablyai/orca/issues/18869 | — | Confirmed (title) |
| ORC-101 | Issue #23823: docs for acceptance and merge gates | https://github.com/stablyai/orca/issues/23823 | — | Confirmed (title) |
| ORC-102 | PR #44007: compact old orchestration DBs | https://github.com/stablyai/orca/pull/44007 | — | Confirmed (title) |
