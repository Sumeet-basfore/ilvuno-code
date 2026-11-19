# ADR-0014 — Schema versioning and compatibility policy

Status: proposed. Date: Phase 3. Related: `VER-105`, `ORQ-106`.

## Context

Five documents cross a boundary: `journal`, `report`, `passport`, `bundle manifest`, and (future)
the RPC frames. Each will outlive a release and may be read by another version, another machine,
or a third-party tool. The MVP's formats are the future wire formats (doc `01` §4), so the
versioning policy is load-bearing, not administrative.

## Options

| Format | Typed | Diffable | Tooling | Verdict |
|---|---|---|---|---|
| **JSON Schema 2020-12, hand-authored, versioned URIs** | yes | yes | ubiquitous | chosen |
| Protobuf | yes | no (binary) | codegen, breaking-change tooling | rejected: nothing needs compactness; evidence must be readable |
| Avro/Parquet | yes | no | analytics-shaped | rejected |
| Go structs as the schema, no external file | yes | n/a | no cross-language consumers | rejected: third-party tools and mobile clients need a language-neutral contract |
| YAML for everything | yes | yes | losses in typing/precision | mixed: YAML only for the passport (human-authored) |

## Decision

1. Every document has `"schema": "ilvuno.dev/<kind>/vN"`.
2. **Within a major version: additive only.** New optional fields allowed; no renames, no
   removals, no type narrowing, no semantic change of an existing field.
3. **Breaking change = new major version**, with a reader that accepts `vN` and `vN-1` for at least
   two minor releases, and a migration command for the passport (the only user-authored doc).
4. Journal records are **never** rewritten; unknown fields are preserved on read and ignored, so a
   newer journal is readable by an older binary in read-only mode (resume is refused across major
   versions).
5. Enums are open: an unknown value is surfaced as `unknown(<value>)` and treated as non-blocking
   for display, blocking for gates (fail closed).
6. Versioned JSON Schemas ship inside the binary (`ilvuno schema <kind>` prints them), so tooling
   never needs a network fetch.
7. Consumers are told the policy; `VER-105` requires it be documented per document.

## Consequences

Positive: third-party tools can pin a major and keep working; evidence stays readable for years;
the mobile client can validate locally; schema changes become reviewable diffs.

Negative: two schema artefacts per document (file + embedded), and a discipline cost on every PR
(additive-only is a review rule). Also `unknown(enum)` handling must be written carefully or gates
could fail open — hence rule 5's asymmetry.

## Risks

| Risk | Mitigation |
|---|---|
| Someone sneaks a breaking change into a patch release | Contract test diffs the schema against the released version and fails on removals |
| Passport schema churn breaks user files | Migration command + accepting `vN-1` |
| Journal read by a much newer binary | Resume refused across majors; read-only rendering allowed |
| YAML precision surprises (dates, `yes`) | Passport fields are strings with explicit formats; parser is strict; no YAML implicit typing relied upon |

## Revisit conditions

1. A consumer needs streaming/compact encoding at scale → add a transport encoding of the same
   schema, never replace the JSON form.
2. The passport schema accumulates historical baggage → v2 with a migration and a deprecation
   window.
