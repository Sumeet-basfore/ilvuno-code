# Ilvuno — Vision

Status: Phase 2 draft, pending approval. Evidence IDs (`T3-`, `ORC-`, `CR-`) resolve in
`docs/research/07-evidence-index.md`.

---

## 1. The promise

**Parallel work. Verified together.**

Ilvuno is an open-source, integration-first AI coding workspace. It exists to answer one
question that no surveyed tool answers today:

> These N agent changes each passed on their own branch. Do they still work *together* —
> and can I prove it?

Every surveyed system verifies an agent's change against that agent's own base: its
worktree, its branch, its sandbox. The moment two such changes meet, the verification that
was performed no longer applies to the artifact being shipped.

| Product | What it merges | What it verifies |
|---|---|---|
| t3code | nothing (`merge_back` transfers conversation context only, T3-048) | per-branch, post-push CI (T3-047) |
| orca | nothing (`merge_ready` is received and ignored, ORC-041) | per-PR GitHub checks (ORC-054) |
| cloudroom | rebases and pushes to the base branch (CR-031) | nothing; pushes with `--no-verify` (CR-032) |
| Copilot / Claude Code `/batch` | one branch, one PR per task | per unit only |

Ilvuno's subject is the *combination*, not the individual run.

## 2. What we believe

1. **Independent success is not integration success.** A green branch is evidence about a
   branch. It is not evidence about a merge. Ilvuno must never blur that line, in the
   product or in the marketing.
2. **Evidence beats claims.** Agent self-reports are the weakest artifact in the stack —
   `filesModified` self-declared (ORC-042), child results as truncated text (CR-026),
   delegated results with "nothing host-recorded to check" (T3-030). Ilvuno records what
   happened, derived from git and process exit codes, not from what an agent said.
3. **Determinism is a feature.** A conflict is a fact. A "likely API mismatch" is an
   opinion. Ilvuno separates them in the data model, in the UI, and in the exit code, so
   automation can trust one and a human can weigh the other.
4. **Local-first, not local-only by accident.** The core runs on the developer's machine,
   against their repo, with their toolchain. No account, no network, no telemetry by
   default. Paid services may make continuity easier later; they may not make the core
   useful.
5. **Isolation by default.** Shared-folder parallelism destroys the isolation the rest of
   the design depends on, and all three surveyed products ship it (T3-031, ORC-033,
   CR-024). Ilvuno never runs two agents in one working tree.
6. **The integration step is the product.** Dispatch, terminals, dashboards and diff
   viewers are commoditized (§4 of the landscape study). We will consume them, not rebuild
   them.

## 3. What Ilvuno is

A workspace in the sense that it owns the *state of a multi-stream development task*: the
task's objective, the approved decisions, the agent runs against it, the git revisions they
produced, the integration attempts, the verification evidence, the pending approvals, and
the next actions. That state is durable, portable, and readable by a human a week later.

Four capabilities, in dependency order:

1. **Integration Intelligence** — build a disposable candidate from independent changes,
   analyse compatibility, run the project's own checks on the combination, and record the
   evidence. *(The differentiator. Nothing surveyed does this.)*
2. **Verification evidence** — tie every completion claim to an artifact: a commit SHA, a
   command, an exit code, an output digest. *(Open in all surveyed tools; only proposed as
   a PR in t3code, T3-030.)*
3. **Task Passport** — a durable, versioned, human-readable record of a task's continuity
   that survives machines, agents, and weeks.
4. **Orchestration** — dispatch, isolation and repair. *(Table stakes. Deferred past MVP;
   consumed from existing tools where possible.)*

## 4. What Ilvuno is not

- **Not another worktree dashboard.** Conductor, Paradev, Sculptor, Vibe Kanban, orca,
  t3code and the first-party vendors all do this. Adding a seventh is not a strategy.
- **Not an agent.** Ilvuno does not own a model, a harness, or a subscription. It invokes
  whatever the developer already pays for, or nothing at all.
- **Not a merge bot.** It does not push to `main`, does not open PRs on your behalf, and
  does not resolve conflicts automatically. A developer approves; the developer lands.
- **Not a cloud IDE.** Execution stays on the machine that owns the workspace.
- **Not a claim that parallel agents are usually fine.** The real-world conflict rate is
  unmeasured for our target workflows (research `05` §5, §7). Ilvuno's first job includes
  producing that measurement for the user's own repository.

## 5. Positioning in one line

> Ilvuno is the integration and verification layer for multi-agent development. It takes
> whatever your agents produced, proves whether it works together, and leaves a record you
> can defend in review.

The comparable wedge is Clash/Grove (read-only conflict prediction, 66 and 13 stars) raised
from "predict" to "build, verify, and record" — a layer every dispatch tool can call into
rather than a competing workspace.

## 6. Three-year shape (non-binding)

| Horizon | Shape | What must be true |
|---|---|---|
| 0–3 months | Local CLI + report artifact | The combined-verification hypothesis holds on real repos |
| 3–9 months | Plugin surface (MCP / CLI) consumed by orca-style workspaces; optional hosted relay for remote runs and notifications | Someone pays for continuity, not for the core |
| 9–24 months | Desktop shell + mobile read/approve client over a durable Passport store | Task continuity across devices is worth a subscription |

Each gate is a hypothesis, not a roadmap promise. See `07-success-metrics.md` for the
kill criteria attached to each.

## 7. Success looks like

A developer runs three agents in parallel on Friday. On Monday they run one command and
learn, in under five minutes, that two of the three combine cleanly, the third breaks
`packages/api`'s contract after combination, the full test suite fails on
`checkout.test.ts` only when both land, and that the failure is reproducible from the
recorded evidence. They fix it once, with the failure in front of them, and the review
comment they leave links to a report instead of an assertion.

That is the whole product at MVP. Everything else is a distribution decision.
