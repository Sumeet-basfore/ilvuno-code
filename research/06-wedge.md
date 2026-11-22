# 06 — Wedges: what someone already pays for, and the one we should take

Inputs: `00-synthesis.md` §5 and §7, plus primary web evidence gathered 2026-11-23.
Method: `web_search`/`web_fetch` were out of credits, so everything below came from `curl` against the
GitHub API, the HN Algolia API, and raw HTML from pricing/docs pages. Reddit's JSON endpoints returned
**403** and are absent. `modulus.so` and `crowsnest.app/pricing` returned 404. Where a claim is my
inference rather than something a source states, it says "inference".

One caveat that colours everything: the closest competitors' issue trackers are firehoses.
`pingdotgg/t3code` has **10,112** issues, `stablyai/orca` **12,478**, and the top of both is dominated
by agent-filed reports with 0 reactions. Reaction counts are not demand. Where I use an issue as
evidence I checked a human wrote it and it describes a workflow, not a null-pointer.

---

## 0. Three candidate wedges and the recommendation

**W1 — The durable team, not the session team.** A project has one team of agents that outlives any
CLI session: who is on it, what each owns, what was decided. Every turn's preamble carries a digest;
any agent can append.

**W2 — Review rounds as data.** `propose → critique → revise → accept` is a row with a verdict, an
artifact pointer and a blocking state, so "Codex reviews Claude's work" is a state machine the app
enforces, not a prompt the user retypes.

**W3 — Cross-vendor conflict and ownership awareness.** Before an agent writes, it is told which
sibling thread already owns `src/foo.ts`.

**Recommendation: build W2, ship W1 only as the substrate W2 needs, reduce W3 to one warning line.**
W2 has the only proven willingness-to-pay here (AI review sells at $24–$72/seat/mo) and sits on the
boundary the platform vendors drew around themselves: Anthropic's native teams are Claude-only, one
team per session, and their quality gate is a hook, not a round. W1 alone is what users currently
build for free from markdown and scripts, and three products already ship it. W3 is being solved by
worktrees, which is a git command.

---

## 1. Who has the pain

### Segment A — the solo dev paying for two $200 subscriptions

They run Claude Code and Codex side by side and hand-carry work between them. The loop is described
consistently:

> "when i have codex review claude-written code, it finds important gaps and fixes. The reverse is
> also true. Both are powerful, but even better when used in combination" — HN `48849267`

> "I use both Claude and Codex, but mostly Claude for planning and coding, and Codex to review
> Claude's work. I follow a sort of waterfall workflow which is verbose but fully transparent."
> — HN `48853334`

Cost: two subscriptions ($100–$400/mo) plus the glue, some of it public — "you just spin up harness B
as a subprocess/tool call from harness A. For example, I had a `/codex-review` claude skill for ages
that did exactly that" (HN `48753404`); "Custom orchestrator for having Codex review Claude or
vice-versa" (HN `50373463`, with a repo link).

The other cost is quota, the loudest number in the ecosystem: `anthropics/claude-code` **#38335**
("session limits exhausted abnormally fast") has **545 reactions and 878 comments**, still open.
Juggling accounts is already a product — `crowsnest.app`: *"Compare provider-reported usage across
accounts, choose where new work starts, and continue on another when one reaches its limit — with a
compact briefing and visible lineage."*

What they would switch for is not another chat pane; it is the review step being one button with a
verdict. The cost today is remembering which agent is the reviewer and re-pasting the constraint — and
the constraint they want is *inverted* framing, a different prompt rather than a different model:
"having a second agent or session to review the changes with the reversed constraint, i.e. 'check if
any of these changes duplicate existing functionality'" (HN `46551461`).

### Segment B — the small team that has to show review evidence

They buy AI review at PR time: CodeRabbit at **$24/$48/$72 per user/month**, Graphite at
**$20/$40**, Greptile at **$1/credit** with a free single-developer tier. All three now advertise that
review happens *inside* the agent loop — CodeRabbit's tier list reads "Agentic code reviews in PR,
CLI, IDE, **and in agent loops**". Cost: $24–$72 per seat per month per tool, on top of agent
subscriptions. It is the highest per-seat price anyone sustains in this space.

