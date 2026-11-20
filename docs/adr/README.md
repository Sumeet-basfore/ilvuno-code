# Architecture Decision Records

One decision per file. Each states context, options, the decision, consequences, risks, and the
conditions under which it should be revisited. Numbering is permanent; a reversed decision gets a
new ADR that supersedes the old one (the old one is marked `superseded by ADR-XXXX`, never edited
into agreement with the present).

Format convention: `ADR-<number>-<slug>.md`, four-digit numbers.

| ADR | Decision | Status | Supersedes |
|---|---|---|---|
| [0001](ADR-0001-language-and-runtime.md) | Go as language/runtime (vs TS/Node, Rust) | accepted (D-04, 2026-11-20) | — |
| [0002](ADR-0002-persistence-jsonl-not-sqlite.md) | Append-only JSONL + files; no DB, no queue | proposed | — |
| [0003](ADR-0003-no-ui-in-mvp-tauri-later.md) | No UI in V1; desktop shell not decided (Tauri+React candidate only); Electron rejected | accepted (D-02, 2026-11-20) | — |
| [0004](ADR-0004-git-subprocess-not-a-library.md) | Git as subprocess, never a library | proposed | — |
| [0005](ADR-0005-pin-to-shas-never-resolve-refs.md) | Pin to SHAs; resolve a ref exactly once | proposed | — |
| [0006](ADR-0006-candidate-construction.md) | Sequential real merges in a disposable worktree | proposed | — |
| [0007](ADR-0007-determinism-envelope.md) | Fixed identity/date; version outside the commit | proposed | — |
| [0008](ADR-0008-verification-executes-project-code.md) | Verification runs project code; no sandbox claim | proposed | — |
| [0009](ADR-0009-evidence-content-addressed-redaction-at-write.md) | Content-addressed bundles; redact at write | proposed | — |
| [0010](ADR-0010-adapter-boundary-and-credential-custody.md) | Protocol-level adapters; no credential custody; `command` only | proposed | — |
| [0011](ADR-0011-repair-as-external-command.md) | Repair = external command, bounded budget | proposed | — |
| [0012](ADR-0012-approval-model.md) | Digest-bound interactive approvals, signed records | proposed | — |
| [0013](ADR-0013-hooks-disabled-in-candidate-worktrees.md) | Hooks/signing disabled in candidate worktrees | proposed | — |
| [0014](ADR-0014-schema-versioning.md) | JSON Schema, additive-only, major for breaks | proposed | — |
| [0015](ADR-0015-cross-device-deferred.md) | Cross-device deferred; host-authoritative + E2EE relay | proposed | — |

## 1. Decisions that constrain everything else

Four ADRs are load-bearing; reversing any of them is a redesign, not a refactor:

| ADR | Why it is structural |
|---|---|
| 0005 (pin to SHAs) | makes determinism, safe resume-by-rebuild, and cache keys possible |
| 0004 (git subprocess) | makes predictions match reality; the honesty of findings depends on it |
| 0002 (journal as truth) | makes recovery and evidence the same mechanism |
| 0010 (no credential custody) | makes the security story short and keeps the core provider-independent |

## 2. ADRs that are deliberately reversible

| ADR | Reversal cost | Signal to reverse |
|---|---|---|
| 0001 language | high after v0.1, low before | team size >3, CPU-heavy core, or Tauri chosen |
| 0003 UI | medium (additive) | a paid cohort demands it |
| 0006 candidate construction | medium | users report merge semantics unrealistic for squashed workflows |
| 0007 determinism envelope | low (flag-gated) | users need real authorship on candidates |
| 0011 repair | low (a feature, not a foundation) | repair success-per-attempt ≈ 0 |
| 0013 hooks | low (flag) | codegen-before-test repos become a common complaint |
| 0015 cross-device | high (new surface, but no rewrite) | gate G4 passes |

## 3. Unresolved technical decisions

Carried from the architecture documents, each with a recommendation and the experiment that should
settle it. These are not open ADRs — they are questions an ADR cannot yet answer because they need
measurement, not judgement.

