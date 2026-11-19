# Ilvuno — Monetization Hypotheses

Status: hypotheses only. No pricing research, no willingness-to-pay data, no interviews were
conducted. Every number here is an assumption with a stated test.

---

## 1. Frame

**Target:** roughly **$500–$1,000 MRR**. Stated as a hypothesis, not a plan, and
deliberately modest — it is the threshold at which a solo-maintained open-source tool is
worth continuing, not a growth ambition.

**What is permanently free.** Local execution, git integration, integration intelligence,
verification, evidence, and the Task Passport. No feature required by a P0 requirement may
depend on a service, a key, or a login (`OSS-102`, `XDC-101`). This is not generosity; it is
the trust precondition that makes the free tool credible to Dana (persona §3) and the paid
layer sellable to a team.

**What can be paid.** Anything whose cost is *ongoing infrastructure or shared coordination*:
hosting, transport, notifications, retention, and multi-human workflows. Never the
computation of a verdict.

The boundary test for any future paid feature:

> Could a user reimplement it with cron, git, and a shell script in an afternoon?
> If yes, it must not be paid.

---

## 2. Candidate offerings

| ID | Offering | What it is | Cost to run | Who pays | Evidence it is wanted |
|---|---|---|---|---|---|
| **H1** | Managed secure remote access | Always-on verification runner (or remote worktree host) reachable from any device, with auth and encrypted transport | Medium — a VM/container per user or a shared pool | Priya (individual), Dana (team) | Remote execution is a real, repeatedly built pattern (T3-052, ORC-063, CR-035). But teams already have CI, so this competes with "just run it in CI" |
| **H2** | Encrypted metadata sync + notifications | Passport and approval state across devices; push when a batch is verified, blocked, or awaiting approval | Low — objects + a small relay | Priya, later teams | Cross-device continuity is universally requested and unreliably delivered in the surveyed tools (ORC-097 documents mobile reconnect bugs; T3-059 makes push depend on a hosted relay) |
| **H3** | Collaboration and approval workflows | Shared approval queue, reviewer assignment, "evidence required" policy, retention of bundles beyond local disk | Low-medium | **Teams** (Dana, Marco) | This is the only offering a *team* buys rather than an individual. Approval evidence is inherently shared (`INT-603`, `EVD-106`) |
| **H4** | Optional hosting of evidence bundles | Hosted, immutable, access-controlled retention of evidence for audit and PR linking | Low | Dana, Marco | Reviewers want a link, not a download. But a git-tracked bundle or a CI artifact is free, so the paid value must be retention + access control + linkability |
| **H5** | Team services / support | Setup, custom verification adapters (e.g. a symbol index for an unusual stack), integration-in-CI configuration | High (time) | Dana | Real but low-ceiling; time-for-money, does not scale |

Rejected as paid, permanently: the CLI itself; verification runs on the user's machine;
evidence format; passport schema; repair; any per-seat gate on reading a report.

---

## 3. Model comparison

### 3.1 Arithmetic against the target

| Model | Price | Users needed for $750 MRR | Plausibility |
|---|---|---|---|
| Individual subscription (H1/H2) | $12/mo | ~63 | Requires ~63 paying individuals from a free base; typical OSS CLI conversion is 1–3%, implying 2,000–6,000 active users |
| Individual pro (H1+H2+H4) | $20/mo | ~38 | Same funnel problem, slightly better ARPU |
| **Team plan (H3+H4)** | **$250/mo for 10 seats** | **3 teams** | Far more plausible: 3 teams is a reachable number through the maintainer's own network and 2–3 design partners |
| Team + services (H3+H4+H5) | $250 + $2k setup | 1–2 teams | Achievable but does not compound |

**Conclusion: the target is only realistic against teams.** A solo-maintained OSS CLI
converting individuals at $12/mo to 63 payers is a distribution problem we cannot fund. Three
teams at $250/mo is a sales problem, which a technical founder can do manually.

This changes the roadmap, not just the price list: **H3 (collaboration and approval) is the
primary paid hypothesis, not H1.** H1 is a prerequisite feature of H3, not a product.

### 3.2 Revised hypothesis ordering

1. **H3 team workflows** — $200–300/mo per team of ~10. Needs: shared approval queue, hosted
   evidence retention, policy ("integration evidence required for PRs touching /api"), and a
   reviewer-facing link. *(Primary)*
2. **H2 sync + notifications** — $8–15/mo individual, or bundled into H3. Cheap to run,
   natural top-of-funnel, but not sufficient alone.
3. **H1 remote runner** — bundled into H3 rather than sold separately; it is infrastructure,
   not a feature a team wants to think about.
4. **H4 hosted evidence** — bundled into H3, priced by retention.
5. **H5 services** — opportunistic; not a plan.

### 3.3 What each offering requires that does not exist yet

