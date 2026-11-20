# CLAUDE.md — Claude Code guidance for Ilvuno

@AGENTS.md

The import above pulls in `AGENTS.md`, which is the shared engineering policy for this repository.
Read it as if it were the top of this file: `AGENTS.md` loads first, then everything below.

This file adds only what is specific to Claude Code. It deliberately does not restate policy —
if a rule applies to every agent, it lives in `AGENTS.md`; if the two ever disagree, `AGENTS.md`
wins and the discrepancy is a bug to report.

---

## 1. Before you touch anything

1. Read `AGENTS.md` (imported above) — especially §1 (implementation has not started), §10
 (safety policies), and §11 (Definition of Done).
2. Read the task spec you were given: `docs/tasks/T-NNNN.md`. If you were not given a task ID,
 ask for one before writing code.
3. Read only the architecture/ADR documents that task cites. Do not bulk-load `docs/`.
4. Confirm the milestone is authorized. **M0 is authorized (2026-11-20) for tasks T-0001 to T-0005 only; M1 and later are not.** Its
 authorization record is the signed sign-off in `docs/readiness/06-m0-readiness-checklist.md` §7,
 and its status is reconciled in `docs/readiness/07-m0-reconciliation.md`. Start only the tasks the
 sign-off names in `docs/tasks/`. Anything else is out of scope, even if it looks
 obviously next.

If a required input is missing — no task spec, no acceptance criteria, ambiguous scope — say so and
stop. Do not infer the requirements from the surrounding docs and proceed.

---

## 2. Planning procedure

For any task larger than a one-line fix:

1. **Restate the task** in one paragraph: goal, the acceptance criteria you are targeting, and the
 files you expect to touch.
2. **List the files** you will create or modify, and confirm each is inside the task's "Allowed file
 scope". Out-of-scope files need approval first.
