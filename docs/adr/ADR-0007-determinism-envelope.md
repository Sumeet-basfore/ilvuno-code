# ADR-0007 — Determinism envelope: fixed identity/date, version outside the commit

Status: proposed. Date: Phase 3. Depends on ADR-0005.

## Context

`NFR-103` and `INT-605` require: same inputs → same candidate SHA and byte-identical normalized
report. Git commits embed author/committer identity and timestamps, so naive commits are
nondeterministic by construction. Reports embed durations and timestamps.

Prior art: none of the surveyed systems attempts reproducible candidate SHAs, because none builds
candidates.

## Decision

### Commits

Every commit-producing command runs with:

```
GIT_AUTHOR_NAME / GIT_COMMITTER_NAME   = ilvuno
GIT_AUTHOR_EMAIL / GIT_COMMITTER_EMAIL = ilvuno@localhost
GIT_AUTHOR_DATE / GIT_COMMITTER_DATE   = <base commit's committer time, truncated to seconds>
```

- The date is **derived from the base commit**, not from `now`. Two runs a week apart produce the
  same SHA.
- The commit message contains only content-derived text (source SHA, step index). **The Ilvuno
  version is not in the commit** — it is recorded in `refs/ilvuno/meta/<run_id>` and in the
  report. Embedding a version would break SHA stability across upgrades, which is the whole point.
- The envelope is applied to the candidate worktree's commands only, via `-c` and env; the user's
  global config is untouched.

### Reports

- Volatile fields (timestamps, durations, host id) live under a single `run` object.
- `Normalize(report)` zeroes that object; determinism tests compare normalized bytes.
- Findings, SHAs, order, statuses, versions, `not_verified` are stable.

### Scope of the guarantee

> Same inputs **+ same ilvuno version + same git version + same OS family** ⇒ identical candidate
> SHA and identical normalized report.

Not claimed across git versions (merge output can differ) or OS families (line endings, path
case). The report records all three so a difference is explainable rather than mysterious.

## Consequences

Positive: cache keys are meaningful; "is this the same candidate I approved?" is answerable; a
reproduced bug is byte-identical; tests can golden-file the report.

Negative: candidate commits look historically dated and authored by "ilvuno" — which is honest
(it *is* a synthetic commit) but will surprise someone reading `git log`. Documented in the report
and in `ilvuno show`.

## Risks

| Risk | Mitigation |
|---|---|
| A user's `commit.gpgsign=true` would produce a different SHA (or prompt) | forced `-c commit.gpgsign=false` (`INV-11`) |
| `.gitattributes` `text=auto` normalisation differs by platform | OS family is part of the guarantee's scope; the report states it |
| A repo hook rewrites the commit message | hooks disabled in candidate worktrees (ADR-0013) |
| Users confuse candidate commits with real history | ref namespace `refs/ilvuno/*` + commit subject prefix `ilvuno:` |

## Revisit conditions

1. If users need candidate commits to carry real authorship for review purposes, offer
   `--attribute-to <ident>` which **breaks** SHA determinism and must be recorded in the report.
2. If git gains reproducible-commit support natively, adopt it.
