# Competitive Landscape (2026-11-19)

All sources accessed 2026-11-19. Confidence tags: **C** = Confirmed (primary source read: official docs, repo README, API output), **I** = Inferred (reasoned from sources, including absence of documentation), **U** = Unverified (secondary or marketing source only, or not checked). "Not documented" means the page I read did not say; it does not prove the feature is absent.

## 1. Scope and method

Covered: tools that run or coordinate several AI coding agents in parallel. Method: GitHub API (`gh repo view`, `gh api`) for license, stars, last push, README; vendor docs via WebFetch; WebSearch for discovery. Star counts are API-reported on 2026-11-19 and are a popularity signal only, not usage or quality evidence.

Not checked or only partly checked: no tool was installed or run. Devin, Jules, Factory and Conductor docs were read only at the overview level, so their isolation and merge details are mostly undocumented in what I saw. Pricing is largely not captured. The three repos under separate deep analysis (t3code, orca, cloudroom) appear only as table rows. I did not read Claude Code's `sub-agents` page or Cursor's release notes.

## 2. Category map

| Category | Members |
|---|---|
| A. Local worktree managers / desktop orchestrators | Conductor, Paradev (ex-Superset), Sculptor, Nimbalyst (ex-Crystal), claude-squad, Vibe Kanban, t3code, orca |
| B. First-party vendor features | Claude Code (worktrees, agent view, agent teams, `/batch`), Codex app + cloud, Cursor cloud agents, GitHub Copilot coding agent, Jules |
| C. Cloud / autonomous agent platforms | Devin, Factory (Missions), OpenHands, cloudroom |
| D. Integration-adjacent tooling | Graphite (stacking + merge queue), Clash and Grove (pre-merge conflict detection) |
| E. Contrast | Aider (single-agent terminal pair programmer) |

## 3. Comparison table

