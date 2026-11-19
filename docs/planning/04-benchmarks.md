# 04 — Benchmarks

Status: planning artifact; the benchmark set is a specification for `T-0305` (harness) and
`T-0503` (crash matrix). Every scenario is deterministic: fixed identity, fixed dates, no network,
no wall-clock assertions tighter than 2× the measured baseline.

Purpose: turn "does it work?" into a number a reviewer can check, and make regressions fail CI
instead of reaching a user.

---

## 1. Baselines

Every scenario is measured against up to three baselines. A benchmark is only meaningful relative
to something a developer would do anyway.

| ID | Baseline | Definition | What it measures |
|---|---|---|---|
| **BL-0** | Do nothing | Ship all branches; find out in production / after merge | The status quo a sceptical user has today |
| **BL-1** | Manual merge loop | `git checkout -b scratch && for b; do git merge $b; done && <test cmd>` by hand, in the user's working tree | The workaround Priya actually uses (persona `03` §1) |
| **BL-2** | Per-branch CI | Each branch's own CI run, no combination | The industry default that the product claims is insufficient |
| **BL-3** | Naive script | Same merge loop, scripted, no preflight, no evidence, no isolation | The "we could do this with bash" objection |

BL-3 exists to answer the most common dismissal: "this is a 40-line shell script". The benchmark
must show what the script cannot do: preflight without mutation, attribution, evidence, recovery,
and safe refusals.

---

## 2. Scenario set (24)

Legend: **Class** = required category from the brief. **Gate** = the measurable assertion.
All scenarios run on the forge fixtures (`T-0002`, `T-0003`).

### 2.1 Git-level conflicts (6)

| ID | Class | Fixture | Expected verdict / exit | Expected findings | Gate |
|---|---|---|---|---|---|
| **B-01** | File conflict (clean) | A and B edit different regions of `src/checkout.ts` | `passed`, 0 | `file_overlap` warning only | Merge succeeds; overlap names both SHAs; exit 0 |
| **B-02** | Line conflict | A and B edit `src/checkout.ts:L40-L45` | `blocked`, 3 | `merge_conflict` deterministic, paths + both SHAs | Preflight detects **before** any worktree exists (assert `.ilvuno/worktrees` absent); prediction == real merge |
| **B-03** | Tuple-only conflict | A∩B, A∩C, B∩C clean; `{A,B,C}` conflicts | `blocked`, 3 | `merge_conflict` with `tuple: true` | Pairwise checks clean, tuple check blocks; exit 3 |
| **B-04** | Rename/edit | A renames `x.ts`→`y.ts`; B edits `x.ts` | `blocked`, 3 | `merge_conflict` (delete/modify or rename) | Classification matches real `git merge` output class |
| **B-05** | Delete/modify | A deletes `legacy.ts`; B edits it | `blocked`, 3 | `merge_conflict` | Exit 3; user tree unchanged |
| **B-06** | Adjacent clean | A and B edit 2 lines apart, no overlap | `passed`, 0 | `hunk_proximity` warning when enabled | With `hunk_proximity=false`: 0 findings; with `true`: exactly 1 |

### 2.2 Compatibility (independent but incompatible) (5)

| ID | Class | Fixture | Expected verdict / exit | Expected findings | Gate |
|---|---|---|---|---|---|
| **B-07** | API incompatibility | A renames exported `parse()`; B adds a caller | `failed`, 1 | `check_failed` (`typecheck`) + `combination_only: true` | Each source alone: `typecheck` exit 0. Candidate: exit ≠ 0. Both recorded |
| **B-08** | Interface symbol touch | Same as B-07 with `index_command` configured | `failed`, 1 | `interface_symbol_touched` (deterministic, warning) + `check_failed` | Finding names symbol `parse` and both SHAs |
| **B-09** | Dependency disagreement | A adds `left-pad@1`, B adds `left-pad@2` | `blocked`, 3 | `dependency_disagreement` (blocking per config) | Both constraints + both SHAs in `data`; with `warn` policy: exit 0 and a warning |
| **B-10** | Lockfile churn | Both touch `pnpm-lock.yaml`, no declaration change | `passed`, 0 | `lockfile_churn` info | Not blocking; info only |
| **B-11** | Schema collision | A and B both add `migrations/007_*.sql` | `blocked`, 3 | `file_overlap` + `merge_conflict` | Path collision reported once per file, not per source |

### 2.3 Verification outcomes (5)

