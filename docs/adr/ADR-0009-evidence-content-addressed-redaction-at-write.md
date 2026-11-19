# ADR-0009 — Evidence: content-addressed bundles, redaction at write time

Status: proposed. Date: Phase 3. Related: ADR-0002, doc `09` §6.

## Context

Evidence is the product's differentiator (`EVD-1xx`): it must outlive the binary, be verifiable
offline, be attachable to a review, and never leak secrets. Two symmetric failures to avoid:
evidence that is unverifiable (a screenshot in a comment) and evidence that is unshareable (raw
logs full of tokens).

## Options

| Option | Verifiable | Shareable | Cost |
|---|---|---|---|
| **Content-addressed bundle with manifest + digests, redaction at write** | yes (offline) | yes (redacted) | ~200 LOC + redaction rules |
| Single JSON blob | partly (no file digests) | yes | simple, but logs get huge and unreadable |
| Screenshot/HTML report | no | yes | not machine-checkable |
| Raw logs only | no | no (secrets) | cheapest |
| Signed bundles | yes + authorship | yes | key management in the hot path |
| Database-backed store | yes | needs export | ADR-0002 rejected |

## Decision

1. A bundle is a directory with `manifest.json`, `report.{md,json}`, `findings.json`, and logs;
   every file is listed with `sha256` and `bytes`; `manifest_digest` covers the manifest body.
2. `bundle verify` recomputes everything offline and reports `valid | invalid(reason)`, including
   *unlisted files*.
3. **Redaction happens before write**, not at export. Rationale: logs are pasted into issues and
   chats long before anyone exports; and "we'll redact later" is how leaks happen.
4. Bundles are **not signed**. Approvals are (ADR-0012). A bundle is digest-bound to an approval,
   which is the binding that matters.
5. `--keep-raw` exists for local debugging only, is refused with `--json`/bundle export, and is
   recorded in the journal.

## Consequences

Positive: `ilvuno bundle verify` is a complete answer to "is this evidence genuine?" within the
unsigned threat model; the bundle is a single attachable artifact; secrets never hit disk.

Negative: redaction can mangle legitimate log content (entropy heuristic). Mitigation: three
classes, a 200+200 corpus test (`RED-1`), and user-configurable patterns. Also: unsigned bundles
can be fully re-forged by a determined editor who also re-approves — documented in `09` §4.5.

## Risks

| Risk | Mitigation |
|---|---|
| Redaction misses a secret (false negative) | Conservative patterns + entropy rule + `spec.redact`; report never claims "no secrets" |
| Redaction destroys useful output (false positive) | Corpus test; markers are reversible only by rerunning with `--keep-raw` locally |
| Bundle size grows unbounded | 64 MiB/stream cap, 512 MiB/bundle cap, `bundle prune` with tombstone |
| Users edit bundles to "fix" output | digest mismatch; approval invalidated |

## Revisit conditions

1. Team adoption (multi-user) → bundles must be signed, or shipped through a store that signs
   (`ADR-0015`).
2. A secret scanner becomes a requirement → separate tool, not a half-accurate feature here.