Inference, but well-supported by those pricing pages: what they would switch for is a place where the
review record *is* the audit trail — who reviewed, what they found, what changed, who accepted.
CodeRabbit sells exactly that and does not know which agent produced the code; we would.

### Segment C — agencies and client handoffs — weakest evidence, treat as a hypothesis

I found no primary source describing an agency running multiple agent CLIs per client. The closest is a
freelancer selling agent-built client projects end to end (HN `49522752`, "Openvisor"), a different job.
**Inference:** the pain exists (per-client context separation, a review record the client can read), but
I would not build for it before A or B has paid.

---

## 2. Competitive set

| Product | What it is | Targets | Pricing | Biggest weakness vs "threads that behave like a team" |
|---|---|---|---|---|
| **t3code** | MIT control plane for Claude Code, Codex, Antigravity, OpenCode, Cursor, Grok; web + mobile + desktop over a local server | devs on multiple subs | free; "70k+ GitHub stars", "Over 500,000 devs" (`t3.codes`) | Coordination is a ~90-tool MCP surface the model must choose to use; no durable team object, no round with a verdict. Already moving in: PR review guides, PR watch wake-on-review, and multiplayer messages (`#31978`) |
| **orca** | MIT Electron IDE; SQLite mailbox + pointer typed into the TUI; coordinator agent | power users, many accounts | free/MIT; cloud connector closed | Coordination is prose into a terminal plus a coordinator prompt. Its own users document the gap: `#46872` "much of it is glue: emoji name prefixes, hooks, a rules file and scripts"; `#46876` "Child chat hands its summary back to the parent, then closes" |
| **cloudroom** | Apache-2.0 desktop + local server + Rust core for cloud sandboxes | teams wanting local/cloud parity | free core; closed control plane | 1:1 parent/child, no peer bus; the interesting half (accounts, sandboxes, credits) is not in the repo |
| **Claude Code Agent Teams** (the real incumbent) | Native lead session + teammates, shared tasks, worktrees, hooks as quality gates | every Claude subscriber | bundled | **Claude-only, one team per session, no nesting, no cross-session resumption.** Its own docs: "The lead's conversation history does not carry over" and "Two teammates editing the same file leads to overwrites" |
| **Paradev** (ex-Superset) | OSS "run an army of coding agents", local or cloud, automations, Linear/Slack | teams running agents daily | Free; Pro $20; **Team $60** (150 cloud h/seat); Ultra $200/user/mo. "$11.5M seed" | Sells the same surface at a real price — the only paid anchor I found for "many agents + team". No review-round primitive visible |
| **CodeRabbit** | AI PR review, now "in agent loops" | teams with review requirements | $24 / $48 / $72 per user/mo | Reviews the PR, not the round; doesn't know which agent wrote it or what the sibling was told |
| **Graphite** | Stacked-PR review + merge queue | teams shipping fast | Free / $20 / $40 per user/mo | Review and shipping only; "Cursor Cloud Agents are now in Graphite" — being absorbed into an agent vendor, not becoming one |
| **Greptile** | AI PR review over a codebase graph | teams wanting deep review | Starter free (1 dev, 50 credits); Pro from $1/credit | Post-hoc, agent-agnostic, no loop state |
| **Sourcegraph Amp** | Agent + hosted "orbs", BYO subscription | devs and teams | Hobby free; Individual $20/mo (45k orb-min); Teams no extra charge, with "team thread sharing, multiplayer, and permissions" | **Amp already ships multiplayer threads.** Weakness is the mirror of ours: no local multi-CLI team, no review round |
| **Devin** | Cloud autonomous SWE agent, SWE-2, Devin Bots | buyers of outcomes | Free; Pro $20; Max $200; Teams $80 | **Not a competitor.** Replaces the agent, not the team. A user running Claude + Codex has already rejected the one-vendor premise |
| **Cursor** | IDE + agents + cloud agents | individual devs | As served 2026-11-23: Free $0, Plus $8, Premium $30, tiers built around Grok models; in flux, not an anchor | Single-vendor by construction; not a team surface |

