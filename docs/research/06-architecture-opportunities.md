# Architecture Opportunities

Date: 2026-11-19. This document turns the research into architectural options for Ilvuno. It does not choose a technology stack. Evidence IDs (T3-, ORC-, CR-) resolve in `07-evidence-index.md`. Source analyses: `01-t3code-analysis.md`, `02-orca-analysis.md`, `03-cloudroom-analysis.md`. Market context: `04-competitive-landscape.md`. Problem decomposition: `05-integration-problem-analysis.md`.

---

## 1. Summary

- The three systems already solve most single-agent problems well: process ownership, session durability, worktree safety, provider adapters, and cross-device access. Ilvuno should reuse those ideas, not rebuild them.
- None of them has a combined integration step. Every opportunity below is aimed at the gap that `05` identifies: candidate construction, combined verification, an evidence ledger, resumable integration, and bounded repair.
- Several security defaults in the surveyed code are weak (fail-open guards, unchecked completion claims, full access by default, plaintext fallbacks). Ilvuno can avoid these from the start, and that is a differentiator in its own right.
- The biggest open architecture decision is where the source of truth for a session lives: a headless runtime that stays up (orca, t3code) or a snapshot that resumes elsewhere (cloudroom Teleport). The choice depends on the offline and continuity requirements, which are still undefined.

---

## 2. Prior art worth adopting

### 2.1 One runtime that owns the work; thin clients

- **t3code:** one Node server owns providers, terminals, Git, and storage. Web, Electron, and mobile are clients over typed RPC (T3-004, T3-005). Clients reconnect to the server rather than hold state.
- **orca:** a detached PTY daemon owns terminals and survives app restarts and crashes (ORC-005, ORC-006). A headless runtime (`orcad`) serves desktop, CLI, SSH remotes, and mobile (ORC-009, ORC-063).

