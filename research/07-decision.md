# 07 — Decisions

Inputs: `00-synthesis.md`, `04-rust-feasibility.md`, `05-shell-choice.md`, `06-wedge.md`.
This file is the decision record, not a report. Each decision carries the condition that reverses it.

---

## D1 · The runner is Rust, and it is a separate process

**Decision.** One long-lived Rust daemon per machine: spawns agent CLIs, owns threads/runs, writes the
journal, serves the UI. The UI is a client of it and may restart freely.

**Why.** Wire protocols, not SDKs. Every target agent speaks newline-framed JSON over its own stdio, or
HTTP: Claude `--input-format/--output-format stream-json` plus a control channel; Codex `app-server`
JSON-RPC; Pi `--mode rpc`; OpenCode HTTP or ACP; ACP for the rest. None of them requires Node. Two
shipped Rust apps already do exactly this (cloudroom's core; agent-of-empires on
`agent-client-protocol 5`), so this is engineering, not research.

**Also why.** Single binary distribution, no Node runtime to pin (t3 pins Node 26, cloudroom
`pnpm@10.34.9` exactly), crash isolation between the supervisor and the UIs it feeds, and a daemon that
outlives the window — which all three references needed anyway.

**Not why.** Memory. Measured delta between Electron and Tauri shells is 26.5 MiB idle, roughly 1% of
what five agent subprocesses cost. Anyone who says Rust buys RAM is measuring the wrong process.

**Cost.** We own hand-modelled types for the two protocols we care most about, and their drift.

**Reverse if.** The one-day spike fails to get a Claude permission round-trip *and* an interrupt through
the control protocol. Then: keep Rust for the supervisor, add a thin Node helper for Claude only, and
reconsider after one release.

---

## D2 · Provider tiers, in this order

| Tier | How | Agents | Why |
|---|---|---|---|
| 1 · ACP | `agent-client-protocol` 5.5.1 (Apache-2.0) | the 46-agent registry, incl. Codex, Claude, Pi, OpenCode, Antigravity, Cursor, Gemini via adapters | One adapter, many agents. The protocol is the product. |
| 2 · Native where it buys fidelity | Codex `app-server`; Claude stream-json + control | Codex, Claude | fork/rewind/review/permissions. Codex's own binary emits its schema: `generate-json-schema`. |
| 3 · PTY typing | — | none in v1 | The long tail. A permanent maintenance tax; orca carries 45 hand-tuned records. |

Rules that follow from the research:
- **Never depend on the `codex-app-server-protocol` crate.** Published 0.63.0 (2025-12-11) against
  `rust-v0.171.0`. Generate types from the installed binary at build time instead.
- **Never take a vendor SDK.** `@cursor/sdk`, `@opencode-ai/sdk` and `claude-agent-sdk` are
  convenience transports; the Python SDK's own code shows it shelling out to the CLI. Taking one
  re-locks the language we just unlocked.
- **Publish the tier.** A visible "this agent is tier 2, no fork/rewind" beats a parity promise that
  breaks on the next CLI release.

---

## D3 · Tauri 2.12 shell, transcript over loopback HTTP + SSE

**Decision.** Tauri 2.12.x with the web UI served by the Rust runner on a loopback port; the UI
subscribes over SSE. Not Tauri IPC for the transcript.

**Why.** The chat surface is the product and it does not exist in Rust. Production line counts from the
references: t3code `components/chat` 67,391; cloudroom `components/thread` 37,577 (one view is 3,355);
orca `native-chat` 98,382. They all buy markdown, highlighting, diffs and a terminal from the web
ecosystem. Rust equivalents are not there (`vix-diff-view` 404 downloads; `egui_term` 4,750, no release
since 2025-04). Rebuilding that in an immediate-mode UI is a multi-month tax paid before the product
works.

**Why loopback and not IPC.** The runner stays headless-capable, a second window is free, UI crashes
cannot take agents with them, and a future phone or remote client is the same wire.

**Rejected.** GPUI: `0.2.2` published 2025-10-22, nothing for 13 months, README pins `gpui = "*"` and a
`gpui_platform` crate that is not on crates.io. Iced: one release a year, no virtualized list, no
accesskit in its lockfile. Both are experiments we would fund.

**Cost / known risk.** Linux is the weak platform: `tauri#14286` (font weight off by 100 on WebKitGTK),
`tauri#16226` (webkit2gtk 2.44 + Wayland `abort()`, needs `WEBKIT_DISABLE_DMABUF_RENDERER=1`). Also
measured: Tauri's first frame was slower than Electron's in the one controlled test, and 4/50 launches
stalled. Test before committing the UI.

**Reverse if.** The spike on the target dev machine shows WebKitGTK unusable for the transcript at
60 fps. Then Electron — with the same Rust runner, since the wire is HTTP either way.

---

## D4 · The wedge is a goal run: rounds until a definition of done

**Decision.** The product is: the user states a goal and a definition of done, the team runs
`propose → critique → revise → accept` in rounds until done or blocked, and every round is a row
with a verdict, a diff pointer and an audit trail. Review is the mechanism, not the whole claim.

