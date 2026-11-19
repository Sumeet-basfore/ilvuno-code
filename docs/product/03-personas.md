# Ilvuno — Personas

These are design targets, not research participants. No user interviews were conducted
(see `docs/research/05-integration-problem-analysis.md` §8 and PRD §3.4). Every behavioural
claim below is a hypothesis to be tested during Phase 3, not a finding.

Requirement IDs referenced here are defined in `02-prd.md`.

---

## 1. Priya — the parallel power user (primary)

**Snapshot.** Staff engineer, 8 years, a 6-person product team, one large TypeScript
monorepo with 4,000 tests and a 9-minute full suite. Runs Claude Code and Codex on Fridays
and returns to the work on Monday.

**Day.** Friday morning: splits "checkout split" into four streams — payments-intent,
fulfilment, API surface, e2e updates. Runs them in four worktrees, reviews each diff, pushes
four branches. Monday: merges locally in some order, hits two conflicts, fixes them,
discovers a third failure only because `checkout.e2e` happens to exercise both paths, and
spends 90 minutes producing a combined state she then has to re-verify anyway.

**Goals.** Finish the batch in one sitting on Monday. Know *before* merging whether the
combination is sound. Stop re-running the full suite to answer "is it still green?"

**Frustrations.**
- Each branch was green, which is exactly the information she already had (T3-047, ORC-054).
- CI on four PRs tells her nothing about the merge of all four.
- Her scratch merge branch is throwaway, undocumented, and gone by Wednesday.
- She has started, twice, to keep a notes file describing what she verified. She stops.

**Current workaround.** `git checkout -b scratch && for b in ...; do git merge $b; done` then
`pnpm test`. Works, silently, in a dirty tree, with no record.

**What would make her switch.** One command, correct answer, under five minutes of her own
time, in her own repo, with no account. If it takes longer than the merge she does by hand,
she will not use it again (`NFR-101`, `NFR-102`).

**Deal-breakers.**
- Touches her working tree. (`INT-105`)
- Needs a login, a daemon, or a cloud account. (`XDC-101`)
- Says "ready to merge" and is wrong once. (`INT-602`, `VER-101`)
- Makes her rewrite the batch in a new tool's format. (`INT-101`)

**Requirements she cares about most:** INT-101, INT-105, INT-108, INT-210, INT-301, INT-306,
INT-601.

**Success looks like:** she pastes one report link into the PR description instead of
explaining what she ran.

**Where she abandons:** first run is slower than expected, or the report is a wall of JSON.

---

## 2. Marco — the reviewer (secondary, high leverage)

**Snapshot.** Tech lead, reviews 15 PRs a week, half of which are agent-produced. Has been
burned twice by "tests pass" claims that were never run.

**Day.** Opens a PR. Reads the diff. Scrolls to CI. Wants to know: was this verified against
anything other than its own branch? Were the checks real? What was *not* run?

**Goals.** Review on evidence, not on assertion. Ask for one artifact instead of three
round trips. Be able to say "no" with a concrete reason.

**Frustrations.**
- Agent-produced PR bodies assert verification with nothing behind it (T3-030, ORC-042).
- CI green on a single PR says nothing about whether it conflicts with its siblings.
- Asking "did you actually run this" is socially expensive and repeats weekly.

**What converts him.** A report that names the exact SHAs, the exact commands, the exit
codes, the failures that only appear in combination, and an explicit "not verified" list
(`EVD-107`). And it has to be readable in 30 seconds; he will not open a browser.

**Deal-breakers.**
- A badge with no detail.
- Interpretation mixed into facts so he cannot tell what is checked. (`INT-206`)
- Evidence that expires when a tool is updated. (`EVD-101`, `NFR-106`)

**Requirements:** EVD-102, EVD-103, EVD-106, EVD-107, VER-101–VER-103, INT-605.

**Success looks like:** his review comment template becomes "evidence: <bundle>" and the
conversation moves to the design question instead of the verification question.

