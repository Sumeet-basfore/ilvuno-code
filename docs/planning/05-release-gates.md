# 05 — Release Gates

Status: planning artifact. A gate is a **decision point with named evidence**, not a status
update. Nothing proceeds past a failed gate; nothing is "conditionally passed" without a written
exception that names the residual risk and its owner.

---

## 1. How a gate works

```
1. Author prepares the gate packet (artefacts listed per gate).
2. One human (not the author, where a second person exists) runs the gate commands from a clean clone.
3. Verdict: PASS | FAIL | PASS WITH EXCEPTIONS (max 2, each with owner + date + follow-up task).
4. Recorded in docs/planning/gate-log.md (created at G0), one entry per gate, with the packet links.
5. On FAIL: the milestone is not "mostly done". It is open. The plan absorbs the slip (03 §8).
```

Gate commands are always reproducible by a third party with no context:

```
make gate G2          # runs the gate's benchmark/scenario subset and writes gate-G2.json
```

A gate that cannot be re-run by someone else is not a gate.

---

## 2. Gate summary

| Gate | Milestone | Decides | Blocking? |
|---|---|---|---|
| **G0** | M0 | Is the test harness trustworthy enough to build on? | hard |
| **G1** | M1 | Can we read repositories safely and refuse the wrong ones? | hard |
| **G2** | M2 | Is a candidate deterministic, safe, and attributable? | hard |
| **G3** | M3 | Is the report explainable and the evidence verifiable? | hard |
| **G4** | M4 | Does combined verification find what per-branch verification misses? | hard |
| **G5** | M5 | Does interruption ever produce a false verdict? | hard |
| **G6** | M6 | Can an outsider use it? | hard (v0.1.0) |
| PG1–PG6 | M7–M12 | provisional commercial/expansion decisions | provisional |

---

## 3. G0 — Fixtures are trustworthy

**Entry:** B-1 approvals recorded; Git ≥ 2.38.1 confirmed (D-05); repo skeleton exists.

**Exit criteria**
| # | Criterion | Evidence |
|---|---|---|
| 1 | `make test` green on a clean clone, no network, `GIT_CONFIG_GLOBAL=/dev/null` | CI log |
| 2 | Forge produces identical repo state across two runs (refs + tree SHAs) | `forge-verify` output |
| 3 | Fixtures F1–F8 and F10 exist with verdicts asserted from `expected.json`; F9 is retained as a fault-injection test (S-1, approved 2026-11-20) | fixture list + smoke tests + fault-injection test |
| 4 | Safety fingerprint detects all five mutations (ref, tree, index, stash, config) | `safety` tests |
| 5 | **Sequential-integration reference conformance (S-3, approved 2026-11-20):** the SP-1R3 reference matrix passes 28/28 with exit 0 on Git 2.55.x and Git 2.38.1, and the four E-1′ cases (3 clean, conflict at step 2, ancestor, unrelated) pass on both approaches. The tuple-conflict question is withdrawn, because no preflight exists | reference conformance note (`docs/research/09-git-floor.md`; T-0005 AC-1, AC-8) |
| 6 | CI matrix runs on linux + macOS | workflow run |

**If reference conformance fails (S-3):** G0 does not pass. The candidate pipeline is re-examined under D-07 before any milestone that depends on it. The former merge-tree plan-change rule is withdrawn with the preflight it described.

**Sign-off:** maintainer. **Artefacts:** `gate-G0.json`, forge self-test log, reference conformance note.

---

## 4. G1 — Safe reading

**Entry:** G0 pass.

**Exit criteria**
| # | Criterion | Evidence |
|---|---|---|
| 1 | All seven repo-condition refusals fire with documented codes and fix commands | scenario log |
| 2 | Pinning is stable: a ref move does not change a pinned SHA | `T-PIN-2` + recorded trace |
| 3 | Config validation rejects unknown keys, denied env names, missing fields | test output |
| 4 | Ownership classification marks user worktrees read-only | scenario log |
| 5 | Safety fingerprint unchanged after every M1 scenario | fingerprint diff, all empty |
| 6 | `doctor` output contains no absolute paths, env values, or identity | `T-SEC-9` |
| 7 | `pin` overhead ≤ 800 ms on the 50k-file fixture | bench output |

