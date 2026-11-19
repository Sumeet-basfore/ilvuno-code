# Ilvuno — User Journeys

Eight journeys. Each states: trigger, steps, what the user sees, what state is written,
exit code, and failure branches. Requirement IDs resolve in `02-prd.md`.

Conventions used in outputs: `$` = user action, `✓ ✗ ! ·` = ok / fail / warn / info.
All paths are repo-relative; Ilvuno writes only `.ilvuno/`, `refs/ilvuno/*`, and the task
directory (`SEC-101`).

---

## J-1 First run (the ten-minute test)

**Persona:** Priya. **Requirements:** NFR-101, INT-101, INT-301, INT-310.

```
$ ilvuno init
· detected: git repo (main, 2 worktrees), 1 test script in package.json
· created: ilvuno.toml
· evidence dir: .ilvuno/ (gitignored)

  verification.commands is empty. Ilvuno will report UNVERIFIED until you add at
  least one command. Suggested from your package.json:

    [verification]
    commands = [
      { name = "typecheck", run = "pnpm -s typecheck", timeout = "300s", required = true },
      { name = "test",      run = "pnpm -s test",      timeout = "900s", required = true },
    ]

· next: add a command above, then `ilvuno integrate --base main --branch <ref>`
```

```
$ ilvuno doctor
✓ git 2.43
✓ worktree support
✓ base fetch: 41 ms
! verification: no commands configured   (INT-303)
! ssh remotes: none
· nothing else will be written outside .ilvuno/ and refs/ilvuno/*
```

**Design notes.**
- `init` proposes commands from the repo's own scripts; it never invents a checkout.
- `doctor` states the two things users get wrong: no verification configured, and a dirty
  working tree.
- No account prompt. No network call. If the ten-minute target is missed, the product is
  wrong, not the user.

**Failure branches:** not a git repo (`unpinnable_source`-adjacent, exit 2); shallow clone
without merge-base (`REC-106`); read-only filesystem (`evidence_write_failed`).

---

## J-2 Happy path: a batch that integrates

**Persona:** Priya. **Requirements:** INT-102–INT-108, INT-301, INT-304, INT-601, INT-602.

```
$ ilvuno integrate --base main \
    --source agent/payments-intent --source agent/fulfilment --source agent/e2e-updates
```

```
· pin      3 sources
           payments-intent  4b825dc  base 7c4758d  drift 0
           fulfilment       9f2c4a1  base 7c4758d  drift 0
           e2e-updates      c1a9e02  base 7c4758d  drift 0
· preflight
           2 files touched by 2+ sources
           no textual conflicts predicted
           no dependency disagreements
· candidate  merging in order: payments-intent → fulfilment → e2e-updates
           built b7d0f31 (3 steps, all clean)
· verify    typecheck  ✓ 41.2s
            test       ✓ 8m52s
            2 of 2 required checks passed
· record    .ilvuno/bundles/2026-11-19T14-02-11Z/

✓ integration candidate is green
  candidate  b7d0f31   base 7c4758d
  evidence   .ilvuno/bundles/2026-11-19T14-02-11Z/
  report     .ilvuno/bundles/2026-11-19T14-02-11Z/report.md

  combination-only failures: 0
  not verified: none

  This does not prove the change is correct. It shows these three revisions
  pass these two checks together. (VER-101)

· next: review the combined diff:  ilvuno show --bundle 2026-11-19T14-02-11Z --diff
        when satisfied:             ilvuno approve --bundle 2026-11-19T14-02-11Z
```

**State written:** evidence bundle; journal entry; run record. **Approval not yet granted**
(`INT-602`) — a green run does not approve itself. **Exit 0.**

**Failure branches:** any predicted conflict → J-3; any check failure → J-4.

---

## J-3 Conflict discovered before anything is touched

**Requirements:** INT-108, INT-107, INT-110, VER-102.

```
$ ilvuno integrate --base main --source agent/a --source agent/b
```

```
· pin      2 sources
· preflight
✗ merge_conflict (deterministic)
           agent/a  packages/api/src/checkout.ts:L40-L71
           agent/b  packages/api/src/checkout.ts:L44-L68
           overlapping hunk; predicted via merge-tree

  Nothing was merged. Your working tree is unchanged.
  candidate would fail at step 2 (agent/b onto agent/a).

  evidence    .ilvuno/runs/2026-11-19T14-20-03Z/preflight.json
  next        ilvuno show --run 2026-11-19T14-20-03Z --conflict
              # or integrate the rest and skip this source:
              ilvuno integrate --base main --source agent/a --continue-on-conflict
```

