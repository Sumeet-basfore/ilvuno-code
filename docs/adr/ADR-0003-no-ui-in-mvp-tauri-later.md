# ADR-0003 — No UI in V1. Desktop shell not decided (Tauri + React is a candidate, not an approval).

Status: accepted (2026-11-20, owner approval of D-02, which records A-1; see `docs/decisions/open-questions.md` §0). Date: Phase 3. Implements product decision `A-1` and the C/D rejection.

**This ADR approves no desktop, web or mobile framework.** V1 being CLI-first does not make any shell approved. Tauri and React are candidates to evaluate if a shell is ever proposed. Electron is not a candidate. Any shell needs its own decision.

## Context

The approved MVP is a CLI. Phase 3 was nonetheless asked to evaluate WebSocket, React/Vite, and
Electron vs Tauri, because the roadmap includes a desktop shell and a mobile read/approve client
(doc `10`). Deciding the shell now costs nothing and prevents a later rewrite of the transport
layer.

## Evaluation

| Option | MVP fitness | Desktop fitness | Verdict |
|---|---|---|---|
| CLI only | Exact | none | **MVP: chosen** |
| CLI + local web server (browser UI) | Adds a server, auth, ports, streaming for zero product requirement | Usable but a second product | Rejected |
| Electron shell | n/a | Mature, huge; ~120 MB baseline, Chromium coupling, frequent security updates, Node in the renderer surface | Rejected |
| Tauri shell | n/a | Small binary (~5–15 MB), system webview, Rust host, explicit capability/permission model | **Candidate to evaluate if a shell is ever proposed. Not approved.** |
| Native (SwiftUI/WinUI) | n/a | Per-platform rewrite ×2 | Rejected |

Transport comparison for the future:

| Transport | Fit |
|---|---|
| Loopback WebSocket + JSON frames | Good: same records as the journal, replay by `seq`, works for desktop and relay |
| stdio JSON-RPC | Works for local CLI↔daemon; useless for mobile |
| gRPC/HTTP2 | Strong typing, but adds protobuf/codegen for a protocol whose records are already JSON |
| Unix socket + JSON | Fine locally, not cross-device |

## Decision

1. **MVP ships no UI, no server, no WebSocket, no React, no Vite.** Not "later this sprint" —
   absent. The contract is stdout/stderr/exit codes plus files.
2. No shell is approved by this ADR. If a shell is later proposed, **Tauri + React** is the
   candidate to evaluate. It would speak WebSocket JSON frames to the host runtime, reusing the
   journal records and the report/passport/bundle documents as the protocol (doc `10` §6). That
   approval would be a separate decision.
3. Electron is rejected specifically for binary size, update cadence, and an unnecessarily large
   attack surface for a tool that holds repository access.
4. React (not Svelte/Solid) for the ecosystem and contributor familiarity, accepting the larger
   bundle — the shell is not performance-critical.

## Consequences

Positive: the MVP stays a 25 MB binary with no daemon; nothing to secure, port-scan, or update
remotely; the scoring of "time to first integration ≤ 10 min" is unaffected by a UI.

Negative: no visual affordance for the combined diff — reviewers get Markdown. That is a
deliberate trade: the report is the artifact, and Marco reads a report before he opens a window.

## Risks

| Risk | Mitigation |
|---|---|
| A Tauri shell with a Go core would mean two languages | Not accepted by this ADR. Assessed when a shell is proposed; ADR-0001 revisit condition #1 applies only then |
| Users expect a GUI and bounce | The report is designed to be readable text; a diff can be `ilvuno show --diff \| delta`. Revisit if `M-2`/`M-5` show UI friction caps adoption |
| Building the shell prematurely | Gate G4 (`07-success-metrics.md` §7) must pass; the shell is not in any v0.x plan |

## Revisit conditions

1. Gate G4 passes **and** ≥3 paying teams ask for a GUI.
2. A Tauri pilot shows the Go/Rust split is workable (or triggers ADR-0001 revisit).
3. A reviewer workflow proves impossible in Markdown (unlikely; it would be an evidence-format
   problem, not a UI problem).
