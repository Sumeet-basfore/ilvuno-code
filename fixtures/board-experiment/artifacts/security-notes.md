# Security notes: Ilvuno security model (v1)

- **Agents are not sandboxed by Ilvuno.** They run under the permissions the user already granted their CLI and under the vendor's own sandbox. Ilvuno's job is to avoid handing agents more access.
- **Scope is narrow:** single user, local machine, no server, no multi-tenancy.
- **The repository is untrusted.** Files such as README, issues, fixtures, and dependencies can contain instructions that a prompt-injected agent may follow. The main risk is repo content leading an agent to read files or run shell commands. Prompt injection is rated high residual risk.
- **The runner enforces a few things:** credentials stay out of agent environments; each worker gets its own worktree and cannot switch branches or touch `main`; merges happen only in the runner after an accepted round; diffs outside the declared scope are flagged (detection, not prevention); and every command and effect is journaled with a request id.
- **Known limits:** there is no network allowlist, so an injected agent with shell access could send files out. Ilvuno does not sandbox agents, cannot stop a determined injection, does not protect a root-compromised machine, and does not protect against a malicious provider CLI.
- **Defaults are conservative:** approvals on (never bypass), round-based messaging, a human gate on every round, a budget of 3 rounds or 150k tokens, and no telemetry.
- **Incident response:** freeze the run to park threads while keeping worktrees, read the journal, and never rely on the runner to delete branches. If credentials may have leaked, rotate them at the provider, since Ilvuno never stores them.
- **Context is an attack surface.** Injected skill descriptions cost tokens on every turn and add untrusted text. v1 shows the injected-context size per thread but does not strip it.
