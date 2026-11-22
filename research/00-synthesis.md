# Synthesis — t3code × orca × cloudroom, and what it means for our product

Sources: `01-t3code.md` (635 lines), `02-orca.md` (390), `03-cloudroom.md` (479).
All three cloned read-only at `/tmp/ilvuno-research/`, analysed by three parallel agents on the same
model. Every claim here traces to a section in those reports, which trace to file paths.
Licenses: **t3code MIT, orca MIT, cloudroom Apache-2.0** — permissive, but see §10 for the
obligations and the closed parts nobody can copy.

---

## 0. The five things that matter

1. **None of them is a pure web app, and that is not laziness — it is structural.** Agent CLIs must
   run on a machine that has the repo, a shell, and the user's login. So every one of them ships a
   *runner*: t3 has `apps/server` (owns state, spawns CLI processes) + web/mobile/desktop clients
   (`01` §2, §7); cloudroom has a host daemon on the Mac plus a Rust core in a sandbox (`03` §1);
   orca is an Electron app whose main process *is* the runner, with a web build only as a shell
   (`02` §2, §7). **Our product is therefore: browser UI + a local runner process + optional remote
   runner.** Not "a website that runs Claude".
2. **They converge on ~80% of the architecture, independently.** Event-sourced-ish state, one
   normalized provider event stream, capability-declared adapters, per-thread process, worktree
   isolation, a tool/MCP surface for cross-agent work. That convergence is the real spec — it is
   what three teams discovered by shipping, not what one person guessed.
3. **The hard problem you named — "five threads that see each other's work and talk" — is solved by
   nobody.** All three have *point-to-point, pull-based, tool-mediated* messaging. t3 is furthest
   (≈90 MCP tools, `delegate_task`, `t3_thread_read/search/send`, `ContextTransfer`); orca has a real
   mailbox with `ask`/`reply` and a worker preamble; cloudroom has `tell`/`output`/`log` + turn
   reports. **None has ambient shared context**: no shared memory, no push subscription, no
   peer discovery, no live "agent B sees agent A's output without asking". All three say so
   explicitly (`01` §10.2, `02` §10, `03` §10).
4. **The tax is provider adapters, and it is brutal and permanent.** orca: 45 PTY quirk records + 17
   ACP dialects + 23 hook installers, each with PR numbers in the comments (`02` §4). cloudroom: 14
   local providers *and* 13 cloud harness adapters *and* a parity harness between them (`03` §4,
   §2). t3: 11 drivers, each with a bespoke protocol binding, plus replay fixtures to test them
   (`01` §4). "Support all subscriptions" is not a feature, it is an org-chart commitment.
5. **A coordinator can be an agent, not a scheduler — and all three ended up there.** t3 injects an
   orchestration contract into every session and lets the model delegate; orca *deleted* its
   programmatic coordinator (retired at a migration fence) and the coordinator agent loops on mail;
   cloudroom's workflows DSL is the only programmatic fan-out, and it is a JS sandbox, not a
   scheduler. The scheduler dream loses to "the model is the scheduler, the app is the bus".

---

## 1. The three, side by side

