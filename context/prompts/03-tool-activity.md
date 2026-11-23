> Paste everything between the two markers as the goal. Nothing else.
> **This goal needs the owner's yes before it runs.** It is the one item the checklist still marks as open.

---8<--- PROMPT STARTS ---8<---

**Goal: the conversation shows what the agent actually ran.**

Read `/home/sumeet/personal-repo/projects/ilvuno-second/context/prompts/README.md` and follow the house
rules in it.

## Today

A turn shows what the agent *said* and a one-line report. It does not show the commands it ran, the files it
edited, or the permissions it was asked for. The journal already records all of it — `tool.call`,
`permission.decided`, `dispatch.message` — and `GET /runs/:id/events` already streams new rows. The window
just never asks for the history.

No provider change is needed. `context/decisions.md` R4 records that this belief was already corrected once;
do not repeat it.

## What to build

**1 · History, not only the stream.**
- `GET /runs/:id/journal?after=<seq>&kinds=<csv>&limit=<n>` returns rows in order:
  `{ seq, at, kind, round, thread, payload }`. Default limit 200, max 1000. `after` defaults to 0.
  Unknown kind names are a `400`.
- Keep it cheap: read straight from the journal table, no joins that pull `result_text`.

**2 · The turn shows its work.**
- Under each turn in the conversation, a collapsible **"What it ran"** block:
  - one line per tool call: the tool's name and its target (file path, command, or query), not the whole
    input;
  - permission decisions folded into the tool line they belong to;
  - failures shown in red, with the first line of the error;
  - a count in the summary line: `What it ran · 12 tools · 2 denied`.
- A turn with no tool rows shows nothing at all — no empty block.
- Cap what you render: the newest 50 tool rows per turn, and say "older rows hidden" when you cut.

**3 · Live, without a second mechanism.**
- New rows arrive on the existing SSE stream. Append them to the open turn if it is the same round and
  thread. Do not re-fetch the whole journal on every event.

## Where

- `crates/api/src/runs.rs` (new route), `crates/api/src/events.rs` (the row shape already exists — reuse it),
  `crates/runner/src/query.rs` (a `journal_after`-style query with filters).
- `crates/desktop/ui/app.js` — the turn rendering (`replyHtml`) and the event handler.
- `crates/desktop/ui/style.css` — the fold and the tool rows.

## Rules for this goal

- Read-only. Nothing in this goal may change a run, a task, or the journal.
- Never render a raw payload as text: names, paths and counts only. A tool input can contain a secret.
- Do not stream the full transcript of a tool's output; the first line of a failure is enough.
- A missing or unknown kind must be shown as `unknown`, never guessed.
- Keep the conversation readable: this is a fold, collapsed by default, not a log window.

## Tests you must add

1. the journal route returns rows in order, honours `after`, `kinds` and `limit`, and `400`s on a bad kind;
2. a scripted run's detail or journal contains `tool.call` and `permission.decided` rows for a turn (the
   scripted provider must emit them — extend it if it does not);
3. the payload of a large tool input is not returned whole (prove the cap).

## Done when

- those tests pass, `RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` is green;
- one **real** run shows "What it ran" with real tool names and a denied permission if one occurred, and you
  have read a screenshot of it (`tools/shot.sh`);
- the block is collapsed by default, opens by keyboard, and does not shift the conversation when it opens;
- `context/timeline.md` and `context/features.md` are updated (M5 is no longer blocked);
- the report file exists.

## Stop and report instead, if

- the scripted provider cannot emit tool rows without touching the provider contract — then report that
  finding and stop, do not change the contract;
- rendering the fold would need a framework or a dependency.

---8<--- PROMPT ENDS ---8<---

## For you, before you paste

- **Decision needed:** this is the open question O1 in `context/risks.md`. Say yes and run it, or leave it.
- **Cost:** test-and-build time plus one real run; the data already exists, so no new agent calls.
- **Check when it reports:** open a run's turn, look at the tool names, and confirm nothing sensitive is
  printed.
