# ADR-0013 — Repo hooks and signing are disabled in candidate worktrees

Status: proposed. Date: Phase 3. Related: `INV-11`.

## Context

Git runs repository hooks on commit (pre-commit, commit-msg, post-commit). Worktrees share the
hooks directory with the main checkout. Hooks are arbitrary code from the repository, and for a
candidate merge they change the resulting commit in ways that break determinism, can exfiltrate
data, and can fail a run for reasons unrelated to the integration.

## Options

| Option | Determinism | Safety | Fidelity to the user's real merge |
|---|---|---|---|
| Run hooks normally | no (hook output varies, may block) | hooks run as the user, unattended | high |
| **Disable hooks in candidates (`core.hooksPath=`)** | yes | hooks never run unattended | lower — record the divergence |
| Ask the user per run | yes | yes | configurable, noisy |
| Run hooks but ignore failures | no | no | nonsense |

## Decision

Disable hooks for every commit-producing git command in candidate and repair worktrees, via
`-c core.hooksPath=` (empty value) and note the divergence in the report metadata:

```json
"gitenv": {"hooks": "disabled", "gpgsign": false, "conflict_style": "merge"}
```

Signing is disabled for the same reason: it changes the SHA and may prompt for a passphrase in the
middle of an unattended run.

## Consequences

Positive: deterministic candidate SHAs; no unattended execution of repository code beyond the
verification commands the user explicitly configured; no passphrase prompts.

Negative: a repo whose pre-commit hook mutates files (formatters, codegen) will produce a candidate
that differs from what the developer's own commit would produce. That divergence is stated in the
report rather than hidden, and the verification commands (which the user did configure) still run.

## Risks

| Risk | Mitigation |
|---|---|
| User expects hooks to run | Report `gitenv.hooks: disabled` + docs; `--run-hooks` opt-in exists in the design with the determinism caveat attached |
| A hook is the reason their commits pass CI | The verification commands are the check that matters; formatters are usually idempotent |
| `core.hooksPath=` is ignored by an old git | Tested against 2.38; `doctor` records the version |

## Revisit conditions

1. Users report meaningful divergence (e.g. codegen required before tests) → offer
   `--run-hooks` with an explicit determinism warning and a recorded flag.
2. Git adds a per-worktree hooks path natively → use it.