**Key behaviour:** the prediction is marked `predicted`, and the run stops *before* the
candidate is built (`INT-108` invariant: nothing is mutated). If the user proceeds with
`--continue-on-conflict`, the real merge result is recorded and the prediction is
reconciled — a prediction that disagrees with reality is itself a finding (`R-6`).

**Exit 3** — distinct from a check failure (exit 1) and from a usage error (exit 2).

---

## J-4 The failure that only exists in combination

**Persona:** Priya, then Marco. **Requirements:** INT-306, INT-210, INT-304, EVD-107.

```
$ ilvuno integrate --base main --source agent/payments-intent --source agent/fulfilment
```

```
· candidate  built 9f2c4a1
· verify     typecheck  ✓ 40.8s
             test       ✗ exit 1   18.4s
             pnpm -s test --filter checkout
             FAIL packages/checkout/test/checkout.e2e.test.ts
               × pays with saved card after split  — 12ms
                 TypeError: cannot read properties of undefined (reading 'intent')

· isolate    re-running on each source alone
             payments-intent  test ✓ 17.9s
             fulfilment       test ✓ 18.1s
· classify
✗ check_failed (deterministic)  combination-only
             passes on each source; fails only when both are present

  evidence    .ilvuno/bundles/2026-11-19T14-02-11Z/
              logs/test.log (full), logs/test-payments-intent.log, logs/test-fulfilment.log
  next        ilvuno repair --bundle 2026-11-19T14-02-11Z
              ilvuno show   --bundle 2026-11-19T14-02-11Z --diff checkout
```

**Why this is the money shot.** Neither agent is wrong on its own. The verification that
ran on each branch (`ORC-054`, `T3-047`) is structurally incapable of finding this. This is
the one scenario that justifies the product's existence, and the report states it in one
line.

**State written:** evidence bundle with three logs and the per-source comparison
(`INT-306`), a passport entry if a task is linked. **Exit 1.**

---

## J-5 Bounded repair

**Requirements:** INT-501–INT-506, EVD-104.

```
$ ilvuno repair --bundle 2026-11-19T14-02-11Z
· attempt  1 of 2  (budget: 2)
· brief    .ilvuno/bundles/.../repair-brief.md
           failing: test (exit 1); combination-only
           sources: payments-intent 4b825dc, fulfilment 9f2c4a1
           constraint: do not modify source branches
· running  repair command: "claude -p < brief"      (INT-502, user-configured)
· branch   refs/ilvuno/repair/9f2c4a1-1
· diff     3 files, 41 insertions, 2 deletions
· verify   typecheck ✓ · test ✗  exit 1  same failure
· record   attempt 1 failed (evidence kept)

· attempt  2 of 2
· verify   test ✓ 18.9s
· record   attempt 2 succeeded

✓ repaired b0e7d22  (candidate 9f2c4a1 + repair 1)
  evidence  .ilvuno/bundles/2026-11-19T14-33-52Z/
  next      ilvuno show --bundle 2026-11-19T14-33-52Z
```

Budget exhausted variant:

```
✗ budget_exhausted — 2 of 2 repair attempts failed
  attempt 1  refs/ilvuno/repair/...-1  test ✗  (evidence kept)
  attempt 2  refs/ilvuno/repair/...-2  test ✗  (evidence kept)
  last error TypeError: cannot read properties of undefined (reading 'intent')
  escalation: this failure is likely a genuine interface mismatch between the two
              sources, not a flaky check. Recommended: change the contract in
              packages/api and re-run both streams, or integrate them sequentially.

  nothing was deleted. source branches are unchanged.
```

**Why a budget and not a loop.** Research shows each surveyed system has a retry limit for
dispatch or a single PR, and none keeps a record of failed attempts (`05` §2.10). Two
attempts is a default, not a belief; the recorded attempts are the deliverable.

**Exit:** 0 on successful repair, 1 on `budget_exhausted`.

---

## J-6 Returning after three days

**Persona:** Priya. **Requirements:** TSK-101–TSK-112.

```
$ ilvuno task show checkout-split
```

```
checkout-split                                    awaiting-approval
Split checkout into payment-intent and fulfilment without changing packages/api.

acceptance criteria
  AC-1  packages/api public surface unchanged                     ✓ verified
  AC-2  checkout e2e green on integrated candidate                ✓ verified

decisions
  D-1   Keep legacy /checkout; add /checkout/v2                   priya · 11-19
        why: mobile 1.x still calls the old route

streams
  payments-intent   claude-code  4b825dc   base 7c4758d
  fulfilment        codex        9f2c4a1   base 7c4758d
  e2e-updates       claude-code  c1a9e02   base 7c4758d

verification
  2026-11-19T14-02-11Z  ✗ failed   candidate 9f2c4a1  combination-only
  2026-11-19T14-33-52Z  ✓ passed   candidate b0e7d22  (after 2 repair attempts)

pending approval
  A-1  land b0e7d22 (3 sources + repair) onto main                blocking · since 11-19

next actions
  1. review combined diff for b0e7d22
  2. ilvuno approve --task checkout-split --candidate b0e7d22

not included in this record
  conversation transcripts · provider session ids · credentials
```

