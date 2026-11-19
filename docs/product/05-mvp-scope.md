# Ilvuno — MVP Scope and Product-Shape Decision

Status: recommendation, requires approval. Requirement IDs resolve in `02-prd.md`.
Evidence IDs resolve in `docs/research/07-evidence-index.md`.

---

## 1. The decision to make

We must not assume the complete workspace is the first product. Four shapes were considered:

| # | Shape | What it is |
|---|---|---|
| A | **Standalone integration CLI** | A single binary that takes git refs in, builds a candidate, verifies it, records evidence, produces a report and a task record. No server, no UI. |
| B | **Integration plugin for existing workspaces** | The same engine shipped as a plugin/MCP tool inside orca, t3code, or a comparable host, using that host's worktree and session state. |
| C | **Full desktop application** | An Electron-style app owning agents, worktrees, terminals, diffs, integration, and state. Ilvuno as a workspace. |
| D | **Browser-based local workspace** | A local server plus web UI: dashboards, parallel streams, live logs, integration. |

---

## 2. Evaluation

Criteria weighted by what the research can actually support. Research here is
supply-side (code archaeology) only — there is no demand data, no pricing data, no user
interviews (PRD §3.4). That weakens every "market" criterion and we weight accordingly.

| Criterion | A: CLI | B: Plugin | C: Desktop | D: Browser |
|---|---|---|---|---|
| **Tests the core hypothesis directly** | **High** — the whole product is the hypothesis | High | Low — hypothesis buried under workspace features | Low |
| **Differentiation per unit of effort** | **High** | Medium — inherits the host's ceilings | Low — commoditized category | Low |
| **Time to a result a user can judge** | **Weeks** | 1–2 months (host API churn) | 4–8 months | 4–6 months |
| **Works with every agent** | **Yes** — refs are universal | Only inside that host | Only integrated agents | Only integrated agents |
| **Survives a host's pivot or shutdown** | **Yes** | No — coupled to one product's roadmap | n/a (we are the product) | n/a |
| **Distribution reach** | Requires the user to want a new command | **Best** — rides an existing install base | Requires a download + trust | Requires running a server |
| **Competitive exposure** | Low — the gap is real and unexploited | Medium — host may build it | **Very high** — Conductor, Paradev, Sculptor, Vibe Kanban, orca, t3code + 5 vendor surfaces | High |
| **Commercial fragility** | Low — nothing to monetize locally, by design | n/a | **High** — see churn below | High |
| **Engineering surface to maintain** | ~1 binary, git + subprocess | + host SDK versions | + process supervision, PTY, IPC, updates, signing, telemetry | + server, auth, streaming, frontend |
| **Evidence/audit quality** | **High** — self-contained artifacts | Medium — host state owns truth | Medium | Medium |
| **Security surface** | Minimal (no daemon, no network) | Inherits host's (incl. its gaps) | Large | Large |
| **Path to $500–1k MRR** | Indirect — needs a paid layer on top | Indirect, and revenue accrues to the host | Direct but requires winning a saturated market | Direct but same |

### 2.1 Why C and D lose on evidence, not taste

- **Commoditized, confirmed.** Worktree-per-task, parallel dashboards, diff review, PR
  creation, phone control are already delivered by at least six products and by the vendors
  themselves (research `04` §6). Building a seventh workspace adds no defensible claim.
- **Category churn is a signal.** Vibe Kanban announced a sunset citing no business model;
  Crystal was deprecated; a secondary source reports Conductor's acquisition. Standalone
  orchestrator UIs have not demonstrated durable willingness to pay.
- **The surveyed products prove the ceiling.** t3code (66k stars) and orca (129k stars) —
  the two most successful workspaces — still ship no integration step (T3-051, ORC-041).
  If the workspace model naturally produced integration, it would exist at that scale.
- **Effort misallocation.** A desktop app spends most of its engineering on process
  supervision, PTY plumbing, and provider adapters — precisely the parts that are already
  solved and undifferentiated (research `06` §3).

### 2.2 Why B loses *for first release*, despite the best distribution

B is attractive: a plugin inside orca or t3code reaches a large install base with no
acquisition cost. But at v0.1 it is strictly worse than A:

1. **Host API churn.** orca ships roughly daily releases with a custom RPC envelope at
   protocol v3 (ORC-008) and a daemon protocol at v60 (ORC-006). t3code has 127 database
   migrations (T3-020). Coupling the first version to that surface means rework before the
   hypothesis is tested.
