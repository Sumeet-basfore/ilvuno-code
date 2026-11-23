# Security notes: Ilvuno security model

Source: `docs/security.md`

## The short version

Ilvuno does not sandbox the agent processes it runs. Those agents are limited only by the permissions
already granted to their CLI and by the vendor's own controls. Ilvuno's job is to avoid handing agents
any extra power. The model covers v1 only: a single user on a local machine, with no server and no
multi-tenancy.

## Who is trusted

- **Trusted:** the user (they have final say on gates), the runner (Ilvuno's own code, which holds
  credentials and the journal), and the UI (it never sees credentials and cannot merge).
- **Not trusted:** the repository (its README, issues, fixtures and dependency names can all contain
  attacker-written text), the agent processes (they may have been prompt-injected by repository content),
  and the network (v1 has no inbound ports; only a local socket).
- **Opaque, treated as data:** the provider CLIs. Their output is read as data, not as instructions.

The main risk path is repository text reaching an agent, and the agent then acting on the filesystem or
shell. An agent will treat instruction-like text in a file as real instructions.

## What the runner enforces

1. Provider credentials never go into an agent's environment beyond what the CLI needs to run. Runner
   secrets are kept in the OS keyring and referred to by name.
2. Each worker gets its own worktree and cannot switch branches. A worker cannot see or commit to `main`.
3. Merges happen only after an accepted round, and the runner performs them. An agent can request a merge,
   but the request does nothing on its own.
4. Scope is checked when work is dispatched and when it is reported. A diff outside the declared scope is
   shown in the readiness report and blocks `pass`. This detects problems; it does not prevent them.
5. Every command and effect is journaled with a request ID, so an incident has a timeline.
6. There is no network allowlist. An injected agent with shell access could send files out, and Ilvuno
   can log commands but cannot stop that without a sandbox.

## Rules a reader must follow

- Do not start an agent in bypass permission mode. Approvals stay on by default.
- Do not give workers a git remote. Local git damage can be recovered; a remote push cannot.
- Workers must not push, force-push, delete branches, or run `reset --hard`. Only the runner touches `main`.
- Keep artifacts local. Reports can include source code and possibly secrets.
- Pin adapter package versions, because the adapters are fetched with `npx -y` on first run.
- Keep the human gate on for every round.

## Defaults

| Setting | Default |
|---|---|
| Permission mode | `default` (approvals on) |
| Message policy | `round` |
| Human gate | on for every round |
| Budget | 3 rounds / 150k tokens |
| Worker network access | whatever the CLI already has; Ilvuno adds none |
| Telemetry | off (none in v1) |

## Context size

Skill descriptions are injected into every agent session, and that adds to token cost and to the amount of
untrusted text an agent reads. v1 shows the injected-context size for each thread in the run detail. It
does not remove anything automatically.

## If something goes wrong

1. Run `ilvuno run freeze <run>` to park every thread and keep the worktrees.
2. Read the journal. Every dispatch, command and diff is recorded with a request ID.
3. The runner never deletes branches. Recover with normal `git`.
4. If credentials may have leaked, rotate them at the provider. Ilvuno never stored them.

## What Ilvuno does not claim

- It does not sandbox agents.
- It does not stop a determined prompt injection from sending data out.
- It does not protect a machine where the user is already root-compromised.
- It does not protect against a malicious provider CLI.