| Axis | t3code | orca | cloudroom |
|---|---|---|---|
| Shape | Web + mobile + desktop clients over a Node/Fastify-ish server (`apps/server`), Effect + SQLite | Electron IDE: main process = server, renderer = React, plus embedded browser, xterm, PTY | Electron desktop + React SPA + local Node server + **Rust core** for cloud sandboxes |
| Agent integration | 11 drivers via **native protocols** (codex app-server, claude-agent-sdk, pi `--mode rpc`, opencode SDK, ACP for grok/kiro/fx/antigravity, ACP registry) (`01` §4.3) | **3 tiers**: PTY heuristics for 45 agents (default), ACP for 17, native SDK for Claude, `codex app-server` for Codex (`02` §4) | **Provider = plugin**: bridge artifact speaking JSON-RPC, 3 first-party CLI drivers + generic ACP bridge declaring 14 agents; Rust core reimplements 13 natively (`03` §4) |
| Coordination model | Orchestrator (pure) → events + projections + receipts + outbox in one txn → effect worker; agents coordinate through ~90 MCP tools (`01` §4, §10.1) | Mailbox in SQLite + **typing a pointer note into the TUI**; coordinator agent wakes on mail; `worker_done` settles a Task (`02` §5) | Parent/child threads; `tell` / `output` / `log` / `fork`; turn reports queued onto parent; tasks tracker resumes worker threads (`03` §10) |
| Shared context | Point-to-point tools + `ContextTransfer`/`ContextHandoff` (fork, provider_handoff, merge_back), lazy: native fork or bounded 240-char-per-item text (`01` §10.1 #7-8) | Mail + files + `worker-read` transcripts + diff comments + AI Vault index; explicitly no blackboard (`02` §10) | Git + `tell` + turn reports + tasks comments; `--shared-sandbox` = same folder (`03` §10) |
| State | SQLite, 55 tables, 137 migrations, **projections are truth**, event log pruned to 1 day (`01` §6) | SQLite `orchestration.db` v50, 20 hand-written migration modules, mailbox/graph/program-status tables (`02` §6) | App: SQLite/Drizzle 40 tables, append-only `events`; Cloud core: JSONL journal + snapshot every 20k + gzip segments, no SQL (`03` §6) |
| Transport | Effect RPC over WS, 247 RPCs, snapshot+cursor streams (`01` §7) | Electron IPC + a local HTTP/WS server; CLI talks to it; remote/mobile via pairing (`02` §7) | Local: HTTP + SSE; Cloud: HTTP API to sandbox + control-plane relay (`03` §7) |
| Extensibility | 33 packages, per-provider packages, skills, MCP, scheduled tasks, `ScheduleWakeup`/webhooks (`01` §9) | `skills/` stubs → `orca skills get` served by the running binary; 23 hook installers; bots as git repos with `SOUL.md`/`MEMORY.md` (`02` §9) | 40 plugin dirs: providers, tasks, workflows, side-chat, snippets, org; nine built-in skills (`03` §9) |
| Licence / size | MIT, ~1.29M prod LOC TS | MIT, ~3.6M LOC | Apache-2.0 (+BB MIT NOTICE), 49k Rust + ~434k TS |
| Forkable? | No | No | No |

---

## 2. Where all three agree (treat these as decided)

Each item below was built independently by ≥2 of them. These are not preferences.

1. **One normalized event stream per turn, adapters translate into it.** 18 variants (t3), 27 delta
   kinds + 13 system events (cloudroom), per-agent dialect mappers (orca ACP). The UI never learns a
   provider name — that is the whole point, and it is what makes swapping agents a config change.
2. **Capabilities are declared, not probed ad hoc.** t3: 11 capability groups + `planSelectionTransition`;
   cloudroom: `providerCapabilitiesSchema` with `supportsFork`, `supportsSessionRewind`,
   `modelCatalogScope`, `permissionModes`; orca: per-agent config records + a support matrix
   (`01` §4.1, `03` §4.5, `02` §4). Missing features **degrade by policy**, never by `if (provider === ...)`.
3. **One thread = one process (except Codex).** t3 states it flatly: every driver except Codex
   refuses to attach two app threads to one session, so N threads ≈ N subprocesses
   (`01` §13). cloudroom: one bridge process per session. orca: one PTY per worker. Budget memory
   and cold-start per thread, not per user.
4. **Worktrees for isolation; shared checkout as opt-in.** orca defaults to one worktree per worker
   (bounded nesting depth 4, enforced server-side); t3 defaults to the project root and only uses a
   worktree when asked; cloudroom gives each cloud child its own branch. All three accept the same
   consequence: isolated agents **cannot see uncommitted work**, and the answer is "commit, then share
   via Git" (`01` §10.1 #1, `02` §10 #1, `03` §10 #2).
5. **The agent's own transcript is the record of truth.** orca's chat mode *reads the CLI's own
   JSONL/SQLite* instead of keeping a parallel copy (`native-chat/transcript-reader.ts`); cloudroom
   keeps a native-identity/history pair and refuses to fabricate a missing one (`03` §4.4); t3 keeps
   provider ids as refs and marks sessions `reusable|broken` (`01` §4.2, §4.4). Do not build "our own
   version of the conversation" as the only copy.
6. **Auth stays on the execution machine, never in the UI.** "Tokens never pass through the GUI or
   core API" (cloudroom, `03` §4.6). t3: the user's existing CLI login, API keys only via secret
   providers or env. orca: never holds model keys, only account *plumbing* (`CLAUDE_CONFIG_DIR`
   isolation, usage scraping). Our runner should own credentials, and the browser should never see them.
7. **Idempotency and an honest `unknown`.** cloudroom: `request_id` + payload hash + caller
   fingerprint, `unknown` as a first-class terminal state, uncertain work never re-sent (`03` §12 #4).
   orca: `mutation_receipts`, `stop_unknown`/`start_unknown`/`abandoned` states, "a live PTY can
   contain a dead agent" (`02` §5). t3: command receipts in the same txn as events+outbox (`01` §12 #1).
   With N agents and M machines, retries are guaranteed; without this you get double dispatches.
8. **The app injects the collaboration contract into the agent's prompt.** t3's
   `orchestrationInstructions.ts` is *literally* the `<t3_code_orchestration>` block I am running under.
   orca's `buildDispatchPreamble()` writes identity, task id, and the exact `check`/`ask`/`worker_done`
   recipes into the worker's first message. cloudroom's skills do the same via `SKILL.md`
   (`01` §10.1 #9, `02` §5, `03` §9). **This is the cheapest mechanism in the entire design space:**
   a prompt is a cross-provider API that needs no SDK.
9. **Bounded, paginated reads of another thread.** t3 `t3_thread_read` (~30k chars/call, cursor
   positions); orca `worker-read --dispatch`; cloudroom `log`. Never dump a whole transcript into a
   prompt — it is the fastest way to blow a context window and a token budget.
10. **Structured results beat prose parsing.** t3 `outputSchema` + `t3_submit_output`; cloudroom
    workflows' JSON-Schema-constrained `agent()` results; orca `worker_done --outcome succeeded|failed`.
    Reported failure can never be recorded as success in all three.
11. **Disk and durability guards are mandatory, not optional.** cloudroom burned a 50 GB disk in days
    (`#606`) and now ships a 32 MiB reserve file plus a published storage policy; t3 prunes its event
    log to 1 day on purpose; orca defers delivery when a recipient cannot take input. Multi-agent
    workloads are a disk/IO workload.

---

## 3. Where they genuinely diverge (and what to copy from whom)

### 3.1 How agents coordinate — three different bets

| | t3 | orca | cloudroom |
|---|---|---|---|
| Primitive | MCP **tool surface** (~90 tools) + injected contract | **Mailbox** + pointer typed into TUI | **Parent/child threads** + `tell` |
| Who decides the plan | the agent, via tools; `delegate_task` makes child thread+run | the coordinator agent, via `worker-start`/`check`/`worker_done` | the user (Tasks/delegate) or a workflow script |
| Deterministic? | no | no (coordinator retired at the fence) | yes, inside the workflow DSL |
| Survives a model change? | mostly — tools are stable | yes — mail is plain text | yes — `tell` is plain text |
| Biggest weakness | requires the agent to know the tool surface | the coordinator's prompt writing is the bottleneck | 1:1 hierarchy, no peer bus |

**Verdict for us:** t3's tool surface *shape* + orca's *delivery mechanics*. Tools are how an agent
expresses intent; typing a pointer into an uninstrumentable TUI is how you make a 45-CLI long tail
interoperate. Neither alone is enough.

### 3.2 Orchestration engine

- t3: **pure `Orchestrator` decide function** → events + projections + receipts + outbox **in one
  SQLite transaction** → an effect worker runs side effects. This is the single best idea in the set:
  it makes "queued vs running vs waiting vs done" honest across 11 providers and 4 clients, and it is
  testable without I/O (`01` §12 #1). **Copy the shape.**
- orca: **no engine.** The coordinator is an agent; the DB holds the state it mutates through CLI calls.
  Cheap, brittle in a different way, and it deleted its own scheduler on purpose (`02` §5).
- cloudroom: **workflows DSL** (JS in a sandbox, no clock/fs/net, `agent()` calls, cached on resume)
  for fan-out, plus human-driven tasks (`03` §10 #10).

**Verdict:** t3's transactional command→event→effect core, with cloudroom's workflow script as the
*optional* deterministic escape hatch. Do not start with a scheduler.

### 3.3 Provider abstraction

- t3: `ProviderAdapterV2` — a 20-method `SessionRuntime` + 18-variant event union + 11 capability
  groups. Most complete contract, and the one you can read to learn the failure modes (`01` §4.1-4.2).
- cloudroom: **provider = plugin, zero first-party privilege** — Claude/Codex/Pi use the same public
  bridge API as third parties, so the abstraction cannot rot (`03` §4.1, and the delegate's surprise
  note). Strictest anti-special-casing stance of the three.
- orca: **honest hackery** — a config *record of quirks* per TUI, 45 entries with PR numbers. The
  abstraction is a bug museum, and that is the correct design for the long tail (`02` §4).

**Verdict:** adopt cloudroom's plugin discipline (no first-party privilege), t3's event union and
capability groups as the vocabulary, orca's quirk-record schema for the PTY tier.

### 3.4 Transport and clients

- t3 is the only one with a real **web + mobile + desktop** client story (snapshot + cursor streaming,
  `ThreadStream`, resumable WS, 247 RPCs). If our product is "mostly web", this is the only relevant
  reference (`01` §7-8).
- cloudroom proves a **browser can attach to a local machine** through a Cloudflare-Worker tunnel with
  gap-hold (`03` §12 #12) — small, self-hostable, correct constants.
- orca optimises for *terminal fidelity* (xterm v6 beta, ghostty VT), which a web product should not
  copy in v1.

---

## 4. The provider tax, quantified — this decides our scope

| Approach | Who | Cost to add an agent | Fidelity | Works when CLI has no API |
|---|---|---|---|---|
| Native protocol/SDK | t3 (11), orca (Claude, Codex), cloudroom core (13) | days–weeks, per provider version | best (fork, rewind, subagents, approvals) | no |
| ACP (Agent Client Protocol) | cloudroom (14 via one bridge), orca (17 dialects) | hours if the agent speaks ACP; a dialect module if it deviates | good chat, weaker approvals | only ACP-speaking agents |
| PTY + typing | orca (45 configs, the default) | hours to first success, **weeks to reliable**, forever | chat-like, no structure | yes, everything |

Hard numbers to keep in mind: **45** PTY quirk records, **17** ACP dialects, **23** hook installers,
**11** native drivers, **14+13** cloudroom providers, **2** runtimes in cloudroom with a parity
harness between them.

Consequences for our plan:

- Building "support every subscription" as a first-class promise **cannot** be v1. Pick the tier
  order: **ACP first, native SDK second, PTY last resort** — and make the unsupported state visible
  instead of pretending.
- The PTY tier is where the long tail actually lives, and it is the reason orca's model list looks
  impressive. It is also why its bug surface is enormous. Treat PTY as an opt-in "beta agents" tier.
- Two runtimes (local + cloud) is a *permanent 2× implementation tax* and cloudroom pays for it with a
  parity harness. **Do not build a cloud runner until a local runner is boring.**

---

## 5. The gap map for "five threads in one project that see each other"

What the reference set actually gives you, in ascending usefulness:

| Capability | t3 | orca | cloudroom | Verdict |
|---|---|---|---|---|
| Shared filesystem by default | ✅ project root | opt-in `--worktree current` | opt-in `--shared-sandbox` | copy t3 (default shared, isolate on request) |
| Read another thread's transcript | ✅ `t3_thread_read` (bounded, paginated) | ✅ `worker-read --dispatch` | ✅ `output` / `log` | universal; copy t3's bounding |
| Search other threads | ✅ FTS over message text | ✗ | ✗ | copy t3; cheap and pays off |
| Send a message to a thread | ✅ `t3_thread_send` (auto/queue/steer/restart) | ✅ mailbox + `ask`/`reply` | ✅ `tell` | needed; copy t3's modes + orca's delivery-hold |
| Wake an idle/terminal agent | ✅ queue a new turn | ✅ type a pointer note into the TUI | ✅ turn report notice on parent | **orca's is the one that generalises to uninstrumentable CLIs** |
| Child sees parent's later edits | ✗ (commit only) | ✗ | ✗ (stated in SKILL.md) | unsolved; our differentiator lives here |
| Ambient shared context / blackboard | ✗ | ✗ | ✗ | **nothing** |
| Push subscription ("tell me when B finishes") | ✗ (parent is woken, not subscribed) | ✓ mailbox delivery + wake | ✓ turn reports | orca/cloudroom shape |
| Peer discovery ("who else is on this project") | ✗ | ✗ (must be told a handle) | ✗ | nothing |
| Conflict detection between agents | ✗ (merge_back is context, not code) | ✗ (no locks) | ✗ (worktrees are the answer) | nothing |
| User-authored cross-thread context (@-mention) | ✓ (`t3-context://thread/<id>` + envelope) | ✗ | ✗ (mention is a text label) | copy t3's envelope format |
| Handoff to a different provider | ✓ `ContextTransfer` (fork/provider_handoff/merge_back, lazy, auditable) | ✓ AI Vault continuation prompt | ✓ `thread fork --provider` | **the most reusable primitive in the whole set** |

### So what is genuinely new in our product?

Not "many agents in one UI" — three products do that. Not "threads that talk" — two do that. The
increment is the combination the user described, and it decomposes into three primitives plus one
missing layer:

1. **Project-scoped shared context that is default-on and agent-readable without being asked.** A
   durable, human-inspectable per-project object — decisions, constraints, changed files, open
   questions, "who is doing what" — that every thread's turn preamble includes a *digest* of, and that
   any thread can append to. All three repos instead rely on the coordinator remembering to write prose.
   That is the failure mode to remove (`02` §13 last bullet).
2. **Turn-boundary push, not polling.** A thread finishing should *notify* sibling/parent turns with a
   compact machine-readable delta (`{threadId, status, filesChanged, summary, verdict}`), not a
   transcript dump. t3 has `outputSchema`, orca has mailbox delivery, cloudroom has turn reports —
   combine them.
3. **Review loops as first-class objects.** "Claude writes, Codex reviews, Pi documents" is a *pipeline
   with a verdict*, not three chats. Nothing in the set models a round (produce → critique → revise →
   accept) as data; it all lives in prompts and mail. `DiffComment` (orca) and `worker_done --outcome`
   are the closest primitives.

Runtime cost reality check: five concurrent threads = five CLI subprocesses with their own node/python
runtimes, typically 300 MB–1.5 GB RAM each depending on agent (cloudroom measured OpenClaw at 1.3 GB
and ~15 s cold start). "Five threads per project" is a *machine* requirement, not a UI feature. Design
for a 16 GB laptop running 3–5 threads, and make "park a thread" (freeze process, keep history)
a first-class state — cloudroom's 17-state session machine is the reference (`03` §12 #6).

---

## 6. What to borrow, concretely

Ranked by value per line we would otherwise write. None of this is "fork the repo" — it is
"read 3 files, copy the shape".

| # | Borrow | From | Why |
|---|---|---|---|
| 1 | Transactional command → events + projections + receipts + outbox → effect worker | t3 `orchestration-v2/{Orchestrator,EventSink,EffectWorker}` | makes run state honest; testable without I/O |
| 2 | `ContextTransfer` / `ContextHandoff` (lazy native-fork-or-text, auditable rows) | t3 `ContextHandoffService` + its doc | the primitive for "B sees A" without transcript dumping |
| 3 | The agent-facing tool surface + its *descriptions* | t3 `apps/server/src/mcp/toolkits/*` | tuned text; encodes real failure modes |
| 4 | Provider-as-plugin, zero first-party privilege, declarative capability manifest | cloudroom `provider-plugin-api.md`, `provider-types.ts` | keeps the abstraction honest as it grows |
| 5 | `thread/delta` → `ThreadEvent` → timeline projection split | cloudroom `provider-bridge-protocol` | the chat UI never learns a provider name |
| 6 | Mailbox schema + message taxonomy + delivery-hold + pointer-note wake | orca `create-core-tables-sql.ts:23-60`, `mailbox-notification-coordinator.ts` | the only mechanism that works for TUIs we cannot instrument |
| 7 | The prompt-injected worker preamble (identity, task id, exact commands, finish-once) | orca `worker-dispatch-preamble.ts:84` | cross-provider API with zero SDK work |
| 8 | Session state machine with park/resume (`sleeping`/`suspended`/`process_lost`/`history-only`) | cloudroom `session/state.rs`, `session-lifecycle.md` | makes many long-lived threads affordable |
| 9 | History as journal + snapshot + segment compaction (and a disk reserve file) | cloudroom `storage.md` | the biggest, least predictable dataset we will own |
| 10 | Per-model reasoning-level validation with fail-don't-downgrade | cloudroom `domain/shared-types.ts` | prevents silent quality loss |
| 11 | Mutation idempotency: `request_id` + payload hash + first-class `unknown` | cloudroom `api.md` Conventions | retries are inevitable with N agents |
| 12 | Task tracker as the human/agent shared surface (seed prompt + `steer-if-active`) | cloudroom `plugins/tasks` | the only cross-agent surface humans actually read |
| 13 | Version-matched agent instructions served by the running binary (`skills get`) | orca `skills/` stubs | stops agent docs from drifting away from code |
| 14 | Bounded paginated cross-thread read + FTS search | t3 `toolkits/thread/tools.ts` | cheap, high value, prevents context blowups |
| 15 | Replay-fixture testing per provider | t3 `scripts/record-*-replay-fixture.ts` | the only way to test N protocols without N CI logins |
| 16 | Tunnel-with-gap-hold (browser → user's machine) | cloudroom `connect/` | if we ever want "open my local project from a browser" |

---

## 7. What nobody solved (our actual roadmap)

1. **Ambient, default-on shared context.** Every repo's answer is "the coordinator should write it
   down". Product answer: a project context ledger (files, decisions, constraints, open questions,
   ownership) that (a) every turn's preamble digests, (b) any thread appends to with one tool call,
   (c) the user can read and edit. Plain files + GitHub-ish conventions, not a vector DB — that is
   what all three converged on for good reasons: survives context loss, model-agnostic, auditable,
   greppable.
2. **Turn completion as a structured push.** `{thread, status, verdict, filesChanged, summary}` on every
   turn boundary, delivered to interested threads as a wake + a digest, not a transcript.
3. **Round-based collaboration as data.** Model `round(n): propose → critique → revise → accept` with a
   verdict and an artifact pointer, so "Codex reviews Claude's work" is a state machine, not a chat.
4. **Peer discovery inside a project.** "Who else is working on this repo and on what file" — needs a
   registry, which no repo has.
5. **Conflict signal between agents.** Not merge arbitration (hard), just *awareness*: "B is editing
   `src/x.ts`" as a warning in A's context. orca explicitly has no locks or ownership signal.

Whether all five are v1 is a decision — see §9.

---

## 8. Hard truths about the reference set

- **Forking any of them is not viable.** 1.29M / 3.6M / 434k+49k LOC; Effect-as-hard-toolchain (t3),
  Electron 43 + native `node-pty` + xterm beta (orca), two runtimes + a closed web control plane
  (cloudroom). T3's own report says it plainly: "Copying patterns is realistic; forking the repo is a
  career move."
- **Closed parts you cannot build on:** t3's `app.t3.codes` + relay + App Store builds; orca's
  connector service / artifact sharing / mobile release pipeline; cloudroom's entire `web/` control
  plane (accounts, sandbox provisioning, Cloud Credits, `/child`, `/relay`, loop cron) — referenced by
  name in code and *absent from the repo* (`03` §13). Any of their "cloud" features are not a
  blueprint, they are a product.
- **Documentation drifts; skills don't.** orca's `skill-guides/` are more accurate than `docs/`
  (delegate's words); t3 says pin a commit (`7ddc81d6`) because these repos move fast.
- **Two agents on the same account can sign each other out.** cloudroom documented refresh-token
  rotation conflicts for Kimi and Codex (`03` §13). If our product runs five agents, we own that class
  of bug.
- **"All subscriptions" includes plans that forbid it.** cloudroom refuses Claude plans for one
  harness on principle (`openclaw` ADR 0265). Expect terms-of-service risk that is a product decision,
  not an engineering one.

---

## 9. Scope proposal for our v1 (a position, to argue with)

Deliberately small, and each item is justified by something above.

**In scope**
- Browser UI + **one** local runner (Node, spawns CLIs, owns SQLite, streams over WS). No cloud runner.
- One project → N threads. Thread = one agent session on one provider instance.
- Providers: **ACP tier** (whatever speaks it) + **native** for Claude and Codex + **PTY** for at most
  2 more, gated as "beta". Nothing else. Say no loudly and visibly.
- Cross-thread: shared project root by default; `read` (bounded), `search`, `send` (queue/steer),
  `delegate` (child thread, task prompt only), `transfer` (fork/handoff/merge-back of *context*).
- The four new primitives from §7: context ledger, turn-boundary push, review rounds, peer registry.
  Conflict awareness: warn, don't arbitrate.
- Transactional run state (command → events/projections/outbox) from day one — retrofitting honesty is
  the expensive kind of rewrite.
- Park/resume a thread; disk reserve; idempotent mutations with an honest `unknown`.

**Out of scope for v1**
- Cloud sandboxes, mobile app, embedded browser, computer use, org sharing, marketplace, multi-machine
  federation, programmatic scheduler/DAG engine, more than ~5 providers.

**Open decisions (the next conversation)**
1. **Runner model:** local-only first (t3/cloudroom-local shape) or local + remote-ssh from day one?
   Everything else follows from this.
2. **Provider tier order:** ACP-first (cloudroom) or native-first for Claude+Codex (t3)? Affects
   fidelity of approvals/fork/rewind.
3. **Is the "connected threads" story a coordinator agent (orca), a tool surface (t3), or a
   user-driven board with agent access (cloudroom tasks)?** All three work; they produce different UIs.
4. **Do we model code isolation per thread (worktrees, t3/orca) or share one checkout (t3 default)?**
   Shared is 10× simpler and fine for write/review/doc splits; worktrees are needed for two writers.
5. **Do we build the ledger as files in the repo (greppable, committed) or app state (clean, hidden)?**
   Files survive everything; app state is prettier.
6. **Web-first means we need auth + a deployment story.** Who owns the machine that runs the CLIs —
   the user's laptop (runner install) or ours (cloud)? This is the business model, not a detail.

---

## 10. Licensing ledger

| Repo | Licence | Copy files? | Must carry | Notes |
|---|---|---|---|---|
| t3code | MIT | yes | copyright + licence text | vendored deps are not all MIT — `third-party-licenses.config.json`; hosted services outside the grant |
| orca | MIT | yes | copyright + licence text | `cloud/` covered; hosted connector/artifact services closed |
| cloudroom | Apache-2.0 (+ MIT NOTICE for BB) | yes | `LICENSE` **and** `NOTICE`, mark modified files | patent grant included; `gui/` is a fork of BB with stale `@bb/*` references; marketplace metadata not ours |

Practical stance: **read for design, copy ≤200-line shapes with attribution, never copy a subsystem.**
Apache-2.0's NOTICE-retention and "state changes" obligations are the only ones that need process, and
they only bind if we copy actual files.

---

## 11. One-paragraph answer to "are they similar?"

Yes — architecturally they are three solutions to the same problem and they agree on the skeleton
(project → thread → provider adapter → normalized event stream → transactional state → client over
WS/SSE), on the isolation model (worktree per agent, shared checkout as opt-in), on credentials staying
on the runner, and on the coordination mechanism (tools + mail + injected prompt contract, with the
model as the scheduler). They diverge on **how much structure they demand from the agent** — t3 a rich
tool API, orca a mailbox typed into a terminal, cloudroom a plugin bridge plus tasks — which is really a
diverge on *which agents you can support at all*. The genuinely unsolved part, and therefore our
product's only defensible core, is **shared context between threads that is default-on, structured, and
readable by both humans and agents** — followed by **review rounds modeled as data** rather than as
chat.
