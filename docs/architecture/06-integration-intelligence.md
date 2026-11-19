# 06 — Integration Intelligence

Status: specification. This is the differentiator (product §15); the rest of the system exists
to make it trustworthy.

---

## 1. Contract

Given a base ref, N source refs, and a verification configuration, the engine returns exactly
one of:

| Outcome | Exit | Meaning |
|---|---|---|
| `blocked` | 3 | a deterministic blocking finding exists; **nothing was merged** |
| `failed` | 1 | a required check failed on a candidate |
| `unverified` | 4 | candidate built, no checks configured |
| `passed` | 0 | candidate built, all required checks passed |
| `error` | 2 | environment/usage refusal, before or during (partial work recorded) |
| `interrupted` | 130 | process died; state is resumable |

Never returns "ready to merge". Never pushes. Never writes to the base ref (`INV-1`).

---

## 2. Stage 1 — Pin

```
Pin(base_ref, source_refs) -> Pinned{
   base_sha, sources[{ref, sha, commit_time, merge_base_sha, ahead, behind, order}]
}
```

Algorithm:

```
1. git -c gc.auto=0 fetch --no-tags <remote> <base_ref>      # unless --offline
      fail → base_fetch_failed (exit 2)   [INT-104]
2. base_sha = git rev-parse --verify <base_ref>^{commit}
3. for each source_ref:
      if ref is a full 40-hex sha:  sha = ref
      elif ref is a ref name:       sha = git rev-parse --verify <ref>^{commit}
      else:                         unpinnable_source          [INT-102]
      if `git cat-file -e <sha>`:   unpinnable_source (missing object)
4. merge_base = git merge-base <base_sha> <sha>
      fail → no_merge_base (shallow or unrelated histories)   [INT-103]
5. ahead/behind = git rev-list --left-right --count <base_sha>...<sha>
6. drift = behind   (recorded, warning above threshold)        [INT-205]
7. reject: sha == base_sha (nothing to integrate)
8. reject: ref points into .ilvuno/ or is HEAD/@{...}          [INV-2, ORQ-101]
9. reject: duplicates (same sha twice) with a clear message
```

**After step 6, the refs are never read again.** Every later stage takes SHAs (`INV-2`). This
is the single most important determinism decision: a ref can move at any time, and a run that
re-resolves `main` half-way through would produce a report about a repository state that never
existed (ADR-0005).

Recorded per source: `commit_time`, so the report can show "this branch is 3 days old" without
resolving anything.

---

## 3. Stage 2 — Deterministic order

Default order: **ascending by `merge_base_sha` commit time, then by `order` in the manifest,
then by SHA** (a total order, so it is reproducible even when timestamps tie).

Rationale: integrating the oldest-based change first means later merges are rebased over less
history, and the first conflict encountered is the most informative. Alternatively, ordering by
"most files changed" first makes conflicts surface earlier but produces noisier first merges.

Overridable: `--order sha1,sha2,...`, validated to be a permutation of the pinned sources;
recorded in the journal so a report can always state exactly what order was used.

Conflict-free alternative considered and rejected: octopus merge (`git merge a b c`) — it fails
atomically without telling you *which* pair conflicted, and it cannot attribute changes to a
step. Attribution is the product (ADR-0006).

---

## 4. Stage 3 — Preflight (nothing is mutated)

Two independent checks:

### 4.1 Textual conflicts, real git semantics

```
git merge-tree --write-tree --name-only <base_sha> <sha_a> <sha_b> ...
  exit 0 → tree oid, no conflicts
  exit 1 → conflict report on stdout (paths + stage entries)
```

Requires git ≥ 2.38 (`--write-tree`). `doctor` probes it; if absent, the engine falls back to
building the candidate worktree and performing **real merges** without the pre-mutation
guarantee, and says so in the report (`preflight: unavailable (git < 2.38)`).

Why `merge-tree` and not a heuristic: it is the same merge machinery, so a "no conflict"
prediction that disagrees with the real merge is close to impossible. That is what makes
`prediction` honest (doc `02` §5.2 `predicted: true`).

