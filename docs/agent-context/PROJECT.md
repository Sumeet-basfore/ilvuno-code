# PROJECT — Ilvuno

Updated: 2026-11-20 · Sources: `docs/product/01`–`07`, `docs/decisions/open-questions.md`.

## Mission

**Parallel work. Verified together.** Ilvuno answers: *these N agent changes each passed alone;
do they still work together, and can I prove it?* It is an integration-first workspace, not
another agent dispatcher. [approved] (`docs/product/01-vision.md`)

## Problem

Every surveyed tool verifies an agent's change against that agent's own base/worktree. When
changes meet, that verification no longer describes the combined artifact. Ilvuno builds the
combination, runs configured checks on it, and records inspectable evidence. [approved]

## V1 shape

- Standalone, local-first **CLI**; single static Go binary; no UI, daemon, hosted service, or
  agent dispatch. [approved D-02, D-04, D-05; ADR-0001, ADR-0003]
- Takes git refs in, pins SHAs, builds a disposable candidate, verifies it, records evidence,
  stops for human approval. [approved architecture]
- Linux-first; macOS tested in CI; Git ≥ 2.38.1; Windows excluded from V1. [approved D-05, A-4]
- Apache-2.0; repository `https://github.com/Sumeet-basfore/ilvuno-code`; module
  `github.com/Sumeet-basfore/ilvuno-code`. [approved D-03, D-17, D-18]
- Config is a committed `ilvuno.toml`. [approved D-06]

## Non-goals (V1)

No UI/desktop shell; no background daemon or hosted service; no Windows; no agent dispatch or
control; no credential custody; no default telemetry. [approved] (`docs/product/05-mvp-scope.md`)

## Success

- M-1: real combined-failure rate measured, not assumed (currently the product's biggest
  unknown, Q-1). [proposal/unverified]
- North-star and supporting metrics: `docs/product/07-success-metrics.md`.
- Monetization is gated on traction (A-10 open); nothing paid is built now.

## Decisions

- **Decided:** D-01…D-06, D-16…D-18; planning A-4, S-1…S-4, C-5, C-7.
  [approved] (`docs/decisions/open-questions.md` §0)
- **Open and blocking M2+:** D-07 candidate refs, D-08 merge-driver policy, D-09 write-scope
  classes, D-10 determinism scope, D-11 V1/V2 split, D-12 requirement re-prioritisation,
  D-13 measurement owner, D-14 claims policy, D-15 defer adapters.
- **Open product:** A-5 passport location, A-6 repair executor, A-7 AI interpretation,
  A-8 telemetry, A-10 monetization gate.

Do not implement an open decision's recommended option as if approved. Link the decision row.