**Sign-off:** maintainer. **Artefacts:** `gate-G1.json`, doctor transcript on 3 real repos.

---

## 5. G2 — Candidate is deterministic and safe

**Entry:** G1 pass.

**Exit criteria**
| # | Criterion | Evidence |
|---|---|---|
| 1 | Two runs a week apart → identical candidate SHA | `DET-1` log with both SHAs |
| 2 | Same input set on macOS and Linux → identical SHA (or E-2 negative formally recorded) | cross-OS job |
| 3 | Preflight prediction == real merge on 200 generated cases (or E-3 negative recorded) | `DET-4` report |
| 4 | Triple-only conflict predicted (B-03) | bench output |
| 5 | Safety fingerprint unchanged after success/failure/SIGKILL | fingerprint logs |
| 6 | Forced failure at each of the 5 creation stages leaves no orphan worktree or ref | fault-injection log |
| 7 | Only `refs/ilvuno/*` written (ref diff) | ref-diff artefact |
| 8 | Hooks and signing disabled in candidates, recorded in report metadata | `gitenv` block + `T-SAFE-3` |

**This is the falsification gate.** If criteria 1–4 fail together, stop and revisit the product
premise with the maintainer before starting M3.

**Sign-off:** maintainer + adversarial review (see `07` §10). **Artefacts:** `gate-G2.json`, 200-case
prediction report, cross-OS SHA table.

---

## 6. G3 — Explainable, verifiable

**Entry:** G2 pass.

**Exit criteria**
| # | Criterion | Evidence |
|---|---|---|
| 1 | Every finding has `classification` + `evidence_refs`; `blocking` + `interpretation` is rejected by the schema | schema test |
| 2 | Normalized report byte-identical across runs | diff artefact |
| 3 | Bundle verifies offline with the binary removed from PATH; one-byte edit fails | verify log |
| 4 | Banned-claims scan fails on a deliberately added phrase | negative test |
| 5 | Redaction corpus: 200 secrets caught, 200 benign intact | corpus report |
| 6 | Report readable by a non-author in ≤ 60 s (timed, 1 person) | observation note |
| 7 | Dependency findings name exact constraints for both sources | B-09 output |

**Sign-off:** maintainer. **Artefacts:** `gate-G3.json`, sample bundle, redaction corpus report,
one outside reader's notes.

---

## 7. G4 — Combined verification proves its point

**Entry:** G3 pass.

**Exit criteria**
| # | Criterion | Evidence |
|---|---|---|
| 1 | All five verification outcomes reachable, with fixtures and exit codes | outcome matrix |
| 2 | `combination_only` is `true`, `false`, and `unknown` in the three respective fixtures | B-07, B-12, B-13 |
| 3 | Per-source re-runs recorded as evidence on failure | bundle inspection |
| 4 | A command that commits → `candidate_mutated`, not attributable | `T-CMD-3` |
| 5 | Timeout kills the process group; zero orphans | `T-CMD-2` + pgid scan |
| 6 | Ambient secret-shaped env absent from child | env assertion |
| 7 | Zero-check repo → `unverified`, exit 4, confidence-low block | B-15 |
| 8 | On 5 real repositories, ≥ 1 combination failure found that per-branch checks missed (or none exists and that is recorded) | real-repo log |

**Criterion 8 is the product gate.** If five real multi-stream repositories produce zero
combination failures, continue to M5/M6 but flag `M-1 < 5%` as a live risk and prepare the
repositioning conversation (product `07` §5).

**Sign-off:** maintainer + one design partner or outside reviewer. **Artefacts:** `gate-G4.json`,
three bundles from real repos, `combination_only` tally.

---

## 8. G5 — Interruption is boring

**Entry:** G4 pass.

**Exit criteria**
| # | Criterion | Evidence |
|---|---|---|
| 1 | Crash matrix: 12 stages × 3 signals, every case leaves a defined state | matrix log |
| 2 | `resume` without decisions exits 2 and lists uncertainties | transcript |
| 3 | `--decide=remerge` reproduces the same candidate SHA | B-17 log |
| 4 | Zero orphan worktrees, refs, or processes across the matrix | orphan scan |
| 5 | Journal `JRN-1`/`JRN-2` pass 10,000 random truncations | fuzz log |
| 6 | Tampered journal refused; cannot produce an approval | `T-SEC-5` |
| 7 | Duplicate `integrate` performs zero repeated command executions | counting fixture (B-22) |
| 8 | `abort` leaves the repo byte-identical except `.ilvuno/` | fingerprint diff |