| Tool | Category | License / OSS | Isolation | Integration model | Verifies combined result? | Cross-device | Conf. |
|---|---|---|---|---|---|---|---|
| Conductor (Melty Labs) | A | Proprietary, free local app (U: from secondary sources) | Git worktree per workspace (C: [docs](https://www.conductor.build/docs/concepts/workspaces-and-branches)); Cloud used microVMs (U) | Per-workspace diff, PR, merge, archive (C: [docs](https://www.conductor.build/docs/)); no multi-workspace merge documented | Checks tab shows CI/status/comments; does not say Conductor runs tests (C: [checks](https://www.conductor.build/docs/reference/checks)) | Mac only on docs I read (C); iOS app and Cloud reported (U) | C/U |
| Paradev (ex-Superset) | A | MIT, OSS (C: [repo](https://github.com/ParadevAI/Paradev); 16,404 stars) | Worktree + branch per task (C: README) | Changes pane, PR pane with checks, merge (C: README) | PR checks only (C) | Remote hosts, iPhone/iPad app on Pro plan (C: README) | C |
| Sculptor (Imbue) | A | MIT (C: API; 278 stars); research preview, external PRs limited (C: README) | Worktree default; clone and in-place experimental (C: [workspaces doc](https://github.com/imbue-ai/sculptor/blob/main/docs/help/workspaces.md)) | Review changes, commit, GitHub PR, merge back to target branch (C: README); merge mechanics not documented | Not documented | Desktop Mac/Linux (C) | C |
| Nimbalyst (successor to Crystal) | A | MIT (C: API; 2,092 stars) | Git worktree isolation (C: [Crystal README](https://github.com/stravu/crystal)) | Not documented | Not documented | Desktop + iOS/Android companion (C: repo description) | C |
| Crystal (stravu) | A | MIT; deprecated Feb 2026 (C: README) | Worktrees | Replaced by Nimbalyst | n/a | n/a | C |
| claude-squad | A | AGPL-3.0 (C: API; 8,836 stars; pushed 2026-11-13) | Terminal sessions (isolation detail not read; U) | Not read | Not read | Terminal | C/U |
| Vibe Kanban (Bloop) | A | Apache-2.0 (C) | Branch + terminal per workspace (C: README) | PR + merge via GitHub (C) | No | Local | C |
| t3code | A | MIT, 66,061 stars (C: API) | Not analyzed here | Not analyzed | Not analyzed | "from any computer or phone" (C: repo description) | C |
| orca | A | MIT, 129,567 stars (C: API) | Not analyzed here | Not analyzed | Not analyzed | "desktop, mobile and remote runtime" (C: repo description) | C |
| cloudroom | C | Apache-2.0, 1,787 stars (C: API) | "isolated sandboxes"; sessions move between Mac and cloud (C: repo description) | Not analyzed | Not analyzed | Mac <-> cloud (C) | C |
| Claude Code (worktrees, agent view, teams, `/batch`) | B | Proprietary | Git worktree per session/subagent; Claude Code blocks edits to the main checkout (C: [worktrees](https://code.claude.com/docs/en/worktrees.md)). Agent teams do NOT isolate in worktrees (C: [agents](https://code.claude.com/docs/en/agents.md)) | Agent view never merges; may open a draft PR (C: [agent-view](https://code.claude.com/docs/en/agent-view.md)). `/batch`: 5-30 units, one worktree subagent each, each "runs tests and opens a PR or reports" (C: [commands](https://code.claude.com/docs/en/commands)) | Per unit only; `TaskCompleted` hook can block completion in agent teams (C: [agent-teams](https://code.claude.com/docs/en/agent-teams.md)) | Remote Control and cloud Projects (public beta) (C) | C |
| Codex app + cloud (OpenAI) | B | Proprietary | Detached-HEAD worktrees locally (C: [worktrees](https://learn.chatgpt.com/docs/environments/git-worktrees)); cloud workspace per task (C: [cloud](https://learn.chatgpt.com/docs/cloud)) | Create branch, commit, push, PR; merge flow not documented (C) | Cloud "runs checks and reports results" (C) | Mobile controls a connected computer or remote env; worktrees do not run on the phone (C) | C |
| Cursor cloud agents | B | Proprietary | Dedicated VM per agent (C: [docs](https://cursor.com/docs/cloud-agent)) | Branch push, "merge-ready PRs with artifacts" (C); conflict handling not covered | Agent builds/tests in its VM; screenshots/video artifacts (C) | iOS app, web, PWA, Slack/Linear/GitHub triggers (C) | C |
| GitHub Copilot coding agent | B | Proprietary | Cloud sandbox or Actions runner; one branch and one PR per task (C: [docs](https://docs.github.com/en/copilot/concepts/agents/coding-agent/about-coding-agent)) | PR per task; "Agent Merge" can work on merge conflicts and failed workflows on a PR (C) | Runs tests/linters; Actions need approval on draft PRs (C) | GitHub Mobile (C) | C |
| Jules (Google) | B | Proprietary, experimental (C) | VM (C: [docs](https://jules.google/docs)) | Branch then PR; Jules is commit author (C: [running tasks](https://jules.google/docs/running-tasks)) | Not documented | Web app | C |
| Devin | C | Proprietary | Not documented in overview (U) | Not documented | Advises tasks "easy to verify" such as CI (C) | Web, CLI, Slack, Teams (C) | C/U |
| Factory Missions | C | Proprietary | Not documented; launch post claims local or cloud containers (U) | Orchestrator plans features across agents (C: [docs](https://docs.factory.com/missions/overview)) | Per-feature QA; combined validation not stated (C) | CLI + app | C/U |
| OpenHands | C | MIT; 93,829 stars (C: API) | Local, Docker, VM, cloud backends (C: README); Agent Canvas control center | Not documented | Not documented | Self-hosted web | C |
| Graphite | D | Proprietary | n/a | Stacked PRs, stack-aware merge queue that tests combined changes (U: vendor/secondary) | Queue CI (U) | Web | U |
| Clash / Grove | D | MIT (66 stars) / Apache-2.0 (13 stars) (C: API) | n/a | Read-only conflict prediction across worktrees; Grove claims symbol/dependency/schema overlap (U: README claims, not read) | Detection only | CLI | C/U |
| Aider | E | Apache-2.0, 50,184 stars, last push 2026-05-22 (C: API) | None (single agent in your checkout) | n/a | n/a | Terminal | C |

## 4. Per-tool notes (most relevant)

**Claude Code** is the most complete first-party reference. It offers five parallel modes (subagents, agent view, agent teams, projects, dynamic workflows), and worktrees, cross-session messaging and `/batch` as support (C: [agents](https://code.claude.com/docs/en/agents.md)). The docs explicitly tell users to partition files between agent-team teammates to avoid overwrites (C), and agent teams are experimental with task-status lag and no resumption of in-process teammates (C). The worktree docs show careful isolation enforcement (blocking git redirects into the main checkout) and a base-branch rule: new worktrees branch from the remote default branch, fetched if older than 24h (C). Integration is left to PRs and the human.

**Codex app/cloud** shows a convergence on worktree-per-chat locally plus cloud tasks, with phone as remote control only (C). Worktrees are disk-heavy and capped (default keep 15) with a snapshot before deletion (C).

**Cursor cloud agents** set the bar for verification evidence: per-agent VM, build/test, and screenshot/video artifacts (C). Conflict handling is not documented.

**GitHub Copilot coding agent** is the only first-party tool I read that documents conflict and CI-failure remediation on an open PR ("Agent Merge", watch-and-fix) (C). It remains one branch and one PR per task, with no multi-PR combination.

**Conductor / Paradev / Sculptor** are the closest direct analogs: desktop worktree dashboards with diff, PR and merge. None of the docs I read describes combining several workspaces' output or testing the combination; checks are PR-level.

**Factory Missions** is the closest to a plan-dispatch-validate loop. Factory's own docs state parallelism benefit is unproven, long plans "accumulate errors," validation is per feature, and reliability depends on repo readiness (C). It documents no conflict handling.

**Graphite** addresses sequential dependent changes via stacks. A stack-aware merge queue that tests combined changes is its claim (U); the primary post returned 404, so I could not read it. Its model is human-authored dependent stacks, not N independent agents.

**Clash / Grove** are tiny (66 and 13 stars, C) but show that cross-worktree conflict prediction exists as a standalone, read-only layer (README claims U).

**Market churn is a signal.** Vibe Kanban announced a sunset on 2026-04-10, citing no business model it was excited about (C: [announcement](https://www.vibekanban.com/blog/shutdown)); it still received a release on 2026-11-15 (C: API). Crystal was deprecated in Feb 2026 (C). A single secondary source (Hashnode, 2026-11-09) says Meta acquired Conductor on 2026-11-04 and Conductor Cloud shuts down 2026-11-20 (U); conductor.build's homepage did not mention it (C: absence on one page).

## 5. Integration-problem coverage matrix

Cells: Solved / Partial / Open / U (unverified). Columns abbreviated: CC = Claude Code; CX = Codex; CUR = Cursor cloud; GH = Copilot agent; WM = worktree managers (Conductor, Paradev, Sculptor); FAC = Factory Missions; GR = Graphite; CL = Clash/Grove.

| Failure mode | CC | CX | CUR | GH | WM | FAC | GR | CL |
|---|---|---|---|---|---|---|---|---|
| (a) Git conflicts across parallel branches | Open (partition files manually; C) | Open (I) | Open (not covered; C) | Partial: Agent Merge fixes conflicts on one PR (C) | Open (not documented; I) | Open (not covered; C) | Partial: stacks reduce, not for independent branches (U) | Partial: predicts, does not resolve (U) |
| (b) Cross-worktree integration | Open: `/batch` = one PR per unit (C) | Open (I) | Open (I) | Open: one PR per task (C) | Open (I) | Partial: orchestrator, mechanism undocumented (C) | Partial: sequential stacks only (U) | Open |
| (c) Dependency / API mismatch between outputs | Open (I) | Open (I) | Open (I) | Open (I) | Open (I) | Open: per-feature QA only (C) | Partial: merge-queue CI on combined stack (U) | Partial: Grove claims symbol/dependency overlap (U) |
| (d) Stale branches | Partial: fresh base at creation, 24h fetch; no continuous rebase documented (C) | Partial: starts from branch HEAD (C) | Open (I) | Partial: Agent Merge on conflicts (C) | Partial: Conductor bases on latest origin at creation (C) | Open (I) | Partial (U) | Partial: shows divergence (U) |
| (e) Unverified agent completion claims | Partial: `TaskCompleted` hook gate (C) | Partial: cloud reports test results (C) | Partial: VM tests plus artifacts (C) | Partial: tests plus approval gate on Actions (C) | Open: PR checks only (C) | Partial: per-feature QA, readiness-dependent (C) | Open | Open |
| (f) Interrupted or partial integrations | Partial: worktree locks/sweeps preserve work; resume refusals documented; no integration transaction (C) | Partial: snapshot before worktree delete (C) | Open (I) | Open (I) | Open (I) | Partial: pause orchestrator and recover (C) | Open | Open |
| (g) Failed repair attempts | Open (I) | Open (I) | Open (I) | Partial: watch-and-fix on one PR; bounded retry not documented (C) | Open (I) | Partial: retry-limit warning, manual recovery (C) | Open | Open |

Evidence notes: "Open (I)" means no relevant documentation was found in the pages read, not proof of absence. Academic evidence that the problem is real: AgenticFlict reports 27.67% of 107K+ simulated agent PRs had textual conflicts (C: [arXiv 2604.03551](https://arxiv.org/abs/2604.03551)); the "stale" benchmark found 97% interference on constructed tasks but only 1 of 834 runs on real merged Django PR pairs, and its authors say constructed rates do not estimate real-world frequency (C: [arXiv 2609.25396](https://arxiv.org/abs/2609.25396)). Telling an agent about the concurrent change recovered 82% of runs (C), which supports the idea that context sharing, not only conflict detection, matters. Both are preprints/workshop papers, so treat the numbers as indicative.

## 6. Commoditized vs. credible gap

**Commoditized (do not lead with these):**
- Worktree-per-task isolation, parallel session dashboards, diff review, PR creation (Conductor, Paradev, Sculptor, Claude Code, Codex, Vibe Kanban; all C).
- BYO agent/subscription across Claude Code, Codex, OpenCode and others (Paradev README, Vibe Kanban README; C).
- Phone as remote control or notifications (Cursor iOS, Codex mobile, Paradev iPhone app, GitHub Mobile; C; t3code/orca/Nimbalyst per repo descriptions C).
- Cloud sandbox per agent with test and artifact evidence (Cursor; C).
- Single-PR conflict/CI remediation (Copilot Agent Merge; C).

**Credible gaps (all based on absence in documentation read, so Inferred):**
1. A combined integration step: merge N agent branches into one integration candidate and run the full suite on it. No tool read documents this; every vendor stops at one PR per task (I).
2. Semantic/API mismatch handling between agents' outputs, including feeding one agent's finished change to another, which the "stale" benchmark shows works (C for the paper, I for the product gap).
3. Completion claims backed by independent evidence on the integrated result, rather than per-task self-report (I; Cursor artifacts and Claude hooks are the nearest, per-task).
4. A transactional, resumable integration (partial merges, interruption, rollback) as opposed to preserving worktrees (I).
5. Bounded repair loops with escalation on the combined result, and a record of failed attempts (I; Copilot and Factory are per-PR or per-feature).

**Cautions (I):** Vendors (Anthropic, OpenAI, Cursor, GitHub) can add an integration step to existing surfaces; the churn (Vibe Kanban sunset, Crystal deprecation, reported Conductor sale) suggests standalone orchestrator UIs struggle commercially, but I have no revenue or adoption data and make no demand claim. Real-world conflict rate in careful workflows may be low (the Django result), so the value case needs measurement, not assumption.

## 7. Unverified items

- Conductor acquisition by Meta and Cloud shutdown on 2026-11-20, iOS app, microVM isolation, pricing (single Hashnode post plus marketing; not on conductor.build's homepage).
- Conductor merge/rebase/conflict behavior (docs page silent).
- Graphite's stack-aware queue behavior and ownership chain (primary blog 404; search summaries only).
- Clash and Grove feature claims (repo metadata checked; READMEs not read).
- claude-squad isolation and integration details (metadata only).
- Devin isolation, parallelism limits, PR flow and pricing; Jules concurrency limits, tests/CI behavior and mobile; Factory isolation and combined-result validation.
- OpenHands integration/PR and verification behavior (Agent Canvas page silent).
- Sculptor multi-agent coordination and merge process; Nimbalyst integration behavior.
- t3code, orca, cloudroom internals (intentionally out of scope).
- Pricing for nearly all tools; adoption and user-complaint data (none collected, none asserted).
- Aider's current maintenance status beyond last push 2026-05-22 and its git behavior (not read).
