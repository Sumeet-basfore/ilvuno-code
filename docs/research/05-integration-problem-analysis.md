# Integration Problem Analysis

Date: 2026-11-19. Scope: how independently successful agent tasks fail when combined, and which systems address each failure. Sources are the three repository analyses (`01-t3code-analysis.md`, `02-orca-analysis.md`, `03-cloudroom-analysis.md`) and the landscape study (`04-competitive-landscape.md`). Evidence IDs (T3-, ORC-, CR-) resolve in `07-evidence-index.md`. Confidence tags follow the project convention: Confirmed, Inferred, Unverified.

Nothing here was measured on real repositories. Where this document says a failure is common, the basis is a preprint or reasoning, and it is labelled that way.

---

## 1. The core claim, stated precisely

Every surveyed system verifies an agent's change against that agent's own base: its worktree, its branch, or its sandbox. A change that passes there can still fail once it meets another change. The three repositories and the landscape show the same pattern. Each stops at one branch or one pull request per task. None builds a combined candidate and tests it.

Confirmed across sources:
- t3code has no code-level merge of child worktrees. `merge_back` moves conversation context only (T3-048, Confirmed).
- orca's coordinator receives `merge_ready` messages and ignores them (ORC-041, Confirmed).
- cloudroom's RepoSync pushes with `--no-verify` and runs no tests (CR-032, Confirmed).
- Copilot's coding agent produces one branch and one PR per task (04, landscape, Confirmed). Claude Code's `/batch` produces one PR per unit (04, Confirmed).

This is a claim about the absence of a feature in the code and docs that were read. It is Inferred, not proven, and document 04 says the same.

---

## 2. Failure modes, mechanism by mechanism

For each mode: why an independently successful change can fail once combined, what the surveyed systems do, and the status.

### 2.1 Textual git conflicts across parallel branches

**Mechanism.** Two branches edit overlapping hunks. Git refuses the merge. This is the most visible failure, and the only one git reports on its own.

**What exists.**
- t3code refuses to commit while conflict markers remain (T3-046). Its PR watch wakes the agent on `conflicting` and `needs-rebase` (T3-047). Resolution is left to the agent or user.
- orca detects conflicts per worktree (MERGE_HEAD, rebase state) and offers an "AI resolve conflicts" action (ORC-052, Confirmed). A PR merge is blocked when `merge-tree` reports conflicts against base (ORC-053, Confirmed).
- cloudroom's RepoSync runs `rebase`, aborts on failure, sets result `conflict`, and tells the agent to resolve it (CR-031, Confirmed).
- Copilot's Agent Merge can resolve conflicts on one open PR (04, Confirmed).

**Status: Partial.** Each system helps resolve a conflict on one branch against its base. None resolves conflicts between two agent branches before either lands, and none reports a conflict between A and B that would appear only after A merges.

### 2.2 Semantic conflicts with no textual conflict

**Mechanism.** Git merges cleanly, but the result is wrong. Examples: both branches add an exported symbol with the same name in different files; one renames a function while another still calls the old name; both change a shared schema in compatible-looking but incompatible ways.

**What exists.** Nothing surveyed checks this. cloudroom's own analysis says that shared-folder children "see each other's edits live but are not coordinated" (CR-024, CR-028, Inferred absence). orca has no combined typecheck or build; only per-PR CI is observed (ORC-054, Inferred absence). The landscape's Clash and Grove, which predict cross-worktree conflicts, are read-only and their claims about symbol overlap are Unverified (04).

**Status: Open.** This is the mode where a typed contract layer could help. t3code's typed Effect Schema contracts (T3-005) and orca's RPC contract (ORC-008) show that typed interfaces already exist inside the products. None of them checks an agent's output against another agent's output.

### 2.3 Dependency conflicts (lockfiles, versions, installs)

**Mechanism.** Two agents add different versions of the same package. Lockfile merges produce inconsistent state. One agent adds a dependency that the other worktree never installs, so its tests pass only in its own environment.

**What exists.**
- t3code setup scripts run per worktree but do not reconcile lockfiles across agents (T3-074, Inferred). Its handoff does not address deps.
- cloudroom's handoff says "Project dependencies are yours to resolve" and worktree rebuilds do not keep deps (CR-021, CR-030, Confirmed).
- orca runs setup and archive hooks per worktree, gated by trust (ORC-046, Confirmed). Nothing compares dependency changes across worktrees (Inferred absence).

**Status: Open.** This mode gets worse as each agent gets its own environment, which is the design most of these systems use.

### 2.4 API and interface mismatches