Pairwise reporting:

```
for each pair (i<j) where i,j include overlapping paths:
    merge-tree(base, sha_i, sha_j)  → conflict? paths, both shas
also merge-tree(base, all_shas)     → conflicts that only appear as a set
```

The second call is the important one: a three-way conflict can exist where every pair is clean
(after A and B merge, C now conflicts). Reporting only pairs would miss exactly the failure
class the product exists for.

### 4.2 Drift and staleness

`behind > drift_threshold` (default 50) emits `base_drift` warning. Not blocking by default:
the point of a candidate is to test *now*, and refusing on drift would make the tool unusable
on an active repo. But the report states it, and a config option makes it blocking for teams
that require it.

### 4.3 Preflight output

```json
{"preflight":{"tool":"git-merge-tree","git_version":"2.43.0","tuple_conflicts":[
  {"sources":["4b825dc","9f2c4a1"],"paths":["packages/api/src/checkout.ts"]}]}}
```

If any `tuple_conflict` exists and `on_conflict: stop`, the run is `blocked` (exit 3) with zero
side effects. This is the "we told you before we touched anything" behaviour (product J-3).

---

## 5. Stage 4 — Candidate construction

```
Candidate(run):
  ref = refs/ilvuno/candidates/<run_id>
  path = .ilvuno/worktrees/<run_id>/candidate
  worktree_add(ref, base_sha)                        [doc 05 §4]

  for i, sha in enumerate(order):
      journal_intent(merge, i, sha, pre = rev_parse(HEAD))
      result = git -c core.hooksPath= -c commit.gpgsign=false \
                   -c merge.conflictStyle=merge \
                   merge --no-ff --no-edit \
                   -m "ilvuno: integrate <sha[:12]> (step <i+1>/<n>)" <sha>
      if result.conflict:
          journal_result(merge, conflict, paths)
          if on_conflict == stop:
              git merge --abort
              → run blocked (exit 3), keep worktree for inspection, record paths
          else:
              git merge --abort
              journal_result(skipped, reason = conflict)
              excluded.append(sha)
      else:
          journal_result(merge, clean, post = rev_parse(HEAD))

  candidate.sha = rev_parse(HEAD)
  candidate.tree = rev_parse(HEAD^{tree})
```

Details and why:

| Choice | Reason | Alternative rejected |
|---|---|---|
| Real `git merge` per source | identical semantics to what the developer will do | `cherry-pick`/`rebase` rewrites SHAs and changes conflict detection (a rebase can conflict where a merge does not) |
| `--no-ff` | guarantees a merge commit per step, so every step is attributable and the candidate's lineage is legible | fast-forward would collapse steps and lose attribution when a source is ahead of base |
| Commit message names the source SHA only | deterministic across Ilvuno versions (`INV-11`); the tool version lives in `refs/ilvuno/meta/<run_id>` | embedding a version would change the candidate SHA on upgrade, breaking "same inputs → same SHA" |
| `--no-edit` + explicit `-m` | avoids opening an editor, which would hang CI | default message (nondeterministic merge description) |
| `merge --abort` on stop | leaves the tree clean so the worktree can be inspected without a half-merged state | leaving conflicts half-applied invites accidental commits |

### 5.1 Determinism envelope

`INV-11` + ADR-0007. Before any commit-producing command the engine sets:

```
GIT_AUTHOR_NAME / GIT_COMMITTER_NAME   = "ilvuno"
GIT_AUTHOR_EMAIL / GIT_COMMITTER_EMAIL = "ilvuno@localhost"
GIT_AUTHOR_DATE / GIT_COMMITTER_DATE   = <base_sha commit time, UTC, :00 seconds>
```

Deriving the date from the base commit (not `now`) means two runs a week apart produce the
same SHA. Consequence, documented: candidate commits have a historical-looking date. That is
the price of a reproducible SHA, and reproducible SHAs are what make cache-hit detection,
report diffs, and "is this the same candidate I approved?" possible.

