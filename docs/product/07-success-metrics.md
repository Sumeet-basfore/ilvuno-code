# Ilvuno — Success Metrics

Every metric here is either a **product-health** signal we act on weekly or a
**hypothesis test** with a kill criterion. No vanity metrics. Baselines are unmeasured; the
first release establishes them.

---

## 1. North star

**Verified integrations per active developer per week.**

An integration counts only if it produces an evidence bundle with at least one configured
check executed against a specific candidate SHA (`INT-301`, `VER-103`). A run that ends
`unverified` does not count — it is a session, not a verification.

Target at 6 months: **≥ 3 per active developer per week.** Rationale: a developer running
2–6 streams weekly who integrates them at least once per batch sits at 2–4; anything below
2 means the tool is used for demos or single one-offs.

**Why not installs, stars, or repos:** all three reward distribution rather than value, and
the landscape shows high-star tools that have no integration step at all (t3code at 66k
stars, T3-051). Popularity is not the gap.

---

## 2. Hypothesis metrics (the ones that decide the product)

| ID | Metric | Definition | Target | Kill / pivot threshold |
|---|---|---|---|---|
| **M-1** | Combination-only failure rate | Integrations where a check fails on the candidate but passes on every source individually, ÷ integrations with ≥2 sources (`INT-306`) | ≥ 5% | **< 2% over ≥ 50 integrations → stop or reposition** |
| **M-2** | Evidence use in review | Share of bundles whose immutable link or file appears in a PR description or review comment (opt-in measurement, or self-reported) | ≥ 25% | < 5% → the artifact is not the product; the CLI is a novelty |
| **M-3** | Approval→land conversion | Runs approved and subsequently landed by the developer, ÷ runs approved | ≥ 60% | < 30% → the report is either wrong or unpersuasive |
| **M-4** | Time to verified candidate | p50 wall time excluding project check execution (`NFR-102` measured separately) | ≤ 5 min | > 10 min → not in the loop; users revert to manual merging |
| **M-5** | Repeated use | Developers running ≥ 2 integrations in week 2 after first use | ≥ 40% | < 15% → one-shot curiosity, not a habit |

M-1 is the single most important number in this document. It is the direct measurement of
the product's premise, and the research explicitly could not answer it (research `05` §7).
When it is low, every other metric is noise.

---

## 3. Product-health metrics

| Metric | Definition | Target | Action if missed |
|---|---|---|---|
| Time to first integration | install → first completed run | p50 ≤ 10 min (`NFR-101`) | Onboarding is broken; fix before adding features |
| Runs with complete evidence | runs whose bundle validates fully (`EVD-101`) | ≥ 95% | Evidence integrity bug; release blocker |
| False-blocking rate | deterministic findings that turned out not to be real problems | ≤ 2% | Determinism is untrustworthy; demote the finding class |
| `unverified` run share | runs with no configured checks | ≤ 15% | Configuration is too hard, or the wrong users are adopting |
| Resume success rate | `resume` completing or precisely explaining, ÷ interruptions (`REC-103`) | ≥ 98% | Crash-safety claim is false |
| Crash rate | abnormal exits per 100 runs | ≤ 0.5 | Release blocker |
| Determinism failures | runs where identical inputs produce different candidate SHA or normalized report (`NFR-103`) | 0 | Release blocker; this is a core promise |
| Verification timeout rate | commands hitting timeout | ≤ 5% | Default timeouts are wrong |
| Flaky-check rate | commands flagged `flaky` by double-run (`VER-104`) | ≤ 3% | Either the default is too aggressive or the projects are; investigate before raising |
| Repair budget exhaustion | repair runs ending `budget_exhausted` ÷ repair runs started | ≤ 60% | If ~100%, repair adds cost without value; if ~0%, verification is too easy |
| Orphan artifact incidents | leftover worktrees/refs after any run (`REC-104`) | 0 | Safety bug; release blocker |
| User-work-touch incidents | any report of modified user files/HEAD/stashes (`REC-107`) | 0 | Severity-1; stop and fix |

---

## 4. Counter-metrics (guardrails against our own success)

Metrics that must *not* move the wrong way as we grow:

| Guardrail | Why |
|---|---|
| Median verification duration reported by users | If our overhead grows relative to their checks, we become noise (NFR-102) |
| Share of findings marked `interpretation` | If interpretation grows as a share of findings, we are drifting toward un-auditable output (`VER-106`) |
| Bypass pressure | Requests for `--no-verify`, `--auto-approve`, or `--skip-preflight` | Each request is a signal we are adding friction without value, or that a real workflow is being blocked (`INT-303`, `INT-602`) |
| Support questions per 100 runs | Rising questions mean the model is unclear, not that users are slow |
| Any telemetry sent, bytes | Must stay zero by default (`NFR-109`) |

The bypass metric deserves emphasis: the three most permissive defaults in the surveyed
products (T3-060 full access, CR-037 fail-open guard, CR-032 `--no-verify`) each began as
someone's reasonable request for less friction. Tracking the request itself is how we
notice before we comply.

---

## 5. Kill criteria and pivots

Honest thresholds, decided before the data arrives so we cannot rationalize afterwards.

| Condition | Conclusion | Action |
|---|---|---|
| M-1 < 2% over ≥ 50 multi-source integrations with configured checks | The combined-failure problem is rare in practice | **Stop building integration verification as a product.** Options: reposition as a "combined review artifact" tool for large batched changes; or release as a free utility with no revenue plan |
| M-2 < 5% and M-5 < 15% | People run it once and do not use the artifact | The CLI is a demo. Investigate whether a host-integrated step (shape B) changes the outcome before investing further |
| M-4 > 10 min at p50 | Too slow to be in the loop | Cut scope: subset checks, parallel commands, cached candidate; if still slow, the product is a CI job, not a local command |
| No team says yes to a $250 quote after 5 conversations (T1–T2) | No buyer | Ship free, keep maintainer-only, drop monetization |
| Security incident touching user work | A single occurrence invalidates the value proposition | Freeze features, publish a postmortem, fix the class of bug |

---

## 6. Measurement plan

**Where numbers come from.** All local; no server.

1. Local run log in `.ilvuno/metrics.jsonl` — per run: sources, checks, states, durations,
   finding classes, `combination_only` count. Readable by the user, always.
2. `ilvuno stats` — the user's own aggregate, shown to them first. This is the reward loop
   and the honesty mechanism: we show them their own M-1 before asking for anything.
3. **Opt-in** aggregate export (`NFR-109`): counts only, no repo names, no paths, no diffs,
   no SHAs. Explicit prompt, one-time, revocable, with a visible log of what would be sent.

**What we refuse to measure.** Repository identity, file paths, code content, check output,
decision text, passport contents. Anything that would make the aggregate useful for
training data or competitive intelligence is out of bounds.

**Review cadence.**
- Weekly: M-1 running average, M-4, crash/orphan incidents, bypass requests.
- Monthly: M-2, M-3, M-5, false-blocking rate.
- At 20 weekly actives: full kill-criteria review against §5. This is a scheduled
  conversation, not an emergency one.

---

## 7. Milestone scorecard

Gates between phases. A phase does not start until its gate passes.

| Gate | Requires |
|---|---|
| **G1** Ship v0.1 | All P0 acceptance criteria (PRD §26), determinism test green in CI, egress test green |
| **G2** Claim usability | ≥ 10 developers run ≥ 2 integrations each; M-4 ≤ 10 min; false-blocking ≤ 5% |
| **G3** Validate the premise | ≥ 20 weekly actives; ≥ 50 multi-source integrations; **M-1 measured, with a stated verdict** |
| **G4** Consider a paid layer | G3 passed (M-1 ≥ 5%); ≥ 3 teams asked for the same continuity/collaboration feature unprompted; T2 pricing test with ≥ 1 yes |
| **G5** Build hosting | G4 plus a written security review of the hosted surface (`SEC-105`, `SEC-107`) |

The ordering encodes the main strategic risk: **we do not build infrastructure for a problem
we have not measured.** Every failed product in the landscape comparison appears to have
built the workspace before validating the integration pain.

---

## 8. What success does not look like

- Thousands of stars and no M-1 data. (This is exactly the state of the two most popular
  tools in the survey.)
- A beautiful combined-diff UI that no reviewer opens.
- A repair loop that always succeeds because verification was too weak to be meaningful.
- A paid tier that exists because we needed revenue, not because a team asked.
- A "verified" badge that everyone trusts and nobody checks — the failure mode the product
  was built to eliminate.
