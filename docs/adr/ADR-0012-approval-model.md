# ADR-0012 — Approvals: interactive, digest-bound, signed records outside the repo

Status: proposed. Date: Phase 3. Related: doc `09` §4.5, `INT-602`/`INT-603`.

## Context

Approval is the human gate before landing. Two constraints collide:

- The passport is a repo file, so anything that can write the repo (including an agent) can edit it.
- The tool must be usable on a laptop with no keys, no accounts, and no server.

Options: no approval concept (report only), an approval flag in the passport, a confirmation
prompt only, signed records in `.ilvuno/`, or a hosted approval service.

## Options

| Option | Prevent forgery | Portable | Friction |
|---|---|---|---|
| Nothing (report only) | n/a | — | none, but no gate |
| `approved: true` in the passport | no (agent can write it) | yes | low |
| Prompt only, nothing persisted | n/a (no record) | — | low |
| **Digest-bound approval record in `.ilvuno/`, ref + digest in the passport; Ed25519 signature when a key exists** | partly (local tamper) → yes with a signature | yes (ref + digest) | low (TTY + typed SHA) |
| Hosted approval service | yes | yes | high (account, network) |

## Decision

1. `approve` requires a TTY; there is no `--yes`, no `--auto-approve`, and non-interactive mode
   refuses. The user types the candidate's short SHA to confirm.
2. The approval body binds `(candidate_sha, evidence_digest, scope, principal, at)`.
3. The authoritative record lives at `.ilvuno/approvals/<apr_id>.json`; the passport holds only
   `{id, digest}`.
4. Ed25519 signature when `ilvuno key init` has been run; otherwise `unsigned: true` and the report
   says so.
5. Validity requires: approval exists locally, digest matches, SHA matches the current candidate,
   passport chain intact. Any failure closes the gate with a specific reason.
6. Invalidation is a field (`invalidated_by`), never a silent status change.

## Consequences

Positive: an agent editing the passport cannot open the gate (no matching local record); no
account or server needed; portability preserved because the passport carries refs, not secrets;
the unsigned/ signed distinction gives a path to team use without breaking the single-user
experience.

Negative: a determined attacker with write access to the repo **and** `.ilvuno/` and the ability to
recompute digests can forge consistency. Only the signature defeats that, and the signature is
optional in MVP. Documented (`09` §4.5), not hidden.

## Risks

| Risk | Mitigation |
|---|---|
| Users find typed-SHA confirmation annoying | It is the point; muscle-memory approval is how wrong candidates land |
| Key loss | Unsigned mode still works locally; a lost key invalidates past signatures for verification purposes — documented |
| Approval on machine A used on machine B | digest-bound; works if the bundle travels; the passport carries the ref |
| Multi-user confusion | Signatures required when a `team` config is set (post-MVP, `SC-3`) |

## Revisit conditions

1. Teams arrive → require signatures, add device keys (doc `10` §3), and refuse unsigned approvals
   under a team config.
2. Remote approval (doc `10` §7) → the host remains the author of record; the device signs a
   request, the host verifies and writes.
3. A hosted approval service is proposed → only if it does not become *required* for local use.