| ID | Class | Fixture | Expected verdict / exit | Expected findings | Gate |
|---|---|---|---|---|---|
| **B-12** | Build failure on candidate | Combined typecheck fails, both sources green | `failed`, 1 | `check_failed` blocking | Evidence includes both per-source runs (`T-0403`) |
| **B-13** | Test failure in combination only | `checkout.e2e` fails only when both present | `failed`, 1 | `check_failed` + `combination_only: true` | Per-source logs exist; assertion that each alone passed |
| **B-14** | Advisory failure | `lint` (required=false) fails; `test` passes | `passed`, 0 | advisory failure visible | Outcome `passed`, advisory failure recorded and rendered |
| **B-15** | No checks configured | Empty `verification.commands` | `unverified`, 4 | `no_verification_configured` | Exit 4, `confidence: low` block present, never `passed` |
| **B-16** | Flaky command | Command alternates pass/fail per run | `failed`, 1 | `flaky_check` warning | `--runs 2` yields `runs: 2` and a mixed-result finding; not `passed` |

### 2.4 Interruption, staleness, recovery (5)

| ID | Class | Injection | Expected state | Gate |
|---|---|---|---|---|
| **B-17** | Interrupted merge | SIGKILL between merge intent and result | `interrupted` + `merge_outcome_unknown` | `resume --decide=remerge` reproduces the **same** candidate SHA as an uninterrupted run |
| **B-18** | Interrupted verification | SIGKILL mid-command | `interrupted` + `verify_outcome_unknown` | `--decide=rerun` → `attempt: 2`; attempt-1 log retained and marked incomplete |
| **B-19** | Interrupted seal | SIGKILL during bundle seal | `interrupted` + `bundle_write_unknown` | `--decide=reseal` is idempotent; no partial `bundle/` ever exists |
| **B-20** | Stale revision | Ref moves after `pin` | run completes on pinned SHA | `--strict-refs` aborts with `source_moved`; without it, report shows ref→SHA and current ref SHA |
| **B-21** | Stale evidence | Approve, then change a source SHA | `stale_approval` | Gate closed; exit 3; reason rendered; no silent reuse |

### 2.5 Duplicates and unauthorized actions (3)

| ID | Class | Injection | Expected | Gate |
|---|---|---|---|---|
| **B-22** | Duplicate command | `integrate` twice, identical inputs | cache hit | Second run < 500 ms, same exit code, **zero** command executions (asserted by a counting fixture command) |
| **B-23** | Unauthorized writes | Config `cwd="../../"`; ref target `refs/heads/main`; journal edited to point at a user worktree | all refused | Exit 2 with `write_outside_scope`; safety fingerprint unchanged; no user ref touched |
| **B-24** | Tampering | One byte edited in a bundle log; then in the journal chain | both invalid | `bundle verify` → `invalid(log)`; journal chain break → `tampered`, no approval, no resume |

---

## 3. Acceptance gates

Gates are per-milestone; each is checked by `make bench` (B-series) or by the scenario suite.

| Gate | Metric | Threshold | Measured by |
|---|---|---|---|
| **BG-1** | Scenario correctness | 24/24 scenarios produce the expected verdict, exit code, and finding set | `make bench` |
| **BG-2** | Determinism | 100% of runs produce identical candidate SHAs and identical normalized reports (two runs, two OSes) | `DET-1`, `DET-2` |
| **BG-3** | Safety | 0 mutations outside `refs/ilvuno/*` and `.ilvuno/**` across all 24 scenarios + crash matrix | fingerprint diff |
| **BG-4** | Recovery | 100% of crash injections leave a parseable journal and a defined state; 0 orphans; `resume` explains every refusal | crash matrix |
| **BG-5** | Precision | false blocking ≤ 2% on the corpus and on 5 real repositories (≥ 100 runs) | manual audit of blocking findings |
| **BG-6** | Engine overhead | ≤ 5 s p50 for 3 sources on the 50k-file fixture, excluding command execution | benchmark timer |
| **BG-7** | Time to answer | ≤ 5 min p50 end-to-end for 3 sources on a repo whose checks take ≤ 4 min | e2e timer |
| **BG-8** | Baseline superiority | vs **BL-1**: ≥ 5× faster to a verified answer for 3 sources. vs **BL-2**: detects ≥ 90% of injected combination-only failures. vs **BL-3**: 100% of BL-3's unhandled cases (preflight, evidence, recovery) are handled | paired runs |
| **BG-9** | Idempotency | Duplicate commands perform zero repeated side effects (B-22) | counting fixture |
| **BG-10** | Reporting integrity | Bundle verifies offline in 100% of cases; tamper detection 100% (B-24) | `bundle verify` |

### 3.1 Why these thresholds

| Gate | Reasoning |
|---|---|
| 5× vs manual (BG-8) | If it is only 2× faster, a developer will not change tools; the manual loop is familiar and requires no install. 5× is the minimum where the switch is rational |
| 90% detection vs per-branch CI | A tool that catches 6 of 10 combination failures still surprises you 40% of the time; 90% is the point where "run it before merging" becomes a habit |
| ≤ 2% false blocks | Above that, developers learn to bypass (`--continue-on-conflict` reflex), and a bypassed gate is worse than no gate |
| ≤ 5 s overhead | The project's own checks dominate. If our overhead is visible, it is a bug (`NFR-102`) |