3. **Name the tests** you will add and which acceptance criterion each one proves.
4. **Name the unknowns.** Anything you cannot verify from the repo (e.g. "does git 2.38 preserve
 `merge-tree` output shape?") goes in a list, with how you would resolve it.
5. **Wait for approval** if the plan touches: an invariant, a schema, a gate criterion, a
 dependency, `.github/`, or anything outside the task scope.

Then implement, smallest complete change first, tests alongside. Do not produce a large speculative
implementation and ask for forgiveness afterwards.

Use plan mode for multi-file changes; it is cheaper than reverting.

---

## 3. Context handoff

Claude Code sessions lose context; the repository must not.


| Instead of                            | Do this                                                |
| ------------------------------------- | ------------------------------------------------------ |
| Re-explaining decisions in chat       | Record them in the task spec or a new ADR              |
| Keeping a plan in your head           | Write it into the task spec's outline as you go        |
| Summarising what you did in chat only | The completion report in `AGENTS.md` §11 is the record |
| Hoping the next agent reads your mind | Leave the four-line self-review in the PR              |


When you finish, a fresh session must be able to continue from: the task spec, the diff, the tests,
and the PR description. If it cannot, your handoff is incomplete regardless of code quality.

Auto-memory may capture your preferences, but it is **not** project policy and must never be cited
as a reason for a change. Repository decisions live in `docs/`.

---

## 4. Coordinating with Codex

Both agents may work this repository. Codex reads `AGENTS.md`; you read this file plus the import.
That asymmetry is handled — do not "fix" it by copying `AGENTS.md` content here.


| Rule                                               | Detail                                                                                                                                   |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| One owner per package                              | Map in `docs/planning/03-dependency-graph.md` §5. If Codex owns the package, you may read and propose, not edit.                         |
| No shared working tree                             | Each agent uses its own worktree: `.worktrees/<agent>-<task-id>/`.                                                                       |
| Handoff artefact                                   | A branch plus a PR description containing the task ID, AC checklist, test output, and the ambiguity note. Chat is not a handoff channel. |
| Do not rebase or force-push another agent's branch | The owner resolves its own history.                                                                                                      |
| Conflict in a frozen file                          | Stop; the file's owner resolves (`AGENTS.md` §8).                                                                                        |
| Duplicate work                                     | Check `git branch -a` and open PRs before starting anything that touches a shared package.                                               |


**Do not delegate autonomously.** Spawning subagents or dispatching work to Codex is allowed only
when all three hold:

1. **Cost** — the subtask is worth a separate model session rather than a few more tool calls.
2. **Isolation** — it touches files no one else is editing, in an eligible parallel pair
 (`docs/planning/02-milestones.md` §11.4).
3. **Dependencies** — nothing downstream blocks on it, and its result can be reviewed independently.

If any of the three is unclear, do the work in this session. Parallel agents that collide cost more
to integrate than they save — that is the product's own thesis, applied to its development.

---

## 5. Review and testing behaviour

- Run the exact commands from the task spec and paste the output. If a command does not exist yet
(`make test` before `T-0001`), report it as a blocker rather than inventing a substitute.
- Run tests **before** claiming success, in the same session as the claim.
- If a test fails intermittently, say so and quantify it (`2 of 5 runs`). Do not retry until green
and report the green run.
- Review your own diff before requesting review: read it backwards, then write the four-line
self-review (`AGENTS.md` §11 item 6).
- Never edit a test to match buggy behaviour without flagging it explicitly in the PR body.
- Never disable a benchmark, gate check, or lint rule to make a change pass. If a threshold is
wrong, escalate — that is a policy change, not a fix.
- Commit only a coherent, reviewable change, after inspecting `git diff --cached` and running the
task's checks. The commit rules in `AGENTS.md` §4.4 are authoritative; this bullet is a reminder.
- After committing, report the SHA(s), the checks with their output (or the blocker), and the files
touched. Do not push until the human has reviewed the commit.

---

## 6. Approval escalation

Stop and ask the human, in this order of urgency:


| Situation                                                                                  | Action                                               |
| ------------------------------------------------------------------------------------------ | ---------------------------------------------------- |
| A safety invariant is at risk (`INV-1` no user state modified, `INV-4` no uncertain retry) | Stop immediately. Do not work around it.             |
| The task cannot meet an acceptance criterion                                               | Stop; propose an amended criterion or narrowed scope |
| A change needs a dependency, schema break, or gate/threshold change                        | Ask before writing code                              |
| You are blocked > 30 minutes on an environmental issue                                     | Report the blocker with what you tried               |
| Two approaches are both defensible and the choice is architectural                         | Ask; do not pick silently                            |
| You want to touch a file outside the task scope                                            | Ask; do not "while I'm here"                         |


Asking costs one message. A wrong autonomous decision costs a review cycle, and sometimes a gate.

---

## 7. Failure reporting

Report failure plainly and early. Required shape:

```
FAILED: <what failed>
Command: <exact command>
Output:  <last relevant lines>
Cause:   <known | suspected: … | unknown>
Blocked: <what you cannot proceed with>
Next:    <what you would do, or what you need>
```

Rules:

- "Tests pass" is a claim that requires the command and its output. Without them it is `unverified`.
- A partial success is reported as partial, with the specific part that is missing.
- Never present a hypothesis as a finding. Use `suspected:` and say what would confirm it.
- If you broke something, say so in the first line.

---

## 8. Scope discipline

Do not expand scope autonomously. Specifically banned without explicit instruction:

- Refactoring code you did not need to touch for the task.
- Adding configuration options, abstractions, or "for later" hooks nobody asked for.
- Implementing a later milestone's task because it is "adjacent".
- Adding a UI, a daemon, a dependency, or a network call — all are out of MVP scope by ADR.
- Writing documentation the task did not request (the repo has a documentation plan; unplanned docs
drift).
- Reformatting files wholesale, which buries the real diff.

When you notice something genuinely important outside scope, record it (a note in the task spec or
an issue) and continue with your task. Do not fix it in passing.

---

## 9. Claude Code specifics

- `/memory` lists the instruction files that actually loaded. If `AGENTS.md` does not appear as an
import of this file, stop and report it — the policy is not in context.
- Treat this file as context, not enforcement: it guides, it does not block. Hard restrictions
(never write to `main`, never push) are additionally enforced by human review and, later, hooks.
- Keep this file under ~180 lines (vendor guidance: under 200). If it grows, move the detail into
`docs/` and link it; long instruction files reduce adherence.
- Do not modify `~/.claude/`, `~/.codex/`, or any global agent configuration as part of a task.
This repository's policy is its own; global config is the user's.

---

## 10. Git workflow

`AGENTS.md` §15 is the authoritative Git workflow policy (approved 2026-11-20); where it conflicts with
§§7–9 of `AGENTS.md`, §15 governs. In short: one branch and one worktree per task
(`.worktrees/<agent>-<task-id>/`), no direct `main` commits, every change through a PR, an independent
SHA-bound review recorded against the exact head commit, the target branch's required status checks
green on the revision or test-merge result the rules evaluate, the PR still mergeable against the
current target — a new SHA means a new review, and a moved target refreshes whatever checks the rules
require — and only the owner merges.

AI review is documented engineering evidence, not GitHub human approval: never claim it satisfies a
human-review requirement, never approve on the owner's behalf, never bypass or change branch
protection. If the repository's settings require a second human reviewer, say the PR is blocked and
leave it open. The first workflow PR follows the §15 CI bootstrap rule — prefer its own CI run; local
evidence only with the owner's one-time approval and the recorded reason. Task assignment is not
permission to commit, push or merge.
