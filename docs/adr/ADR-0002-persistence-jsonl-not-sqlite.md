# ADR-0002 — Persistence: append-only JSONL and files. No database, no queue.

Status: proposed. Date: Phase 3. Depends on ADR-0004 (git is truth for VCS).

## Context

We need: a crash-recoverable record of what a run did; an evidence store that outlives the
binary; a task record that is human-readable and committed to git; and duplicate-request
protection. Required scale: ~10¹–10³ records per run, one writer, single user, no queries beyond
"read this run".

Phase 2 open questions `T-8` (journal format) and `T-1` (source of truth) pre-answered this; this
ADR records the comparison and the rejection reasons, since the prompt requires evaluating SQLite,
persistent job queues, and event logs explicitly.

## Evaluation

| Option | Fit | Cost |
|---|---|---|
| **Append-only JSONL + files** | Exact fit: one writer, linear replay, human-readable, `tail -f` debuggable, no dependency | Hand-rolled fsync discipline and torn-tail handling (~200 LOC, fully testable) |
| SQLite | Overkill; brings CGO or a pure-Go port with different locking semantics; migrations for every schema change; opaque to users | 127 migrations in the closest prior art (`T3-020`), and projections that cannot be rebuilt after pruning (`T3-021`) |
| Embedded KV (bbolt/badger) | Binary format, no human readability, no benefit over files at this scale | Opaque evidence |
| Event log with an outbox (t3code-style) | Right shape, wrong scale: an outbox exists to decouple async side effects across a long-lived server | A worker + queue infrastructure for a foreground process |
| Persistent job queue (Redis/Asynq/SQLite queue) | No producer exists without a daemon; a refused run is an exit code | Operational surface with no consumer |
| Message broker | No | No |

## Decision

**Append-only JSONL journals per run, fsynced per record, with a SHA-256 hash chain and
contiguous `seq`; plus content-addressed files for evidence, YAML for passports, JSON for
reports.** No database. No queue. The journal *is* the queue: steps are executed in order by the
one process that owns the run, and a duplicate invocation returns the recorded result (`08` §8).

Three sub-decisions:

1. **`fsync` per record, not per batch.** Cost ~1–5 ms × ~50 records. A batch window is a window
   in which a side effect happened with no record — the one unrecoverable state.
2. **The journal is authoritative; `state.json` is a rebuildable cache.** Prevents the t3code
   failure mode where the log is pruned and the projection becomes unfalsifiable.
3. **Records are immutable and hash-chained.** A chain break is corruption/tampering and refuses
   resume and approval, rather than being repaired by guesswork.

## Alternatives rejected, with the reason that matters

- **SQLite**: the migration and schema-coupling burden is real, but the decisive point is
  *inspectability*. Evidence that a reviewer can `cat` and `jq` is a product feature
  (`EVD-101`); a `.db` file is not.
- **Queue**: would only exist to serve a daemon that does not exist. Adding it now would also
  invite background execution, which the product explicitly avoids (no daemon, `NFR-101`).

## Consequences

- Everything is grep/tail/jq-able; support is cheap.
- Deletion of `.ilvuno/` loses history, never meaning (`INV-14`).
- We own ~200 lines of fsync/roll-over/truncation logic and must test it hard (`JRN-1`, `JRN-2`).
- No indexed queries: `stats` and `task show` do linear scans. Fine at these sizes; measured by
  the `NFR-102` budget.

## Risks

| Risk | Mitigation |
|---|---|
| Torn write corrupts a record | Tolerate only a torn *last* line; truncate to last newline; note the recovery. Any mid-file corruption refuses (never guesses) |
| Unbounded growth | 32 MiB segments, bounded to 3; logs pruned by explicit `gc` |
| Many concurrent runs (future daemon) | Per-run files are independent; only a shared index would need a DB. Trigger below |
| User edits a journal | Hash chain detects; refuses |

## Revisit conditions

1. A daemon with multiple producers and a real queue appears (post-revenue) → introduce a
   per-host queue and possibly SQLite for *indexing only*, keeping journals as the record.
2. A single run exceeds ~10⁵ records or journals exceed ~1 GiB → revisit streaming/compaction.
3. Cross-run queries become a product need (e.g. "findings across 500 runs") → add an index built
   from journals (derived, never authoritative).