**Mechanism.** Agent A changes a contract (an RPC method, an event schema, a public function signature). Agent B, working from the old contract, produces code that builds against its own base and fails after integration.

**What exists.** Nothing combined. The per-PR CI in the systems catches failures in one PR only (ORC-054, CR-032). The landscape notes Factory validates per feature and does not describe combined validation (04).

**Status: Open.** Same comment as 2.2: the building blocks (typed contracts, CI) exist, but no step compares one change to another.

### 2.5 Stale branch state

**Mechanism.** Agent B branches from a base that has since moved. Its tests passed on the old base. Merging after A has landed is where it breaks.

**What exists.**
- orca fetches the base before creating a worktree and rolls back on failure (ORC-045, Confirmed). Its drift probe refuses dispatch when a worktree is more than 20 commits behind, but only when a coordinator worktree exists, and it fails open if the probe itself fails (ORC-039, ORC-040, Confirmed). The probe is the nearest thing to a stale-base check in the surveyed code, and it has a gap.
- cloudroom's RepoSync fetches before push, retries once, and refuses to push others' commits (CR-031, CR-029, Confirmed). If the remote base ref is missing but a local copy exists, the worktree is based on that local copy without a refresh. Other base-fetch failures throw (CR-061).
- t3code runs `startFromOrigin` and base-ref checks at launch, and `WorktreeBranchSync` reconciles branch records (T3-033, T3-073, Confirmed). It reports `needs-rebase` but does not rebase (T3-047).
- Claude Code fetches the base if older than 24 hours (04, Confirmed).

**Status: Partial.** Each system protects the moment a worktree is created. None re-checks the base at integration time, after other changes have landed.

### 2.6 Shared-workspace collisions (before any merge)

**Mechanism.** Agents share one folder or worktree, so their edits interleave in real time. Tests run against a mixed state. One agent's "done" refers to a state that another agent has since changed. Nothing is merged, so there is no conflict to report, and the failure is harder to see.

**What exists.** This is the design choice that most directly causes it.
- orca's coordinator creates all workers in the coordinator's single worktree. `workerStart` defaults to the caller's worktree (ORC-033, ORC-090, Confirmed). Issue #42415 is open on the default (ORC-090, title read).
- cloudroom's `--shared-sandbox` children run in the parent's same folder (CR-024, Confirmed). Workflows fan out up to 8 concurrent agents into one environment workspace (CR-028, Confirmed). Open issue #65 reports shared-sandbox fan-outs OOM-killing the coordinator (CR-044, title read).
- t3code's `create_threads` puts up to 20 threads in the parent's checkout (T3-031, Confirmed, doc). Checkpoint restore is refused in shared workspaces (T3-039, Confirmed).
- Folder workspaces in orca share the repo path. Issue #44715 reports cleanup removing an in-use worktree in that mode (ORC-050, ORC-095, Confirmed).

**Status: Open, and self-inflicted.** All three products offer a shared-folder mode, for speed or for parallel children. That mode removes the isolation that the rest of their design relies on.

### 2.7 Test failures on the combined result

**Mechanism.** Each task passes its own tests. The integrated result fails tests that exercise both changes.

**What exists.**
- t3code: CI results from the forge wake the agent after a push (T3-047, Confirmed). No local run on a combined result.
- orca: GitHub check status is watched and checks can be rerun. Merge is gated on required checks through GitHub. Orca never runs tests locally (ORC-053, ORC-054, Confirmed).
- cloudroom: RepoSync pushes with `--no-verify` and has no CI gate (CR-032, Confirmed).
- Landscape: Cursor's cloud agents build and test in their own VM and attach screenshots and video (04, Confirmed). Copilot runs tests and linters per PR (04, Confirmed).

**Status: Partial, and only per branch.** Every test run is against one branch. The combined-result run is absent from all surveyed systems.

### 2.8 Unverified agent completion claims

**Mechanism.** The agent says the task is done and the work is correct. The claim is recorded without being checked against artifacts such as a commit SHA, a test run, or a file list.

**What exists.**
- t3code: a delegated task returns a text summary. PR #30507 notes that the parent has "nothing host-recorded to check", and it proposes recording execution evidence (T3-030, Confirmed).
- orca: `worker_done` sender and IDs are checked, but the outcome and `filesModified` are self-reported (ORC-042, Confirmed). PR-opened and push claims are checked against GitHub (ORC-054, Confirmed).
- cloudroom: a child's result is its last reply text, truncated at 32 KiB (CR-026, Confirmed). A `TaskCompleted` hook can block completion, but only in agent teams (04, Confirmed).
- Cursor artifacts (screenshots, video) are evidence of a kind, but they are per agent and not tied to a combined state (04).

