# 08 — Persistence and Recovery

Status: specification. Everything here is on-disk format and recovery behaviour; formats are
versioned as public contracts (ADR-0014).

---

## 1. Layout

```
<repo>/
├── ilvuno.toml                       committed: config (A-9)
├── docs/ilvuno/tasks/<slug>.yaml     committed: passports (A-5)
└── .ilvuno/                          gitignored: all machine-local state
    ├── locks/project.lock
    ├── runs/<run_id>/
    │   ├── journal.jsonl             append-only, fsynced, seq-numbered
    │   ├── journal.jsonl.1           rolled segment (≤32 MiB each)
    │   ├── state.json                rebuildable cache (never authoritative)
    │   ├── logs/<seq>-<name>.log     redacted command output
    │   └── candidate.patch           archived before cleanup if dirty
    ├── worktrees/<run_id>/…
    ├── approvals/<apr_id>.json       signed approval bodies (outside the repo tree)
    └── bundles/<bundle_id>/
        ├── manifest.json             digests + schema versions
        ├── report.md
        ├── report.json
        ├── findings.json
        └── logs/…                    same logs, content-addressed
```

Two deliberate decisions:

1. **Nothing in `.ilvuno/` is required to read a report.** A report and its manifest are
   self-sufficient (`INV-14`). Deleting `.ilvuno/` loses history, not meaning.
2. **Approvals live outside the repo tree** so that an agent with write access to the repo
   cannot silently fabricate one (doc `09` §4.5).

---

## 2. Why not SQLite

Evaluated and rejected for the MVP (ADR-0002). Summary of reasoning:

| Criterion | JSONL + files | SQLite |
|---|---|---|
| Dependency | none | a C dependency (or a pure-Go port with different semantics) |
| Human readable / debuggable | yes, `jq`/`cat` | needs tooling |
| Reproducible in tests | byte-comparable | needs dump/restore discipline |
| Migration burden | schema version field | 1 migration per change |
| Concurrent writers | one process (we have one) | overkill |
| Query needs | none (single run, linear scan) | indexes unneeded |
| Crash safety | append+fsync per record; torn tail recoverable | WAL is excellent |
| Scale ceiling | fine to ~10⁵ records/run | much higher |

The decisive point: the MVP has exactly **one writer and no queries**. Prior art shows the cost
of the other path — t3code carries 127 migrations for its event store (`T3-020`) and prunes its
log to ~1 day, making projections authoritative and unrebuildable (`T3-021`). We take the
lesson (append-only, rebuildable) without the machinery.

Revisit condition and trigger are in ADR-0002.

---

## 3. The journal

### 3.1 Record format

```json
{"schema":"ilvuno.dev/journal/v1","seq":7,"at":"2026-11-19T14:02:11.004Z",
 "run":"irn_20261119T140211Z_a1b2","kind":"merge",
 "phase":"intent",
 "data":{"index":1,"source_sha":"9f2c...","pre_sha":"b7d0..."},
 "prev":"sha256:...","digest":"sha256:..."}
```

| Field | Rule |
|---|---|
| `seq` | contiguous from 1; a gap is corruption, not a tolerable hole (`C-11`) |
| `phase` | `intent` \| `result` \| `note` \| `uncertain` |
| `prev`/`digest` | hash chain over the canonical serialization of the record body |
| `kind` | the step kinds in doc `04` §4.1 |

### 3.2 Write discipline

```
append(record):
  serialize canonical (sorted keys, no insignificant whitespace)
  write "<json>\n" to the open fd
  fsync(fd)                       # required, per record
  (no rename, no temp file: append-only)
```

Per-record `fsync` costs ~1–5 ms and is unavoidable: the entire recovery story rests on "if the
intent is in the file, and the process dies, we know what was attempted". Batching would create a
window where a side effect happened with no record, which is precisely the state we cannot
recover from (ADR-0002).

### 3.3 Roll-over

At 32 MiB, close `journal.jsonl`, rename to `journal.jsonl.1` (and `-2` → `-3` etc., bounded to
3 segments), open a new `journal.jsonl`. `seq` continues across segments. Readers concatenate in
segment order then verify `seq` continuity.

If the bound is exceeded (a pathological run), older segments are deleted with a `journal.truncated`
note retained in the newest segment that records the last retained `seq`. The report then says
history is partial rather than pretending otherwise.

---

## 4. Recovery

### 4.1 Classification on start

```
read journal (all segments) → records[]
if any record fails to parse:
    if it is the last line and truncated → tolerate, truncate file to last good newline,
                                           note "torn_tail_recovered"
    else                                  → corrupt: quarantine journal, refuse resume
verify seq continuity and hash chain:
    gap or chain break → corrupt: quarantine, refuse resume (never guess)
last = last record
state = project(last)
```

`Never guess` is the rule: a hash-chain break means the file was edited, and acting on an edited
journal is acting on attacker-controlled state (doc `09` §5.3).

### 4.2 In-flight detection

