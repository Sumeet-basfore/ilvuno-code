# ADR-0010 — Agent adapters: protocol-level, no credential custody, `command` only in MVP

Status: proposed. Date: Phase 3. Related: doc `03`, product `A-6`, `A-7`.

## Context

Ilvuno must stay provider-independent at the core (architecture principle) while eventually
supporting Claude Code, Codex, and others. Two temptations must be resolved explicitly:

1. Integrate a provider SDK now, because it is the fastest path to a "runs agents" demo.
2. Route provider traffic through Ilvuno so a hosted feature can use "the user's subscription".

Both are traps: (1) drags the product into the commoditized dispatch segment; (2) makes Ilvuno an
unlicensed reseller of someone else's subscription and a custodian of credentials.

## Options (integration mechanism)

| Mechanism | Fidelity | Brittleness | Credential exposure |
|---|---|---|---|
| **Documented protocols: Claude Agent SDK / stream-JSON, Codex `app-server` JSON-RPC, ACP** | High | Version-coupled per adapter, isolated | none (the user's binary authenticates) |
| Provider HTTP APIs directly | High | Stable | **requires custody of API keys → rejected** |
| PTY + screen scraping / OSC sequences | Low | Very high (orca's ~45-agent path is the cautionary tale, `ORC-011`, `ORC-088`) | low, but status is guesswork |
| No adapter at all (refs only) | n/a | none | none |

## Decision

1. **MVP ships exactly one adapter: `command`** (an arbitrary argv/shell command with a repair
   brief on stdin). No provider SDK, no credential, no session. It exists to satisfy `INT-502`.
2. Post-MVP adapters use **documented structured protocols only**. No PTY scraping, no OSC
   parsing, no output-text inference.
3. **No credential custody (`AUTH-1..4`)**: Ilvuno never reads, stores, refreshes, proxies, or
   transmits provider credentials. It executes the user's installed binary.
4. **Consumer subscriptions are not API entitlements (`AUTH-3`)**: no server-side feature may
   assume that a logged-in CLI implies API access.
5. `engine` must not import `agent`; enforced by a static check (`11` §6).
6. Capability sets are validated against implementations at construction (the orca registry rule,
   `ORC-012`), so a declared-but-unimplemented capability is a startup error.

## Consequences

Positive: the core is genuinely provider-independent; the MVP has no provider surface to break;
a provider's version drift cannot break integration; the security story has no credential to leak.

Negative: no "run my agents" demo — the product must earn attention on verification rather than on
orchestration. Adapters, when built, will each need conformance fixtures and version probes.

## Risks

| Risk | Mitigation |
|---|---|
| Users expect Ilvuno to launch agents | Product messaging is explicit: "any agent that produces git commits"; `command` is for repair only |
| An adapter's protocol changes | Conformance fixtures + `doctor` probe; drift becomes a test failure |
| Pressure to add credential-proxying for a hosted feature | Written rule (`AUTH-2`) and an ADR amendment required to change it |
| `command` adapter is used as a de-facto agent runner | Its capabilities include `StructuredEvents:false`; dispatch is refused without a real adapter |

## Revisit conditions

1. A metric justifies dispatch (gate G4 + user request by name) → build `claude` and `codex`
   adapters against the interface, starting with the two most-requested.
2. A provider publishes a stable, versioned, language-neutral protocol (ACP-like) for all major
   agents → prefer it over per-provider adapters.
3. Ilvuno ever needs to run agents on a server it operates → that is a different product decision
   (`AUTH-2`, `AUTH-3`), requiring its own ADR.
