# START HERE — Ilvuno agent context

Updated: 2026-11-20 · Status: navigation aid, **not policy**. On conflict, `AGENTS.md`, the
task spec, and `docs/decisions/open-questions.md` win.

## 60-second state

- **M0 is authorized** (2026-11-20, `docs/readiness/06` §7). **G0 is in progress, not passed.**
  **M1 and later are not authorized.**
- Governance records are committed **locally** on `docs/commit-rules` as G1–G5, 6 commits ahead of
  `main` and not pushed. M0 implementation is still uncommitted: 2 modified files (`README.md`,
  `.gitignore`), 82 untracked files.
- The product is a standalone CLI. No candidate merge, verification, evidence, passport, or
  approval behaviour exists yet. `ilvuno` currently has `--version`/`--help` only.
- Do not commit, push, merge, switch branches, or start M1 without the owner.

## Read in this order

| Tier | Read | Budget |
|---|---|---|
| 0 | this file | ~1 min |
| 1 | `CURRENT-STATE.md` + your `docs/tasks/T-NNNN.md` | ~5 min |
| 2 | `HANDOFF.md` (last session) | ~2 min |
| 3 | `PROJECT.md` or `ARCHITECTURE.md` only if the task needs product/design context | ~10 min |
| 4 | only the ADR/architecture section the task cites | on demand |

Do **not** bulk-read `docs/experiments/`, `docs/reviews/`, `docs/research/`, or all of
`docs/architecture/`. Open them only to verify a specific claim. Use `rg` for IDs
(`INV-`, `ADR-`, `T-`, `D-`, `G0-`) and stop when the task's acceptance criteria are answerable.

## Truth labels

Use these in every context update and handoff:

- `[implemented]` — code exists and a named command produced the recorded result.
- `[approved]` — a decision record, accepted ADR, or signed authorization names it.
- `[proposal]` — drafted, not approved; not usable as authority.
- `[unverified]` — documented or claimed, but no reproducible evidence is committed.

## Boundaries

- One task per agent, inside its **Allowed file scope**; one branch; separate worktrees for
  parallel agents. Never share a branch or working tree.
- Never commit to `main`, merge a PR, force-push, or bypass checks. Only the owner merges.
- Git is invoked only through `internal/gitx` in production; never parse human-readable git output.
- Verification runs arbitrary project code; it is not sandboxed. Do not claim otherwise.
- Agent workspace (T3 Code or any other) is a **host**, not a dependency. No provider credentials
  or provider-specific config in the repository.

## Links

- Policy: `AGENTS.md` (authoritative) · `CLAUDE.md` (Claude-specific reference)
- Decisions: `docs/decisions/open-questions.md` · `docs/readiness/05-owner-decision-register.md`
- Authorization / gates: `docs/readiness/06-m0-readiness-checklist.md` ·
  `docs/planning/gate-log.md`
- Task specs: `docs/tasks/README.md` · `docs/tasks/T-NNNN.md`
- Product: `docs/product/01-vision.md` · `docs/product/02-prd.md` ·
  `docs/product/05-mvp-scope.md`
- Architecture: `docs/architecture/01-overview.md` · `docs/adr/README.md`
- Current truth: `docs/agent-context/CURRENT-STATE.md` · session record: `HANDOFF.md`