**Sign-off:** maintainer + adversarial crash hunt. **Artefacts:** `gate-G5.json`, fuzz report,
orphan scans, 2 sample resumed-run transcripts.

---

## 9. G6 — Usable by someone else

**Entry:** G5 pass.

**Exit criteria**
| # | Criterion | Evidence |
|---|---|---|
| 1 | All 12 PRD §26 acceptance criteria pass on a clean machine | traceability matrix |
| 2 | An outsider completes install → init → integrate → approve in ≤ 10 min unassisted | recorded session + timing |
| 3 | `approve` refuses without TTY and without the typed SHA | tests |
| 4 | Post-approval input change → `stale_approval`, gate closed | traceability row |
| 5 | Passport renders identically from a clean clone on a second machine | two transcripts |
| 6 | Repair respects budget, never modifies sources, keeps attempts | B-set + repair fixture |
| 7 | Zero network calls to any Ilvuno endpoint in the P0 suite | egress test |
| 8 | Docs: quickstart, config reference, "what this does not prove", troubleshooting | links |
| 9 | Error catalogue complete: every code has a next action | generated table diff |
| 10 | 3 real repositories integrated by someone other than the author | session notes |

**Sign-off:** maintainer + outsider (the outsider's session is the evidence). **Artefacts:**
`gate-G6.json`, traceability matrix, outsider session log, v0.1.0 checksums.

---

## 10. Provisional gates (not approved)

| Gate | Milestone | Would decide | Precondition |
|---|---|---|---|
| PG1 | M7 adapters | Can we start agents without becoming a provider product? | G6 + user request |
| PG2 | M8 desktop | Does a shell add adoption or just maintenance? | G4-commercial + paid demand |
| PG3 | M9 dispatch | Does orchestration + integration beat integration alone? | PG1 + M-1 ≥ 5% |
| PG4 | M10 cross-device | Is continuity worth a network surface? | Gate G4 of product metrics + `SEC-105`/`SEC-107` reviews |
| PG5 | M11 CI | Does CI usage spread the artifact? | G6 |
| PG6 | M12 commercial | Does anyone pay? | 3 teams in writing |

Each provisional gate requires: a written business case, a threat-model delta, and a rollback plan
that leaves the local product unchanged.

---

## 11. Release checklist — v0.1.0

Beyond G6:

- [ ] Version bumped; CHANGELOG written in user terms (no internal jargon)
- [ ] Schemas frozen for the release; compatibility note published (ADR-0014)
- [ ] Binaries: macOS arm64/amd64, Linux arm64/amd64; checksums; no CGO
- [ ] Install docs tested on a clean VM (not the author's machine)
- [ ] `SECURITY.md` with a contact and a threat-model summary link
- [ ] "What this does not prove" page linked from README, `init` output, and the report footer
- [ ] Known-issues list naming Windows, submodules/LFS/sparse, network filesystems
- [ ] Non-goals restated (PRD §13) so expectations are set before install
- [ ] A recorded 90-second demo of the failure case (B-07) for the README
- [ ] Release notes include the benchmark table vs BL-1/BL-2/BL-3 (`04` §5)
- [ ] Rollback instruction: `git update-ref -d refs/ilvuno/...`, `git worktree prune`, delete `.ilvuno/`

---

## 12. What a failed gate must not become

| Antipattern | Why it is forbidden |
|---|---|
| "PASS with exceptions" used more than twice per gate | exceptions are for surprises, not for unfinished work |
| Criteria edited to match what was built | the criteria come from PRD/architecture; changing them is a separate, reviewed decision |
| Gate run only by the author | a gate the author grades is not a gate |
| A gate passed on a different machine/config than documented | the packet records `doctor` output and versions |
| Deferring a failed safety criterion to "post-release" | `INV-1` failures are severity-1 and stop the release |
