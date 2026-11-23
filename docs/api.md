# Local API (`ilvunod`)

The door the desktop app uses to start, watch and decide runs. It is a thin HTTP layer. Every action calls
the same function the `ilvuno` CLI calls, and the run data has the same shape as an export's `run.json`.

Run it:

```bash
RUSTUP_TOOLCHAIN=1.100.0 cargo run -p api --bin ilvunod
# http://127.0.0.1:4737   ILVUNO_PORT=… changes the port, ILVUNO_HOME the data folder
```

## Safety in v1

- **Loopback only.** It listens on `127.0.0.1`. It is not reachable from another machine.
- **No authentication.** Any program running as your user can start runs and send tasks. Do not put it
  behind a proxy or a port forward.
- Agents run under the same rules as the CLI. The API adds no permission.
- **Only the window's origins may read it.** CORS allows `tauri://localhost` (macOS, Linux) and
  `http://tauri.localhost` (Windows). A page from any other origin gets no CORS header, so the browser
  blocks it from reading runs, goals or agent messages.
- **Every write (POST or PUT) must be `application/json`.** A cross-site form can POST `text/plain`,
  so requiring JSON forces a preflight — and the origin check refuses it. Anything else is a 415.
- **Still unauthenticated for local programs.** Any process running as your user can call the API. Do not
  put it behind a proxy or a port forward.

Before any of these changes, add authentication or drop the HTTP door entirely: a second user on the
machine, a port reachable from elsewhere, or any client that is not the window.

## Endpoints

| Method | Path | What it does | Success | Runner function |
|---|---|---|---|---|
| GET | `/health` | Is the process up | `200 {"ok":true,"version":"0.0.1"}` | — |
| GET | `/runs` | Run summaries, newest first | `200 [{id,status,title,project,rounds_used,tokens_used,started_at}]` | `query::list_runs` |
| GET | `/runs/:id` | One run, as `run.json` in an export | `200` export shape | `export::run_json` |
| GET | `/runs/:id/board` | The board a thread reads | `200 {"markdown":"…"}` | `ledger::render_board` |
| GET | `/runs/:id/events` | Live journal, server-sent events | `200 text/event-stream` | `query::journal_after` |
| POST | `/runs` | Start a run from goal JSON, drive it in the background | `202 {"run_id","status":"working"}` | `cli::ops::create_run`, `drive_with` |
| POST | `/runs/:id/tasks` | Add a task: `{"thread","prompt","scope":[…]}` | `201 {"task_id"}` | `tasks::submit_task` |
| POST | `/runs/:id/gate` | `{"decision":"accept"}` or `{"decision":"send_back","note":"…"}` | `200 {"run_id","status"}` | `run_status::decide_gate_and_render` |
| POST | `/runs/:id/resume` | Continue a run, as `ilvuno run --resume` does | `202 {"run_id","status"}` | `goal::load_snapshot`, `drive_with` |
| GET | `/agents` | Which agent CLIs are on this machine, and their versions | `200 [{id,name,path,found,version,safety}]` | — |
| GET | `/settings` | The window's settings, defaults filled in | `200 {budget_rounds,budget_tokens,message_policy,theme,human_gate}` | — |
| PUT | `/settings` | Write the keys the body names | `200` the full object after the write | — |

`POST /runs` takes the same keys as `examples/board-experiment.json`, plus one optional key:

- `"base": "/abs/path/to/project"` — the folder relative paths resolve from. A goal file on disk uses its
  own folder; a goal sent over HTTP has none, so the app must send `base`. Without it, relative paths
  resolve from the run folder and `../` sources will not be found. A `base` that is not a folder is a 400.

### Events

Each journal row appended while you are connected sends one event:

```
event: journal
id: 6
data: {"seq":6,"kind":"dispatch.queued","at":1795417142551}

```

- `id` is the journal `seq`. A client that reconnects sends it back as `Last-Event-ID`, and the stream
  resumes from the row after it.
- Without `Last-Event-ID`, the stream starts from the newest row at connect time.
- An empty comment line (a single `:`) is sent every 15 seconds so the connection stays open.

### Agents

`GET /agents` looks for six CLIs on the `PATH` of the `ilvunod` process: `claude`, `codex`, `pi`,
`opencode`, `agy`, `cursor-agent`.

The answer is one object per agent, always in the same order:

```json
[{ "id": "claude", "name": "Claude Code", "path": "/home/u/.local/bin/claude", "found": true,
   "version": "2.1.329", "safety": "asks_before_each_tool" },
 { "id": "codex", "name": "Codex", "path": null, "found": false, "version": null,
   "safety": "works_in_folder_asks_outside" }]
```

- `found` is whether the binary is on the `PATH`. When it is not, `path` and `version` are `null`.
- `version` is the first non-empty line of `<bin> --version`. The call gets **3 seconds**; one that does
  not answer in time leaves `version` as `null` with `found` still `true`. We do not invent a version.
- `safety` is the CLI's **own** permission behaviour, one fixed value per agent:
  - `asks_before_each_tool` — Claude Code.
  - `works_in_folder_asks_outside` — Codex and every ACP agent (Pi, opencode, Antigravity, Cursor Agent).
  - `unknown` — anything else.

**Ilvuno does not sandbox agents; this describes the CLI's own permissions.**

Detection spawns up to six processes, so the answer is **cached for 30 seconds** per `ilvunod` process.
Start an agent after it is installed and it shows up within half a minute.

### Settings

`GET /settings` returns all five keys, with the defaults filled in for anything never written:

```json
{ "budget_rounds": 3, "budget_tokens": 150000,
  "message_policy": "round", "theme": "dark", "human_gate": true }
```

`PUT /settings` takes a body with **any subset** of those keys. Only the keys in the body are written;
the rest keep their value. The answer is the full object after the write.

| Key | Accepted |
|---|---|
| `budget_rounds` | whole number, 1–20 |
| `budget_tokens` | whole number, 1 000–10 000 000 |
| `message_policy` | `anytime`, `round`, `never` |
| `theme` | `dark`, `light`, `system` |
| `human_gate` | `true` or `false` |

Unknown keys, wrong types and out-of-range values are all a **400**, and nothing is written. The values
live in the `settings` table as JSON under the key names above; `docs/data-model.md` has the table.

## Errors

Every error is JSON with the same two keys:

```json
{"error": "unknown thread nobody", "kind": "conflict"}
```

| Kind | Status | When |
|---|---|---|
| `bad_request` | 400 | The body is not valid JSON, a field is missing, the goal is invalid, or the decision is unknown |
| `not_found` | 404 | No run has that id, or no such route |
| `conflict` | 409 | The runner refused the command (the `error` text is the runner's reason, word for word), or another run is already being driven by this process |
| `internal` | 500 | Something broke inside. The detail goes to the `ilvunod` log; the response only says so |

## Limits in v1

- **One run is driven at a time per process.** A second start, resume or send-back returns 409 until the
  first reaches the gate or stops. Tasks and gate accepts do not need that slot.
- **Relative paths in a goal sent over HTTP resolve from the run's folder** (`<home>/runs/<id>`), because
  there is no goal file to be next to. Use absolute paths for sources and checks in those goals. The goal
  as sent is kept in `<home>/runs/<id>/goal.json`; the engine reads the copy taken at start.
