# ADR-0004 — Git is invoked as a subprocess. Never reimplemented.

Status: proposed. Date: Phase 3.

## Context

The product's correctness claim rests on merge semantics matching what the developer would get
by hand. There are three ways to get git behaviour: shell out to the `git` binary, link a library
(go-git, libgit2/git2, gitoxide), or implement the parts we need.

## Evaluation

| Approach | Merge/conflict fidelity | Feature coverage | Failure modes |
|---|---|---|---|
| **`git` subprocess** | Exact — it *is* git, including the user's config, attributes, and git version | Everything: worktrees, merge-tree, range-diff, rev-list, cat-file | Parsing stdout/stderr; version differences; one process per call (~5–15 ms) |
| go-git | Approximates; different conflict output; missing features (worktrees, some merge strategies); known divergences from git | Incomplete | Silent divergence — the worst kind |
| libgit2 / git2 | High fidelity but not identical (its own merge implementation) | Broad, not complete | CGO, plus behavioural drift on edge cases |
| gitoxide | Excellent and fast, pure Rust | Growing; worktrees/merge less complete | Would force Rust (ADR-0001) and still not guarantee identity with the user's git |
| Reimplement | No | No | No |

The product's honesty depends on `06` §4.1: a prediction must agree with the real merge. That is
only achievable if both are the same implementation.

## Decision

**All VCS operations are `git` subprocess invocations**, in a single `gitx` package that:

1. runs with an explicit, minimal, auditable argument vector (never a shell string);
2. forces semantics-affecting config via `-c` (`core.hooksPath=`, `commit.gpgsign=false`,
   `merge.conflictStyle=merge`, `gc.auto=0`, `--no-pager`, `core.quotepath=false`);
3. requires git ≥ 2.38 for `merge-tree --write-tree`; degrades explicitly (documented in the
   report) rather than silently;
4. captures and records the git version in every run's metadata;
5. parses only stable, documented output formats (`--porcelain`, `rev-parse`, `for-each-ref
   --format`, `ls-tree -z`, `merge-tree --write-tree`); refuses to parse human-facing output.

## Consequences

Positive: perfect fidelity; the user's `.gitattributes`, filters, and version apply; no CGO; the
binary stays dependency-free.

Negative: ~10–40 subprocess calls per run (acceptable: ~200 ms total); stdout parsing must be
version-aware; some operations are slower than an in-process library would be.

## Risks

| Risk | Mitigation |
|---|---|
| Parsing breaks on a future git version | Golden tests over 2.38 + latest in CI; `doctor` warns on untested versions; parsers require exact structural matches |
| Locale/encoding surprises | `LC_ALL=C`, `-z` where available, `core.quotepath=false` |
| Git missing/too old | `doctor` refuses with the version found; run refuses (exit 2) |
| Perf regression on huge repos | `NFR-102` budget; measure before optimising |
| A needed feature is missing on old git | Explicit degradation with a report note, never a silent approximation |

## Revisit conditions

1. A specific operation becomes a measured bottleneck (e.g. per-file diffing of 100k files) →
   consider `gitoxide` for read-only paths while keeping merges on `git`.
2. Git gains a stable structured output mode (e.g. JSON) for the commands we parse → adopt it.
3. Windows support demands different argument handling (v0.2) → platform-specific argv builder,
   same package.
