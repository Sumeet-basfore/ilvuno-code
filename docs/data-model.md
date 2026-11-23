# Data model

SQLite, one file per machine at `~/.local/share/ilvuno/ilvuno.db`. Access through `rusqlite` on a single
writer task; reads on the UI thread pool. No ORM. Migrations are numbered SQL files, applied at startup,
recorded in `schema_migrations`.

Why SQLite and not a journal file: cloudroom's JSONL + snapshot works because its core owns one sandbox.
We need queries (find every round where this file was touched, search the ledger) on day one. The event
journal *is* a table; it just happens to also be append-only.

Use `PRAGMA journal_mode=WAL`, `foreign_keys=ON`, `synchronous=NORMAL`.

---

## Entities

### `goals`

| column | type | notes |
|---|---|---|
| `id` | TEXT PK | uuid |
| `created_at` | INTEGER | unix ms |
| `title` | TEXT | first line of the goal |
| `goal_text` | TEXT | the full statement |
| `done_spec` | TEXT | JSON: `{done, checks}`; `done` is free text or null, `checks` as written in the goal file |
| `message_policy` | TEXT | `anytime` \| `round` \| `never`, default `round` |
| `budget_rounds` | INTEGER | default 3 (research) / 5 (feature) |
| `budget_tokens` | INTEGER | default 150000 |
| `budget_usd` | REAL | nullable; CLI-level cap where supported |
| `providers` | TEXT | JSON: agent id → `{driver, command, model, mode}`; `mode` is passed to the adapter (`INITIAL_AGENT_MODE` for codex-acp) |

### `runs`

| column | type | notes |
|---|---|---|
| `id` | TEXT PK | uuid |
| `goal_id` | TEXT FK | |
| `status` | TEXT | `planned` … `merged`, or a terminal failure |
| `started_at`, `ended_at` | INTEGER | |
| `rounds_used` | INTEGER | |
| `tokens_used` | INTEGER | summed from provider usage events |
| `blocked_reason` | TEXT | nullable |
| `accepted_by`, `merged_by` | TEXT | `human` \| `agent:<thread>` |

### `threads`

| column | type | notes |
|---|---|---|
| `id` | TEXT PK | uuid |
| `run_id` | TEXT FK | |
| `name` | TEXT | how a task names it, e.g. `reader-1`; unique per run. The name `orchestrator` marks the planning thread |
| `prompt` | TEXT | the thread's standing job, sent with every task on it |
| `role` | TEXT | `orchestrator` \| `worker` (what the policy allows the thread) |
| `provider` | TEXT | agent id (a key of `goals.providers`), e.g. `claude-main`, `codex-main` |
| `native_session_id` | TEXT | provider's own id; nullable until created |
| `branch` | TEXT | `run/<run>/<slug>`; null for roles without a branch |
| `worktree_path` | TEXT | null for roles without a worktree |
| `status` | TEXT | `idle` \| `working` \| `waiting` \| `parked` \| `dead` |
| `created_at`, `last_active_at` | INTEGER | |

### `rounds`

| column | type | notes |
|---|---|---|
| `id` | TEXT PK | uuid |
| `run_id` | TEXT FK | |
| `index` | INTEGER | 0-based, unique per run |
| `plan` | TEXT | JSON: dispatches the orchestrator asked for |
| `status` | TEXT | `planned` … `merged` |
| `verdict` | TEXT | `pass` \| `pass_with_notes` \| `block` \| null |
| `tokens_delta`, `cost_delta` | INTEGER, REAL | |
| `readiness` | TEXT | JSON, assembled before the gate |
| `started_at`, `ended_at` | INTEGER | |

### `dispatch`

One task inside a round: a prompt, the thread it runs on, and the scope it may touch.

| column | type | notes |
|---|---|---|
| `id` | TEXT PK | uuid |
| `round_id` | TEXT FK | |
| `thread_id` | TEXT FK | |
| `task` | TEXT | the prompt the task was written with |
| `scope` | TEXT | JSON array of paths or sources it may touch |
| `created_by` | TEXT | `user` \| `orchestrator` \| `runner` (the runner, when a failed round wakes the orchestrator) |
| `status` | TEXT | `queued` \| `running` \| `reported` \| `failed` \| `cancelled` |
| `result_summary` | TEXT | worker's structured result, short |
| `result_json` | TEXT | validated output if the dispatch requested a schema |
| `request_id` | TEXT UNIQUE | idempotency key; a retry with the same id is deduplicated |

### `ledger`

Append-only. The board the humans read and the agents query.

