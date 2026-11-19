# ADR-0008 — Verification executes project code: allowlist env, process groups, no sandbox claim

Status: proposed. Date: Phase 3. Related: doc `09` §7.

## Context

There is no way to verify a change without running the project's own toolchain. That means
executing arbitrary code from the repository — the same trust decision as running CI, but on the
developer's machine and inside our process tree.

Options considered: no sandbox (documented), process-group supervision with env allowlist, OS
sandboxing (seatbelt/landlock/seccomp), container/VM execution, or refusing to run anything and
staying static-only.

## Evaluation

| Option | Security gain | Cost | Fit for MVP |
|---|---|---|---|
| Static-only (no execution) | n/a | Product becomes useless (`unverified` always) | rejected |
| **Process groups + env allowlist + explicit non-claim** | Bounded blast radius from *our* mistakes; secrets kept out of the child | ~200 LOC | **chosen** |
| seatbelt (macOS) / landlock+seccomp (Linux) | Real filesystem/network restriction | Per-OS implementation, breaks most toolchains (`~/.cache`, network for package downloads), false sense of security on macOS | rejected for MVP |
| Container per verification | Strong | Requires docker/podman on the user's machine; cold start dominates a 5-min budget; credential mounting problems (cloudroom's sandbox struggles, `CR-035`–`CR-038`) | deferred |
| VM | Strongest | Absurd for a CLI | deferred |

The decisive argument against OS sandboxing in v0.1: most real check suites need the network
(first `pnpm install`), the home cache, and writes outside the worktree. A sandbox tight enough
to matter would make the tool fail on the majority of real repos, and a sandbox loose enough to
work would be security theatre — which the research singles out as worse than none
(`CR-037` fail-open guard).

## Decision

1. Verification runs as a child process in a **new process group**, with an **allowlisted
   environment** (deny-by-default), a per-command timeout, and TERM→KILL escalation.
2. Output is capped and redacted at write time.
3. The report states, in a machine-readable `environment` block, what isolation was and was not
   applied: `{"sandbox": "none", "process_group": true, "env_allowlist": true}`.
4. Documentation and `doctor` say plainly: *verification runs the repository's code as you, with
   your filesystem and network access* (`09` §7).
5. A future container executor must implement the same `(argv, cwd, env)` interface — the seam
   exists deliberately, with no redesign required.

## Consequences

Positive: works on real repos; no dependencies; secrets are kept out of the child; orphans are
killed; the claim is honest.

Negative: a malicious repo can still do anything the user can. That is a documented limitation,
not a hidden one, and it is the same exposure as `npm test`.

## Risks

| Risk | Mitigation |
|---|---|
| A test suite deletes files outside the worktree | Cannot prevent. Detected only in the sense that the user notices. Documented |
| A check writes to `$HOME` and pollutes later runs | Allowed (needed); `tree_before/after` covers the worktree only |
| Users over-trust a `passed` | Report wording rules + banned-claims test (`VER-101`) |
| Container users expect isolation | `doctor` states isolation: none; roadmap documents the seam (SC-1) |

## Revisit conditions

1. A user (or a team) requires containment for compliance → implement a container executor behind
   the same interface, opt-in, with its own honest capability block.
2. Package downloads make network isolation viable (offline caches standard) → tighten.
3. A macOS/Linux sandbox that does not break common toolchains appears → evaluate again.