**Why it matters.** Continuity across devices becomes re-attaching to running work instead of copying state (orca's own framing, ORC-009). Process ownership means a UI crash does not kill agents.

**Trade-off.** The runtime host must stay up, and mobile does nothing offline (orca's own design note, `02` decision 3). See §5.1.

### 2.2 Provider adapters over native protocols

- **Native protocols win over screen scraping.** cloudroom drives each harness through its own protocol behind one private Rust `Adapter` trait: Claude's stream-JSON mode, Codex's app-server JSON-RPC, Pi's RPC mode (CR-007, CR-008, CR-009, CR-010). t3code does the same through an `Adapter` service interface with a normalized event union (T3-009, T3-010). orca's structured registry covers Claude, Codex, Pi and 9 ACP agents (ORC-012 to ORC-015, ORC-018).
- **PTY as the universal fallback.** orca runs about 45 agent CLIs in PTYs and infers status from hooks and escape sequences (ORC-011, ORC-016). That gives breadth at the cost of brittle status inference, and orca's own design notes call this out (02, design decision 1).

**Recommendation for Ilvuno.** Use native protocols for the agents Ilvuno supports first. Treat the PTY path as an opt-in escape hatch for agents without a protocol. Keep the adapter interface narrow enough that a new provider is a bounded piece of work.

### 2.3 Journaled, idempotent session state

- cloudroom: append-only, fsynced per-session journal; idempotent request IDs; uncertain in-flight work becomes `unknown_after_restart` and is never resent (CR-012, CR-013, CR-016).
- t3code: one SQLite transaction writes events, projections, the command receipt, and outbox rows together; subscribers are notified after commit (T3-018). Outbox workers retire lost-process effects on recovery (T3-025).

**Lesson from t3code.** Pruning the event log to about one day keeps the database small, but it makes the projections authoritative: they cannot be rebuilt from the log (T3-021, Confirmed). Ilvuno should decide early whether the log is the source of truth. If it is, retention must be long enough to rebuild, or the projections must be checkpointed in a rebuildable form.

**Recommendation.** Adopt the cloudroom rule ("never resend uncertain work") for integration steps too. Adopt the t3code transaction boundary (state plus outbox in one commit). Make retention an explicit decision.

### 2.4 Worktree safety rules

orca's worktree lifecycle is the most careful in the set:
- fetch the base before create, and fail if the fetch fails (ORC-045);
- roll back a failed create (ORC-044);
- refuse removal of a dirty worktree (ORC-047);
- delete a branch only after proving it is merged, and force only in that case (ORC-048);
- journal removal so an interrupted one resumes (ORC-049).

**Recommendation.** Adopt these rules as the baseline for every worktree Ilvuno creates or removes. They are cheap to implement and close several failure modes in `05` (§2.5, §2.9).

### 2.5 Scoped, expiring credentials for agents

- t3code requires a scope for each RPC method (T3-057). Its agent-facing MCP endpoint uses bearer tokens scoped to environment, thread, instance, and session, which expire and are not persisted (T3-063).
- t3code keeps checkpoints as hidden refs, so rollback needs no commit on the user's branch (T3-038). Restore is refused in shared workspaces (T3-039).

**Recommendation.** Scope every agent-facing credential to one task and one worktree, and make it expire. Use hidden refs or equivalent for checkpoints so the user's branch is not modified by the system.

### 2.6 Relay as broker, not proxy

t3code's T3 Connect relay brokers identity, links, and tunnel allocation; traffic goes directly over tunnels (T3-054). orca's cloud relay splices two outbound sockets (ORC-069). Both avoid carrying all traffic through a central proxy.

**Trade-off noted by t3code.** The relay holds signing authority, so its key compromise is serious (T3-055, Confirmed doc).

---

## 3. Anti-patterns to avoid (each with evidence)

| Anti-pattern | Where observed | Why it matters for Ilvuno |
|---|---|---|
| Permissions bypassed for unattended agents, with a fail-open guard | cloudroom: `--dangerously-skip-permissions` and Codex `danger-full-access` (CR-008, CR-009). Command Guard catches an exception and allows the command (CR-037, Confirmed) | A guard that fails open is worse than none, because it looks like protection. |
| Full access as the default | t3code default runtime mode is `full-access` (T3-060, Confirmed in code) | Permissive defaults make the risky path the easy one. |
| Completion claims accepted on self-report | orca `filesModified` and outcome self-reported (ORC-042); t3code delegated results are text (T3-030); cloudroom child result is last text (CR-026) | This is the failure mode Ilvuno exists to address (`05` §2.8). |
| Shared-folder parallelism as a default | orca coordinator workers share one worktree (ORC-033); cloudroom shared-sandbox children share the folder (CR-024) | Destroys the isolation the rest of the design depends on (`05` §2.6). |
| Pushing with hooks disabled, and no CI gate | cloudroom RepoSync `--no-verify` commit and push (CR-032, Confirmed) | Skips the one check that might catch a bad merge. |
| Plaintext secret fallback | orca API keys stored plaintext when safeStorage is unavailable (ORC-074) | Silent downgrade of protection. |
| Trust-on-first-use for SSH hosts | orca accepts unknown SSH host keys without a prompt (ORC-073) | Man-in-the-middle exposure on first contact. |
| Non-constant-time token comparison | orca `request.authToken === this.authToken` (ORC-072) | Standard timing-attack exposure; trivial to avoid. |
| Custom encryption with a legacy downgrade | orca mobile E2EE, with the v1 handshake still accepted on direct connections (ORC-065, ORC-067) | A protocol that accepts an older, weaker version is only as strong as its weakest accepted version. |
| Unsigned webhooks | t3code scheduled-task webhooks triggerable by anyone with the URL (T3-064) | Easy to avoid; needs signing from the start. |
| Editing global agent configuration | orca merges hooks into `~/.claude/settings.json` (ORC-016) | Changes the user's global state without a clear reversal path. |

---

## 4. Components Ilvuno would need

These are architectural components, not a stack. Each maps to failure modes in `05`. The first three are the differentiator; the others are required supporting parts.

### 4.1 Integration candidate builder (differentiator)

Given N task branches or worktrees, build a candidate integration branch in an isolated worktree, in a deterministic order. Record each merge step and its result. Report textual conflicts with `merge-tree`-style analysis before anything is merged.

- **Covers:** `05` §2.1, §2.5.
- **Prior art:** orca already uses `merge-tree` for PR merges (ORC-053). No surveyed tool builds a multi-branch candidate (Inferred absence, 04).
- **Design question:** order of merges, and what to do when a later branch conflicts with an earlier one.

### 4.2 Combined verification runner (differentiator)

Run the project's own checks on the candidate: tests, typecheck, build, lint, as configured by the project. Record evidence: commit SHA, command, exit code, and an output digest. Run in an isolated environment, because the checks execute project code.

- **Covers:** `05` §2.7.
- **Prior art:** Cursor's VM-based tests with artifacts (04, Confirmed). Per-PR CI only in the others (ORC-054, CR-032).
- **Design question:** how to get dependencies into the candidate environment without the per-worktree drift that cloudroom describes (CR-021, CR-030).

### 4.3 Evidence ledger (differentiator)

An append-only record linking each agent's claims ("tests pass", "file X changed") to observed artifacts: a SHA, a command run, an exit code, a file list computed from git, not from the agent. A claim with no artifact is recorded as unverified.

- **Covers:** `05` §2.8.
- **Prior art:** PR #30507 in t3code proposes recording execution evidence and states that none exists today (T3-030). orca checks push and PR claims against GitHub (ORC-054), which is the same idea applied to one step.
- **Design question:** how much of the ledger is derived from git versus from the agent's report.

### 4.4 Integration journal and state machine

Each integration has explicit states, for example: planned, candidate-building, candidate-built, verifying, conflicted, repairing, ready-for-review, abandoned, and done. Every transition is written before its side effect, and recovery follows the cloudroom rule: uncertain effects are not repeated automatically.

- **Covers:** `05` §2.9.
- **Prior art:** cloudroom's journal and receipts (CR-012, CR-013, CR-016), orca's removal journal (ORC-049), t3code's outbox (T3-018, T3-019).
- **Design question:** whether the journal is the source of truth or a checkpoint for a derived state. See §2.3.

### 4.5 Bounded repair loop

When verification fails, attempt a repair on a separate branch with a fixed attempt budget. Record every failed attempt with its evidence. Escalate to the user when the budget is spent.

- **Covers:** `05` §2.10.
- **Prior art:** t3code caps PR-watch wakes at 10 (T3-047). orca circuit-breaks after 3 dispatch failures (ORC-038). cloudroom has one automatic recovery and no repair loop (CR-017). Copilot's watch-and-fix has an undocumented bound (04).
- **Design question:** who performs the repair (a fresh agent, the original agent with the failure context, or the user), and how the context is passed. The 82% result in `04` (agents told about the concurrent change) suggests passing context matters as much as the budget.

### 4.6 Review surface

A combined diff with per-task provenance, the evidence ledger, and the integration state. The reviewer sees what was integrated, what was verified, and what was not.

- **Covers:** the human gate before ship. None of the surveyed tools shows a combined review (04).
- **Prior art:** orca's per-worktree git review (ORC-051 to ORC-053), t3code's diff derivation from checkpoints (T3-049).

### 4.7 Dispatch and agent control (table stakes)

Starting, stopping, and steering agents. This is necessary, and crowded (04). Ilvuno should consume dispatch from existing systems or build the minimum needed, and not make it the product.

---

## 5. Architecture decisions to defer

Each decision below has a trade-off the research supports. None is decided here.

### 5.1 Where the source of truth lives

- **Option A: a headless runtime that stays up** (orca, t3code). Live continuity across devices; the host must be running; mobile is online-only (`02` decision 3).
- **Option B: snapshot and resume on the other side** (cloudroom Teleport). Works when the laptop is off, but uncommitted edits to tracked files stay on the laptop, and the handoff is not live (CR-019 to CR-022).
- **What decides it:** whether Ilvuno's target users need offline mobile use, and whether work must continue when their machine sleeps. Those are product questions, not research questions.

### 5.2 Where agents run

- Local worktrees (orca, t3code, the local side of cloudroom), cloud sandboxes (cloudroom's hosted side, Cursor, Copilot), or both.
- Trade-off: local is cheap and private; cloud isolates the agent from the user's machine but needs credential handling (CR-038) and a hosted service the research did not inspect (`03` Unverified items).

### 5.3 How integration state is shared between devices

If the runtime is headless (§5.1 A), integration state lives there. If work moves (§5.1 B), the integration journal must move with it, and cloudroom's manifest-and-activation pattern (CR-019) is one model to study.

### 5.4 Process supervision

orca bounds daemon respawns to 5 per 60 seconds (ORC-060) and describes cgroup scopes (`02` decision 2). t3code's process ownership and orphan cleanup are Linux-only (T3-041). cloudroom's containment escapes in container mode (CR-035). Containment quality varies by OS. Ilvuno must state its containment per platform.

### 5.5 Storage engine and event schema

Several systems use SQLite. The choice of engine is less important than the rules: transaction boundaries, retention, and whether the log is replayable (§2.3). Defer until those rules are fixed.

---

## 6. Security baseline (derived from the anti-patterns)

Ilvuno should begin with these, because retrofitting them is expensive (§3 shows each one failing somewhere):
1. Guards that fail closed. If the check errors, the action is refused.
2. Approval and permission defaults that are restrictive, with an explicit opt-in to broader modes.
3. Agent credentials scoped to one task and one worktree, with expiry.
4. Constant-time comparison for every token.
5. No silent trust on first contact for remote hosts.
6. Secret storage with no plaintext fallback. If secure storage is unavailable, refuse to store the secret and say so.
7. Signed webhooks.
8. A verification gate that is not skipped with `--no-verify`.
9. No custom crypto without a version-pinned, downgrade-proof handshake.
10. Explicit reporting of what the containment does and does not cover on each platform.

---

## 7. Mapping: components to failure modes

| Failure mode (`05`) | Component that addresses it | Status in surveyed tools |
|---|---|---|
| 2.1 Textual conflicts | 4.1 candidate builder | Partial per branch |
| 2.2 Semantic conflicts | 4.2 verification, plus a compatibility check not yet specified | Open |
| 2.3 Dependency conflicts | 4.2 with dependency-aware candidate setup | Open |
| 2.4 API/interface mismatch | 4.2 with contract checks, if projects define them | Open |
| 2.5 Stale base | 4.1 rebuilds candidate from current base | Partial at creation only |
| 2.6 Shared-workspace collisions | Design rule: isolation by default (§6) | Open, and a default in some tools |
| 2.7 Test failures on combined result | 4.2 | Open |
| 2.8 Unverified completion | 4.3 evidence ledger | Open |
| 2.9 Interrupted integration | 4.4 journal | Partial primitives only |
| 2.10 Failed repair | 4.5 bounded loop | Partial per PR |

---

## 8. Three MVP directions

Each is a bounded slice that tests the core claim from `05` §5. Each is evaluated against the research, not against market demand, which is not measured (04 cautions).

### MVP A: Integration candidate and verification (no dispatch)

Input: N existing branches or worktrees, from any source (manual, orca, t3code, cloudroom, or a script). Output: a candidate branch, a merge report, the project's checks run on the candidate, and an evidence record.

- **Tests:** the core hypothesis that combined verification catches failures per-branch CI misses. It also tests how often conflicts actually occur (`05` §7).
- **Strengths:** works with any agent, so it complements the crowded dispatch market. Small, testable, and the differentiator itself (4.1, 4.2, 4.3).
- **Weaknesses:** no dispatch, so less of a workspace. Depends on projects having runnable checks. Could be absorbed by a vendor adding an integration step (04).

### MVP B: Dispatch plus integration for one harness

Dispatch N tasks to one agent (for example, Claude Code) into isolated worktrees, then run the integration loop.

- **Tests:** the full loop, including repair, end to end.
- **Strengths:** a complete product path. Dispatch and worktree safety are well understood from orca and t3code (§2.4).
- **Weaknesses:** closest to the crowded category (Conductor, Paradev, Sculptor, Vibe Kanban, 04). Depends on one vendor's harness. Risks spending most effort on dispatch, which `04` says is commoditized.

### MVP C: Pre-flight compatibility scanner

Before dispatch, analyze planned work for likely conflicts: overlapping file sets, shared dependency changes, and changed interfaces. Report risk; do not merge.

- **Tests:** whether predicted conflicts match real ones, which needs measurement.
- **Strengths:** small, cheap to build, and it directly addresses the open semantic and dependency gaps (2.2, 2.3).
- **Weaknesses:** a thin product on its own. Clash and Grove already do read-only prediction (04, Unverified), and the Django result suggests real conflicts may be rare (`05` §5), so the value is unproven.

---

## 9. Recommended direction for architecture

Build MVP A first, and design it so that dispatch can be added later as MVP B without changing the core.

Reasons, all tied to the evidence:
- The integration step is the gap every surveyed system leaves open (`05` §3, Inferred from absence).
- MVP A can be fed by the other tools. Their worktree outputs are inputs, not competitors, so the product does not depend on winning the dispatch market (`04` §6).
- It tests the central hypothesis directly, before a dispatch layer is built around it.
- It creates the evidence ledger and journal as first-class parts of the product, which is where the research found the most weakness (§3).

Conditions and risks:
- **Measure first.** Before committing, measure on real repositories how often independently produced branches conflict or fail after combination (`05` §7 questions 1 and 2). If the rate is very low, MVP C's value collapses and the pitch changes.
- **Verification depends on the project.** Projects without runnable checks get little value from MVP A. That limits the early user base to projects with tests.
- **Vendor response.** Vendors can add an integration step to existing surfaces (`04` cautions). The defensible part is cross-tool, cross-agent verification with evidence, not any single agent's feature.
- **No stack decision yet.** Candidate construction is git plumbing and works with any stack. The verification runner depends on project tooling. The choice of language, storage, and UI is deferred to after the measurement above.

Decisions that remain open and need an owner:
1. Source of truth and offline behavior (§5.1).
2. Whether the integration journal is authoritative or derived (§2.3).
3. Containment target per platform (§5.4).
4. Who performs repair, and how much context a repair attempt receives (§4.5).

---

## 10. Limits of this document

- Built on static reading at pinned commits. Runtime behavior was not observed.
- Architecture recommendations assume the research's absence claims hold. A feature may exist in an unread area (`02`, `03` Unverified items).
- No user, adoption, or revenue data was used. The market claims are limited to what `04` could verify.