| Last record | Meaning | Action |
|---|---|---|
| `*.intent` with no matching `*.result` | the side effect may or may not have happened | create `Uncertainty{kind}`, state `interrupted` |
| `*.result` | step completed | continue from the next step |
| `run.planned` only | nothing happened | safe to restart from scratch |
| `bundle.seal.result` | run finished | nothing to resume |

### 4.3 Resume algorithm

```
resume(run, decisions):
  switch on uncertainty.kind:
    merge_outcome_unknown:
       decide=remerge → discard candidate worktree+ref, rebuild from pinned SHAs (§4.4)
       decide=abort   → abort(run)
    verify_outcome_unknown:
       decide=rerun   → new VerificationRun (attempt+1); old partial evidence retained
       decide=abort   → abort(run)
    bundle_write_unknown:
       decide=reseal  → re-seal from existing records (idempotent by manifest digest)
       decide=abort   → abort(run)
    cleanup_unknown:
       decide=retry   → re-run cleanup steps
       decide=leave   → mark orphaned, settle run
  if uncertainty list is empty:
       continue from the next journaled step
```

There is no `--force` that auto-resolves all of them, and no default resolution
(`INV-4`). `resume` without `--decide` prints the list and exits 2.

### 4.4 Why rebuilding is always safe

Because the inputs are SHAs (`INV-2`), a candidate is a pure function of
`(base_sha, [source_shas], order, ilvuno_version, git_version)`. Discarding a half-built
candidate and rebuilding cannot lose user work — the user's branches are untouched — and cannot
produce a different result than the interrupted run would have.

That is the single design decision that makes recovery simple instead of clever.

### 4.5 Evidence preservation across resume

A resume never deletes prior evidence:

- a partial command log stays with the old `vrf_id`, marked `incomplete`;
- the new attempt gets `attempt = <n+1>`;
- the report lists attempts in order, so a flaky/slow history is visible rather than flattened.

---

## 5. Evidence bundles

### 5.1 Structure and sealing

```
bundle/
  manifest.json
  report.md / report.json / findings.json
  logs/<seq>-<name>.log
```

`manifest.json`:

```json
{"schema":"ilvuno.dev/bundle/v1","bundle_id":"evb_20261119T140211Z_a1b2",
 "created_at":"...","run_id":"irn_...",
 "candidate":{"sha":"9f2c...","tree":"..."},"base_sha":"7c47...",
 "sources":[{"sha":"4b82...","ref":"refs/heads/agent/payments-intent"}],
 "verification":{"state":"passed","required_passed":2,"required_total":2,
                 "not_verified":[]},
 "schema_versions":{"report":"ilvuno.dev/report/v1","passport":"ilvuno.dev/passport/v1"},
 "versions":{"ilvuno":"0.1.0","git":"2.43.0","os":"darwin","arch":"arm64"},
 "files":[{"path":"report.md","sha256":"...","bytes":8123}, …],
 "manifest_digest":"sha256:…"}
```

Sealing is atomic: write to `bundle.tmp/`, `fsync` every file and the directory, then `rename`
to `bundle_id/`. A crash leaves `bundle.tmp`, never a half-valid bundle. `resume` with
`decide=reseal` deletes `bundle.tmp` and re-seals from records.

### 5.2 Verification

`ilvuno bundle verify <path>`:

1. parse manifest; check `schema` is known;
2. recompute the digest of every listed file; compare;
3. check for unlisted files (a bundle with extra files is reported, not silently accepted);
4. recompute `manifest_digest` over the canonical manifest body (excluding the digest field);
5. report `valid | invalid(reason)`.

This runs with no network and no repo, which is what makes it usable in a review thread
(`EVD-101`).

### 5.3 Tampering

| Tampering | Detection |
|---|---|
| Edit a log | file digest mismatch |
| Edit the report | digest mismatch |
| Edit the manifest and update digests | `manifest_digest` no longer matches `Approval.evidence_digest`, so the approval is `invalidated_by: digest_mismatch` |
| Delete a file | listed-but-missing |
| Add a file | unlisted file reported |

Note the honest limit: a bundle has no signature in the MVP. It is tamper-*evident* against
accident and against naive edits, not tamper-*proof* against a determined editor who recomputes
everything **and** re-approves. Approvals are the signed artifact (§6), not bundles.

---

## 6. Approvals on disk

```
.ilvuno/approvals/<apr_id>.json
{ "schema":"ilvuno.dev/approval/v1",
  "id":"apr_7f3", "run_id":"irn_...", "candidate_sha":"9f2c...",
  "evidence_digest":"sha256:...", "scope":"land", "decision":"approved",
  "by":{"kind":"local-user","label":"priya","key_id":"ed25519:ab12"},
  "at":"...", "signature":"sig:…" }
```

Signature is Ed25519 over the canonical body when a key is configured; otherwise `signature: null`
and `by.kind` stays `local-user` with `unsigned: true`. The passport stores only
`{id, digest}` — an approval cannot be forged by editing the passport alone, because the passport
points at a record that must exist locally and whose digest must match (doc `09` §4.5).

