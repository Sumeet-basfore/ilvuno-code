# Ilvuno — project rules

A desktop app that runs a small team of coding-agent CLIs inside one project, in recorded rounds, until
a goal is met. Rust runner, Tauri shell, SQLite. Linux first.

Read in this order before touching code:

1. `docs/prd.md` — who it is for, the two templates, what v0 must prove.
2. `research/08-round-machine.md` — the engine: objects, states, rights, done specs.
3. `docs/data-model.md` — the tables and the invariants.
4. `docs/provider-contract.md` — the adapter contract and the two prompt texts.
5. `docs/security.md` — trust boundaries and what we refuse to promise.
6. `research/07-decision.md` — decisions and the conditions that reverse each one.

Everything else in `research/` is evidence. Read it when a claim here needs defending.

---

## Toolchain

| Tool | Version / note |
|---|---|
| Rust | **`1.100.0`**, used explicitly. The `stable` toolchain on this machine is corrupt (`missing manifest`). |
| cargo | `RUSTUP_TOOLCHAIN=1.100.0 cargo build` — or `cargo +1.100.0`. Do not run plain `cargo`. |
| Node | present; needed only to launch ACP adapters via `npx`. We do not own a JS dependency. |
| SQLite | via `rusqlite`, bundled feature. One writer task. |
| Python | 3.14, `markdown_it` — used by `tools/render-report.py` only. |

Do not "fix" the broken toolchain by reinstalling without asking; the workaround is cheap and the download
is not.

## Build and test (target state — created with the scaffold)

```bash
RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace
RUSTUP_TOOLCHAIN=1.100.0 cargo run -p ilvuno -- run --goal examples/board-experiment.json
RUSTUP_TOOLCHAIN=1.100.0 cargo run -p api --bin ilvunod   # the app's local API on 127.0.0.1:4737 (docs/api.md)
```

A change is not done until `cargo test --workspace` passes and `ilvuno run` still reaches a gate on the
example goal.

## Layout (target)

```
crates/
  runner/            round state machine, journal, ledger, invariants
  provider-claude/   stream-json + control protocol
  provider-acp/      agent-client-protocol wrapper
  cli/               `ilvuno run | status | gate | freeze | replay`
docs/                prd, data-model, provider-contract, security
research/            reports (.md source, .html to read)
tools/render-report.py
```

## Conventions

- **No UI until the runner works.** The v0 acceptance in `docs/prd.md` §7 is the gate. A Tauri window built
  before it would be built against imagined state.
- **Smallest change that fully works.** No abstraction, option, or "for later" layer nobody asked for. No
  new dependency for a few lines; the workspace starts with `tokio`, `serde`, `rusqlite`, `anyhow`,
  `agent-client-protocol`.
- **One command, one transaction.** A command writes its journal row, projection update and receipt in a
  single SQLite transaction. Never write state outside it.
- **Providers translate, the runner decides.** An adapter never writes the DB, never judges done, never
  retries a turn by itself. Unknowns are emitted as `unknown`, not guessed.
- **Prompts are versioned.** The worker preamble and orchestrator contract live in
  `crates/runner/src/prompts/` with a version constant; the journal records which version each thread got.
- **Credentials never enter a child's environment** beyond what its own CLI needs. See `docs/security.md`.
- **Truthful state.** `unknown` is a state we render, not an error we hide.

## How to talk to the user

English is the user's second language. Write so a child could follow.

- Short sentences. One idea in each sentence.
- Everyday words. If a technical word is needed, explain it in a few words the first time.
- No idioms, no clever phrases. A simple comparison is fine.
- Lists and small tables beat long paragraphs.
- Start with what it means for the user. Then the details.
- Same rule for reports and HTML pages: explain, do not impress. No jargon for its own sake.

## Hard-won facts — do not rediscover these

1. Claude's `--session-id` must be a **UUID**; anything else exits before you can write to it.
2. Claude silently auto-denies every tool if you skip `control_request{"subtype":"initialize"}`. No error;
   the turn completes having done nothing. Send it first, always.
3. Claude approvals need `--permission-prompt-tool stdio`. `--permission-prompts host` was not enough in
   our spike.
4. Do **not** depend on `codex-app-server-protocol` (0.63.0 vs runtime 0.171.0). Generate types from the
   installed binary: `codex app-server generate-json-schema --out <dir>`.
5. Never take a vendor SDK (`@cursor/sdk`, `@opencode-ai/sdk`, `claude-agent-sdk`). They are transports,
   and taking one re-locks the language.
6. ACP permission answers select by **option kind** (`allow_once` before `allow_always`), never by
   position.
7. `webkit2gtk-4.1` is installed here (2.52.6) and Tauri builds. The window's CORS origin is
   `tauri://localhost`; `docs/api.md` lists the two allowed origins and why writes must be JSON.

## One command exports a run for the UI

```bash
RUSTUP_TOOLCHAIN=1.100.0 ./target/debug/ilvuno export <run-id> --out fixtures/<name>
```

Writes `run.json`, `board.md`, `journal.jsonl`, `artifacts/`. `fixtures/board-experiment/` is a real
accepted run: use it for UI work instead of inventing data. The shape is documented in
`docs/data-model.md` under "Export bundle".

## Board prototypes

`tools/render-prototype.py fixtures/<name> proto` renders three layouts from one exported run:
`proto/index.html` links them. Same real data, three arrangements; the chosen one becomes the app's run page.

## Reports are HTML, not markdown

Markdown is the source; HTML is the reading artifact. Never hand-write report HTML.

```bash
python3 tools/render-report.py --index research "Ilvuno — research reports" "<lede>"
python3 tools/render-report.py --index docs "Ilvuno — project docs" "<lede>"
python3 tools/render-report.py docs/<file>.md
```

## Do not

- Copy code from t3code, orca or cloudroom. They are MIT/Apache but enormous and coupled to toolchains we
  are not adopting; we copy *shapes*, with attribution, and never a subsystem. Apache-2.0 would require
  carrying `NOTICE` and marking modified files.
- Let an agent's shell touch `main`. The runner executes merges.
- Start an agent in `bypassPermissions` by default.
- Add a scheduler, cron, cloud runner, or multi-user support to v1.
- Add a dependency, a doc, or a directory that the PRD's non-goals do not need.
