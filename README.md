# Ilvuno

A desktop app where several AI agents work on one project, and the project remembers what each did.
Rust engine, local API, Tauri window (not built yet). The app decides nothing about who does what — you
write the goal, name the threads, and say what finished looks like.

Status: the engine, the board, the gate, the replay and the local API work. There is no window yet.

## Run it

```bash
# one real run end to end: plan, two readers, one brief, a failed round, a fix, the gate
tools/demo.sh

# any goal file
tools/demo.sh path/to/goal.json
```

`tools/demo.sh` builds, runs `examples/board-experiment.json`, then prints the status, the replay check
and an export. State goes to `./.demo-home`; delete it to start clean. The default goal costs about
12 cents of Claude quota.

Needs: Rust **1.100.0** (always set `RUSTUP_TOOLCHAIN=1.100.0`; the `stable` toolchain on this machine is
broken), `node` for ACP agents, and at least one agent CLI already logged in (`claude`, `codex`, …).

## The CLI, by hand

```bash
export RUSTUP_TOOLCHAIN=1.100.0
cargo build -q -p cli
export ILVUNO_HOME=/tmp/ilvuno-demo          # optional; default is ~/.local/share/ilvuno

./target/debug/ilvuno run --goal examples/board-experiment.json
./target/debug/ilvuno status <run-id>
./target/debug/ilvuno replay <run-id>         # journal, checked against the database
./target/debug/ilvuno export <run-id> --out /tmp/out
./target/debug/ilvuno gate <run-id> accept    # or: send-back --note "why"
```

A goal with a thread named `orchestrator` plans itself. A goal without one starts nothing: you add tasks.

```bash
./target/debug/ilvuno task --thread reader-1 --prompt "read docs/prd.md" --scope notes.md
./target/debug/ilvuno run --resume <run-id>
```

## The app's door

```bash
ILVUNO_HOME=/tmp/ilvuno-demo ./target/debug/ilvunod &   # 127.0.0.1:4737
curl -s localhost:4737/runs
curl -s localhost:4737/runs/<run-id>
```

Endpoints and error shapes: `docs/api.md`. Loopback only, no auth in v1.

## The board prototypes

`proto/index.html` — four layouts of one real run, `4-mix.html` is the chosen one. They read
`fixtures/board-experiment/`, nothing invented. Regenerate with:

```bash
python3 tools/render-prototype.py fixtures/board-experiment proto
```

## Tests

```bash
RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace
```

## Docs

| | |
|---|---|
| `docs/prd.md` | what the app is, who for, what v0 proves |
| `docs/data-model.md` | tables, invariants, the export bundle |
| `docs/provider-contract.md` | how agents connect; the two prompt texts |
| `docs/api.md` | the local door |
| `docs/security.md` | trust boundaries and what we refuse to promise |
| `research/07-decision.md` | decisions and what reverses them |
| `research/11-product-thesis.md` | the argument, and the three disagreements |