Key management in MVP: `ilvuno key init` writes `~/.config/ilvuno/keys/ed25519.key` (0600). If
the file is absent, approvals are unsigned and the report says so. There is no key server, no
passphrase prompt in non-interactive mode (which is why `approve` requires a TTY anyway).

---

## 7. Retention, compaction, and pruning

| Artifact | Default retention | Mechanism |
|---|---|---|
| Candidate ref/worktree | until run settles | removed at settle; patch archived first |
| Run journal | 30 days | `ilvuno gc` prunes runs with a sealed bundle older than N days |
| Logs in run dir | 30 days | same |
| Bundles | forever (user-owned) | `ilvuno bundle prune` writes a tombstone: `{bundle_id, manifest_digest, summary}` and removes files (`TSK-113`) |
| Approvals | forever | they are the record |
| Passports | forever | in git |

Pruning is explicit (`ilvuno gc`), never automatic in the background, so a user is never
surprised. The tombstone is what keeps an old approval `verifiable-as-pruned` rather than
`missing` — a third state, distinct from valid and invalid.

---

## 8. Idempotency and duplicate requests

| Request | Idempotency key | Behaviour on repeat |
|---|---|---|
| `integrate` | `(project, base_sha, sorted source_shas, order, config_digest)` | if a settled run with the same key and a bundle exists, print it and exit with its code; `--fresh` forces a new run |
| `verify` | `(candidate_sha, config_digest)` | same |
| `approve` | `(candidate_sha, evidence_digest, scope)` | second identical approval is a no-op, not a duplicate record |
| `repair` | `(run_id, attempt)` | replay returns the recorded attempt |
| `bundle export` | `(bundle_id, redaction_profile)` | deterministic output path, overwrite only with `--force` |

The `integrate` cache is the feature that makes the tool cheap to use: re-running yesterday's
command on unchanged inputs must not re-run a 9-minute suite. It is safe because the key is made
of SHAs, not refs — a moved ref produces a new key (`ADR-0005` again).

Caveat recorded in the report: a cache hit means "this exact input set previously produced this
result", not "the code is still fine". The report always shows the original timestamp and
versions.

---

## 9. Corruption matrix

| Corruption | Detection | Recovery |
|---|---|---|
| Torn last line of journal | parse failure on last line | truncate to last newline, note `torn_tail_recovered` |
| Hash-chain break mid-file | `prev` mismatch | quarantine journal, refuse resume, refuse approval (`tampered`) |
| `seq` gap | continuity check | same |
| `state.json` wrong/garbage | it is a cache | delete and reproject from journal; never an error |
| Bundle manifest unparsable | parse | `invalid(manifest)`; approval invalidated |
| Log file truncated | size/digest | `invalid(log)`; result becomes `inconclusive`, not `passed` |
| Passport YAML unparsable | parse | `passport_invalid`; task renders `state: unknown`; no gate opens |
| Passport digest chain broken | recompute | `tampered`; approvals `unverifiable` |
| Approval record missing but referenced | lookup | `evidence_missing`/`approval_missing`; gate closed |
| Worktree metadata corrupt | `git worktree list` oddity | `git worktree prune` on resume; rebuild from SHAs |
| Disk full mid-write | write error | stop before side effect if possible; else `interrupted` + uncertainty |
| Clock moved backwards | timestamps not monotonic | `seq` is authoritative; timestamps are advisory and flagged `clock_nonmonotonic` |

The pattern: **`seq` and digests are authoritative; timestamps and caches are advisory.**

---

## 10. Cross-machine safety

| Situation | Behaviour |
|---|---|
| Journal copied to another machine | readable; refuse `resume` unless the referenced objects exist; `--allow-foreign-resume` not provided (deliberate) |
| Bundle moved | fully verifiable, path-independent |
| Passport from another machine | renders; SHAs may be dangling until fetched (`git fetch` hint printed) |
| Two machines run the same integrate key | both produce the same candidate SHA (deterministic) and mutually verifiable bundles; no coordination needed |
| Future multi-host | the journal becomes per-host; the passport and bundles are the merge surface (doc `10` §6) |

---

## 11. Open questions

| ID | Question | Recommendation |
|---|---|---|
| `PR-1` | Is per-record fsync too slow for large journals? | Measure; 1–5 ms × ~50 records/run is negligible. If a 1000-command pipeline appears, batch per command, never per merge |
| `PR-2` | Should `.ilvuno/` be inside the repo at all? | Yes: survives moves, easy to gitignore, avoids a config-driven path. Revisit if CI mounts are read-only (`WT-2`) |
| `PR-3` | Should bundles be signed by default? | No: signing needs a key and a prompt. Approvals are signed; bundles are digest-bound to approvals |
| `PR-4` | How long should `integrate` cache results? | Until the input SHA set changes; a `ttl` is unnecessary because correctness is input-derived |
| `PR-5` | Do we need a `journal compact` after settle? | Post-MVP; a settled run's journal is small relative to its logs |