**Status: Open at the system level, Partial at the per-task level.** The ledger that would tie a claim to a commit SHA and test output does not exist in any surveyed product.

### 2.9 Partial execution and interrupted integrations

**Mechanism.** The process dies mid-integration: after some branches are merged and before others, or after a push but before the record is updated. Recovery must know what happened and avoid repeating side effects.

**What exists.**
- cloudroom: dispatched work whose outcome is uncertain is marked `unknown(_after_restart)` and is never resent (CR-016, Confirmed). That is a sound principle, but it covers dispatch, not multi-branch merges. RepoSync writes a marker file and aborts its own half-finished rebase on the next check (CR-031, Confirmed).
- orca: interrupted worktree removals are journaled and resume (ORC-049, Confirmed). An in-progress rebase or merge can be aborted, but finishing it in-app is incomplete (ORC-098, title read; issue #25346).
- t3code: command receipts make retries idempotent, and the outbox retires or retries side effects after a crash (T3-024, T3-025, Confirmed). Retained events are pruned to about one day, and projections cannot be rebuilt from the log (T3-021, Confirmed).

**Status: Partial.** The durability primitives exist and are good (journals, idempotent receipts, outbox). None is applied to an integration step, and none records the state of a multi-branch merge in progress.

### 2.10 Failed repair attempts

**Mechanism.** The integrated result fails. The system tries to fix it, and the fix fails too. Without an attempt budget and a record of failed attempts, the loop either never ends or hides the failure.

**What exists.**
- t3code: the PR watch has a cap of 10 consecutive comment-only wakes (T3-047, Confirmed). Usage limits are resumed (T3-027). There is no repair loop beyond that.
- orca: a circuit breaker opens after 3 dispatch failures (ORC-038, Confirmed). Convergence logs "Stuck" and does not repair (ORC-043, Confirmed). "AI resolve conflicts" is one-shot and user-initiated (ORC-052).
- cloudroom: one automatic harness recovery, then `process_lost`. Provider retries cap at 4. There is no agent-level repair loop (CR-017, CR-049, Confirmed).
- Copilot's Agent Merge and watch-and-fix work on one PR. Its retry bound is not documented (04, Confirmed for behavior, Unverified for the bound).

**Status: Partial at the per-PR level, Open for a repair loop on a combined result.** Each system has a retry limit, but each limit guards dispatch or one PR, not a combined integration. None keeps a record of the failed attempts.

---

## 3. Status summary

| Failure mode | t3code | orca | cloudroom | Best surveyed analogue | Status for combined work |
|---|---|---|---|---|---|
| 2.1 Textual conflicts | Partial (T3-046, T3-047) | Partial (ORC-052, ORC-053) | Partial (CR-031) | Copilot Agent Merge, per PR (04) | Partial, per branch only |
| 2.2 Semantic conflicts | Open | Open | Open | Clash/Grove, read-only, Unverified (04) | Open |
| 2.3 Dependency conflicts | Open (T3-074) | Open | Open (CR-021) | None | Open |
| 2.4 API/interface mismatch | Open | Open (ORC-054) | Open (CR-024) | Factory, per feature (04) | Open |
| 2.5 Stale base | Partial (T3-033, T3-047) | Partial (ORC-039, ORC-040) | Partial (CR-031, CR-029) | Claude Code, 24 h fetch (04) | Partial, at creation only |
| 2.6 Shared-workspace collisions | Open (T3-031, T3-039) | Open (ORC-033, ORC-090) | Open (CR-024, CR-028) | None | Open, and default in some modes |
| 2.7 Test failures on combined result | Partial, post-push (T3-047) | Partial, post-push (ORC-053) | Open (CR-032) | Cursor VM tests (04) | Open |
| 2.8 Unverified completion | Open (T3-030) | Partial (ORC-042, ORC-054) | Open (CR-026) | Claude TaskCompleted hook (04) | Open |
| 2.9 Partial and interrupted integration | Partial (T3-024) | Partial (ORC-049) | Partial (CR-016, CR-031) | None for integration | Open |
| 2.10 Failed repair | Partial (T3-047) | Partial (ORC-038) | Partial (CR-017) | Copilot watch-and-fix (04) | Open |

No mode is Solved for the combined case.

---

## 4. What is already commoditized

These are solved well enough that leading with them would not differentiate Ilvuno (04, Confirmed):
- worktree-per-task isolation and parallel session dashboards;
- diff review and PR creation;
- per-PR conflict help and CI watching;
- crash-safe journals, idempotent commands, and outboxes (strong in t3code and cloudroom);
- safe worktree removal (orca's clean-check, proven-merged branch delete, and journaled removal, ORC-047, ORC-048, ORC-049).

---

## 5. The credible gap, and the case against it

**Gap.** The integration step itself: build a candidate from N independent branches, check it for textual, semantic, dependency, and interface problems, run the project's own checks on it, record evidence, and repair with a bounded budget. The surveyed products cover the per-branch parts. None covers the combined step (Inferred, from absence).

**Counter-evidence that must shape the product.**
- AgenticFlict reports that 27.67% of about 107,000 simulated agent PRs had textual conflicts (preprint arXiv 2604.03551, Confirmed as reported in 04). The rate is for simulated PRs, not real ones.
- The "stale" benchmark found 97% interference on constructed tasks, but only 1 of 834 runs on real merged Django PR pairs conflicted. Its authors say the constructed rate does not estimate real frequency (preprint arXiv 2609.25396, Confirmed as reported in 04).
- The same benchmark found that telling an agent about the concurrent change recovered 82% of runs (04, Confirmed as reported). That result points to context sharing, not only detection, as a lever.

So the real-world conflict rate for careful workflows may be low. The value of combined integration depends on that rate, and nobody has measured it for the kind of work Ilvuno targets. This is the first open question, and it should be answered before the product is built.

**Implication for the framing.** Conflict detection alone may be the weakest version of the idea. The stronger hypotheses are:
1. A combined candidate that is tested, with evidence attached, catches failures that per-PR CI does not.
2. Sharing the concurrent change with the agent (the 82% result) prevents failures that detection would only report.
3. An evidence ledger makes "done" checkable, which none of the surveyed products provides.

---

## 6. Refined problem decomposition

"Integration Intelligence" breaks down into six separable capabilities. Each maps to failure modes above:

| Capability | Question it answers | Failure modes | Prior art |
|---|---|---|---|
| Candidate construction | Can these N changes be combined at all, and in what order? | 2.1, 2.5 | `git merge-tree` in orca (ORC-053); RepoSync's rebase logic (CR-031) |
| Pre-merge compatibility analysis | Do the changes collide semantically, in dependencies, or in interfaces? | 2.2, 2.3, 2.4 | Clash/Grove (read-only, Unverified); typed contracts (T3-005, ORC-008) |
| Combined verification | Does the candidate pass the project's checks? | 2.7 | Per-PR CI only (ORC-054) |
| Evidence ledger | Is each "done" tied to an artifact (SHA, command, exit code)? | 2.8 | None; PR #30507 proposes it (T3-030) |
| Resumable integration | Can an interrupted integration be finished or rolled back safely? | 2.9 | Journals (CR-016, ORC-049); outbox (T3-024) |
| Bounded repair | Does a repair attempt stop, and is every failed attempt kept? | 2.10 | Attempt caps (T3-047, ORC-038, CR-017) |

The shared-workspace collision (2.6) is not a merge problem. It is a design choice upstream. The integration layer can reject or flag it, but it is also an argument against the shared-folder modes that two products offer.

---

## 7. Questions to answer before committing to a design

These need measurement on real repositories, not more reading:
1. What fraction of independently produced agent branches on the same repository conflict textually, semantically (broken build or failing tests on the combination), or in dependencies? Measure on the target users' own repositories.
2. How often does a combined candidate fail where every individual branch passed? This is the direct test of the core claim.
3. Does giving an agent the concurrent change reduce combined failures enough to justify the added context cost?
4. How often do lockfile and dependency collisions occur, and do they dominate textual conflicts?
5. Can a combined-candidate test run finish fast enough to be part of the loop? The answer depends on the repository's test cost, not on the tool.

Each question has a clear observable outcome. None of them can be answered from the repositories, which is why the product gap is recorded as Inferred.

---

## 8. Limitations of this analysis

- All repository claims rest on static reading at pinned commits (`da33a53d` for t3code, `e3484730` for orca, `21b5c6df` for cloudroom). Runtime behaviour was not observed.
- Absence claims ("nothing merges X") rest on greps and reading of the relevant directories. A feature could exist in an area that was not read. The orca and cloudroom analyses name their unread areas (`02` Unverified items, `03` Unverified items).
- Issue and PR statements rest on titles and state, not bodies, except where a document says otherwise.
- The landscape study's absence claims are limited to documentation read, and two competitors (Graphite, Devin) could not be read in primary form (04, Unverified).