Scope of the guarantee: **same inputs + same Ilvuno version + same git version ⇒ same SHA.**
Across git versions, merge output can differ; the report records all three versions, and the
guarantee is explicitly not claimed across them.

### 5.2 Partial integration

`--continue-on-conflict` records each conflict, excludes that source, continues, and produces a
candidate from the survivors. The report lists `included` / `excluded` with reasons
(`INT-109`). Verifying a partial candidate is legitimate ("does the two-way subset work?") and
is labelled as such everywhere so nobody mistakes it for the full batch.

### 5.3 Never touches `main`

Enforced structurally, not by convention:

1. `gitx` accepts only `refs/ilvuno/*` as a write target (a type-level constraint, doc `02`
   §7 `C-3`).
2. The candidate ref is derived from `run_id`, never from a flag.
3. A test asserts the full ref list before/after every scenario is identical except
   `refs/ilvuno/*` (`T-SAFE-1`, doc `11` §5).
4. No code path calls `git push`, `git update-ref refs/heads/*`, or `git checkout <base>` in
   the main worktree.

---

## 6. Stage 5 — Compatibility analysis (deterministic)

All findings below are computed from SHAs, trees, and file contents — no model involvement.
Each maps to a failure mode in research `05` §2.

### 6.1 `file_overlap` (warning)

```
for each source: files_i = git diff --name-only --no-renames <base_sha> <sha_i>
overlap = { f | |{i : f ∈ files_i}| ≥ 2 }
```

Severity warning. Rationale: overlap is the strongest cheap signal for both textual and
semantic conflict, but plenty of benign overlap exists (a changelog, a lockfile).

### 6.2 `hunk_proximity` (warning, opt-in)

For files touched by ≥2 sources, parse zero-context diffs (`git diff -U0`), collect changed
line intervals, and report pairs within `window` (default 5 lines) that merged cleanly.

Why it exists: adjacent-but-not-overlapping edits are where semantic breakage concentrates and
where git will happily produce a wrong result. Off by default because the false-positive rate on
large refactors is high.

### 6.3 `dependency_disagreement` (blocking if configured)

```
manifests = config.dependency_manifests        # e.g. package.json, pnpm-lock.yaml, Cargo.toml, go.mod
for each source: parse changed dependency declarations → {name, constraint}
for each name declared by ≥2 sources with different constraints:
    finding(dependency_disagreement, blocking: config.on_dependency_conflict=="block")
        data: {name, per_source: [{sha, constraint}]}
```

Parsing is per-manifest-kind with a small, explicit parser set. An unknown manifest is skipped
with a `manifest_unparsed` info finding rather than guessed at — a wrong dependency claim is
worse than none.

Lockfile-only changes (no declaration change) are reported as `lockfile_churn` info: they
frequently mean someone ran an install, and they merge badly.

### 6.4 `interface_symbol_touched` (warning, requires a configured index command)

```
index_cmd                         # e.g. "ctags -x --_xformat=..." or a repo script
symbols_i = index(<sha_i>)        # exported symbols (declaration-level)
for each symbol s changed by source i:
    for each source j != i:
        if s referenced by files changed in j  → finding(interface_symbol_touched)
```

Two-stage: symbol *extraction* is delegated to a configured command (no bundled per-language
analyzers, `T-5`); the *correlation* is ours. When no index command is configured, this
finding is absent and the report says so in `not_verified`.

### 6.5 `base_drift` (warning; blocking if configured)

From `ahead/behind` in pinning (§2). Message names the exact count and the threshold.

### 6.6 `claim_divergence` (warning)

When an `AgentRun` carries `reported_files[]`:

```
actual = git diff --name-only <base_sha> <produced_sha>
reported \ actual  → files the agent claimed but did not change
actual \ reported  → files it changed but did not mention (usually benign; info)
```

This is the only place agent text is used, and it is used *against* the agent, never as an
input to a gate (`EVD-103`).

### 6.7 `combination_only` (the measurement)

```
after a required check fails on the candidate:
  for each source sha:
      verify(source_sha, only the failing command)      # in a fresh worktree
  if every source passes the same command:
      finding(check_failed, combination_only: true at the run level)
      record the per-source results as evidence
```

