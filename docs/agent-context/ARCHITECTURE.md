# ARCHITECTURE — Ilvuno

Updated: 2026-11-20 · Status: **design approved; implementation partial**.
Authoritative detail: `docs/architecture/01`–`12`, `docs/adr/README.md`.

## Hard properties

Local-first; modular; recoverable; provider-independent core; explicit write permissions;
deterministic where correctness matters; fail closed; security treated as arbitrary code
execution; small dependency surface. [approved] (`docs/architecture/01` §1)

## Boundaries that must not be crossed

1. **Ilvuno never talks to an agent.** Agents produce commits; Ilvuno consumes refs. No provider
   SDK, credential, or session in V1. [approved ADR-0010]
2. **Ilvuno never re-implements git.** Every VCS operation is a `git` subprocess. [approved ADR-0004]
3. **`engine` never imports `agent`.** Enforced by `internal/lint/importrule`.
   [approved architecture; implemented lint rule]
4. **No absolute paths, hostnames, or environment values in persisted artifacts.**
   (`INV-9`, `INV-13`) [approved]
5. **Fail closed.** A failed check refuses; it does not pass with a warning. [approved ADR-0008]

## Module map

`cli → engine → {analyze, verify, evidence, passport, gitx, model, policy}`
`agent → {model, policy}` (post-MVP; never imported by engine)
`model`, `policy` → standard library only.

| Module | Responsibility | Status |
|---|---|---|
| `cli` | args, exit codes, output | partial: version/help |
| `engine` | run orchestration | not built |
| `gitx` | all git access | not built |
| `analyze` | deterministic findings | not built |
| `verify` | project commands + evidence | not built |
| `evidence` | content-addressed record store | not built |
| `passport` | task state/approvals | not built |
| `policy` | single write-scope gate | not built |
| `model` | domain types/invariants | not built |
| `agent` | provider adapters | post-MVP; not built |

M0 exists to prove the harness; it creates only `cmd/ilvuno`, `internal/lint/importrule`, and
`internal/testkit/*`. Empty production packages are forbidden. [approved readiness/06 §3.2]

## Current M0 structure

- `cmd/ilvuno` — `--version`/`--help` only.
- `internal/lint/importrule` — dependency-boundary analyzer.
- `internal/testkit/{forge,fixtures,safety}` — deterministic fixture construction, corpus,
  safety fingerprint.
- `testdata/reference` — SP-1R3 reference harness and expectations.
- `.github/workflows/ci.yml` — build/test/lint, Git floor, reference jobs. [implemented,
  uncommitted; CI not run]

## Design documents

| Topic | Doc |
|---|---|
| Overview, boundaries | `docs/architecture/01-overview.md` |
| Domain model, invariants | `02-domain-model.md` |
| Adapters (post-MVP) | `03-agent-adapters.md` |
| Orchestration | `04-task-orchestration.md` |
| Worktrees / candidate refs | `05-worktrees.md` |
| Integration intelligence | `06-integration-intelligence.md` |
| Verification and safety | `07-verification.md` |
| Persistence and recovery | `08-persistence-and-recovery.md` |
| Security boundary | `09-security.md` |
| Cross-device (future) | `10-cross-device.md` |
| Testing strategy | `11-testing.md` |
| Observability | `12-observability.md` |

Read only the doc a task cites; do not infer approval from a design document. Accepted ADRs are
the decision record; proposals such as D-07…D-10 are not.
