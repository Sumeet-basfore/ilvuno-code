# Security notes: docs/security.md

## Main topic

Ilvuno's security model for v1 (single user, local machine, no server). The key point: **Ilvuno does not sandbox the agent processes.** They are limited only by the permissions already granted to their CLI and by the vendor. Ilvuno's job is to avoid handing them more access.

## Trust model

- **Trusted:** the user, and the runner (our code, which holds credentials and the journal).
- **Not trusted:** the repository (its files can carry prompt injection) and the agent processes (they can be injected by the repo).
- **Vendor-trusted but opaque:** provider CLIs. Their output is treated as data.
- The main risk is a hostile file in the repo steering an agent into reading secrets or running shell commands.

## Rules the runner enforces

1. Provider credentials never go into an agent's environment. They stay in the CLI's store or the OS keyring.
2. One worktree per worker. No branch switching. Workers cannot touch `main`.
3. Merges happen only in the runner, and only after an accepted round. An agent's merge request does nothing by itself.
4. Scope is checked at dispatch and at report. Out-of-scope diffs are shown and block `pass`. This detects problems but does not prevent them.
5. Every command and effect is journaled with a request id.

## Known limits (not sandboxed)

- No network allowlist. An injected agent with shell access could send files out.
- No protection against a malicious provider CLI, or a machine where the user is already root-compromised.
- Malicious skills or MCP servers loaded by the agent are outside our control. We only report what the agent loaded.

## Defaults

- Permission mode `default` (approvals on). Never start an agent in bypass mode.
- Message policy `round`.
- Human gate on every round.
- Budget 3 rounds / 150k tokens.
- Telemetry off.

## Action for readers

- **Do not give workers a git remote**, and do not allow force-push, `branch -D`, or `reset --hard` by workers. Local damage is recoverable; remote damage is not.
- **Pin adapter versions** that are fetched with `npx -y`, and record the resolved version in the journal. The first run downloads code.
- **Keep the human gate on.** Do not turn on bypass permissions.
- **Review the context size** shown per thread. Injected skill descriptions are a cost and an attack surface, and the user can trim them.
- **If credentials may have leaked,** rotate them at the provider. Ilvuno does not store them.
- **Incident steps:** `ilvuno run freeze <run>`, then read the journal. Branches are never deleted by the runner.
- **Artifacts** (reports, journal) can contain source and possibly secrets. Keep them local. This changes if anything is ever hosted.
