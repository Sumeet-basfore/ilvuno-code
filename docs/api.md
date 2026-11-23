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
- **Any origin may read it (`Access-Control-Allow-Origin: *`).** The desktop window's page is not served
  from `127.0.0.1:4737`, so the browser needs this header to read answers. It is acceptable in v1 only
  because the data is the user's own and the port is loopback-only: a web page open in the same browser
  can *read* runs, goals and agent messages from it, and a `POST` from a web page still reaches the runner
  (the runner needs no origin check to start a run). Before any of these, replace `*` with the window's
  origin list (`tauri://localhost`, `http://tauri.localhost`): an API that holds data a stranger should not
  read, a port reachable from another machine, or any auth-free write from a web page.

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
