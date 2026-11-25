# 09 — What Paseo's five reviews add

**Status:** plan, corrected 2026-11-25 after a code review. The first pass is the paste block below: items
5, 1, 4 and 3. Item 2 is a separate prompt that writes a design note and no code.
**Base:** `feat/checks-see-the-work` (PR #10, stage 6), after it merges.

## What this revision changed, and why

| Before | Now | Why |
|---|---|---|
| Item 3 recorded a message's delivery in the `messages` table | Item 3 deletes the dead table and says the truth about the policy | `messages` has no writer and no reader anywhere (`0001_init.sql:92`). A message is a `dispatch` row with `kind='message'` (`0005_messages.sql`). Writing into it would create a second, empty source of truth |
| Item 1's step 4 asserted a malformed line becomes an `unknown` event | The mapping is `Event::Raw`, and a line that is not JSON fails before the mapping | `wire::parse_frame` sends an unknown `type` to `Event::Raw` (`wire.rs:32`). A non-JSON line is a `Protocol` error at `lib.rs:160` |
| Item 1 told the worker to find the parse function | It names it: `provider_claude::wire::parse_frame` | `lib.rs:161` calls it. No hunt needed |
| Item 2 asked for a proxy and a gate URL | Item 2 writes one design note and stops | Three unknowns have no answer yet: nobody starts the app, `*.localhost` resolves to `::1` first, and the window's CSP forbids an in-window preview |
| No paste markers, no index row | Both added | Every other prompt has them |

You do not need the five reports to run this. Every fact you need is inline, and the evidence table is at the
end.

---

## The first pass — paste this

---8<--- PROMPT STARTS ---8<---

**Goal: take four cheap lessons from the Paseo review into Ilvuno. None of them changes how a run behaves,
except deleting a table nothing uses.**

Read `/home/sumeet/personal-repo/projects/ilvuno-second/context/prompts/README.md` and follow the house
rules in it. Work on a new branch off `main` after PR #10 (stage 6) has merged. If `main` does not yet have
`git::integration_tree`, stop and say so.

Do the four items in this order. Commit after each one, with its test.

### Item 1 of 4 · Write the Paseo comparison, once

A new file `research/15-paseo.md`, about one page, plain English. It is the durable copy of five reviews that
live in `/tmp/paseo-review/reports/`, which a reboot deletes.

Read those five reports while they exist. If they are gone, write the page from the evidence table at the end
of this prompt and say in the page that it was written from notes.

Content, in this order:

1. **What it is.** A local daemon with four clients, several agent providers behind it, one maintainer.
   Take the numbers (providers, stars, licence) from the reports; do not invent or round them.
2. **What it proves.** Many agents in one app is commodity. A round, a done spec, a journal and an accept gate
   are not. Nobody in that set built them.
3. **Four facts we verified**, each with its file and line, in the table at the end of this prompt.
4. **Copy, avoid, ignore.** Three short lists, with a reason per line.

Then generate the reading copy and do not commit it:

```bash
python3 tools/render-report.py research/15-paseo.md
```

**Done when:** another person can answer "should we copy Paseo?" from this one file without opening `/tmp`.
`research/15-paseo.html` exists and you have read it.

### Item 2 of 4 · Replay a recorded stream in the Claude adapter tests

A test that feeds saved `stream-json` lines through the same mapping the runtime uses, and asserts the events.
Test only. No network, no provider run, no new dependency, no change to the mapping.

**The function to call is `provider_claude::wire::parse_frame(&frame)`**, where `frame` is one parsed line as
`serde_json::Value`. The runtime calls exactly this at `crates/provider-claude/src/lib.rs:161`. It returns
`Vec<Event>`. Do not write your own parser and do not change this one.

Touch:

- `crates/provider-claude/tests/fixtures/normal-turn.jsonl` (new)
- `crates/provider-claude/tests/fixtures/permission-prompt.jsonl` (new)
- `crates/provider-claude/tests/replay.rs` (new)

The fixtures:

1. **`normal-turn.jsonl`**: one turn with a tool call and its result. The frames are `system`/`init`, then
   `assistant` text and a `tool_use`, then a `user` frame carrying the `tool_result`, then `result`.
2. **`permission-prompt.jsonl`**: one `control_request` that asks for permission.

Capture them from a cheap real run if you can; hand-writing them from the frames the adapter already parses is
allowed and must be said in the file. Each fixture starts with one comment line, beginning `#`, recording the
CLI version, the date, and the command that produced it. Scrub home paths, tokens and session ids. A comment
line is skipped by the test, not parsed.

The tests:

- one test per fixture: read the file, skip `#` lines, `serde_json::from_str` each line, pass the value to
  `wire::parse_frame`, collect the events, and assert the sequence. `normal-turn` must produce
  `Session`, `TextDelta`, `ToolCall`, `ToolResult`, `TurnFinished` in that order. `permission-prompt` must
  produce `PermissionRequest` with the tool name and its options.
- one extra line in `normal-turn.jsonl` (or a third small fixture) whose `type` is `something_new`, asserted
  to become `Event::Raw`. That is the rule from `docs/provider-contract.md`: an unknown frame is kept, not
  guessed at.
- **Do not** assert anything about a line that is not JSON. That is rejected before the mapping, at
  `lib.rs:160`, as a `Protocol` error, and reaching it needs a fake child process. Out of scope here.

**Done when:** changing one arm of `parse_frame` makes `cargo test -p provider-claude --test replay` fail.

### Item 3 of 4 · Pin the credential boundary with two tests

The code already does the right thing and nothing holds it in place.

1. **The ACP child gets a clean environment.** `AcpAgent::launch(&self, env)` at
   `crates/provider-acp/src/lib.rs:51` builds an `AcpAgentConfig`. Its serialized form is the public
   description of the command (`serde_json::to_value`), which is how the code already inspects it. Add a unit
   test: call `launch` with `[("HOME", "/tmp/x"), ("PATH", "/usr/bin")]` and assert the serialized command is
   `env`, that its args begin with `-i`, that the two variables follow in order, and that nothing else
   appears. Deleting `.arg("-i")` must break this test.
2. **The API listens on loopback.** In `crates/api`, bind port `0` on the same address the server uses and
   assert `listener.local_addr()` is a loopback address. Use the constant the server uses, not a copy of the
   literal. Deleting the loopback from the bind must break this test.
3. **`docs/security.md`** gains three lines, in plain English, in the section that explains why there is no
   authentication: `ilvunod` has no login because it binds `127.0.0.1` only; any LAN or remote mode must add
   a token first; the window's two allowed CORS origins (`tauri://localhost`, `http://tauri.localhost`) assume
   the same local-only boundary.

**Done when:** `cargo test -p provider-acp -p api` passes, and either deletion above breaks a test.

### Item 4 of 4 · Delete the dead `messages` table, and say how policy really works

`crates/runner/migrations/0001_init.sql:92` creates a `messages` table with a `delivery` column. Nothing in
`crates/` writes to it or reads from it. A message is already a `dispatch` row with `kind='message'` (added in
`0005_messages.sql`). Two homes for one idea, one of them empty, is the kind of thing a reader trusts and
should not.

1. New migration `crates/runner/migrations/0007_drop_messages.sql` containing `DROP TABLE messages;` (the
   directory already has `0006_git.sql`, so 0007 is next).
   Register it in `crates/runner/src/store.rs` next to the others, in order.
2. `docs/data-model.md`: delete the `messages` section, and add one paragraph where it was: a user's message
   is a `dispatch` row with `kind='message'`; agent-to-agent handoff is a task, not a message; no delivery
   outcome is recorded because the runner has no delivery step to record.
3. `docs/data-model.md`, at `message_policy` in the goals table: say what it really is. It is validated,
   stored, exported and written into the worker prompt. **The runner does not enforce it.** A person should
   not read "round" as a guarantee.
4. One test that a fresh database has no such table: after `Store::open`, `SELECT name FROM sqlite_master
   WHERE type='table' AND name='messages'` returns nothing. Put it where the other schema assertions live.

Do not add enforcement, a queue, steering, or an interruption path. That is a product decision, not this
item.

**Done when:** the workspace suite passes on a fresh database, and grep finds no `messages` table outside the
migration that drops it.

### Constraints for all four

- `RUSTUP_TOOLCHAIN=1.100.0` on every cargo command.
- New branch off `main`. Commit per item. Push. Do not merge and do not push `main`.
- No new dependency. Item 2 uses `serde_json`, which is already there.
- Item 4 is the only schema change in this pass, and it only removes.
- Do not touch the window. Nothing here is visible.

### Done when

- `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` passes once, at the end;
- the four items above each have their test, and the two "deleting this breaks a test" checks are real;
- `context/timeline.md` has a line, `context/decisions.md` records that `messages` is gone and policy is
  advisory, and the report is written at `context/prompts/reports/<date>-09-paseo-lessons.md`.

### Stop and report instead, if

- the five reports in `/tmp` are gone **and** the evidence table below is not enough for item 1. Write what you
  have and say what is missing; do not invent numbers.
- dropping `messages` fails on a database that already exists. Report the error and stop; do not edit
  `0001_init.sql`.
- item 2 cannot find a permission-request frame shape in `wire.rs`. Read
  `parse_control_request` first; if the frame shape is not there, report it instead of inventing one.

---8<--- PROMPT ENDS ---8<---

## Item 2 (the gate URL) is a later prompt

Do not run this with the first pass. The idea is good and the design has three unknowns that need answering
before a line of code.

**Goal block for that later prompt:**

> Write `research/16-gate-url.md`, one page, and no code. Show how a person could open the app an agent built
> from the gate screen, on loopback, without stopping another run's server.

The page must answer these, in this order:

1. **Who starts the app.** Nothing in the runner starts a project command, and it should not. The smallest
   honest design is a port a person types on the gate card, stored with the run, and a plain sentence when it
   is empty (`409`, not a guess). Say if you disagree.
2. **Where the name resolves.** On this machine `abc.localhost` resolves to `::1` first, then `127.0.0.1`
   (`getent ahosts abc.localhost`). `ilvunod` binds `127.0.0.1`. Decide: bind `::1` as well, prefer IPv4, or
   ask for a hosts entry. State the cost of each.
3. **Where the page opens.** The window's CSP is `default-src 'self'` with no `frame-src`, so a preview inside
   the window cannot load. The system browser is the honest answer. Say what that means for the gate card.
4. The rest of the shape, only if it survives the three answers: `<label>.localhost` label validation
   (`^[a-z0-9][a-z0-9-]*$`), unknown label `404`, nothing proxied when no port is set, `Host` passed through,
   WebSocket upgrade or an explicit `shortcut:` note saying what is missing.

**Done when:** a person can say yes or no to the feature after reading the page, and knows what it costs.

---

## Not in any of this, and why

| Not doing | Why | Where it is decided |
|---|---|---|
| Plugin marketplace | Unaudited code with the user's full access | `AGENTS.md`, `docs/roadmap.md` non-goals |
| Mobile, web client, relay | A different product with different support cost | `docs/prd.md`, `research/07-decision.md` D6 |
| Schedules, cron or heartbeats as progress | A timed prompt is not a done spec | `research/08-round-machine.md` |
| 30-provider parity | Each native adapter is thousands of lines | `research/07-decision.md` D2 |
| Vendor SDKs | Paseo takes two and still carries ~16k lines of adapter | `AGENTS.md` hard-won fact 5 |
| Event-sourcing every stream | Our journal is already append-only per command | `docs/data-model.md` |
| Their plugin and hook surface | Our runner decides, adapters translate | `docs/provider-contract.md` |
| Electron, React Native, Bun workspaces | We are Rust and Tauri | `research/07-decision.md` D1, D3 |

## Open question you own, not the worker

**Messages.** Today `message_policy` is a sentence in the agent's prompt, not a rule the runner applies. If
v0.1 is queue-only, that is fine, and item 4 of the first pass says so in the docs. If you want the policy
enforced, that is a separate prompt and a product decision: enforced `round` and `never` change what an agent
can do mid-round.

## Appendix — the evidence

| Paseo location | What it shows | Item |
|---|---|---|
| `packages/server/src/server/session-admission-auth.ts:17-19` | no password means owner permissions | 4 |
| `packages/server/src/server/agent/provider-launch-config.ts:250-252` | the child env is `process.env` minus four variables | 4 |
| `packages/server/src/server/service-proxy.ts:147` | `<label>.localhost` per worktree service | 2 (later) |
| `packages/server/package.json:110,121` | Claude Agent SDK and OpenCode SDK dependencies | none, the rule stands |
| `plugins/muse-provider/test/fixtures/README.md:3-45` | recorded NDJSON fixtures with provenance | 2 |
| `packages/server/src/server/agent/agent-messages/delivery.ts` | queue, steer or interrupt outcome | none, we do not deliver |
| `packages/server/src/server/agent/providers/acp-agent.ts:4304-4362` | ACP answers chosen by option kind | none, we already do it |
| `packages/server/src/server/session.ts` (9,794 lines) | what one harness for every agent costs | none |
| `public-docs/why.md` | their pitch, for contrast | 1 |

And on our side, the three lines the tests in item 3 hold in place: `cmd.env_clear()`
(`crates/provider-claude/src/lib.rs:92`), `env -i` (`crates/provider-acp/src/lib.rs:62`), and
`TcpListener::bind(("127.0.0.1", port))` (`crates/api/src/main.rs:28`).