| ID | Question | Recommendation | Experiment | Blocks |
|---|---|---|---|---|
| `U-1` | Is per-record `fsync` affordable at real record counts? | yes (≈50 records/run) | instrument a 5-source/3-command run; measure journal time as a share of engine overhead vs the 5 s budget | ADR-0002 confirmation |
| `U-2` | Does `merge-tree --write-tree` match real merges on rename-heavy repos? | yes | generated rename/dir-move matrix, compare predicted vs actual (test `DET-4` on a rename corpus) | preflight trust |
| `U-3` | What is the real `combination_only` rate on user repos? | unknown — this is the product hypothesis | ship `INT-306`, measure `M-1` across ≥50 integrations | whole value case |
| `U-4` | Is the 2-attempt repair budget right? | yes as a default | log success-per-attempt and `budget_exhausted` rate; tune or remove | `INT-503` |
| `U-5` | Does the deterministic brief produce usable repairs at all? | plausible | A/B: structured brief vs raw log tail, same command, 20 failures | ADR-0011 |
| `U-6` | How large is the entropy-redaction false-positive rate on real logs? | acceptable | run the redactor over 100 real CI logs (user-contributed, opt-in) | ADR-0009 |
| `U-7` | Does `merge.conflictStyle=merge` (forced) surprise users enough to matter? | no | report when the user's global style differs; count complaints | ADR-0006 |
| `U-8` | Are worktree-add costs acceptable on 100k-file repos? | unknown | benchmark the large fixture; if > 5 s, consider partial checkout | ADR-0006, `NFR-102` |
| `U-9` | Is a single project lock enough once a user runs two agents' batches? | yes for MVP | measure `run_in_progress` refusals; if frequent, add a read-only concurrent mode | doc `04` §3 |
| `U-10` | Do reviewers actually open a bundle, or do they read the report in the PR? | report-first | `M-2` plus 5 reviewer interviews with a real bundle | report design |
| `U-11` | Does the passport get edited by hand (and how often does that break the chain)? | rarely | telemetry-free local counter surfaced in `stats` | ADR-0012 friction |
| `U-12` | Is 25 MiB binary / 30 ms cold start achievable with the redaction corpus and schema embedding? | yes | release job budget | `NFR-105` |

## 4. Recommended experiments before writing S1

Ordered, cheap, and each has a defined negative outcome.

| # | Experiment | Duration | Negative outcome → action |
|---|---|---|---|
| E-1 | Build the fixture forge + `T-CONF-2` only. Prove the triple-conflict case is detectable with `merge-tree` | 2 days | If `merge-tree` cannot see tuple conflicts, ADR-0006 preflight changes to "build then report" (weaker guarantee, must be said in the report) |
| E-2 | Prototype candidate construction + determinism envelope on a real repo; run `DET-1` twice a day apart and once on another machine | 3 days | If SHAs differ across machines, narrow the guarantee to same-machine and document it |
| E-3 | Measure `merge-tree` vs real merge on a generated 200-case corpus, including renames and deletions | 2 days | Any mismatch → preflight findings become `predicted` warnings only, never blocking |
| E-4 | Spike the journal + crash matrix for **one** stage (merge) | 2 days | If `resume` cannot be made unconditional, drop `resume` from v0.1 and require `abort` + re-run |
| E-5 | Redaction corpus pass (200 secrets / 200 benign) | 1 day | If benign false positives mangle > 2% of lines, disable entropy class by default |
| E-6 | Run the whole flow on a 100k-file monorepo | 1 day | If worktree add > 5 s, add partial-checkout mode before v0.1 |

Nothing after E-6 is a foundation decision; everything is feature work.

## 5. How to use this directory

1. Before implementing anything structural, check whether an ADR covers it.
2. If an ADR's "revisit condition" fires, write a new ADR; do not edit the old one's decision.
3. If an ADR's consequences turn out wrong, add a "Consequences observed" section (dated) — the
   record of being wrong is more useful than a clean document.
