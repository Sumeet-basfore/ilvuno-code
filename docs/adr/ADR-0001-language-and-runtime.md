# ADR-0001 — Language and runtime: Go

Status: proposed (supersedes the Phase 2 recommendation only in *justification*, not in choice;
requires approval as `A-3`). Date: Phase 3.

## Context

The MVP is a single-binary local CLI that: shells out to `git` for every VCS operation, spawns
and supervises project commands in process groups, appends fsynced JSONL records, renders
JSON/Markdown, and must be trivially installable by a developer who already has git and a
language toolchain of their own. Phase 2 recommended Go (`A-3`); Phase 3 was asked to evaluate
TypeScript/Node and Rust. This ADR evaluates all three honestly, then decides.

## Evaluation

| Dimension | TypeScript/Node | Rust | Go |
|---|---|---|---|
| Single static binary | Needs SEA/pkg/bun-compile; 40–90 MB with runtime; platform quirks | Yes, static, ~10–20 MB | Yes, static, ~15–25 MB |
| Cross-compile | Painful (native addons, platform binaries) | Good (`--target`), needs linkers for some targets | Trivial (`GOOS`/`GOARCH`) |
| Process groups / signals | `detached:true` + `kill(-pid)` works; weak typing around `WaitStatus` | Excellent (`nix`, `libc`) | Good (`SysProcAttr{Setpgid}`, `os.ProcessState.Sys()`) |
| Crash robustness | Runtime exceptions are easy to miss; unhandled rejection can exit oddly | Excellent (no unwinding surprises, `?` everywhere) | Good (explicit errors; panics must be avoided by discipline) |
| State-machine expressiveness | Weak (unions by convention unless heavy zod work) | Strong (enums with data, exhaustive `match`) | Adequate (tagged structs + validated transitions) |
| Event/JSONL processing | Excellent (`JSON`, streams) | Excellent (serde) | Good (`encoding/json`, streaming decoders) |
| Git integration | Shell out (same as all three) | Shell out, or git2/gitoxide | Shell out, or go-git |
| Agent SDK availability | Best (`@anthropic-ai/claude-agent-sdk`) | None; protocol-level only | None; protocol-level only |
| Contributor pool | Largest | Large and growing | Large, ops/CLI-heavy |
| Compile/iterate speed | Instant (no compile) | Slow-to-medium (borrow checker on shared state) | Fast |
| Memory footprint | ~40–80 MB RSS | Low | Low |
| Dependency surface risk | Large transitive tree | Medium (though `cargo` audits work well) | Small (stdlib-heavy) |
| Time to a testable v0.1 (est.) | 0.8× | 1.6× | 1.0× |

## Decision

**Go.** Reasons that survive scrutiny:

1. The strongest argument for TypeScript — the Claude/Codex SDKs — is **neutralised by ADR-0010**:
   the MVP has no provider SDK, and the long-term adapter boundary is protocol-level
   (`codex app-server` JSON-RPC, ACP), where language is irrelevant.
2. Distribution hygiene is a product requirement (`NFR-104`, `NFR-105`: zero runtime deps, one
   binary). Go and Rust both satisfy it; Node satisfies it only with a bundler whose output is
   large and platform-fragile.
3. The work is I/O orchestration — subprocess, files, git — which is Go's centre of mass. It is
   not CPU-bound (no diffing engine, no parsing at scale) and not concurrency-exotic (one
   mutating run, sequential commands).
4. Small-team practicality: fast compiles, one obvious way to do most things, easy onboarding for
   a contributor who wants to fix a bug in a git-plumbing tool.

## Alternatives considered

- **TypeScript/Node** — rejected for MVP. Would win if the product were a workspace app with a
  React shell and provider SDK integration; that is precisely the shape we rejected (product
  `05` §2.1). Would also win if contributor volume were the binding constraint.
- **Rust** — rejected *for now*, not on merit. It gives stronger invariants (which the state
  machines would benefit from) and a stronger process-control story. It costs iteration speed on
  exactly the code that will churn most (the engine's state machine) and shrinks the maintainer
  pool. It becomes the right answer if (a) the team grows past 3 engineers, (b) the core becomes
  CPU-heavy (large-scale semantic diffing), or (c) the desktop shell becomes Tauri.

## Consequences

Positive: one binary, trivial cross-compile, fast CI, small dependency tree (stdlib + nothing),
which makes the "zero runtime deps" claim auditable.

Negative: state-machine invariants are enforced by tests and constructors rather than the type
system; the future Tauri desktop shell would introduce a Rust component alongside a Go core.

## Risks

| Risk | Mitigation |
|---|---|
| Invariant drift in a dynamically-shaped model | Constructor-only construction for entities; transition functions validated; property tests (`DET-3`, `JRN-2`) |
| Panics in a CLI are ugly | `recover()` at the command boundary → typed `internal_error` + `explain`-able message; a panic test asserts the exit code |
| Two-language future (Go core + Rust/Tauri shell) | Accepted for now; revisit trigger below |

## Revisit conditions

1. The desktop shell is approved **and** Tauri is chosen → re-evaluate moving the core to Rust to
   avoid a Go+Rust systems split (see ADR-0003).
2. The engine gains a CPU-heavy component (semantic diffing, symbol indexing at scale).
3. The maintainer team exceeds ~3 engineers and invariant failures become the top bug class.
4. A provider's only viable integration path becomes a language-specific SDK with no protocol
   equivalent.