2. **Truth ownership.** In a plugin, session and worktree state belong to the host, so the
   evidence ledger becomes host-derived and less portable — weakening differentiation #4
   (PRD §8).
3. **We cannot measure independently.** The core unknown is the real combined-failure rate
   on real repositories. A CLI user running it on their own repo gives that signal directly;
   a plugin user inside a host gives it mediated through another product's abstractions.
4. **It is additive, not exclusive.** A stable non-interactive contract (`ORQ-106`) is the
   prerequisite for B anyway. Build the contract first, as a CLI, and B becomes a thin
   adapter later.

So: **A first, B as the first distribution extension, C and D only if a paid cohort demands
a shell.**

### 2.3 The honest argument against A

A has the weakest distribution of the four. A developer must learn a new command. That is a
real cost, and the research cannot tell us whether the pain is sharp enough to pay it. Two
mitigations, both cheap:

- The command is one line and runs on refs the developer already has (`INT-101`); no
  migration, no import, no new mental model beyond "point it at the branches".
- The output is the artifact: a Markdown report a reviewer can read. That makes the report
  the distribution channel (Marco asks for it), which is how the tool spreads without a UI.

---

## 3. Recommendation

**Build a standalone CLI (A).** Ship it open source under a permissive license, local-first,
with no account and no network. Design a stable machine contract and a plugin seam from day
one so B is an adapter, not a rewrite.

### 3.1 Rationale in one paragraph

Ilvuno's only defensible claim is that it verifies the *combination*, and that claim is
testable with a single binary and a git repo. Every alternative shape adds months of
undifferentiated surface — process supervision, UI, sync, auth — before the hypothesis can
be tested, and each of those surfaces is already occupied by products with 60k–130k stars
and daily release cadence. A CLI is also the only shape that works with every agent and
survives every host's pivot, which is what makes it a layer rather than a competitor.

### 3.2 The measurement obligation

The research's strongest caution is that the combined-failure rate may be low (research `05`
§5: 1 of 834 real Django pairs). We therefore make measurement a **product output**, not a
research prerequisite:

- Every run reports `combination_only` counts (`INT-210`, `INT-306`).
- The report shows what per-branch verification would have missed.
- Opt-in aggregate statistics (off by default, `NFR-109`) let us learn the real rate across
  users without a data-collection surprise.

Kill criteria are attached: if after 20 weekly-active developers the observed
combination-only failure rate is under ~2%, the value proposition is wrong and we stop or
reposition (see `07-success-metrics.md` §5).

**Consequence for scope:** the MVP cannot ship without `INT-306` (per-source re-runs on
failure), because that is the instrument that produces the measurement. It is P1 in the PRD
but is treated as P0 for the *decision*, not for the first usable release.

---

## 4. MVP boundary

### 4.1 In scope

| Area | Included | Requirements |
|---|---|---|
| Input | git refs only; any producer; strict stream manifest | INT-101–INT-104, ORQ-102–103 |
| Candidate | disposable worktree, deterministic order, per-step record, rollback | INT-105–INT-111 |
| Analysis | file overlap, hunk proximity, dependency disagreement, interface touch, base drift, textual conflict prediction | INT-201–INT-208, INT-210 |
| Verification | configured commands, timeouts, evidence capture, env allowlist, five-state result | INT-301–INT-310 |
| Evidence | append-only store, portable bundle, digests, negative evidence | EVD-101–EVD-107 |
| Repair | external command + brief, budget 2, keep all attempts | INT-501–INT-506 |
| Review | combined diff with attribution, approval gate bound to SHA | INT-601–INT-605 |
| Task | passport file, decisions, streams, refs, verification refs, pending approvals, derived state | TSK-101–TSK-110, TSK-112 |
| Output | Markdown + JSON report, stable schema | VER-105, INT-208 |
| Failure | typed errors, exit-code contract, resume/abort, no uncertain retry | REC-101–REC-107 |
| Platform | macOS + Linux, single binary, **zero** runtime deps beyond git | NFR-104–NFR-107 |
| Security | fail-closed, allowlisted env, no network, write-path audit | SEC-101, SEC-102, SEC-104, SEC-106 |
| OSS | permissive license, core has no paid gate, fixtures in repo | OSS-101, OSS-102, OSS-104 |

### 4.2 Out of scope (explicit)