**How he drives adoption:** he is the one who asks Priya to run it — or blocks a PR without
it. That is why the report, not the CLI UX, is the highest-leverage surface.

---

## 3. Dana — the platform owner (buyer-shaped)

**Snapshot.** Staff infra, 40 engineers, sanctioned agent usage with a written policy
("agents may run, humans must verify"). Owns CI cost and tool sprawl.

**Goals.** Reproducibility (same inputs, same result — `NFR-103`). No shadow data leaving the
network (`XDC-101`). No tool that silently mutates global config. Something she can put in
a policy document and audit.

**Frustrations.**
- Tools that write to `~/.claude/settings.json` (ORC-016) or default to full access (T3-060).
- "Trust us" guards that fail open (CR-037).
- Per-seat pricing that punishes the 40 engineers who only need read access.

**What converts her.** A single static binary, deterministic output, an explicit list of what
it writes and does not write (`SEC-101`), and evidence a compliance reviewer can read.
Optional paid retention only if it is genuinely optional (`OSS-102`).

**Deal-breakers.** Any mandatory service. Any unlogged network call. Any secret handling she
cannot audit. Any per-seat price for people who only read reports.

**Requirements:** SEC-101–SEC-106, OSS-101–OSS-104, NFR-103–NFR-106, XDC-101.

**Commercial signal:** Dana represents the team-shaped buyer in `06-monetization.md` (H3,
H4). She is the persona most likely to convert a $200–500/month team plan — and the one who
will insist the core stays free.

---

## 4. Jonas — the integration-heavy maintainer (opportunistic)

**Snapshot.** Maintainer of an OSS library with a contribution queue that includes agent
PRs. Not an agent power user himself; other people's agents arrive at his door.

**Goals.** Triage incoming PRs that overlap each other. Know whether two community PRs would
conflict before spending review time on either.

**Fits because:** the product takes arbitrary git refs from arbitrary producers (`INT-101`).
No agent of his own required. This is the persona that makes "any repo, any ref, any agent"
matter commercially — and the one whose repos offer the cheapest measurement corpus for R-1
(real conflict rate).

**Does not fit:** he needs cross-repository views and queue integration, which are P2.

---

## 5. Sam — the solo builder (explicit anti-persona)

**Snapshot.** Ships a small SaaS alone. No test suite. Two agents, one branch at a time.
Wants the tool to write his code.

**Why we do not serve him at MVP.**
- Verification without anything to run is `unverified` by design (`INT-303`).
- He has no parallel streams to integrate, so the core capability is unused.
- Serving him would push the roadmap toward dispatch, chat, and generation — the
  commoditized segment the research warns against (research `04` §6; Vibe Kanban sunset).

**We say this out loud** on the docs landing page. A product that names who it is not for is
more credible to the person it *is* for.

---

## 6. Persona → requirement map

| Persona | Primary jobs | Load-bearing requirements | Persona-specific risk |
|---|---|---|---|
| Priya | J1, J2 | INT-101, INT-105, INT-108, INT-210, INT-301, INT-306 | Slower than her manual workaround → never used twice |
| Marco | J3 | EVD-102, EVD-103, EVD-106, EVD-107, VER-101, INT-605 | Report reads as marketing → trust never earned |
| Dana | J6, policy | SEC-101…106, OSS-102, NFR-103, XDC-101 | Any mandatory service → disqualified for the org |
| Jonas | J1 (inbound) | INT-101, ORQ-102, ORQ-103 | Needs multi-repo, which is P2 |
| Sam | — | none (anti-persona) | Roadmap drift toward dispatch/chat |

## 7. Anti-persona guardrail

If a proposed feature's primary beneficiary is Sam, it is out of scope by default. Examples
that fail this test: a chat UI, an agent selector, a "generate tests" button, a
prompt-improvement panel, a one-click deploy.

If a proposed feature's primary beneficiary is Marco but it requires Priya to change tools,
it is also out of scope. The adoption path runs through her existing git workflow.