| column | type | notes |
|---|---|---|
| `id` | INTEGER PK AUTOINCREMENT | monotonic; ordering matters |
| `run_id` | TEXT FK | |
| `round_id` | TEXT FK | nullable for run-level entries |
| `thread_id` | TEXT | author; null when written by the runner |
| `kind` | TEXT | `intent` \| `finding` \| `decision` \| `constraint` \| `question` \| `answer` |
| `scope` | TEXT | JSON: paths, URLs, thread ids this entry is about |
| `body` | TEXT | one or two sentences; longer belongs in an artifact |
| `evidence` | TEXT | JSON: path, url, artifact id, or message id |
| `created_at` | INTEGER | |

Indexes: `(run_id, id)`, `(kind)`, and an FTS5 virtual table over `body` for search.

### `messages`

Agent-to-agent and runner-to-agent delivery. Separate from `ledger` because delivery has state.

| column | type | notes |
|---|---|---|
| `id` | TEXT PK | uuid |
| `run_id`, `round_id` | TEXT | |
| `from_thread`, `to_thread` | TEXT | `to_thread` null = broadcast (only if policy allows) |
| `body` | TEXT | |
| `delivery` | TEXT | `queued` \| `delivered` \| `read` \| `unknown` \| `failed` |
| `created_at`, `delivered_at` | INTEGER | |

`unknown` is a real state (cloudroom's lesson): a message whose delivery we cannot confirm is never
silently re-sent, and it is visible in the UI.

### `artifacts`

Files a run produces: reports, diffs, notes.

| column | type | notes |
|---|---|---|
| `id` | TEXT PK | uuid |
| `run_id`, `round_id`, `thread_id` | TEXT | |
| `kind` | TEXT | `report` \| `note` \| `diff` \| `export` |
| `path` | TEXT | absolute path on disk |
| `mime`, `bytes`, `sha256` | TEXT, INTEGER, TEXT | |
| `created_at` | INTEGER | |

Artifacts live on disk under `<run>/artifacts/`; the DB holds pointers and hashes so a missing file is
detectable rather than mysterious.

### `journal`

Append-only event log. Source of truth for replay; projections are caches.

| column | type | notes |
|---|---|---|
| `seq` | INTEGER PK AUTOINCREMENT | global order |
| `run_id` | TEXT | indexed |
| `round_id`, `thread_id` | TEXT | nullable |
| `at` | INTEGER | unix ms |
| `kind` | TEXT | `run.created`, `round.planned`, `dispatch.queued`, `dispatch.reported`, `ledger.appended`, `gate.opened`, `gate.decided`, `merge.requested`, `merge.done`, `run.blocked`, `thread.spawned`, `thread.parked` … |
| `payload` | TEXT | JSON; the exact data the transition carried |

Two rules:

1. **A command writes its journal row and its projection update in one transaction**, with the command
   receipt. A crash between them is impossible; a crash after them is replayable.
2. **Retention:** journal is kept for 30 days, then archived to `<run>/journal.ndjson.gz`. Unlike t3 (one
   day, not replayable), we keep enough to replay a run for review. Disk cost is small at our scale,
   but the guard is the same: a reserve file and a hard stop before the disk fills.

### `provider_instances`

| column | type | notes |
|---|---|---|
| `id` | TEXT PK | user-facing: `claude-main` |
| `driver` | TEXT | `claude` \| `codex` \| `acp` |
| `command` | TEXT | binary or adapter command; null for native |
| `display_name` | TEXT | |
| `capabilities` | TEXT | JSON cache of the last probed capabilities |
| `last_seen_at` | INTEGER | |

Credentials are **not** here. They are either the CLI's own login on disk, or an entry in the OS secret
service, referenced by name. See `docs/security.md`.

### `settings`

Single row, key/value. Defaults for budget, message policy, gate behaviour, theme, telemetry (off).

---

## Derived views, not tables

- **Readiness report** for a round: computed on demand from `rounds`, `dispatch`, `provider_instances`
  and the last test/check results. Stored as JSON on `rounds.readiness` only at gate time, so the report
  you approved is reproducible even if the world changed afterwards.
- **Board view**: `ledger` joined to threads, ordered by `id`.
- **Run list**: `runs` + `goals` + a count of open questions.

## Invariants the runner enforces in code

1. No `round` starts while the previous round has no verdict.
2. No `dispatch` without a ledger `intent` naming its scope.
3. No `merge` action unless `rounds.verdict ∈ {pass, pass_with_notes}` **and** `runs.status = accepted`.
4. No write to a worktree other than the thread's own.
5. `same request_id` twice ⇒ one dispatch, one receipt.