Also in the set, and the reason "shared context" is not a moat: **Elpis** (Codex-CLI fork, "Context
Ledger lists every instruction file and memory with its token cost", HN `50350853`), **Modulus**
("multiple coding agents with shared project memory", HN `47292101`, site now 404), **CAS**
(`cas.dev`: worktrees, `pending_verification`, verification gates, HN `47341352`), plus an MIT tail:
dmux, FleetCode, baag, ROMA, Emdash, rondoflow. Of ~15 "run many agents" Show HN posts, all scored
under 10 points except FleetCode (103). **Inference:** the category is saturated with free
implementations and starved of attention — a pricing problem before a product problem.

---

## 3. The three wedges, evaluated

### W1 — The durable team, not the session team

- **Pitch:** your project has a team that persists. New session, same team, same decisions, same
  ownership — across vendors.
- **Job:** stop re-briefing; stop losing what the last session learned.
- **Why incumbents can't copy it easily:** Anthropic *can't* — "one team per session", "no nested
  teams", and the lead's history not carrying over are documented boundaries, and a cross-vendor team
  is outside their interest by definition. t3/orca/cloudroom *could* and are partly there (t3 has thread
  FTS and context envelopes; orca users build a "home repo" by hand).
- **First user:** the orca `#46872` author — one conductor, ~20 coordinators, three Claude accounts,
  glued with emoji prefixes and scripts.
- **Demo:** kill the app mid-project; reopen; start a *new* Codex thread; ask "what did we decide about
  the retry policy and who owns `queue.ts`" — correct answer, one turn, no re-reading.
- **Would falsify it:** five target users say their CLAUDE.md plus a `NOTES.md` is enough.
- **Scores:** pain 4/5, WTP **2/5**, defensibility 2/5, build cost 2/5 (low).

### W2 — Review rounds as data

- **Pitch:** "Claude writes, Codex reviews" is one object with a verdict that blocks done.
- **Job:** make the critique step non-optional and auditable, without re-pasting the inverted-framing
  prompt every round.
- **Why incumbents can't copy it easily:** the vendor's version is a hook (`TeammateIdle`/
  `TaskCompleted`, exit 2 to reject) that exists only inside Claude Code and only knows Claude's
  teammates. CodeRabbit and Greptile review a PR with no idea a second agent produced it, what it was
  told, or that a round is open. t3's PR review guides are a diff-reading aid, not a verdict state.
  Nobody sits in the *middle* of two vendors.
- **First user:** Segment A — already runs the loop manually, already pays $100–$400 for models.
- **Demo:** Codex's review returns three findings; the round is `changes_requested`; Claude's "done"
  control is disabled with "3 findings open". Accept two, dismiss one with a written reason, round
  closes, done enables, an audit line records who accepted what.
- **Would falsify it:** users keep the review in chat because the round UI costs more clicks than
  typing "review this".
- **Scores:** pain 5/5, WTP **4/5**, defensibility 3/5, build cost 3/5 (medium).

### W3 — Cross-vendor conflict and ownership awareness

- **Pitch:** before an agent writes, it knows a sibling already owns that file.
- **Job:** stop two agents silently clobbering each other.
- **Why incumbents can't copy it easily:** they can, and did, differently. Anthropic's answer is
  procedural ("break the work so each teammate owns a different set of files"); CAS uses worktrees plus
  supervisor merge; orca ships no locks. Worktrees are the free, standard answer.
- **First user:** someone who already lost an afternoon to an overwrite — e.g. the Elpis author, who
  built a product because "parallel coding agents kept overwriting each other's files".
- **Demo:** two threads, one file; the second write is blocked with "owned by thread B since 14:02"
  and a one-click "request handoff".
- **Would falsify it:** every user already works in worktrees, so the warning never fires.
- **Scores:** pain 4/5, WTP 2/5, defensibility 2/5, build cost 1/5 (lowest).

**Reading the scores.** W2 is the only one where pain is documented in public quotes, the price is
already paid for an adjacent product, and the biggest competitors sit on the wrong side of a vendor
boundary. Its weak score is defensibility: the moat is not review, it is that the app owns the round
across two vendors and holds the "done" button.

---

## 4. The demo test

One project, two threads: Claude Code (writer), Codex (reviewer). One sentence — "add retry with
backoff to the queue worker" — opens a **round** and gives Claude a turn whose preamble carries the
project digest (owner map, last three decisions, files owned by other threads). Claude edits two files
and ends with `{status:"proposed", filesChanged, summary, claims:"retry is bounded by maxAttempts"}`.
That becomes the round's `proposal` and is **pushed** to the Codex thread — no polling, no copy-paste.
Codex's turn arrives with the diff, the claims, and one instruction: return findings, not prose. It
returns `{verdict:"changes_requested", findings:[...]}`; the round flips, findings get ids, Claude is
woken with them inline. Claude revises, the round returns to `review`, the reviewer accepts two and
returns `wontfix` (with a reason) for one. Only now does "mark done" on Claude's thread become
clickable, and clicking writes one ledger line: `round #14 accepted by codex, 3 findings, 1 wontfix
(reason recorded)`. Surface: two panes, one round card, one disabled button. The whole demo is that
the button is **off** until a second vendor said yes.

---

## 5. What would make this fail

**1. The vendor absorbs it, on their side of the boundary.** Agent Teams landed Feb 2026 to 396 points
on HN with automatic message delivery ("The lead doesn't need to poll for updates"), idle notifications
carrying the final answer, task claim, worktrees, and hooks that reject a task completion. It is
experimental, Claude-only, one team per session, and explicitly overwrites on shared files — exactly the
seam we would sell into. But the seam moves: Cursor shipped cloud agents and code review, those cloud
agents are now *inside Graphite*, and t3 already ships PR review guides and PR watch. **Inference:** a
cross-vendor seam outlasts a feature seam, but it is one roadmap decision from being half-solved.

**2. The buyer is at the ceiling, and everything else is free.** These users pay $100–$400/month for
models, and the loudest thread in the ecosystem is quota running out (`#38335`, 545 reactions). Every
"run many agents" product I found except Paradev is MIT or free, anchoring the category at $0; Paradev's
$60/seat Team tier is the single counter-example and it sells cloud compute alongside the app.
**Inference:** charging for coordination alone is unproven; charging for the review record is proven.

**3. The integration tax outruns a small team.** orca carries 45 PTY quirk records, 17 ACP dialects, 23
hook installers; t3 carries 11 native drivers with replay fixtures; cloudroom carries 14 local + 13
cloud providers and a parity harness (`00-synthesis.md` §4). Our version of this failure is not "we
can't add agent #12" — it is "Claude Code's result format changed and the round stopped closing". If the
app is a review gate, a broken adapter means a blocked `done`, which is worse than no gate.

---

## 6. Recommendation and the first 30 days

**Build W2 on a minimal W1. Two vendors (Claude Code + Codex) and nothing else.** The sentence to hold
ourselves to: *the app where a second agent's verdict is required before work counts as done.*

Days 1–10: one runner, two providers on native protocols, one project, two threads, a structured turn
result (`status`, `filesChanged`, `summary`, `claims`) persisted as a row, and a push from one thread's
turn end into the other's next turn. No rounds yet. Test: the result survives an app restart and a new
session.

Days 11–20: the round object — `proposal → critique → revise → accept`, verdict with finding ids,
`wontfix` requiring a reason, `done` bound to `accepted`. The review prompt is generated by the app from
the diff plus the proposal's claims, not typed. Measure keystrokes saved against the user's current
`/codex-review` skill.

Days 21–30: the append-only, greppable ledger line per closed round, plus the single W3 warning ("thread
B owns this file" in the preamble, never a lock). Then five people who already run the manual loop, and
one question: after a week, did you go back to pasting the prompt?

Not in the first 30 days: cloud, more than two providers, a scheduler, a coordinator agent, multiplayer,
mobile.

---

## 7. Sources — everything actually fetched

**GitHub API**
- `search/repositories?q=t3code` → `pingdotgg/t3code`, "The best way to control Claude Code, Codex and
  more from any computer or phone".
- `pingdotgg/t3code`: 10,112 issues. `#402` Pi provider (198 reactions), `#193` Copilot CLI (185),
  `#14871` Orchestrator V1→V2 (89, 62 comments), `#231` Steer/Queue (77), `#538` nested subagent
  threads (37), `#35197` review-from-teams, `#31978` multiplayer messages.
- `stablyai/orca`: 12,478 issues. `#1099` multi-repo (122), `#6074` multiple windows (72), `#11557`
  multiple Claude subscriptions (36); `#46872` conductor + ~20 coordinators, `#46876` child-summary
  hand-back.
- `anthropics/claude-code` `#38335` → Max limits exhausted, 545 reactions / 878 comments;
  `subagent in:title` → 2,175 hits, `#6915` (377), `#43869` (35).
- `openai/codex` `review in:title` → 859 hits; `#5547` (87), `#10874` (31), `#40721` (21).

**Hacker News (Algolia)**
- `items/46902368` "Orchestrate teams of Claude Code sessions", 396 pts, 25 comments.
- `items/50350849` Elpis; `items/47292101` Modulus.
- Comments quoted: `48849267`, `48853334`, `50373463`, `48753404`, `46551461`, `50542456`, `50389190`
  ("Orchestration answers 'how does work get distributed.' It doesn't answer 'how does what the team
  learned stay with the team.'"), `47341352` (CAS: worktrees, `pending_verification`, "agents that say
  'done' when they're not"), `46909505`, `45557120`.
- `search?tags=story` sweep → FleetCode (103 pts), Superset (24), then Baag, Rover, Modulus, Crowsnest,
  Emdash, ROMA, dmux, rondoflow, live-log-viewer, roundtable, CCB, Endy, kiln.bot, all ≤9.

**Docs and pricing (raw HTML)**
- `code.claude.com/docs/en/agent-teams` — teams, task claim, worktrees, "Automatic message delivery…",
  "The lead's conversation history does not carry over", "Two teammates editing the same file leads to
  overwrites", quality gates via `TeammateIdle`/`TaskCreated`/`TaskCompleted`; limitations: no session
  resumption with in-process teammates, task-status lag, one team per session, no nested teams.
- `t3.codes` — control-plane positioning, "70k+ GitHub stars", "Over 500,000 devs", BYO subscription,
  inline diff review, stacked PRs.
- `superset.sh`, `superset.sh/pricing` — Paradev; $11.5M seed; Free $0 / Pro $20 ($16) / Team $60 ($54)
  / Ultra $200 per user/mo; cloud 150 h/seat.
- `crowsnest.app` — multi-account Claude Code + Codex, usage comparison, continue-on-other-account.
- `coderabbit.ai/pricing` — $24 / $48 / $72 per user/mo; "in agent loops".
- `graphite.dev/pricing` — Free / $20 / $40; "Cursor Cloud Agents are now in Graphite".
- `greptile.com/pricing` — free 1 dev + 50 credits; Pro from $1/credit.
- `devin.ai/pricing` — Free / Pro $20 / Max $200; Teams $80; SWE-2, Devin Bots, Devin Cloud.
- `ampcode.com/pricing` — Hobby free (orbs, BYO subs/keys), Individual $20/mo (45k orb-min), Teams "no
  extra charge… team thread sharing, multiplayer, and permissions".
- `cursor.com/pricing` — as served 2026-11-23: Free $0, Plus $8, Premium $30, tiers around Grok models
  ("Only Grok models are available on this plan"). Reported as fetched; looks mid-repositioning and I
  would not use it as an anchor.
- `t3.gg` (index, no pricing), `dmux.ai` (MIT parallel agents). **Unreachable:** Reddit JSON (403),
  `modulus.so` (404), `crowsnest.app/pricing` (404).