The corpus is curated, so 100% scenario correctness and 90% real-world detection are different
claims and must stay separate in reporting.

---

## 4. Benchmark harness

```
make bench                      # all 24, ~3 min (fixtures are tiny)
make bench SCENARIO=B-17        # one
make bench-filter CONFLICT      # category
make bench-update-baseline      # records timings (review required in the diff)
```

Output (`bench.json`, deterministic except timings):

```json
{"schema":"ilvuno.dev/bench/v1","ilvuno":"0.1.0","git":"2.43.0","os":"linux",
 "scenarios":[{"id":"B-02","verdict":"blocked","exit":3,"findings":["merge_conflict"],
               "side_effects":0,"timings":{"engine_ms":812,"total_ms":1904}}],
 "gates":{"BG-1":{"pass":true,"detail":"24/24"},"BG-6":{"pass":true,"p50_ms":3800}}}
```

Rules:

| Rule | Reason |
|---|---|
| Timing assertions use 2× headroom over the recorded baseline | CI machines vary; a tight bound is a flaky test |
| Verdict/exit/finding assertions are exact | these are correctness, not performance |
| `side_effects` is a count of writes outside scope, always asserted 0 | BG-3 |
| Baseline updates require a reviewer to look at the delta | prevents "fix the test by raising the number" |
| Benchmarks never touch the network | determinism |

---

## 5. Measured-against-baseline protocol

For BG-8, each scenario that involves time or detection is run three ways on the same machine:

```
1. BL-1 manual: scripted reproduction of the manual loop (a shell script, timed)
   - stopwatch from "start merging" to "I know the answer"
   - includes: merge attempts, conflict resolution reading time, test run
2. BL-2 CI-only: per-branch pipeline result (simulated: run the check on each source)
3. ilvuno: integrate + verify
```

Recorded per run: wall time, whether the combination failure was detected, whether evidence exists
afterwards, and how many manual steps were needed. The comparison table goes in the release notes,
not just the repo — a benchmark nobody reads is a ritual.

**Explicit fairness note.** BL-1 assumes a competent developer who already has the branches. It does
not include the time to *create* those branches with agents, because that is equal in all arms. Any
comparison that includes agent run time would flatter Ilvuno and is therefore excluded.

---

## 6. Benchmark ↔ requirement traceability

| Requirement | Benchmarks |
|---|---|
| `INT-108` preflight before mutation | B-02, B-03, B-04 |
| `INT-203` dependency disagreement | B-09, B-10 |
| `INT-210`/`INT-306` combination-only | B-07, B-13, B-23 |
| `INT-303` five-state outcomes | B-14, B-15, B-16 |
| `REC-101..107` recovery | B-17, B-18, B-19, B-24 |
| `INT-603` approval invalidation | B-21 |
| `REC-105` idempotency | B-22 |
| `INV-1`/`SEC-101` write scope | B-23 |
| `EVD-101..107` evidence integrity | B-24, B-12 |
| `NFR-102`/`NFR-103` performance & determinism | BG-2, BG-6 |
| `M-1` product metric | B-13 prevalence across 50 real repos (post-v0.1) |

Every P0 requirement has at least one benchmark or scenario; the traceability matrix in
`T-0605` checks that mechanically.

---

## 7. Concurrency benchmark (agent throughput)

To keep the multi-agent plan honest (`01-master-roadmap.md` §7), measure rather than assert:

| Conf | Setup | Metric | Expected |
|---|---|---|---|
| C1 | 1 agent, M3 tasks | tasks/week, review hours/task | baseline |
| C2 | 2 agents, split by package (waves W4/W8) | tasks/week, review hours/task, rebase count | ~1.7× throughput, +10–15% review time |
| C3 | 3 agents, same split | tasks/week, **merge conflict count**, rework rate | ≤ 1.6× and higher rework |

Acceptance for the multi-agent plan: **C3 must not beat C2** before adopting a third agent. If it
does, the owner-per-package rule is wrong and the plan is amended. This is the measurable version of
"more agents do not always help".

---

## 8. Post-release monitoring of the real distribution

Once v0.1 has external users, the benchmark corpus is no longer the main signal:

| Metric | Source | Gate for continuing |
|---|---|---|
| `combination_only` rate (`M-1`) | opt-in aggregate `stats` | ≥ 5% (else stop/reposition, product `07` §5) |
| False-blocking reports | issues | ≤ 2% of runs reported |
| Time to verified candidate | local `stats` (user-visible) | p50 ≤ 5 min |
| Bypass pressure | feature requests for `--no-verify`/`--auto-approve` | any request is reviewed; repeated requests mean friction is real |

No benchmark substitutes for this. The corpus proves the mechanism; users prove the value.