**Design notes.**
- State and next actions are derived (`TSK-107`, `TSK-106`); the user never types a status.
- It reads as a handover document. Marco can read it without the tool.
- `not_included` is visible, not buried (`TSK-112`).

**Failure branches:** evidence bundle missing → `evidence_missing` on that line, status
downgraded to `unverified` (never silently "passed"); refs not fetched on this machine →
dangling ref flagged with the fetch command.

---

## J-7 Refusal: unsafe input

**Requirements:** ORC-101, INT-102, INT-104, INT-110, REC-102.

```
$ ilvuno integrate --base main --source .
✗ shared_workspace — refusing to integrate your working tree as a source

  '.' is your active working tree. Integrating it would mix uncommitted changes
  into a candidate and could modify files you are editing.

  next  commit, or integrate a branch:
        ilvuno integrate --base main --source refs/heads/agent/foo
```

```
$ ilvuno integrate --base main --source agent/moving-target
! agent/moving-target moved during the run (was 4b825dc, now 3ac11f0)
✗ aborted — source changed underneath the run
  no candidate was created; no files were written to your working tree.
```

```
$ ilvuno resume --run 2026-11-19T15-02-44Z
! this run was interrupted while merging source 2 of 3
  known:      source 1 merged (candidate ref written)
  unknown:    whether source 2's tree update reached disk before the crash
  policy:     Ilvuno does not repeat uncertain side effects automatically (REC-102)

  next  ilvuno resume --run ... --decide=remerge   (rebuild from scratch, safe)
        ilvuno abort  --run ...                    (discard the candidate)
```

**Design notes.** Every refusal names the reason, the risk, and one command. The third
example is the honesty case: after a crash we say what we do not know, which is more
trustworthy than a silent re-run that might double-apply a merge.

**Exit 2** for refusals; **130** for interruption.

---

## J-8 The platform team runs it in CI

**Persona:** Dana, Marco. **Requirements:** ORC-106, VER-105, EVD-106, SEC-101, NFR-107.

```yaml
# .github/workflows/integration.yml (added by Dana)
- run: ilvuno integrate --config ilvuno.toml --base origin/main \
         --sources-from plan.yaml --json --report integration.json
- uses: actions/upload-artifact@v4
  with: { name: ilvuno-evidence, path: .ilvuno/bundles }
```

```
$ ilvuno integrate ... --json
{"schema":"ilvuno.dev/report/v1","status":"failed","exit":1,
 "base_sha":"7c4758d...","candidate_sha":"9f2c4a1...",
 "findings":[{"id":"F-3","classification":"deterministic","kind":"check_failed",
              "combination_only":true,"source_shas":["4b825dc","9f2c4a1"]}],
 "verification":{"passed":1,"failed":1,"unverified":0,"inconclusive":0},
 "not_verified":[], "confidence":"high"}
```

**Design notes.**
- The JSON contract is versioned independently of the CLI, so a pinned consumer keeps
  working across minor releases (`VER-105`).
- The evidence bundle is the CI artifact. Reviewers download one file, not a log haystack.
- No service account, no token, no network call to Ilvuno (`XDC-101`, `OSS-102`).
- `sun` on failure is a *report*, not a blocked merge: Dana decides policy, not the tool.

**Failure branches:** artifact upload fails → the run is still valid locally but the
evidence is lost; the report says `evidence_write_failed` rather than pretending success.
Concurrent CI runs on the same repo → distinct run ids, no shared mutable state
(`REC-105`).

---

## 9. Journey → requirement coverage check

| Journey | Primary requirements | Covered? |
|---|---|---|
| J-1 | NFR-101, INT-310, REC-106 | yes |
| J-2 | INT-102–108, INT-301–304, INT-601–602 | yes |
| J-3 | INT-107, INT-108, INT-110, VER-102 | yes |
| J-4 | INT-306, INT-210, EVD-107 | yes |
| J-5 | INT-501–506 | yes |
| J-6 | TSK-101–112 | yes |
| J-7 | ORC-101, REC-102, REC-103 | yes |
| J-8 | ORC-106, VER-105, EVD-106 | yes |

Gaps deliberately deferred: multi-machine task viewing (XDC-102 is P0 but its *journey*
depends on rung 2 transport, post-MVP), mobile approval (J7 persona job, `XDC-103` is the
enabling requirement, the journey is not written because the feature is not in scope).