Cost control: only the *failing* command is re-run per source (not the whole pipeline), and only
for sources already pinned, in worktrees created in parallel-safe order. This is `INT-306` — the
instrument that produces `M-1`, the metric that decides the product (`07-success-metrics.md`).

If a per-source re-run cannot be completed (workspace limit, timeout), the finding stays
`check_failed` with `combination_only: unknown`, not `true`. Unknown is a legal value;
fabricating a favourable one is not.

---

## 7. Stage 6 — Classification

Classification is a *value on the record*, decided by origin, not by content:

| Origin | Classification | Can block |
|---|---|---|
| git, process exit codes, file parsing | `deterministic` | yes |
| configured index command output (counts/symbols) | `deterministic` | no (warning) |
| model output, heuristics on prose, "likely" statements | `interpretation` | **never** |

The gate (`Eligible`) filters `classification == deterministic && severity == blocking`. There
is no code path by which an interpretation changes the exit code (`INV-6`, `INT-207`), and the
report schema rejects `severity: blocking` on an interpretation (`C-6`).

Interpretation, when enabled (`--interpret`):

- runs **after** the report is otherwise complete;
- receives only the finding list and the diff stat, never full file contents by default (cost
  and leakage control);
- must cite `path:line` from at least two sources, or the finding is dropped;
- is written under a `interpretation` key that can be stripped and still leave a valid report
  (`VER-106`).

---

## 8. Stage 7 — Repair

```
Repair(run, budget):
  brief   = RepairBrief(from evidence)               # deterministic, no model
  for attempt in 1..budget:
      journal(repair.start, attempt, brief_digest)
      branch = refs/ilvuno/repair/<run_id>-<attempt>
      worktree_add(branch, candidate_sha)
      write brief to <wt>/.ilvuno-brief.md
      run configured repair command (cwd = worktree, brief on stdin)
      journal(repair.attempt, exit, produced_refs)
      if exit != 0 and no new commit:  continue      # counts as a failed attempt
      re-verify(candidate' = HEAD)
      if passed: return repaired                         [INT-506]
  return budget_exhausted
```

Rules:

| Rule | Reason |
|---|---|
| Sources are never modified (`INT-505`) | a repair that rewrites a source destroys the ability to attribute failure |
| Each attempt gets its own branch + worktree | attempts are independent evidence; overlapping attempts are uninterpretable |
| Every attempt's diff is archived, even on failure | `INT-504`; the record of a failed fix is the deliverable |
| The brief is derived from evidence, not from a model | `INT-501`; reproducibility |
| Budget is a hard stop with `budget_exhausted` and an escalation block | `INT-503`, `05` §2.10 |
| A repair that produces no new commit counts as a failed attempt | otherwise a no-op command burns the budget invisibly |

Brief shape (deterministic):

```markdown
# Repair brief
Candidate: <sha> (base <base_sha>)
Failing check: <name> — exit <code>, <duration>
Command: <argv>
Relevant output:
<last N lines, redacted, ≤ 4 KiB>
Sources (do not modify):
  - <sha> <ref>  (alone: passed)
  - <sha> <ref>  (alone: passed)
Constraint: fix the combination, not either branch.
Deliverable: commits on the current branch; do not push; do not change other branches.
```

---

## 9. Stage 8 — Report and approval

The report is the product's visible surface. Canonical sections (order fixed, wording governed by
the banned-claims list):

1. **Verdict** — one of `blocked | failed | unverified | passed | error`, plus one sentence that
   does not overclaim.
2. **Inputs** — base SHA, sources with SHAs and drift, ilvuno/git versions, config digest.
3. **Order** — the exact merge order and why.
4. **Findings** — deterministic first, each with evidence refs; interpretations in a separate
   block.
5. **Steps** — the journal timeline: per-source merge result.
6. **Verification** — per-command matrix: status, exit, duration, and the `not_verified` list.
7. **Combination-only** — the measured comparison, including `unknown`.
8. **Repair** — attempts and outcomes, if any.
9. **Evidence** — bundle id, manifest digest, how to verify, how to attach to a review.
10. **Next actions** — deterministic, ≤5, each a runnable command or a decision.

