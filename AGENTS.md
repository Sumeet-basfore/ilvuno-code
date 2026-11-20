# AGENTS.md — Ilvuno Engineering Policy

**Name:** Ilvuno
**Tagline:** Parallel work. Verified together.
**Mission:** Build an integration-first multi-agent coding workspace: take independently produced
agent changes, prove whether they work *together*, and leave evidence a reviewer can check.

This file is the shared engineering policy for every agent and human working in this repository.
`CLAUDE.md` imports it; Codex reads it directly. Tool-specific guidance belongs in `CLAUDE.md`,
never here.

---

## 1. Repository status — read this before doing anything


| Fact               | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Phase              | M0 implementation, authorized 2026-11-20 (see Authorization). Research, PRD, architecture and roadmap are complete.                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Code today         | M0 code is being added by T-0001 to T-0005. Before T-0001 there was no `go.mod`, `Makefile` or CI.                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Authorization      | **M0 authorized 2026-11-20** by the project owner (`docs/readiness/06-m0-readiness-checklist.md` §7), for tasks T-0001 to T-0005 only. Gate G0 is not yet passed. **M1 and later milestones are not authorized.** Decision status: `docs/decisions/open-questions.md`. Reconciliation and entry-gate results: `docs/readiness/07-m0-reconciliation.md`.                                                                                                                                                                                                             |
| Decided 2026-11-20 | D-01…D-06 (`docs/decisions/open-questions.md` §0): the owner signs the readiness checklist before each milestone; V1 is a standalone CLI; Apache-2.0; Go; Linux-first with macOS tested in CI, Git ≥ 2.38.1; repository-committed `ilvuno.toml`. Planning items A-4 (Windows excluded from V1), S-1 to S-4, C-5 and C-7 are also decided (§0). D-16 (M0 lint: `gofmt`, `go vet`, import-rule test), D-17 (canonical repository and module path), D-18 (Apache-2.0 LICENSE text) and D-19 (M0 delivery shape: two PRs, T-0001/T-0005 scope amendments, deferrals and exclusions) are recorded. All other decisions are OPEN. These approvals do not authorize M0 on their own; M0 is authorized separately by the signed record in `docs/readiness/06` §7. |
| Harness status     | SP-1R3 has **APPROVE WITH CONDITIONS** as a controlled M0 *reference harness* (`docs/reviews/sp1r3-final-independent-verdict.md`). That verdict covers the experimental harness only. It authorizes no milestone and no production code.                                                                                                                                                                                                                                                                                                                            |
| Authoritative plan | `docs/planning/01-master-roadmap.md`, `docs/planning/02-milestones.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Task specs         | `docs/tasks/T-NNNN.md` — one file per task, with its own acceptance criteria                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |


If you are asked to implement something that is not a defined task, stop and say so (§10).

---

## 2. Reading order

Read in this order; do not skip to code.

1. `AGENTS.md` (this file)
2. `docs/planning/01-master-roadmap.md` — what is approved, what is not
3. `docs/planning/07-definition-of-done.md` — what "done" means here
4. The task spec you are working on: `docs/tasks/T-NNNN.md`
5. Only the architecture/ADR documents that task cites

Do not read the entire `docs/` tree by default. These are large documents; load what the task
requires. If you need a document the task does not cite, say which one and why.

---

## 3. Repository structure

Current (documentation only):

```
docs/research/        7 evidence-backed analyses (read-only inputs)
docs/product/         vision, PRD, personas, journeys, MVP scope, monetization, metrics
docs/decisions/       open questions requiring approval
docs/architecture/    12 design documents
docs/adr/             15 architecture decision records + README
docs/planning/        7 planning documents (roadmap, milestones, gates, risks, DoD)
docs/tasks/           task specifications
```

Planned, created by task `T-0001` (do not create ahead of it):

```
cmd/ilvuno/                 binary entry point
internal/cli/               command surface, exit codes
internal/{model,gitx,repo,engine,analyze,verify,evidence,passport,policy,proc,redact,journal,config}
internal/testkit/           forge, fixtures, safety fingerprint, crash harness
internal/lint/importrule/   architecture-boundary analyzer
schemas/                    versioned JSON Schemas
```

**Architecture boundaries are enforced, not advisory.** Dependency direction
(`internal/engine` → `{gitx, analyze, verify, evidence, model, policy, proc}`; `internal/agent`
never imported by `engine`; `model`/`policy` depend on the standard library only) is asserted by
`internal/lint/importrule`. A change that violates it is rejected, not discussed.

---

## 4. Engineering rules

### 4.1 Code conventions (Go, per ADR-0001; S-2)

- Go is the only production language. Python 3, Bash and Docker are permitted for development and test tooling only (S-2, approved 2026-11-20). None of them ships in the `ilvuno` binary.
- `gofmt` and `go vet` clean. `staticcheck` clean once it is approved as a tool dependency. **M0 lint is `gofmt`, `go vet` and the in-repository import-rule test only** (D-16, approved 2026-11-20); `golangci-lint` is not used. No new dependency without an ADR reference or an explicit owner approval.
- Package names match their directory; one owner per package (see §7).
- Errors are typed with a **stable code**, an exit class, and a `next` action
(`docs/planning/07-definition-of-done.md` §4). No `fmt.Errorf("failed")` reaching a user.
- Never parse git's human-readable output; parse `--porcelain`, `-z`, `rev-parse`,
`for-each-ref --format`, `merge-tree --write-tree` only (ADR-0004).
- No absolute paths, hostnames, or environment values in any persisted artifact
(`INV-9`, `INV-13`).
- Git is invoked only through `internal/gitx`. Nothing else runs `git` (ADR-0004).
- Timestamps are UTC RFC3339 with milliseconds; sequence numbers are authoritative, clocks are not.

### 4.2 Determinism

Correctness-critical output must be reproducible: same inputs + same binary + same git version ⇒
same candidate SHA and same normalized report (ADR-0005, ADR-0007). If your change introduces
wall-clock, locale, filesystem-ordering, or map-iteration dependence into a report or a commit,
it is a bug even if the tests pass on your machine.

### 4.3 Prohibited shortcuts

- No `--no-verify` equivalent. Verification cannot be skipped by a flag; it is disabled only by
editing the committed config, which is reviewable.
- No fail-open guards. If a check errors, refuse (ADR-0008, `docs/architecture/09-security.md`).
- No "temporary" writes outside `refs/ilvuno/*`, `.ilvuno/**`, or `docs/ilvuno/tasks/`.
- No rewriting another agent's branch history.

### 4.4 Commits

Commit only a **coherent, reviewable change**: one logical unit of work per commit, complete enough
that its checks pass on that commit. Do not split work just to increase the commit count, and do not
bundle unrelated cleanup, refactoring, or formatting into it.

Before committing:

1. Inspect the full staged diff (`git diff --cached`). Confirm every file is inside the task's
 "Allowed file scope" (§7, §10.2).
2. Confirm nothing temporary, generated, or scratch is staged: build output, coverage files, editor
 backups, local notes, `.worktrees/`, `.ilvuno/`. Unstage them. Do not commit them.
3. Run the checks the task spec names, from the task's worktree (§7). If a named check does not exist
 yet, record it as a blocker. Do not substitute another check, and do not report it as passed (§12).

Message format: Conventional Commits, scoped to the package, with the task ID in the subject:

```
feat(engine): add candidate merge-order report (T-NNNN)
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`, `ci`. The subject is imperative and
says what changed. Add a body only when the reason is not obvious from the diff.

After committing, report: the commit SHA(s), the files touched, each check run with its exact command
and result (or the blocker), and the worktree path.

Do not push. Wait for explicit human review of the commit(s) before pushing the branch (§8).

---

## 5. Testing requirements


| Rule                                 | Detail                                                                                                                               |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| Failing test first                   | Every new behaviour ships with a test that fails without the change. Bug fixes reproduce the bug.                                    |
| Fixtures, not hand-built repos       | All repository fixtures come from `internal/testkit/forge` (deterministic identity, dates, `TZ=UTC`, `GIT_CONFIG_GLOBAL=/dev/null`). |
| Safety test                          | Any task that writes must include a `T-SAFE-*` test asserting the user's refs, tree, index, stashes, and `HEAD` are unchanged.       |
| Determinism test                     | Changes to the engine or report must keep `DET-1` green (two runs, two OSes, identical output).                                      |
| Crash test                           | Changes to journal/engine/verify must extend the crash matrix (`T-0503`), not bypass it.                                             |
| No live network, no live model calls | Benchmarks and scenario tests are offline and deterministic.                                                                         |
| Coverage floors                      | `engine`, `gitx`, `verify`, `evidence` ≥ 80%; `model` ≥ 90%. A floor is not a goal.                                                  |


A passing test suite that does not assert the user-visible behaviour does not satisfy this section.

---

## 6. Documentation requirements

Update documentation in the same change that alters behaviour:


| Change                       | Doc to update                                                                  |
| ---------------------------- | ------------------------------------------------------------------------------ |
| CLI command, flag, exit code | `docs/` (created with T-0605), `--help` text, exit-code table                  |
| Config key or default        | config reference (§10 of `docs/architecture/07-verification.md` is the source) |
| Report or passport field     | schema + `docs/` format page; additive only within a major (ADR-0014)          |
| User-facing wording          | check against the banned-claims list (`docs/product/02-prd.md` Appendix A)     |
| Structural decision          | a new ADR; never edit an accepted decision to match the present                |


Long architectural detail lives in `docs/architecture/` and is **linked**, not copied into this
file or `CLAUDE.md`. If you find yourself explaining a subsystem in a commit message, link instead.

---

## 7. Task ownership and worktrees

- Exactly one task in flight per agent. Tasks are ≤ 3 engineering days; if yours is larger, split
the spec first and say so.
- **One owner per package at a time.** Ownership map: `docs/planning/03-dependency-graph.md` §5.
- Development worktrees: `.worktrees/<agent>-<task-id>/` on branch `feat/<task-id>`.
Do not use Ilvuno's own candidate worktrees (`.ilvuno/worktrees/`) for development.
- Never two agents in one working tree. Never share a branch.
- Concurrency cap: **two agents**. A third is adopted only if the C3 benchmark beats C2
(`docs/planning/04-benchmarks.md` §7).

---

## 8. Git and worktree workflow

```
1. git worktree add .worktrees/<agent>-<task-id> -b feat/<task-id>   (from origin/main)
2. work only inside that worktree and inside the task's "Allowed file scope"
3. commit one coherent, reviewable change at a time, in the format and with the checks in §4.4
4. after human review of the commits (self-review when the owner is the author) and a non-author
   engineering assessment of the same commit SHA, push the branch and open a PR; squash-merge with
   the task ID in the subject
5. delete the worktree after merge
```

Rules:


| Rule                                                                                                      | Reason                           |
| --------------------------------------------------------------------------------------------------------- | -------------------------------- |
| Never commit to `main` directly                                                                           | every change is reviewed         |
| Never merge a PR                                                                                          | a human merges; agents do not    |
| Never force-push                                                                                          | it destroys review evidence      |
| Never rebase another agent's branch                                                                       | the owner rebases its own        |
| Never `git reset --hard`, `git clean -xdf`, or delete branches you did not create                         | destructive, unrecoverable       |
| Frozen files during a wave: `internal/model/**`, `internal/policy/**`, `schemas/**`, `Makefile`, `go.mod` | they ripple across every package |


Conflicts: the package owner resolves, even when the other agent's change triggered the conflict.
If the same file conflicts twice in one wave, escalate (§10).

---

## 9. Agent collaboration protocol


| Concern                      | Rule                                                                                                                                                                                        |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Assignment                   | One task per agent, delivered as a task ID plus its spec. No "improve the codebase".                                                                                                        |
| Handoff artefact             | Diff + tests + updated spec + the 4-line self-review + an ambiguity note (`docs/planning/02-milestones.md` §11.3).                                                                          |
| Review                       | A human reviews every diff; where the author is the sole human, the owner's reading is recorded as self-review and a non-author AI engineering assessment supplies the independence — findings only, never approval. An agent may review another agent's diff for clarity and test quality, but **must not approve** `policy`, `redact`, `approval`, `journal`, or any release gate; it may record findings with a recommendation for the owner's decision. |
| Ownership transfer           | Announced before a wave starts, never mid-task.                                                                                                                                             |
| Disagreement on an interface | Escalate the same day; write the decision into the task spec. Do not "try both".                                                                                                            |
| Parallelism                  | Only pairs marked eligible in `docs/planning/02-milestones.md` §11.4.                                                                                                                       |


---

## 10. Mandatory safety policies

### 10.1 Never do these

1. Modify `main` without authorization.
2. Run a destructive operation without approval (`rm -rf`, `git reset --hard`, `git clean -xdf`,
 `update-ref -d` outside `refs/ilvuno/*`, branch deletion you did not create, `pkill`/`killall`).
3. Fabricate evidence — including plausible-looking logs, SHAs, durations, or test output.
4. Claim tests passed without having run them in this session.
5. Mark an integration verified without an observed verification result bound to a commit SHA.
6. Commit secrets, credentials, tokens, or `.env` contents.
7. Merge unreviewed changes.
8. Silently change approved architecture (ADRs, invariants, schemas, milestone scope).
9. Treat an AI-generated summary — yours or another agent's — as an authoritative execution record.
10. Deploy, publish, tag a release, or push to a package registry without explicit authorization.

### 10.2 Approval boundaries — stop and ask


| Action                                                                 | Approval required from                 |
| ---------------------------------------------------------------------- | -------------------------------------- |
| Editing anything outside the task's "Allowed file scope"               | the human who assigned the task        |
| Adding a dependency                                                    | maintainer (ADR amendment)             |
| Changing a schema in a non-additive way                                | maintainer (major version + migration) |
| Changing an invariant (`INV-1`…`INV-14`), gate criterion, or threshold | maintainer, in writing                 |
| Deleting a fixture, test, or benchmark scenario                        | maintainer                             |
| Touching `.github/`, release config, or git hooks                      | maintainer                             |
| Any operation on a repository other than this one                      | the owner of that repository           |
| Starting a milestone whose gate has not passed                         | maintainer                             |


### 10.3 Unknowns

State them explicitly and stop at the boundary. An agent that reports "I do not know whether X
holds, and here is how to find out" is doing its job; an agent that guesses and proceeds is not.
Required phrasing in the completion report: *Unknowns* and *Would need to verify*.

---

## 11. Definition of Done

A task is complete only when all of the following exist **as artefacts**, not as assertions.


| #   | Evidence                                                                                                                                                                                                               |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Code changes**, within the task's allowed file scope. Out-of-scope edits are listed and justified or reverted.                                                                                                       |
| 2   | **Passing required tests**: the exact commands from the task spec, with their output.                                                                                                                                  |
| 3   | **Accurate failure reporting**: any test, check, or benchmark that fails is listed with its output — never summarised as "mostly passing".                                                                             |
| 4   | **Documentation updates** for every user-visible change (§6).                                                                                                                                                          |
| 5   | **Git state summary**: branch, base SHA, commit count, files touched, whether pushed, worktree path.                                                                                                                   |
| 6   | **Review status**: who reviewed (or "self-review only — no second reviewer available"), plus the 4-line adversarial self-review: what surprised me / what I tried to break / what remains on kill / what I did not do. |
| 7   | **Remaining risks**: known limitations introduced or left, with the trigger that would make them matter.                                                                                                               |
| 8   | **Task spec updated** where reality diverged from the plan (scope changes are recorded, not hidden).                                                                                                                   |


Nothing on this list may be inferred. If the command was not run, the report says so.

---

## 12. Commands

There is no build system yet. The command contract is defined by task `T-0001` and
`docs/planning/07-definition-of-done.md`; once `T-0001` lands, `make build`, `make test`,
`make lint`, `make bench`, `make docs-check`, and `make gate G<N>` are the only supported entry
points.

Until then:


| Command                                  | Status                                                |
| ---------------------------------------- | ----------------------------------------------------- |
| `git`, `rg`, `ls`, `cat`, `jq`           | available                                             |
| `make build` / `make test` / `make lint` | **does not exist — do not invent or claim to run it** |
| `ilvuno …`                               | does not exist                                        |


If a task asks you to run a command that does not exist, stop and report it as a blocker. Do not
substitute a hand-rolled equivalent and call it the required check.

---

## 13. How this file is discovered

- **Codex** reads the `AGENTS.md` chain: `~/.codex/AGENTS.override.md` or `~/.codex/AGENTS.md`,
then one file per directory from the repo root down to cwd (`AGENTS.override.md` → `AGENTS.md`),
concatenated root→cwd, capped at `project_doc_max_bytes` (32 KiB default). This file is the
project-level entry.
- **Claude Code** reads `CLAUDE.md`. When a `CLAUDE.md` exists, Claude reads it *instead of* this
file; that is why `CLAUDE.md` imports this document with `@AGENTS.md`.
- Neither tool reads the other's file by default. **Do not assume an agent has read both.**
Anything that must bind every agent lives here; anything tool-specific lives in `CLAUDE.md`.

Verified against vendor documentation on 2026-11-19:
`code.claude.com/docs/en/memory` (Claude Code memory and AGENTS.md loading) and
`developers.openai.com/codex/agent-configuration/agents-md` (Codex instruction discovery).

---

## 14. Changing this file

Changes to this file are policy changes: they require human review and a one-line rationale in the
commit message. Do not weaken §10 without explicit maintainer approval, and do not add rules that
duplicate `docs/` content — link to it instead.

---

## 15. Git Branching and Pull Request Review Policy

> **Status: APPROVED 2026-11-20** by the project owner, as revised on 2026-11-20. This section is the
> Git workflow policy for this repository and governs where it conflicts with §§7–9. **Approval
> rationale (§14 record):** establish task-isolated development, independent SHA-bound review,
> CI-based verification, and owner-controlled merges. Approval authorizes no staging, commit, push,
> merge, branch-protection or repository-settings change, or milestone work by itself: those remain
> subject to §10, the task's allowed file scope, and the signed readiness record.

This section reconciles §§7–9 rather than replacing them. It is approved and in force for Git workflow;
the task worktree layout, the one-task-per-agent rule and the default `feat/<task-id>` branch name from
§7 remain. §4.4's no-push-before-review rule and §8's human review remain, and the owner authorizes every
push. §8's step 4 is read as "open the PR"; the owner performs the squash merge after the gate below.
One SHA-bound review may satisfy both the pre-push review and the PR review (see "One review for both
stages"). Where this section and §§7–9 conflict on Git workflow, this section governs.

### Branch isolation

- One branch per feature, bug fix, or logically independent task — not one branch per commit.
  Multiple related commits may share the branch.
- Branch names include the task ID: `<type>/<task-id>[-<slug>]`, where `<type>` is `feat`, `fix`,
  `docs`, `test`, `chore` or `ci`. The default remains `feat/<task-id>`. A branch without a task ID
  needs explicit owner approval.
- Branch from the reviewed base: `origin/main`, or the base named in the task for a stacked change.
- No direct development commits to `main`. No shared branch between agents.
- Parallel agents modifying the same repository use separate branches and separate worktrees
  (`.worktrees/<agent>-<task-id>/`, §7). Never two agents in one working tree.

### Pull requests

- Every change intended for `main` goes through a pull request. A PR is one logical change; unrelated
  cleanup, refactoring or formatting belongs in another PR.
- The PR body states: purpose and task/milestone reference; change summary and affected areas; tests
  performed with commands and results; known limitations; security or data-loss considerations where
  applicable; and any out-of-scope or pre-existing changes deliberately excluded.
- A PR that changes milestone scope, an ADR, an invariant, a schema, or a gate criterion says so
  explicitly and links the approval; the PR is not the approval itself.

### Independent review

- Every PR is examined by a reviewer other than the implementation author, who records an
  assessment of the exact head SHA. An author does not supply the independent assessment of their own
  change, and no agent may act as the sole reviewer for a change it wrote. Approval of policy,
  governance and release-gate records is the owner's decision and a separate act.
- The reviewer inspects the actual diff and the surrounding code, not the PR description alone.
- The review records the **head commit SHA** it examined and evaluates correctness, regressions,
  security, repository safety (refs, worktrees, user data), scope compliance, and test evidence.
- For implementation changes, the verdict is one of **APPROVE**, **REQUEST CHANGES**, or
  **INSUFFICIENT EVIDENCE**. Blocking findings name the file and line and the required fix. Silence or
  a summary is not approval. For policy, governance and release-gate records the record is findings
  with a recommendation, and the owner's decision is separate.
- New commits after a review invalidate it. The reviewer re-reviews or explicitly reconfirms against
  the new head SHA; an approval of an older SHA does not approve later commits. A rebase, amend,
  force-push, or squash-rebase onto the target branch produces a new SHA and therefore a new review.
- AI review is **engineering evidence**, not GitHub human approval. The two are recorded separately:
  the engineering review as a PR review comment or PR-body section naming its head SHA; the human
  approval as the GitHub review state from an account that branch protection accepts. An AI review
  never substitutes for a required human approval, and no agent may approve on the owner's behalf,
  dismiss a required review, or use an admin override.
- For `policy`, `redact`, `approval`, `journal`, or any release gate (§9), an AI reviewer may report
  findings but cannot be the approving reviewer; the owner decides. The required record in that case is
  findings with a recommendation, not an APPROVE verdict.

### One review for both stages

One SHA-bound independent review satisfies both the pre-push review (§8) and the PR review when all of
the following hold:

- The review examined the **exact commit** that was pushed, and that commit is still the PR head SHA.
  No commit, amend, rebase or cherry-pick exists between the review and the push.
- The assessment is by a reviewer who is not the author; for `policy`, `redact`, `approval`,
  `journal` or a release gate, the approval decision is the owner's (§9), recorded separately from the
  assessment.
- Independent review for the pre-push stage may be supplied before the PR exists: a non-author
  engineering assessment of the exact local commit SHA, attached or reproduced in the PR afterwards and
  reconfirmed at the same SHA, satisfies it. The human pre-push review in §8 is satisfied by the owner's
  reading of the commits, recorded as self-review (§11 item 6) when the owner is the author; self-review
  is never described as independent human review.
- After the PR opens, the reviewer reconfirms the **same SHA** in the PR thread ("reconfirmed at
  `<sha>`"). That comment, not the pre-push note, is what the merge gate reads.
- If the merge rewrites history, the owner confirms in the PR that the merged content contains nothing
  the reviewer did not see. A squash merge of an already-reviewed branch satisfies the gate; re-review
  of the squash commit is not required, but a squash that introduces content beyond the reviewed and
  verified integrated result is a new revision and needs a new review.

A second review is required only when a new SHA invalidates the first, when the first reviewer returns
REQUEST CHANGES or INSUFFICIENT EVIDENCE, or when a stack's layers were not each reviewed.

### Sole-maintainer review and human approval

The project owner is currently the only human approver. "A reviewer other than the author" is therefore
impossible by construction for owner-authored changes, and this policy does not pretend otherwise.

- An **AI reviewer other than the implementing agent** may supply the engineering assessment. It
  records its head SHA and that it is AI review, and either a verdict (APPROVE / REQUEST CHANGES /
  INSUFFICIENT EVIDENCE) for implementation changes, or findings with a recommendation for policy,
  governance and release-gate records.
- The standing sole-maintainer configuration is: PRs required for `main`, applicable CI required on
  the head SHA, an independent SHA-bound engineering assessment recorded against the exact commit —
  pre-push for implementation and for policy, governance and release-gate records, attached or
  reproduced in the PR after push — and explicit owner merge approval. This policy has no temporary
  branch-protection waiver, no admin override and no per-PR settings change.
- AI engineering review is recorded evidence, never human approval. It does not satisfy a GitHub
  human-review requirement, and this policy does not claim that it does.
- If the repository's GitHub settings require an approving review from a second human account, a sole
  maintainer cannot meet that requirement. Report the conflict and leave the PR open: do not change
  settings, do not merge around it, do not record a waiver. The owner resolves it outside the PR — by
  adding a human reviewer, or by changing settings as their own separate decision.
- **Never bypass active branch protection**: no admin override, no "merge without waiting", no
  dismissal of a required review, no force-push to a protected branch, and no agent action on
  repository settings. If the gate cannot be met, the PR stays open.

### Sole-maintainer authorization, assessment and approval (approved 2026-11-20)

Four separate acts are required on a sole-maintainer PR, and none substitutes for another.

1. **Owner authorization (§8, §10).** Policy and governance decisions, and every state-changing or
   remote Git operation — branch creation, commit on a shared branch, push, PR creation, merge — are
   authorized by the project owner in writing, naming the decision, branch or commit. Read-only
   inspection needs no authorization: `git status`, `git diff`, `git log`, `git show`, `git rev-parse`
   and equivalent queries remain permitted at any time. Authorization is permission, not review, and it
   never implies that a review happened.
2. **Pre-push independent engineering assessment.** Before a branch is pushed, a reviewer who is not the
   author examines the **exact local commit SHA** and records, in the handoff or the PR: that SHA,
   findings with file and line, and a recommendation. This runs locally and requires no GitHub PR. For
   ordinary implementation changes the record is a verdict — APPROVE, REQUEST CHANGES, or INSUFFICIENT
   EVIDENCE. For owner-authored `policy`, `redact`, `approval`, `journal`, governance and release-gate
   records the record is **findings with a recommendation, not an APPROVE verdict**, because §9 forbids
   an AI reviewer from being the approving reviewer of those artefacts.
3. **Post-push reconfirmation.** After the push, the assessment is attached to or reproduced in the PR,
   states that the PR head SHA is the same local commit it examined, and is reconfirmed at that SHA. A
   push that produces a different SHA invalidates the assessment and requires a new one.
4. **Owner decision and GitHub approval.** For `policy`, `redact`, `approval`, `journal`, governance and
   release-gate records, the owner reads the independent findings and records the decision themselves —
   approval or rejection with reasons. No AI verdict is required, and none may be presented as that
   approval. For ordinary implementation changes the gate requires an independent engineering APPROVE.
   Independently of both, when branch protection or a ruleset requires a human approval, only an
   eligible GitHub account can supply it: an AI assessment is not it, the owner's self-review is not it,
   and a green CI run is not it.
5. **What independence is not.** Where the owner wrote the change, the owner's pre-push reading is
   recorded as self-review (§11 item 6). It is human, and it is never described as independent human
   review. Independence is supplied only by a reviewer other than the author.

**Resolution of the pre-push / policy-review deadlock.** §8 requires human review of the commits before
push, §15 requires a non-author review, and §9 forbids an AI reviewer from approving policy. A sole
maintainer satisfies all three as follows: the owner reads the commits before push and records that
review as self-review; a non-author AI engineering assessment of the same local commit SHA supplies the
independence; both records precede the push; after the push the assessment is attached or reproduced in
the PR and reconfirmed at the unchanged head SHA. Where the owner is not the author, the owner's
pre-push reading is itself the human review. For policy, governance and release-gate records the AI
assessment stays findings-only, and the owner's written decision is the approval. This does not weaken
§9: an AI review still cannot approve policy, and it never stands in for a human approval the platform
requires.

**Approval does not excuse blocking findings.** A finding recorded as blocking remains blocking until it
is resolved, or until the owner records a written decision that resolves it as accepted risk, with the
reason and the residual risk in the handoff. Owner authorization of a Git operation is not a resolution
of a finding.

**Prohibited in every case.** No admin override, no dismissal of a required review, no "merge without
waiting", no force-push to a protected branch, no agent merging or approving its own work, and no
repository-settings change made as part of a PR. When a required approval cannot be obtained, the PR
stays open and the conflict is reported.

**Unchanged.** Task ownership, worktree isolation (§7), the M0 authorized scope and the owner-only merge
rule are unaffected by this clarification.

### Merge gate

A PR may be merged only when all of the following hold for the **current** revision:

1. Every status check that the applicable GitHub branch-protection rule or ruleset requires for the
   target branch passes on the head SHA. Those rules, not the workflows present in the repository,
   determine what is required. The CI bootstrap exception below covers only the absence of a required
   check; it never adds to, replaces or waives one that protection already requires.
2. For implementation changes, an independent reviewer has recorded APPROVE for that head SHA; when
   that reviewer is an AI agent, the record says so and counts as engineering evidence only. For
   policy, redact, approval, journal, governance and release-gate records, an independent non-author
   assessment of findings with a recommendation is recorded for that SHA and the owner's written
   decision is recorded separately; no AI APPROVE is required or accepted. A human approval demanded
   by GitHub settings is separate in both cases and is met only by an eligible human account.
3. All blocking review findings are resolved.
4. The change stays inside the authorized milestone and task scope; it does not change milestone
   status.
5. The reviewed revision is still fresh against the target branch (see "Merge-base freshness").
6. The project owner explicitly approves the merge, naming the SHA being merged.

Only the project owner merges. No agent merges a PR, force-pushes, pushes to `main`, or bypasses branch
protection, required checks, or required review. Squash merge is the default; the task ID stays in the
subject.

### Merge-base freshness

A review and a CI result describe one revision, while the merge target moves independently. Before
merging, these must hold:

- **Reviewed revision.** The PR head SHA is the revision the independent review examined. A commit,
  amend, rebase, cherry-pick or squash after review creates a new revision ("Independent review").
- **Required checks.** The status checks that branch protection requires pass for the revision or
  test-merge result that protection evaluates. Which checks apply, and whether they must be re-run
  after the target moves, is decided by those rules — not by this policy.
- **Mergeable against the current target.** If the target branch advanced, the PR must remain
  mergeable against the updated target, by updating the branch or through GitHub's own mergeability
  result for the revision being merged. Where the rules require checks on the updated result, they are
  refreshed before the merge.
- **Renewed review.** Changes to the PR head, or conflict resolutions the reviewer has not seen,
  require a new independent review of the resulting revision. An approval of an earlier revision is
  not evidence for it.
- **No clean-merge inference.** Git reporting a branch as mergeable says nothing about whether the
  combined result is correct; that claim comes only from the required checks and the review.
- **Squash content.** A squash merge must not introduce content beyond the reviewed and verified
  integrated result.

An approval or a check result for an earlier head SHA or an earlier target state is not evidence for a
later one.

### CI bootstrap exception (first workflow PR only)

Merge-gate item 1 is defined by the target branch's protection rule or ruleset, not by the workflow
files in the repository, so a PR that introduces the first workflow is not necessarily blocked: a
workflow introduced on a PR branch can execute for that PR before merging, depending on its event
configuration and the token permissions it needs. This exception exists only where no required status
check applies yet, and only for that PR.

- **Scope:** the first PR that adds `.github/workflows/ci.yml` when the target branch has no required
  status check. One PR, one use, one target branch. It is named as a bootstrap PR in its body and in
  its gate-log entry. Every later PR, follow-up workflow change and branch-protection change uses the
  normal gate.
- **Preferred evidence:** if the workflow runs on the bootstrap PR's own head SHA and is green, that
  run is the check result and the fallback is not needed.
- **Fallback evidence (one-time, explicitly owner-approved):** permitted only when the workflow cannot
  run on that PR, with the exact reason recorded in the PR body — workflow disabled on the repository,
  event not configured for the branch, token permission absent, or Actions unavailable — and only with
  the owner's explicit approval naming that PR and that reviewed SHA. It is never used where a
  required status check already applies; that check must pass. The PR body then carries the
  **exact local commands with their output and exit codes**, run on that same SHA with
  `GIT_CONFIG_GLOBAL=/dev/null`: `make build`, `make test`, `make lint`, `make docs-check`,
  `make forge-verify`, plus `make reference-matrix` and `make reference-tests` when the PR adds the
  reference harness. A command that was not run is listed as not run; a summary is not evidence.
- **Review:** the independent review examines that exact SHA, states which evidence path was used,
  confirms the pasted output matches the diff, and reconfirms the SHA in the PR thread.
- **Merge:** the owner names the reviewed SHA in the merge approval and confirms that the squash merge
  introduces no content beyond the reviewed and verified integrated result.

### Failure handling

- A failed required check blocks merging. So does a missing check result, a stale approval, or an
  unresolved critical/high-severity finding.
- Missing evidence is reported as missing, never assumed to have passed.
- Agents must not disable, skip or weaken checks, branch protection, or test expectations to obtain
  approval. A disputed requirement is escalated to the owner, not negotiated in the diff.

### Authority

This section governs Git workflow only, and is approved as of 2026-11-20. It grants no permission to
commit, push, open a PR, merge, deploy, change repository settings, or start/continue a milestone; those
remain subject to §10, the task's allowed file scope and the signed readiness record. Task ownership,
worktree isolation (§7) and owner-only merging are unchanged by its approval.

### Recorded exception (2026-11-20)

M0 proceeds as two PRs instead of one branch per task: `docs/commit-rules` (governance, pre-existing
and pushed) and one implementation branch carrying T-0001…T-0005 as separately attributable commits.
Approved by the project owner on 2026-11-20 as a one-time exception to "one branch per task" and "one
PR per task". It expires at G0; no later milestone may reuse it. Task ownership, worktree isolation,
SHA-bound review and owner-only merging are unchanged.
