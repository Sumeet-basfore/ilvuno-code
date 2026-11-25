# Paseo and Ilvuno

Review target: [`getpaseo/paseo`](https://github.com/getpaseo/paseo/tree/cfcf0dc7d11957199730d6722653e729cfa571b2)
at commit `cfcf0dc`, checked on 2026-11-25.

## What Paseo is

One local daemon owns the workspaces (`public-docs/why.md:11-21`). Paseo has four app clients: desktop, web,
iOS and Android. The CLI also connects to it (`README.md:42-50`). The five-review summary says Paseo has
nine native providers. Its provider docs list ten (`public-docs/providers.md:21`). The docs also list 30+ ACP
entries (`public-docs/providers.md:22`). The summary recorded 25.1k GitHub stars on 2026-11-25. This is a
dated snapshot, not a current count. The repository is Apache-2.0, with a carve-out for third-party code
(`LICENSE:1-8`). Its README says one person built it (`README.md:227`). That is the publisher's claim, not an
independent count.

## What it proves

Many agents in one app are already common. Paseo has several clients, worktrees, agent messages, schedules
and plugins. A longer provider list does not make Ilvuno different.

Two reviews independently looked for a run-level done spec, round journal, replay and accept gate in Paseo.
They found provider permission prompts and a human merge, not a recorded round. Ilvuno's distinct work is to
record rounds, show whether the done checks passed, replay the run and wait for a person to accept the
changes.

## Four verified facts

| Fact | Paseo source |
|---|---|
| Without a password, an admitted session receives owner permissions. | `packages/server/src/server/session-admission-auth.ts:17-19` |
| Agent processes inherit the daemon environment, except for four Claude variables. | `packages/server/src/server/agent/provider-launch-config.ts:208-216,238-252` |
| The server package depends on the Claude Agent SDK. | `packages/server/package.json:110` |
| The worktree service proxy builds a `<label>.localhost` address. | `packages/server/src/server/service-proxy.ts:147` |

## Copy, avoid and ignore

**Copy**

- Keep small recorded provider fixtures with their source and version. Tests can replay them without a live
  provider. The Muse fixture guide shows this practice at `plugins/muse-provider/test/fixtures/README.md:3-45`.
- Keep one typed provider contract, a generic ACP adapter and separate native adapters when a protocol needs
  them. This matches Ilvuno's adapter boundary.
- Keep permission policy in the runner. Adapters should translate provider requests and answers.

**Avoid**

- Avoid vendor SDKs. Paseo's Claude and OpenCode adapters still span thousands of lines, and Ilvuno chose to
  speak the provider protocols directly.
- Avoid passing the daemon's environment through to agents. It can expose credentials that the runner does
  not need to give them.
- Avoid treating an absent password as permission to grant owner access. Ilvuno keeps its API on loopback and
  must add a token before any remote mode.

**Ignore**

- Ignore Paseo's mobile apps, relay and IDE panels. They support Paseo's broader cross-device workbench.
  Ilvuno's PRD defines a local desktop app and excludes mobile and hosted runners.
- Ignore Paseo's embedded browser and plugin marketplace. Ilvuno's PRD names both as v1 non-goals.
- Ignore schedules and 30-provider parity. They add surface without proving that a recorded round reaches a
  human gate.