Approval eligibility is a pure function of (deterministic findings, verification state,
candidate SHA) and is stated in the report so a reviewer sees the same gate the CLI enforces.

---

## 10. Critical failure scenarios

| Scenario | Required behaviour |
|---|---|
| Base ref moved between pin and merge | irrelevant: merges use `base_sha`; report notes the ref's current SHA if different |
| A source ref moved after pin | irrelevant by construction (`INV-2`); `--strict-refs` re-checks and aborts if moved (`source_moved`) |
| Source commit becomes unreachable mid-run | `cat-file -e` before each merge → `source_object_missing`, abort, journal records prior steps |
| Conflict at step 3 of 5 with `stop` | abort merge, keep worktree, exit 3, report lists step 1–2 results (they merged cleanly) |
| Conflict at step 3 with `continue` | skip source 3, record, continue with 4 |
| Candidate build succeeds but candidate SHA == base SHA | impossible (every source adds a merge commit); treated as `candidate_empty` error |
| Verification command mutates the candidate (build artifacts) | recorded; `tree_after` warns; evidence stays valid because the SHAs are captured before and after |
| Verification command deletes source files and commits | the candidate SHA changes; `verify` detects `HEAD` movement and fails the run (`candidate_mutated`) — a command may not change the commit under test |
| Two runs on the same project | second refuses on the lock |
| Disk fills during bundle seal | `evidence_write_failed`; the run is invalid, not "passed" |
| `git` missing or too old | `doctor` refuses; run refuses with the version found |
| Report generated but bundle seal failed | `exit 2`; the report is marked `evidence: missing`; no approval possible |
| Interpretation API key missing but `--interpret` set | warning, run continues without interpretation (it cannot affect the result anyway) |
| Repair command escalates privileges (sudo) | not prevented (see doc `09` §7); recorded as part of the attempt's exit/evidence |

---

## 11. Interface summary

```go
type Engine interface {
    Integrate(ctx, IntegrateSpec) (IntegrationRun, error)   // never pushes, never approves
    Verify(ctx, VerifySpec) (VerificationRun, error)
    Repair(ctx, RepairSpec) (RepairOutcome, error)
    Resume(ctx, RunID, Decisions) (IntegrationRun, error)
    Abort(ctx, RunID) error
}

type IntegrateSpec struct {
    Repo        string            // must resolve to a project
    BaseRef     string            // pinned immediately
    Sources     []SourceRef
    Order       []string          // optional; validated permutation
    OnConflict  ConflictPolicy    // stop | continue
    Manifest    *string           // optional batch input
    Interpret   bool              // off by default
    Offline     bool              // no fetch; records stale_base_accepted
}
```

`Integrate` is pure with respect to the user's repository: its only writes are
`.ilvuno/**` and `refs/ilvuno/**` (`INV-1`). That signature is the contract other tools can
rely on (`ORQ-106`).

---

## 12. Open questions

| ID | Question | Recommendation |
|---|---|---|
| `II-1` | Should fast-forwardable chains be detected and represented as a linear base advance? | No in MVP: uniform steps make the report and evidence uniform; a linear case is just a clean merge |
| `II-2` | Is `merge-tree --write-tree` sufficient for rename-heavy repos? | Yes with `--name-only`; record rename detection settings in the report for reproducibility |
| `II-3` | Should dependency parsing cover every ecosystem? | No: a declarative parser registry, ship 4 (`package.json`, `pnpm-lock`, `Cargo.toml`, `go.mod`); unknown manifests are `info`, never guessed |
| `II-4` | Should `combination_only` also compare against a "base + newest source only" candidate? | Post-MVP; expensive but it would separate "any two" from "all N" |
| `II-5` | Rebase-based candidate as an option (matching repos that squash)? | Not in MVP: it changes SHAs and conflict semantics; revisit only if users report the merge view is unrealistic |
