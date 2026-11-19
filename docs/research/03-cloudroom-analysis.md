# cloudroom — Technical Analysis

- **Repository:** [davidondrej/cloudroom](https://github.com/davidondrej/cloudroom), Apache-2.0 (BB's MIT notice in `NOTICE`)
- **Commit examined:** `21b5c6dfa86276581bd00401f40ddcfd01d09435` (branch `main`, "Publish GUI v159 and core 0.1.116", 2026-11-19)
- **Releases examined:** `core-v0.1.116` (Rust core), `gui-v159` (desktop app)
- **Date of analysis:** 2026-11-19
- **Method:** static reading of source, manifests, tests, in-repo docs and ADR, plus GitHub issues/PRs via `gh`. Nothing was built, installed or run. The hosted service ("the website", `CLOUDROOM_API_URL`) is **not** in this repository, so anything that depends on it is marked Inferred or Unverified.
- **Permalink base:** `https://github.com/davidondrej/cloudroom/blob/21b5c6dfa86276581bd00401f40ddcfd01d09435/`. The evidence table at the end has the full URLs.

## Summary

- cloudroom has two products in one repo. One is a **Rust "Core" service** (`core/`, one crate, axum + tokio) that runs one coding-agent CLI per session inside a Linux sandbox and records its history durably. The other is a **TypeScript desktop app** (`gui/`, Electron + Hono server + host daemon + React, forked from BB) that runs agents locally and drives Core over HTTP/SSE (CR-001, CR-002, CR-003, CR-005).
- Harnesses are plugged in as **native CLIs speaking their own protocol** behind one private Rust `Adapter` trait. Claude Code uses `-p --input-format stream-json`, Codex uses `app-server` JSON-RPC over stdio, Pi uses `--mode rpc`, and OpenCode/Droid/Kimi/OpenClaw/Agent Zero use ACP. None of this is PTY scraping (CR-007, CR-008, CR-009, CR-010).
- In the cloud, **every harness runs with permissions fully bypassed** (`--dangerously-skip-permissions`, Codex `approvalPolicy: never` + `danger-full-access`). Approval prompts are not supported, and the only safety layer is a regex "Command Guard" that fails open (CR-008, CR-009, CR-037, CR-039).
- Persistence uses an **fsynced, append-only per-session journal of receipts and records**, with idempotent `request_id`s, snapshots, gzip segments, SSE replay by sequence, and a hard rule that uncertain dispatched work is **never resent** (CR-012, CR-013, CR-015, CR-016).
- "Live sessions move between Mac and cloud" (**Teleport**) is a **snapshot + native-resume handoff, not a live stream**. The local agent is stopped, its native transcript file and some context files are uploaded, and a single fsynced activation record makes the cloud session resume the same native conversation with an injected "[Teleport handoff]" turn. Uncommitted edits to tracked files on an existing project stay on the laptop (CR-019, CR-020, CR-021, CR-022).
- Multi-agent work exists in three forms, and none of them integrates the results: (a) Core child sessions, either `--shared-sandbox` in **the parent's same folder** or own-sandbox through the hosted website; (b) the GUI **Workflows** plugin, which fans out up to 8 concurrent agents by default into **the same environment workspace**; (c) local sub-threads on fresh git worktrees (CR-023, CR-024, CR-025, CR-027, CR-028, CR-056).
- **Nothing merges multiple agents' outputs, runs tests on a combined result, or detects cross-agent conflicts.** The closest mechanism is **RepoSync**: it auto-commits and rebases between turns and pushes straight to the target branch (e.g. `main`), with no verification step. On a rebase conflict it aborts and tells the agent to resolve it (CR-031, CR-032).
- A child's "result" is its last reply text, truncated to 32 KiB. Completion claims are not verified (CR-026).
- Open issues document operational edges: shared-sandbox fan-outs OOM-killing the coordinator (#65), duplicate failure notices (#66), a teleport crash loop (#169), stuck sessions with missing native history (#82), and a cross-thread read-scope bypass via `mac run` (#103) (CR-043 to CR-047).

## Repository facts

| Fact | Value | Evidence |
|---|---|---|
| Layout | `core/` (Rust service + docs + Python e2e tests), `gui/` (pnpm/turbo monorepo: 7 apps, ~40 packages, ~40 bundled plugins), `skills/` (bundled agent skills) | CR-001, CR-002 |
| Rust | single crate `cloudroom` v0.1.116, edition 2024, rust 1.89; ~49.6k lines of `.rs` (core `src`+`tests` ≈ 59k incl. Python) | CR-001 |
| TypeScript | ~1.27M lines of `.ts/.tsx`, **including generated Codex app-server schemas, recordings and fixtures**, so hand-written code is much smaller (not measured) | measured with `wc -l`, Inferred split |
| Python | ~11.6k lines (Core e2e tests, sync/preview helpers) | measured |
| GitHub API (2026-11-19) | 1,787 stars, 315 forks, created 2026-09-17, last push 2026-11-19; languages by bytes: TypeScript 40.9 MB, Rust 1.9 MB, JS 0.6 MB, Python 0.3 MB | `gh repo view` |
| History | 210 commits, all "Publish GUI vN and core X" snapshots, so development happens elsewhere and the public repo is a release mirror | CR-054 |
| Issues/PRs | 49 open issues returned by `gh issue list --state open --limit 100` at analysis time (the brief's "~77" was not reproduced); recent PRs are mostly Windows fixes and new harnesses | `gh issue list`, `gh pr list` |
| Tests | Rust unit/integration tests (`core/tests/*.rs`), Python HTTP/e2e fixtures (`core_fixture.py`, `pi_e2e.py`, `storage_e2e.py`), vitest across GUI packages, recorded provider-bridge conformance fixtures | file listing |

## Architecture map

| Component | Path | Responsibility | Evidence |
|---|---|---|---|
| Core HTTP API | `core/src/api/mod.rs` | `/v1/sessions/*` (start, prompt, steer, interrupt, stop, sleep, resume, close, events, stream), `/v1/teleports/*`, `/v1/accounts/*`, `/v1/sync/*`; bearer-token middleware | CR-005, CR-006 |
| Session manager | `core/src/session/{mod,state,thread,children,relay,teleport,sleep,guard}.rs` | Durable receipts/queue, state machine, child sessions, cross-sandbox relay, teleport staging, idle sleep, disk guard | CR-012, CR-016, CR-019, CR-023 |
| Journal | `core/src/session/outbox.rs` | Append-only fsynced records, snapshots, gzip segments, upload batching | CR-012, CR-014 |
| Runtime / adapters | `core/src/runtime/{mod,claude,codex,pi,opencode,cursor_print,...}.rs` | `Adapter` trait, one module per harness, shared process driver | CR-007 to CR-010 |
| Linux containment | `core/src/runtime/linux.rs`, `core/install/*` | Separate agent UID, delegated cgroup per workload, freeze/kill, OOM-score helpers | CR-035 |
| Git reads/commit | `core/src/git.rs` (`cloudroom git`) | JSON status/diff/patch for the app's Git panel; the only write is `commit`; same binary used for Local and Cloud | CR-033 |
| RepoSync | `core/src/workspace/repo_sync.rs` | Between-turn auto commit + rebase + push to a configured branch | CR-031 |
| History upload | `core/src/session/history.rs` | gzip JSONL batches to the website over HTTPS | CR-053 |
| GUI server | `gui/apps/server` | SQLite (drizzle) source of truth, HTTP + WebSocket to app/CLI, routes work to host daemons, Cloud/Teleport orchestration | CR-003, CR-022, CR-051 |
| Host daemon | `gui/apps/host-daemon` | Per-machine executor: provisions workspaces, runs provider bridges, teleport capture | CR-004, CR-020 |
| Provider plugins | `gui/plugins/provider-{claude-code,codex,pi,acp}` + `gui/packages/provider-bridge-protocol` | Local harness bridges (Claude Agent SDK, Codex app-server, ACP) behind a versioned bridge protocol | CR-011 |
| Worktree environments | `gui/plugins/environment-git-worktree`, `gui/apps/server/src/services/environments` | `git worktree` per thread, spares, save-to-branch on retire | CR-029, CR-030 |
| Workflows | `gui/plugins/workflows` | QuickJS-sandboxed JS scripts that fan out `agent()` worker threads | CR-027, CR-028 |

## 1. Architecture and technology stack

Core is a single Rust binary (axum 0.8, tokio, reqwest/rustls, flate2, ring). Its modules are `api`, `session`, `runtime`, `workspace`, `sync`, `mac`, `preview`, `secrets`, `gpu`, `computer_use` and `git`. Only `git` and `fs` build on Windows (CR-001, CR-059). The GUI is a fork of BB: an Electron shell (`apps/desktop`), a Hono + better-sqlite3/drizzle server (`apps/server`), a host daemon using node-pty, @parcel/watcher and ws (`apps/host-daemon`), a React app (`apps/app`) and a `room-cli`/`bb` CLI (CR-002, CR-003). An ADR knowingly accepts Core's elevated service privileges as a trade-off (CR-039).

## 2. Frontend/backend separation

The structure has three tiers. (1) The React app and CLI talk to the **GUI server** over HTTP + WebSocket, using the `@cloudroom/server-contract` package. (2) The server talks to **host daemons** over a daemon WebSocket with host RPC, using the `@cloudroom/host-daemon-contract` package (CR-003, CR-004). (3) For Cloud threads, the GUI server calls **Core** directly over authenticated HTTP. It follows `GET /v1/sessions/{id}/stream?after=N` (SSE, `id` = sequence, resumable with `Last-Event-ID`) and mirrors Core records into its own SQLite `events` table (CR-005, CR-015, CR-051). The bearer-token check is constant-time and also accepts `X-Cloudroom-Token` for proxies that rewrite `Authorization` (CR-005). Core binds to `127.0.0.1:9840` by default (CR-006). The hosted website sits between Core instances for child relay, loops and history storage (CR-025, CR-053).

## 3. Agent provider adapter system

There are **two independent adapter layers**:

- **Core (cloud):** `enum Kind` lists 12 harnesses (Codex, Pi, Cursor, Claude, fx, OpenCode, Hermes, Devin, Droid, Kimi, OpenClaw, Agent Zero) (CR-007). Every adapter implements the private `trait Adapter`: `encode` (request → native frame), `receive` (native frame → `Event`s, with a shared `Progress`), `response`, `respond`, `capture` (read native history), `close`, `closed` and `clean_exit`. The shared `Handle`/process driver owns stdio, timeouts and containment. A code comment states the intent: "Protocol differences stay here, not in Session Management or the process driver" (CR-007). The output vocabulary is a small `Event` enum (`Started`, `Finished{status,text}`, `Record`, `ChildRequest`, `Compacted`, `Background`, `Exited{reason,expected,cleaned_up}`) (CR-007).
- **GUI (local):** each provider is a plugin with a "bridge" speaking the versioned **provider-bridge protocol**. Capability negotiation covers `sessionRestore`, `fork`, `steerMode: inject|queue`, `approvalEnforcedBy: runtime|provider` and grammar version ranges (CR-011).

## 4. Native Claude Code and Codex integrations

- **Cloud Claude Code:** Core spawns the `claude` CLI with `-p --input-format stream-json --output-format stream-json --include-partial-messages --replay-user-messages --dangerously-skip-permissions --permission-prompt-tool stdio`. It sets `--session-id` or `--resume` (with `--fork-session`/`--resume-session-at` for forks) and passes one merged `--settings` JSON (CR-008). It **disallows native subagent tools** (`Agent,Task,Workflow,SendMessage,AskUserQuestion,EnterPlanMode,...`) and registers an in-process SDK MCP server `cloudroom` plus a `SubagentStart` hook callback, so delegation is routed into Cloudroom child sessions (CR-057). No SDK is used in the cloud ("without an SDK bridge") (CR-008).
- **Cloud Codex:** `codex app-server --listen stdio://`, JSON-RPC `initialize` → `thread/start|resume|fork` with `approvalPolicy:"never"`, `sandbox:"danger-full-access"`. Turns use `turn/start`, `turn/steer`, `turn/interrupt` and `thread/compact/start`. Core rejects a start if Codex reports a different model or a different resumed thread id (CR-009).
- **Local:** the Claude Code bridge uses `@anthropic-ai/claude-agent-sdk` `query()`. The Codex bridge spawns an app-server child (CR-011).

## 5. Session persistence and reconnection

Every command is a `Receipt` (`request_id`, `command`, `input`, `state`). It moves `accepted → delivered → completed|interrupted|failed|unknown|unknown_after_restart`. Retrying the same id returns the stored receipt, and different content under the same id is rejected (CR-012, CR-013). Acceptance and queue membership come from one fsynced record. Records are fsynced before subscribers are notified (CR-012). Startup restores `snapshot.json` and replays newer records. Saved records are folded into ≤1,024-record gzip segments (CR-014). Clients reconnect by sequence (`events?after=N`, SSE `Last-Event-ID`). A client disconnect never stops execution (CR-015). Native resume validates the saved conversation's identity, workspace and model before admitting prompts (`check_resume_history`) (CR-016). Sessions idle for 30 minutes are stopped to `sleeping` and resumed on the next prompt (CR-018).

## 6. Agent orchestration and delegation

- **Native delegation → child sessions:** only Claude and Pi parents can raise a `ChildRequest`. The child is a durable session `cr_child_<key>` created **with the parent's `workspace`**. The parent's tool call receives the child's **last reply text, capped at 32,768 chars**, or an error string (CR-023, CR-026).
- **`room-cli thread spawn`:** with `--shared-sandbox`, a child of any of 10 harnesses runs in the parent's folder and sandbox. Without the flag, the website starts it in its own sandbox and relays `tell/stop/archive` and turn reports between sandboxes (CR-024, CR-025). A `request_id` retry that started a duplicate child was reported in #61 (closed) (CR-048).
- **Workflows plugin (GUI):** a JS script in QuickJS with no fs, network or clock access calls `agent()`, `pipeline()` and `parallel()`. Workers are normal hidden threads with optional JSON-Schema outputs. Successful calls are cached and replayed on resume (CR-027).

This is a **tree of independent agents with message passing**. There is no planner/integrator role in code. Coordination quality depends on the parent agent's prompting.

## 7. Parallel task execution

Hosted Cloud gives "one isolated sandbox per thread". A self-hosted Core runs all threads on one machine (CR-039). In-sandbox parallelism (shared children, workflow workers) is limited only by count settings: Workflows defaults to `maxConcurrentAgents: 8`. Shared-sandbox children have **no admission control** (#65, ~40 children OOM-killed the coordinator) (CR-027, CR-044). Harness *startup* is limited to half the logical CPUs, but running sessions are not capped (session-lifecycle doc; CR-017).

## 8. Git worktree lifecycle

- **Local:** the `environment-git-worktree` plugin creates `git worktree add` per thread under the app data dir. Project sub-threads get fresh worktrees by default. Spare worktrees are pre-built (CR-056, CR-029). On retire it runs a teardown script, kills every process whose cwd is inside the worktree, runs `git worktree remove --force`, and deletes the folder (CR-029). Before removal the server commits uncommitted work to the thread branch, or to `cloudroom/saved-<env>` when HEAD is detached. If the save fails, the folder is kept and rechecked (CR-030). The docs state that worktrees on **other machines are removed 3 days after archive "without saving first"** (CR-030).
- **Cloud:** a sandbox per thread plus a project folder (`/code/...` or `/repos/<name>`). Not found in source: worktrees inside Core. Shared children and workflow workers use the same folder (CR-024, CR-028).

## 9. Process supervision and cancellation

The shared process driver bounds frames to 16 MiB, applies a 4-second shutdown grace, and escalates to kill (CR-036). On Linux each workload gets a delegated cgroup. `terminate` writes `cgroup.kill`, and close waits until no descendants remain. Cleanup failure blocks replacement (CR-035). In hosted sandboxes the policy is container mode (`cgroup_root: null`, per #65). In that mode workloads are tagged with an env var and signalled, and "processes that clear their environment or switch user through sudo escape" (CR-035). Interrupt stops only the current turn, and queued prompts continue. A process-generation check discards stale replies (CR-016). The disk guard freezes cgroups under 2 GB free and, after 30 s, kills the heaviest writing process group (CR-041).

## 10. Error handling and recovery

An unexpected harness exit gets **one** automatic recovery after confirmed cleanup. A new user request re-arms it. A failed native resume becomes `process_lost`, with no restart loop or fresh-conversation fallback (CR-017). In-flight turns at a crash become `unknown_after_restart` and are never replayed (CR-016). Launch errors use stable codes (`claude_auth_required`, `storage_blocked`, ...). A journal write with an uncertain outcome blocks all new mutations until restart (CR-016). The GUI `provider-retry` plugin works differently: it automatically sends "continue" after an app/daemon restart mid-turn and retries overload/limit failures up to 4 times. This conflicts with Core's "never resend" principle (CR-049). Issue #82 shows a Cloud Codex thread stuck with no supported recovery when native history is missing (CR-047).

## 11. Remote execution

Core runs on an Ubuntu VM via systemd (self-host) or on a sandbox via `sandbox-start.sh` (CR-035). Code reaches the cloud through a GitHub clone or an upload ("project copy"), Mac↔VM file sync (`core/src/sync`), and teleport uploads (CR-020, CR-060). Credentials reach the sandbox by native login on the VM (Claude/Codex device flows, where Core never sees the tokens) or by **copying Mac logins** for Pi, Kimi and OpenClaw (CR-042). Agents can also run commands on the paired Mac (`room-cli mac run`) (CR-043). **Unverified:** which hosting provider supplies the sandboxes. The website code (`web/lib/sandbox-logins.ts` is referenced) is not in this repo (CR-052).

## 12. Cross-device session continuity

The mechanism is **Teleport**, and it is a **stop → snapshot → upload → native resume + handoff prompt** sequence:

1. The GUI server writes the transfer in one SQLite transaction and stops the local thread via `stopThreadForCurrentState` (CR-022).
2. The host daemon captures the harness's **native session file** (OpenCode via export), child traces, `git-state.txt` (`git status --short --branch` + HEAD), untracked files the conversation mentions, and attachments. It excludes `.git`, `node_modules`, `target`, etc. Its stated reason: "Tracked files reach the cloud through Git" (CR-020).
3. Core stages files (sha256 + size checked) and, on activate, writes **one fsynced `teleport` record** that holds the receipt, the queued prompts and the native-ownership decision. It refuses if that native conversation already belongs to another cloud session (CR-019).
4. The resumed harness first receives a `[Teleport handoff]` prompt: "Never repeat an uncertain tool action blindly", "use Git to match the local branch and commit in git-state.txt, which also lists uncommitted tracked edits that stayed on the laptop", and the children's worktrees "stayed on the laptop" (CR-021).

So conversation state moves exactly (native file), while **working-tree state moves only partially**. For a new project the copy includes uncommitted edits and `.env`. For an existing project, uncommitted tracked edits are only *listed*, and reconciling them is left to the agent (CR-021). There is no live process migration.

## 13. Git integration and change review

`cloudroom git` provides read-only status, files, patch and diff JSON for the app's diff view (`GitDiffCard`, `@pierre/diffs`). It runs with hooks, textconv and lazy fetch disabled. Its only write is `commit` (CR-033). RepoSync auto-pushes (CR-031). A Git config hook strips AI `Co-authored-by` trailers from commits in session repos (CR-040). The GitHub plugin lists issues and PRs and can "Send agent"/"Review with agent". Not found in source: a PR-creation flow in Core or the GitHub plugin. Agents would use `gh` themselves (CR-034, Inferred).

## 14. Integration and verification workflows

**Not found in source:** any step that merges multiple agents' branches, runs tests or builds on a combined result, detects semantic/API conflicts, or verifies an agent's completion claim. Specifically:
- Child results are free text, not verified (CR-026). The Workflows docs suggest *prompt-level* "adversarial verify" patterns, but these are author conventions and not enforced (CR-027).
- RepoSync rebases and pushes without running tests. On conflict it runs `rebase --abort` and asks the agent to rebase manually (CR-031, CR-032).
- Teleport offloads integration of children's laptop worktrees to the agent ("read or push it from the laptop... don't redo it") (CR-021).

## 15. Permissions and security

- All cloud harnesses run unattended with full access. Pi extension dialogs are auto-cancelled and never auto-approved (CR-008, CR-009, CR-050). The setup guide says to use a VM for **one trusted user** and never give the agent account sudo or Docker (CR-039).
- Boundaries: a separate `cloudroom-agent` UID, a protected `cloudroom` service account, and cgroups. In container mode the container is the boundary and agents may have sudo (CR-035).
- Command Guard: regex rules for `rm -rf ~`, `mkfs`, `dd of=/dev/...`, `gh repo delete` and fork bombs. A guard error **allows** the command (CR-037).
- **Secret-handling concerns observed:** (a) the Claude subscription token is passed to the harness as `CLAUDE_CODE_OAUTH_TOKEN`. Tool subprocesses most likely inherit it, so any agent command can read it. This is Inferred because Claude Code's env propagation was not verified (CR-038). (b) Mac logins for Pi/Kimi/OpenClaw are copied into sandboxes, and Kimi refresh-token rotation can sign the devices out of each other (CR-042). (c) RepoSync's secret filter matches file **names** only ("no content scan") (CR-031). (d) `mac run` lets any thread read any other thread's output on the account (#103) (CR-043). (e) User Anthropic keys are hidden from Claude itself but re-exported into its commands (CR-038).

## 16. Data persistence and event handling

- Core: per-session journal directories with one record per file, folded into gzip segments. `Record{sequence, session_id, kind, timestamp_ms, data, native}` keeps raw native frames. `Receipt` holds commands. `SessionState` has 16 named states plus `Other` (CR-012). History is uploaded as gzip JSONL to the website via presigned links, and records stay local without `CLOUDROOM_API_URL` (CR-053).
- GUI: SQLite via drizzle. The `events` table holds (`thread_id`, `sequence` unique, `type`, `item_kind`, `turn_id`, `provider_thread_id`, `data` JSON) plus `cloudroom_threads` and `cloudroom_commands` for cloud bindings and teleport state (CR-051).

## 17. Known engineering limitations

See "Gaps and limitations" below. Highlights: no integration/verification layer; same-folder parallelism; partial working-tree teleport; container-mode containment escapes; admission control missing; recovery dead-ends; the public repo is a publish mirror with no PR-level history (CR-054).

## Special focus: failure-mode coverage

| Failure mode | Handled? | How | Evidence | Confidence |
|---|---|---|---|---|
| Git conflicts | Partially (detect only) | RepoSync: `rebase` fails → `rebase --abort`, result `conflict`, agent told to rebase and resolve; refuses when MERGE_HEAD/rebase/unmerged state exists | CR-031 | Confirmed |
| Cross-worktree integration | No | Local sub-threads get separate worktrees, but nothing merges them. Teleport tells the agent children's worktrees stayed on the laptop | CR-021, CR-056 | Confirmed (absence: Inferred from full-text search) |
| Dependency conflicts | No | Handoff says "Project dependencies are yours to resolve"; worktree rebuilds do not keep deps | CR-021, CR-030 | Confirmed |
| API/interface mismatches between agents' outputs | No | No contract or typecheck step; shared-folder children see each other's edits live but are not coordinated | CR-024, CR-028 | Inferred (absence) |
| Stale branch state | Partially | RepoSync fetches before push and retries once; `foreign`/`refused` guards stop pushing others' commits; if the remote base ref is missing but a local copy exists, the worktree is based on that local copy without a refresh; other base-fetch failures throw (CR-061) | CR-031, CR-061 | Confirmed |
| Test failures | No | RepoSync pushes with `--no-verify` and runs no tests; no CI gate | CR-032 | Confirmed |
| Unverified agent completion claims | No | Child result = last text (≤32 KiB); `Finished` status comes from the harness turn end; "Prompt dispatch is not completion" only concerns dispatch | CR-026, CR-010 | Confirmed |
| Partial task execution | Partially | Durable queue; unfinished dispatched work marked `unknown(_after_restart)` and never resent; Teleport handoff asks the agent to finish only what is unfinished | CR-016, CR-021 | Confirmed |
| Interrupted integrations | Partially | RepoSync writes a marker file and aborts its own half-finished rebase on the next check; never syncs mid-turn | CR-031 | Confirmed |
| Failed repair attempts | Partially | One automatic harness recovery, then `process_lost` (no loop); explicit `resume` after preflight; provider-retry caps at 4 retries; no agent-level repair loop | CR-017, CR-049 | Confirmed |

## Notable design decisions worth studying

1. **Native protocols over PTY or SDK in the cloud** (CR-008, CR-009). Each harness keeps its own agent loop, tools and login, which gives high fidelity and lets native resume work. The cost is that each CLI upgrade can break an adapter, and harness versions have to be pinned and probed.
2. **Receipts with "never resend uncertain work"** (CR-013, CR-016). This gives at-most-once semantics for side effects and pushes recovery decisions to users/agents. The GUI retry plugin weakens it (CR-049).
3. **One fsynced activation record for Teleport** (CR-019). The handoff is atomic and idempotent. The cost is that file arrival is eventual, so the agent must check which files actually arrived.
4. **Disable native subagents and route delegation into first-class child sessions** (CR-057). Child sessions are observable, durable and cross-harness. The cost is the loss of native subagent context sharing, and result passing reduced to text.
5. **Permission bypass plus OS boundary instead of approvals** (CR-039). This enables unattended runs. Safety then depends entirely on the sandbox and the regex guard. It is weak in container mode (CR-035).
6. **Polling disk guard with cgroup freeze** (CR-041). It is pragmatic for small sandboxes, and the docs say outright that it is "not a kernel guarantee".
7. **RepoSync direct-to-main** (CR-031). It optimises for solo-user velocity at the cost of review and verification.

## Gaps and limitations

- No multi-agent integration, merge queue, combined test run or conflict prediction (CR-032, §14).
- Shared-folder parallelism without locking or isolation (shared children and workflow workers) (CR-024, CR-028).
- No admission control for shared-sandbox fan-out; agents see host memory, not the cgroup limit (#65, CR-044).
- Duplicate or confusing child failure reporting after OOM (#66, CR-045). Children vanish when the website forgets links (#149, #159).
- Teleport: partial working-tree transfer (CR-021); a crash loop from missing saved transfer state (#169, CR-046); repeated errors (#143).
- Recovery dead-end when native history is missing (#82, CR-047).
- Container-mode containment can be escaped by clearing the env or using sudo (CR-035).
- Cross-thread read-scope bypass via `mac run` (#103, CR-043). Token exposure risk (CR-038).
- Remote-machine worktrees deleted after 3 days without saving (CR-030).
- The public repo is a squash-published mirror, so the design rationale (most ADRs referenced as "ADR 0xxx") is not available. Only `core/adr/0001` is present (CR-054).

## Unverified items

- The hosted sandbox provider and VM/container technology, plus how sandboxes sleep and wake (website not in repo) (CR-052).
- Whether Claude Code propagates `CLAUDE_CODE_OAUTH_TOKEN` to Bash tool subprocesses (CR-038).
- Whether `gh pr create` or any PR flow is wired in the hosted website (CR-034).
- Website-side child start/relay idempotency after the #61 fix.
- Real-inference behaviour. The docs themselves say their fixtures "are not real-inference evidence" (harnesses.md "Verification boundary").

## Appendix: EVIDENCE TABLE

Base `B` = `https://github.com/davidondrej/cloudroom/blob/21b5c6dfa86276581bd00401f40ddcfd01d09435/`

| ID | Claim (short) | Permalink | Symbol | Confidence |
|---|---|---|---|---|
| CR-001 | Core is one Rust crate `cloudroom` 0.1.116 (axum/tokio/reqwest) | B + `core/Cargo.toml#L1-L20` | `[package]` | Confirmed |
| CR-002 | GUI is a fork of BB; Electron/Hono/SQLite apps | B + `README.md#L38` ; B + `gui/package.json#L1-L10` | — | Confirmed |
| CR-003 | GUI server: SQLite source of truth, HTTP+WS, routes to daemons | B + `gui/docs/system-overview.md#L7` | — | Confirmed |
| CR-004 | Host daemon runs provider processes per machine | B + `gui/docs/system-overview.md#L8` | — | Confirmed |
| CR-005 | Core HTTP routes and constant-time bearer/X-Cloudroom-Token auth | B + `core/src/api/mod.rs#L35-L107` ; B + `core/src/api/mod.rs#L503-L541` | `authenticate`, `same_token` | Confirmed |
| CR-006 | Default listen 127.0.0.1:9840 | B + `core/src/config.rs#L43-L46` | `Config` | Confirmed |
| CR-007 | `Kind` enum of 12 harnesses; private `Adapter` trait; `Event` enum | B + `core/src/runtime/mod.rs#L46-L64` ; B + `core/src/runtime/mod.rs#L306-L342` ; B + `core/src/runtime/mod.rs#L406-L435` | `Kind`, `Event`, `Adapter` | Confirmed |
| CR-008 | Cloud Claude: `-p` stream-json, `--dangerously-skip-permissions`, resume/fork flags | B + `core/src/runtime/claude.rs#L39-L118` | `claude::command` | Confirmed |
| CR-009 | Cloud Codex: `app-server --listen stdio://`; `approvalPolicy:never`, `danger-full-access`; identity/model checks | B + `core/src/runtime/codex.rs#L15-L21` ; B + `core/src/runtime/codex.rs#L99-L136` | `codex::command`, `codex::start` | Confirmed |
| CR-010 | Pi via `--mode rpc`; OpenCode via `acp`; dispatch ≠ completion | B + `core/src/runtime/pi.rs#L73` ; B + `core/src/runtime/opencode.rs#L39` ; B + `core/docs/harnesses.md#L122` | `pi::command`, `opencode::command` | Confirmed |
| CR-011 | Local Claude bridge uses Claude Agent SDK `query()`; Codex bridge spawns app-server; bridge capabilities negotiation | B + `gui/plugins/provider-claude-code/src/bridge/sdk-session.ts#L508` ; B + `gui/plugins/provider-codex/src/bridge/app-server-connection.ts#L123` ; B + `gui/packages/provider-bridge-protocol/src/handshake.ts#L24-L40` | `bridgeCapabilitiesSchema` | Confirmed |
| CR-012 | Append-only fsynced journal; `Record`/`Receipt`/`SessionState` schemas | B + `core/src/session/outbox.rs#L9-L20` ; B + `core/src/session/outbox.rs#L268-L300` ; B + `core/src/session/mod.rs#L44-L87` ; B + `core/src/session/state.rs#L13-L33` | `Journal::append`, `Record`, `Receipt`, `SessionState` | Confirmed |
| CR-013 | Receipt lifecycle and idempotent request IDs; uncertain work never resent | B + `core/docs/api.md#L11` | — | Confirmed |
| CR-014 | Snapshot every 20k records; gzip segments | B + `core/docs/session-lifecycle.md#L58-L59` ; B + `core/src/session/outbox.rs#L22-L27` | `SEGMENT` | Confirmed |
| CR-015 | SSE stream resumable via `Last-Event-ID`; events paging `after=N` | B + `core/docs/api.md#L56-L57` ; B + `core/src/api/mod.rs#L1671-L1682` | `events`, `stream` | Confirmed |
| CR-016 | In-flight turns → `unknown`/`unknown_after_restart`, never resent; resume history checked | B + `core/src/session/mod.rs#L3752-L3771` ; B + `core/docs/session-lifecycle.md#L40` ; B + `core/src/runtime/mod.rs#L1157` | `check_resume_history` | Confirmed |
| CR-017 | One automatic recovery, then `process_lost`; startup concurrency ≤ half CPUs | B + `core/src/session/mod.rs#L3721-L3728` ; B + `core/docs/session-lifecycle.md#L38-L39` | `recovery_attempted` | Confirmed |
| CR-018 | Idle sleep after 30 min | B + `core/src/session/sleep.rs#L5` | `IDLE_SLEEP_MS` | Confirmed |
| CR-019 | Teleport staged, single fsynced activation record; manifest validation; native ownership check | B + `core/src/session/teleport.rs#L1` ; B + `core/src/session/teleport.rs#L149-L217` ; B + `core/src/session/teleport.rs#L862-L881` | `teleport_activate`, `validate` | Confirmed |
| CR-020 | Host capture: native file, git-state.txt, mentioned untracked files; excludes `.git`/`node_modules` | B + `gui/apps/host-daemon/src/command-handlers/teleport.ts#L39-L53` ; B + `gui/apps/host-daemon/src/command-handlers/teleport.ts#L574-L618` | `excluded`, capture | Confirmed |
| CR-021 | Handoff prompt: no blind repeats; uncommitted tracked edits stay on laptop; children's worktrees stay local | B + `core/src/session/teleport.rs#L905-L910` ; B + `gui/apps/server/src/services/cloudroom/teleport.ts#L1072` | `teleport_prompts` | Confirmed |
| CR-022 | GUI stops local thread and records transfer in one SQLite transaction | B + `gui/apps/server/src/services/cloudroom/teleport.ts#L571-L599` ; B + `gui/apps/server/src/services/cloudroom/teleport.ts#L794` | `teleports`, `stopThreadForCurrentState` | Confirmed |
| CR-023 | Native child requests only for Claude/Pi; child created durably with parent's workspace | B + `core/src/session/children.rs#L4-L77` | `Manager::start_child` | Confirmed |
| CR-024 | `--shared-sandbox` children (10 harnesses) run in parent's folder | B + `core/src/session/children.rs#L142-L153` ; B + `core/src/session/children.rs#L690-L782` | `CHILD_HARNESSES`, `spawn_child` | Confirmed |
| CR-025 | Own-sandbox children relayed through the website | B + `core/src/session/relay.rs#L1-L4` ; B + `core/docs/api.md#L27` | relay | Confirmed (website side Unverified) |
| CR-026 | Parent receives child's last text, ≤32,768 chars, or error | B + `core/src/session/children.rs#L106-L130` | `start_child` watcher | Confirmed |
| CR-027 | Workflows: QuickJS script fans out `agent()` workers; cached replay; default 8 concurrent | B + `gui/plugins/workflows/PLUGIN_OVERVIEW.md#L13` ; B + `gui/plugins/workflows/src/settings.ts#L123` | `maxConcurrentAgents` | Confirmed |
| CR-028 | Workflow workers reuse origin environment; cached calls keep side effects | B + `gui/plugins/workflows/src/service.ts#L910-L913` ; B + `gui/plugins/workflows/skills/workflows/references/runs.md#L58-L62` | `threads.spawn({environment:{type:"reuse"}})` | Confirmed |
| CR-029 | Worktree removal kills cwd processes then `git worktree remove --force` | B + `gui/plugins/environment-git-worktree/host/worktree.ts#L916-L984` ; B + `gui/plugins/environment-git-worktree/host.ts#L201-L242` | `removeWorktree` | Confirmed |
| CR-030 | Save-to-branch before removal; remote-machine worktrees removed after 3 days without saving | B + `gui/apps/server/src/services/environments/environment-engine.ts#L791-L817` ; B + `gui/apps/server/src/services/environments/worktree-work.ts#L35` ; B + `gui/docs/worktrees.md#L163-L184` | `saveWorktreeWork` | Confirmed (3-day rule: doc only) |
| CR-031 | RepoSync: auto commit, filename secret filter, rebase, abort on conflict, push to branch, never mid-turn | B + `core/src/workspace/repo_sync.rs#L1-L3` ; B + `core/src/workspace/repo_sync.rs#L34-L92` ; B + `core/src/workspace/repo_sync.rs#L224-L231` ; B + `core/src/workspace/repo_sync.rs#L270-L280` | `SCRIPT`, `RepoSync::check` | Confirmed |
| CR-032 | RepoSync runs no tests; commits/pushes `--no-verify` | B + `core/src/workspace/repo_sync.rs#L78-L90` | `SCRIPT` | Confirmed |
| CR-033 | `cloudroom git`: read-only diffs, single `commit` write; shared Local/Cloud | B + `core/src/git.rs#L1-L8` ; B + `core/src/git.rs#L1458` | `git::cli`, `commit` | Confirmed |
| CR-034 | GitHub plugin lists/sends agents on issues/PRs; no PR-creation flow found | B + `gui/plugins/github/PLUGIN_OVERVIEW.md#L1-L10` | — | Inferred |
| CR-035 | Linux cgroup containment (`cgroup.kill`); container mode signals by env marker, escapable | B + `core/src/runtime/linux.rs#L1-L2` ; B + `core/src/runtime/linux.rs#L339-L342` ; B + `core/docs/storage.md#L44-L49` ; B + `core/install/sandbox-start.sh#L1-L8` | `Workload::terminate` | Confirmed |
| CR-036 | 16 MiB frame bound; 4 s shutdown grace | B + `core/src/runtime/process.rs#L16` ; B + `core/src/runtime/mod.rs#L40` | `MAX_LINE`, `SHUTDOWN_GRACE` | Confirmed |
| CR-037 | Command Guard is regex-based and fails open | B + `core/src/runtime/command-guard.mjs#L3-L24` ; B + `core/docs/harnesses.md#L96-L108` | `checkCommand` | Confirmed |
| CR-038 | Claude OAuth token in harness env; app keys re-exported to commands | B + `core/src/runtime/claude.rs#L172-L200` ; B + `core/src/runtime/linux.rs#L20-L29` | `profile_env`, `hide_app_keys` | Confirmed (inheritance by tools: Inferred) |
| CR-039 | No approval prompts; one trusted user per VM; privileged service ADR | B + `core/docs/setup.md#L7` ; B + `core/docs/setup.md#L198` ; B + `core/adr/0001-accept-privileged-core-service.md#L1-L7` | — | Confirmed |
| CR-040 | Git hook strips AI co-author trailers | B + `core/src/runtime/mod.rs#L440-L473` | `strip_ai_co_authors_env` | Confirmed |
| CR-041 | Disk guard freezes work <2 GB, kills top writer after 30 s | B + `core/docs/storage.md#L8-L12` ; B + `core/src/session/guard.rs#L6-L11` | `RESCUE_AFTER` | Confirmed |
| CR-042 | Mac logins copied to sandboxes (Pi, OpenClaw, Kimi); token rotation conflicts | B + `core/docs/harnesses.md#L70` ; B + `core/docs/harnesses.md#L148` ; B + `core/docs/harnesses.md#L160` | — | Confirmed (doc) |
| CR-043 | `mac run` bypasses per-thread read scope | https://github.com/davidondrej/cloudroom/issues/103 ; B + `core/src/mac/mod.rs#L1` | — | Confirmed (issue) |
| CR-044 | Shared-sandbox fan-out OOM-killed coordinator; no admission control | https://github.com/davidondrej/cloudroom/issues/65 | — | Confirmed (issue) |
| CR-045 | Duplicate failure notices / stale child state after OOM | https://github.com/davidondrej/cloudroom/issues/66 | — | Confirmed (issue) |
| CR-046 | Teleport missing saved transfer → server crash loop | https://github.com/davidondrej/cloudroom/issues/169 | `Teleport.load` | Confirmed (issue) |
| CR-047 | Cloud Codex thread stuck after missing native conversation | https://github.com/davidondrej/cloudroom/issues/82 | — | Confirmed (issue) |
| CR-048 | Child spawn retry could start duplicate (closed) | https://github.com/davidondrej/cloudroom/issues/61 | `spawn` | Confirmed (issue) |
| CR-049 | provider-retry auto-"continue" after restart; ≤4 retries | B + `gui/plugins/provider-retry/PLUGIN_OVERVIEW.md#L1-L14` | — | Confirmed |
| CR-050 | Pi extension dialogs auto-cancelled, never approved | B + `core/docs/harnesses.md#L180` | — | Confirmed (doc) |
| CR-051 | GUI SQLite `events` table schema | B + `gui/packages/db/src/schema.ts#L787-L807` | `events` | Confirmed |
| CR-052 | Hosted sandbox provider lives in the website, not in repo | B + `core/docs/harnesses.md#L164` ; B + `core/docs/setup.md#L76` | — | Unverified |
| CR-053 | History uploaded as gzip JSONL via HTTPS to website | B + `core/src/session/history.rs#L1-L3` | `Api` | Confirmed |
| CR-054 | Public repo = 210 "Publish GUI/core" snapshot commits | B + `core/Cargo.toml#L3` (version); `git log` at SHA | — | Confirmed |
| CR-055 | Teleporting last thread removes local worktree after 2 days | B + `gui/docs/worktrees.md#L184` | — | Confirmed (doc) |
| CR-056 | Local project sub-threads get fresh worktrees by default | B + `gui/plugins/environment-git-worktree/README.md#L3` | — | Confirmed (doc) |
| CR-057 | Cloud Claude disallows native subagents; SubagentStart hook + SDK MCP server for children | B + `core/src/runtime/claude.rs#L97-L104` ; B + `core/src/runtime/claude.rs#L333` ; B + `core/docs/harnesses.md#L44` | `claude::command` | Confirmed |
| CR-058 | Codex start rejects different model or resumed identity | B + `core/src/runtime/codex.rs#L121-L135` | `codex::start` | Confirmed |
| CR-059 | Core module list; only `git`/`fs` build on Windows | B + `core/src/lib.rs#L1-L27` | — | Confirmed |
| CR-060 | Mac↔VM sync keeps displaced originals in `.cloudroom-sync-pending` | B + `core/docs/storage.md#L24-L30` | — | Confirmed (doc) |
| CR-061 | Base-branch fetch: if the remote ref is missing and a local copy exists, use the local copy (no refresh); other fetch failures are rethrown | B + `gui/plugins/environment-git-worktree/host/worktree.ts#L552-L563` ; B + `gui/plugins/environment-git-worktree/host/worktree.ts#L567-L573` | `fetchRemoteBaseBranch` | Confirmed |
