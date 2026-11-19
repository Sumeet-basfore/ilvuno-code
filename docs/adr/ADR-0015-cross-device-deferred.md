# ADR-0015 — Cross-device is designed and deferred: authoritative host + E2EE relay

Status: proposed. Date: Phase 3. Related: doc `10`, product `21`, `A-10`.

## Context

Desktop/mobile continuity is a stated objective but not an MVP requirement. Three architectures
were possible, and each has a distinct failure story in the survey:

| Model | Example | Failure story |
|---|---|---|
| **Authoritative host runtime; thin clients** | orca (`orcad`), t3code | host must stay up; mobile is online-only; mobile reconnect bugs (`ORC-097`) |
| Snapshot-and-resume elsewhere | cloudroom Teleport | moves the native session file exactly but leaves uncommitted tracked edits behind (`CR-021`); crash loops when state is missing (`CR-046`) |
| Sync files + no runtime | (design) | no live streaming; approval flow lags |

## Decision

1. **Nothing remote is implemented in the MVP.** The MVP is local-only, with no listener.
2. The chosen future model is **authoritative host + thin clients** (doc `10` §2), because:
   - it reuses the MVP's on-disk formats as the wire format;
   - it needs no state migration primitive (the failure-prone part of cloudroom's design);
   - it keeps one writer per worktree, which is the only writer model we have tested.
3. Transport is **WebSocket JSON frames replayed by `seq`**, E2EE to the host's key, with the relay
   as a broker that sees ciphertext and routing metadata only (`T3-054` model).
4. Pairing uses a single-use short-TTL code, an explicit fingerprint confirmation on the host, and
   DPoP-bound sessions — no trust-on-first-use, no legacy protocol version (`ORC-067`/`ORC-073`
   anti-patterns).
5. Scope model: `read:task`, `read:stream`, `approve`, `operate` (off by default), `admin`
   (host-only). Approving from a phone is allowed; **triggering a shell-capable run from a phone is
   not** (`XD-3`).
6. Conversation continuity is never claimed (`TSK-112`).

## Consequences

Positive: no network surface in v0.1 (nothing to secure, nothing to update); the formats are ready;
the hard part (state migration) is avoided by design.

Negative: no remote story until revenue justifies the surface; if a competitor ships remote first,
we are behind on a feature that is adjacent rather than core.

## Risks

| Risk | Mitigation |
|---|---|
| Relay becomes a trust bottleneck | ciphertext only; approvals signed by device keys; relay compromise cannot forge approvals |
| `operate` scope becomes an RCE surface | off by default, per-device toggle, and refused on mobile form factors by policy |
| Building it before demand | Gate G4→G5 (`07-success-metrics.md` §7); the paid hypothesis must be tested first |
| Two-device editing of one passport | digest chain + `passport_conflict` (doc `04` §5); no CRDT |

## Revisit conditions

1. Gate G4 passes and ≥3 teams ask for continuity → build rung 2 (encrypted metadata sync) before
   any daemon.
2. Live streaming is requested by a paying cohort → daemon (doc `10` §4), with admission control.
3. A standards-based agent/session portability protocol emerges → re-evaluate the
   "no conversation migration" stance; it would still require provider support to be truthful.