| Excluded | Reason |
|---|---|
| Dispatching/starting agents | Commoditized; research `04` §6, `06` §9 |
| Agent adapters, provider SDKs | Would force a provider roadmap; repair uses an external command (`INT-502`) |
| TUI / dashboard / desktop shell / web UI | Shape C and D rejected (§2) |
| Hosted service, relay, sync, accounts | Pre-revenue; nothing in the core needs it (`XDC-101`) |
| Mobile client | Depends on rung 3 transport (`21.2`) |
| Team collaboration, approval queues, RBAC | Only after revenue hypothesis is validated |
| Billing, license keys | No paid feature exists yet |
| Windows | v0.2; process/signal differences are a real cost, not a checkbox |
| AI interpretation on by default | `INT-207`; interpretation is opt-in and never blocking |
| Automatic conflict resolution | Product decision: we report, humans and agents decide |
| Push / PR creation / merge execution | `INT-604` |
| Submodules, LFS, sparse-checkout deep support | Refuse with a typed error; v0.2 candidate |
| Non-git VCS | Out |
| Coverage/quality metrics, "AI code review" | Different product |

### 4.3 Sequencing (6 slices, each independently useful)

| Slice | Deliverable | Proves | Est. |
|---|---|---|---|
| S1 | `init`, `doctor`, `pin`, candidate build, step list, report skeleton | Git plumbing, determinism, safety | 2 wk |
| S2 | Preflight analysis (conflicts, overlap, deps, drift) + typed findings | Determinism vs interpretation works | 2 wk |
| S3 | Verification runner + evidence store + bundle + exit-code contract | The actual value: combined checks with evidence | 3 wk |
| S4 | Task passport + `task show` | Continuity works with files alone | 1.5 wk |
| S5 | Repair (external command, budget) + `combination_only` isolation | The measurement instrument | 2 wk |
| S6 | Approval gate, combined diff, JSON contract, docs, banned-phrase test | Shippable v0.1 | 2 wk |

S1–S3 is the smallest release that can answer "does this work for me". If forced to cut,
ship S1–S3 and defer S4–S6.

### 4.4 Deliberate omissions with a known cost

| Omission | Cost | When to revisit |
|---|---|---|
| No dispatch | Users must already run parallel agents | Post-revenue, gated by ORQ-105 |
| No Windows | ~20–30% of developers | v0.2 |
| No AI interpretation default | Users who want "smart" analysis must opt in | After trust is established |
| No hosted anything | No recurring revenue path at launch | After 20 weekly actives |
| Passport in repo | Users with confidential task notes may object | `--local-only` flag, v0.2 |

---

## 5. Decisions requiring approval

| ID | Decision | Recommendation | Why it must be an explicit decision |
|---|---|---|---|
| **A-1** | Product shape | **Standalone CLI (A)** | Commits the next 3 months; excludes UI work |
| **A-2** | License | **Apache-2.0** | Determines enterprise adoption and clone protection |
| **A-3** | Implementation language | **Go** (single binary, cross-compile, cheap concurrency) — alternative Rust (safety, slower iteration) or TypeScript/Bun (contributor pool, worse distribution) | Hard to reverse after v0.1 |
| **A-4** | Windows in MVP | **No** — v0.2 | Scope, and honest support claims |
| **A-5** | Passport location | **Repo-committed** by default, `--local-only` optional | Affects privacy posture and portability story |
| **A-6** | Repair executor | **External command** the user configures; no bundled agent | Keeps Ilvuno agent-neutral and dependency-free |
| **A-7** | AI interpretation | **Opt-in, non-blocking, removable** (`INT-207`) | Determines whether we are auditable |
| **A-8** | Measurement/telemetry | **Off by default**, aggregate opt-in only | Trust is the product; must be decided up front |
| **A-9** | Verification config location | Committed `ilvuno.toml` | Makes verification a reviewable team decision |
| **A-10** | Monetization timing | **No paid feature before 20 weekly actives and ≥5% combination-only rate** | Prevents building a paid surface nobody needs |

Full open questions, including unresolved research gaps and their owners, are in
`docs/decisions/open-questions.md`.

---

## 6. What "done" means for the MVP

Restating the gate from PRD §26 in decision terms:

1. The smallest release runs on a real user's real repo, on refs produced by tools we do not
   control, in under ten minutes from install.
2. It refuses unsafe inputs instead of degrading.
3. It produces an artifact a reviewer accepts as evidence.
4. It measures whether combined verification finds anything per-branch verification missed
   — and reports that number even when it is zero.

If item 4 consistently reports zero, the product is not viable in this shape and we say so
rather than shipping a conference-talk tool.