**Why.** The first users are people already running several agents on one project. Their pain is not
"another pane" — it is that context dies every time work crosses from one agent to the next, so the
human becomes the messenger. A goal run is that pain repeated a hundred times a day, which makes the
context ledger load-bearing instead of decorative. Review stays the acceptance step because AI review
is the only thing in this market with a proven price: CodeRabbit $24/$48/$72 per user/month, Graphite
$20/$40, Paradev $60/seat.

**Why cross-vendor is the defensible part.** Anthropic's agent teams already do messaging, idle
notifications, worktrees and completion-rejecting hooks, and Codex has native multi-turn goal runs.
Both are single-vendor. A round that a different vendor's agent executes, with an explicit ledger, is
the thing each vendor is structurally disincentivised to build.

**Why "no memory loss" is the wrong promise.** Every handoff is lossy; t3 trims each handoff item to
240 characters on purpose. What we can promise is **no silent loss**: what was dropped is recorded on
the round, so the user can see and restore it.

**Falsified if.** Users run the loop twice and then go back to doing it by hand, or they keep
automating only the single review step. Then the goal was the review, and the loop was decoration.

---

## D5 · The loop has a budget, a stop rule and a human gate

**Decision.** No loop ships without all six of these. They are policy, enforced by the runner, not
prompt suggestions.

| Control | Rule | Why, from the evidence |
|---|---|---|
| Definition of done | machine-checkable: tests, a diff predicate, a named command, or an explicit human accept | orca's own guidance: *"silence, a timeout, or unverifiable liveness alone never proves exit"* |
| Round cap | hard maximum, default 3–5 | orca retired its programmatic coordinator; loops that cannot prove done do not terminate |
| Token / cost budget | per run, checked before each round | the loudest complaint in this ecosystem is quota: `claude-code#38335`, 545 reactions |
| No-progress detector | same failing assertions, or no diff for N rounds → stop | otherwise "revise" becomes a slot machine |
| Human gate | default: a person accepts each round boundary; auto-accept is opt-in | every reference keeps a human approval lane; t3 routes approvals, orca books them |
| Journal | every round resumable, replayable, with the exact context each agent received | cloudroom's honesty model: `unknown` is a state, not an error to hide |

**Reverse if.** In beta, every user turns the gate off and the rounds still converge. Then the gate is
friction, not safety — and we ship auto-accept as the default with the budget raised.

---

## D6 · What v1 refuses to build

Cloud sandboxes, teleport, remote runners, federation. Mobile. Embedded browser, computer use, org
sharing, marketplace. A scheduler/DAG engine. Merge arbitration. PTY agents. More than five providers.
"Parity with every agent" as a promise.

Each has a documented cost in the reports. None is a growth blocker for the review-round claim.

---

## Spike — two days, before any scaffolding

Goal: answer "does Rust drive a real agent end to end, with a permission round-trip and a clean
interrupt, and what is it like to debug". Write down the answer; ship no product.

1. **ACP leg.** `cargo new`, `cargo add agent-client-protocol@5 --features process`, copy the SDK's
   `yolo_one_shot_client.rs` example, point it at `@agentclientprotocol/codex-acp@2.9.0` and
   `@agentclientprotocol/claude-agent-acp@0.109.1`. Success: a file written, tool calls observed,
   session resumed.
2. **Native Claude leg.** Drive `claude -p --input-format stream-json --output-format stream-json
   --verbose --permission-prompt-tool stdio` by hand: do we get `control_request{can_use_tool}`, can we
   answer it, then `interrupt` and see a `control_response`? That single answer decides D1's reversal.
3. **Codex drift leg.** `codex app-server generate-json-schema --out <dir>`; confirm we can generate
   types from the installed binary rather than a stale crate.
4. **Shell leg.** Tauri 2.12 on the real dev machine: render 5k lines of markdown with a diff and a
   live terminal at 60 fps, then measure launch time and memory. Record the WebKitGTK symptoms.

Exit criteria: all four pass, or we know exactly which one does not and what it costs.

---

## Still unanswered — needs you, not an agent

1. **What "done" means for the first goal you want to run.** Tests green? A diff you accept? A doc
   written? This is the single input that makes the loop terminate; without it, v1 is a chat app.
2. **Autonomy level for the first version:** a person accepts every round, or it auto-advances until
   a budget or a failing check stops it.
3. **Budget per run.** Tokens or dollars, per goal. Five agents in a loop is the fastest way to burn
   two subscriptions.
4. **Which subscriptions are live on your machine:** the spike needs real logins. Whatever is missing
   is tier-3 and we do not design around it until it exists.

## Unverified, as of these reports

Codex crate drift vs `main` was not diffed. `claude-acp` is listed proprietary in the ACP registry while
its repo is Apache-2.0 — unresolved. Gemini CLI's deprecation banner (replaced by Antigravity for unpaid
users, 2026-06-18) was read but not confirmed against a second source. No Rust precedent ships on
Windows.