| Offering | Blocking engineering | Blocking trust |
|---|---|---|
| H3 | Accounts, org model, RBAC-lite (reviewer vs approver), hosted store, audit log | SOC2-shaped questions from Dana; a written data policy |
| H2 | E2EE sync, key management, device pairing, push infra | Explicit "what the server sees" statement (`SEC-107`) |
| H1 | Sandboxed execution, secret handling for the runner, isolation per run | The cloudroom lesson: full-access agents in a shared sandbox is an incident waiting to happen (CR-035, CR-037) |

None of these exist. That is the point of gating on traction (`A-10`).

---

## 4. Pricing shape (if it happens)

**Free (forever):** everything local. Unlimited integrations, unlimited repos, full evidence,
full passport, full report. No seat count, no watermark, no expiring trial.

**Team — $25/seat/mo, minimum 5 seats (~$125/mo floor):**
- H3 shared approval queue and policy checks
- H4 hosted evidence retention (e.g. 90 days, configurable)
- H2 sync across a team's devices
- H1 remote runner with a concurrency cap
- SSO at a higher tier, later

**Why per-seat with a floor:** Dana's objection to per-seat pricing is that readers should
not pay full price (persona §3). Answer it with two roles: `approver` (paid) and `viewer`
(free). A 10-person team with 3 approvers is a smaller, more defensible bill than 10 seats,
and it matches how review actually works.

**Why not usage-based for verification:** it would make the moment of truth
("is this green?") a meter. That is the worst possible thing to meter, and it would
reintroduce exactly the trust problem the product exists to solve.

**Why not a one-time license:** the cost being recovered is ongoing hosting, not
development. One-time pricing on recurring cost is how OSS tools die.

---

## 5. Unit economics sketch

Rough, to be replaced by measurement:

| Item | Estimate |
|---|---|
| Hosted evidence retention per active team/month | 2–10 GB → $1–3 |
| Relay/push for a team | < $1 |
| Remote runner (if bundled, ~2 h/week per team) | $5–15 at commodity compute |
| Payment processing | ~3% |
| **Gross margin per $250 team** | **~90%** |
| Support load | 1–2 h/team/month at launch (the real cost) |

The constraint at $750 MRR is **support time**, not infrastructure. Three teams is roughly
3–6 hours/month of support — sustainable for one maintainer, and the reason the target is
sized where it is rather than higher.

---

## 6. Falsification tests, in order

Each test is cheap, and each has a defined negative outcome that changes the plan.

| # | Test | Method | Negative outcome → action |
|---|---|---|---|
| T1 | Do teams care about *shared* evidence? | Show 5 engineering leads a real report + bundle. Ask: "would you require this on a PR?" | If ≤1 says yes, H3 is dead. Fall back to individual H2 and accept a lower target or stop. |
| T2 | Will they pay for retention/linkability? | Quote $250/mo for 10 seats, 3 approvers. Ask for a purchase order or a signature | ≤1 of 5 → pricing or value is wrong; do not build |
| T3 | Is continuity worth $10/mo individually? | Landing page + waitlist for H2 with a price shown; measure click-to-email | <1% click-through → H2 is a feature, not a product |
| T4 | Does the free tool spread without a paid hook? | Track report bundles shared (opt-in), inbound issues from non-authors | No organic spread → distribution is the real problem, not monetization |
| T5 | Is the core hypothesis even true? | `combination_only` rate across ≥20 weekly actives | <2% → stop; do not monetize a non-problem (see `07-success-metrics.md` §5) |

Order matters: T5 before T1–T4. Monetizing an unvalidated core is the most expensive
mistake available, and it is the one every failed orchestrator UI in the landscape study
appears to have made.

---

## 7. Risks specific to monetization

| Risk | Why it matters | Mitigation |
|---|---|---|
| Free tier is too good | Local verification is the entire value; a team may never need the paid layer | Intentional. The paid layer is coordination, which individuals do not need; that is the point |
| Hosted runner becomes a security liability | Running untrusted agent code on our infrastructure is what broke cloudroom's shared sandbox (CR-044) | Per-run isolation, no shared sandbox, fail-closed guards, no full-access default |
| Enterprise sales drag | One maintainer cannot do security questionnaires | Publish a security page and a data-flow diagram from day one; decline enterprise until there is help |
| Price anchoring against CI | "Our CI already does this" | Position as pre-merge evidence, not build infrastructure; never claim to replace CI (`VER-101`) |
| Open-core creep | Pressure to move verification behind a paywall | Written policy: `OSS-102` is a release-blocking test, not a value statement |

---

## 8. Decision summary

- Target $500–1,000 MRR, reached primarily through **3–5 teams at ~$200–300/mo**, not through
  individual subscriptions.
- Primary paid hypothesis: **H3 team collaboration and approval**, bundling H1/H2/H4.
- Free core is permanent and complete for local use; this is a design constraint, not a
  marketing promise.
- Nothing is built until T5 passes (the core hypothesis is measured) and T1–T2 show at least
  one team saying yes with a price attached.
- If those fail, the honest outcomes are: reposition as a free OSS utility with no revenue
  plan, or stop.
