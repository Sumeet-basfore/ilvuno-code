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

| Fact | Value |
|---|---|
| Phase | Documentation complete (research → PRD → architecture → roadmap). **Implementation has not started.** |
| Code today | None. No `go.mod`, no `Makefile`, no CI. |
| Approved scope | M0 only. Do not begin M1–M6 without explicit authorization. |
| Authoritative plan | `docs/planning/01-master-roadmap.md`, `docs/planning/02-milestones.md` |
| Task specs | `docs/tasks/T-NNNN.md` — one file per task, with its own acceptance criteria |

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

### 4.1 Code conventions (Go, per ADR-0001)

- `gofmt` and `go vet` clean; `staticcheck` clean. No new dependency without an ADR reference.
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

---

## 5. Testing requirements

| Rule | Detail |
|---|---|
| Failing test first | Every new behaviour ships with a test that fails without the change. Bug fixes reproduce the bug. |
| Fixtures, not hand-built repos | All repository fixtures come from `internal/testkit/forge` (deterministic identity, dates, `TZ=UTC`, `GIT_CONFIG_GLOBAL=/dev/null`). |
| Safety test | Any task that writes must include a `T-SAFE-*` test asserting the user's refs, tree, index, stashes, and `HEAD` are unchanged. |
| Determinism test | Changes to the engine or report must keep `DET-1` green (two runs, two OSes, identical output). |
| Crash test | Changes to journal/engine/verify must extend the crash matrix (`T-0503`), not bypass it. |
| No live network, no live model calls | Benchmarks and scenario tests are offline and deterministic. |
| Coverage floors | `engine`, `gitx`, `verify`, `evidence` ≥ 80%; `model` ≥ 90%. A floor is not a goal. |

A passing test suite that does not assert the user-visible behaviour does not satisfy this section.

---

## 6. Documentation requirements

Update documentation in the same change that alters behaviour:

| Change | Doc to update |
|---|---|
| CLI command, flag, exit code | `docs/` (created with T-0605), `--help` text, exit-code table |
| Config key or default | config reference (§10 of `docs/architecture/07-verification.md` is the source) |
| Report or passport field | schema + `docs/` format page; additive only within a major (ADR-0014) |
| User-facing wording | check against the banned-claims list (`docs/product/02-prd.md` Appendix A) |
| Structural decision | a new ADR; never edit an accepted decision to match the present |

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
3. commit in small, reviewable steps: <area>: <what> (T-NNNN)
4. push the branch, open a PR; squash-merge with the task ID in the subject
5. delete the worktree after merge
```

Rules:

| Rule | Reason |
|---|---|
| Never commit to `main` directly | every change is reviewed |
| Never merge a PR | a human merges; agents do not |
| Never force-push | it destroys review evidence |
| Never rebase another agent's branch | the owner rebases its own |
| Never `git reset --hard`, `git clean -xdf`, or delete branches you did not create | destructive, unrecoverable |
| Frozen files during a wave: `internal/model/**`, `internal/policy/**`, `schemas/**`, `Makefile`, `go.mod` | they ripple across every package |

Conflicts: the package owner resolves, even when the other agent's change triggered the conflict.
If the same file conflicts twice in one wave, escalate (§10).

---

## 9. Agent collaboration protocol

| Concern | Rule |
|---|---|
| Assignment | One task per agent, delivered as a task ID plus its spec. No "improve the codebase". |
| Handoff artefact | Diff + tests + updated spec + the 4-line self-review + an ambiguity note (`docs/planning/02-milestones.md` §11.3). |
| Review | A human reviews every diff. An agent may review another agent's diff for clarity and test quality, but **must not approve** `policy`, `redact`, `approval`, `journal`, or any release gate. |
| Ownership transfer | Announced before a wave starts, never mid-task. |
| Disagreement on an interface | Escalate the same day; write the decision into the task spec. Do not "try both". |
| Parallelism | Only pairs marked eligible in `docs/planning/02-milestones.md` §11.4. |

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

| Action | Approval required from |
|---|---|
| Editing anything outside the task's "Allowed file scope" | the human who assigned the task |
| Adding a dependency | maintainer (ADR amendment) |
| Changing a schema in a non-additive way | maintainer (major version + migration) |
| Changing an invariant (`INV-1`…`INV-14`), gate criterion, or threshold | maintainer, in writing |
| Deleting a fixture, test, or benchmark scenario | maintainer |
| Touching `.github/`, release config, or git hooks | maintainer |
| Any operation on a repository other than this one | the owner of that repository |
| Starting a milestone whose gate has not passed | maintainer |

### 10.3 Unknowns

State them explicitly and stop at the boundary. An agent that reports "I do not know whether X
holds, and here is how to find out" is doing its job; an agent that guesses and proceeds is not.
Required phrasing in the completion report: *Unknowns* and *Would need to verify*.

---

## 11. Definition of Done

A task is complete only when all of the following exist **as artefacts**, not as assertions.

| # | Evidence |
|---|---|
| 1 | **Code changes**, within the task's allowed file scope. Out-of-scope edits are listed and justified or reverted. |
| 2 | **Passing required tests**: the exact commands from the task spec, with their output. |
| 3 | **Accurate failure reporting**: any test, check, or benchmark that fails is listed with its output — never summarised as "mostly passing". |
| 4 | **Documentation updates** for every user-visible change (§6). |
| 5 | **Git state summary**: branch, base SHA, commit count, files touched, whether pushed, worktree path. |
| 6 | **Review status**: who reviewed (or "self-review only — no second reviewer available"), plus the 4-line adversarial self-review: what surprised me / what I tried to break / what remains on kill / what I did not do. |
| 7 | **Remaining risks**: known limitations introduced or left, with the trigger that would make them matter. |
| 8 | **Task spec updated** where reality diverged from the plan (scope changes are recorded, not hidden). |

Nothing on this list may be inferred. If the command was not run, the report says so.

---

## 12. Commands

There is no build system yet. The command contract is defined by task `T-0001` and
`docs/planning/07-definition-of-done.md`; once `T-0001` lands, `make build`, `make test`,
`make lint`, `make bench`, `make docs-check`, and `make gate G<N>` are the only supported entry
points.

Until then:

| Command | Status |
|---|---|
| `git`, `rg`, `ls`, `cat`, `jq` | available |
| `make build` / `make test` / `make lint` | **does not exist — do not invent or claim to run it** |
| `ilvuno …` | does not exist |

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
