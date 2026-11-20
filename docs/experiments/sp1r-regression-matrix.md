# SP-1R — Regression Matrix

The matrix is now an **enforcing test**

> **SP-1R2 note:** case count and verdicts are unchanged (28). The expectation source is now
> `spec.json` + `oracle.json` (independent Git-native merges), not the pipeline. The assertion suite
> grew to 67. See `docs/reviews/sp1r2-closure-evidence.md`., not a recording. `./sp1.sh matrix` exits 0 only when every
expected row exists, parses as JSON, and matches `expected.json` on verdict, exit class, conflict
step and paths, pins, candidate/tree SHA, content hashes, cleanup state, residual refs/worktrees,
and whether a driver ran. Anything else exits 1.

Verified on **Git 2.55.0** (CachyOS Linux) and **Git 2.38.1** (pinned container). Identical results.

---

## 1. Cases (28 = 14 fixture/variant pairs × 2 approaches)

| Fixture | Variant | A exit / verdict | B exit / verdict | Step | Conflict paths | Candidate A=B | Cleanup | Residual wt / refs |
|---|---|---|---|---|---|---|---|---|
| F1 two clean | plain | 0 · ok | 0 · ok | — | — | **yes** | removed / removed | 0 / 0 |
| F1 two clean | index-lock | 0 · ok | 0 · ok | — | — | **yes** | removed / removed | 0 / 0 |
| F2 three clean | plain | 0 · ok | 0 · ok | — | — | **yes** | removed / removed | 0 / 0 |
| F2 three clean | stop-after1 | 9 · stopped | 9 · stopped | — | — | n/a | kept / kept | 0 / 1 |
| F3 direct conflict | plain | 3 · conflict | 3 · conflict | 2 | `shared.txt` | n/a | kept / kept | 0 / 1 |
| F3 direct conflict | keep-conflict | 3 · conflict | 3 · conflict | 2 | `shared.txt` | n/a | kept / kept | 0 / 1 |
| F4 conflict at step 2 | plain | 3 · conflict | 3 · conflict | 2 | `shared.txt` | n/a | kept / kept | 0 / 1 |
| F5 already integrated | plain | 0 · ok | 0 · ok | — | — | **yes** | removed / removed | 0 / 0 |
| F6 unrelated histories | plain | 3 · unrelated | 3 · unrelated | 2 | — | n/a | kept / kept | 0 / 1 |
| F7 combined check failure | plain | 1 · check_failed | 1 · check_failed | — | — | **yes** | removed / removed | 0 / 0 |
| F8 custom driver | plain | 4 · refused | 4 · refused | — | — | n/a | none / none | 0 / 0 |
| F9 cleanup | plain | 0 · ok | 0 · ok | — | — | **yes** | removed / removed | 0 / 0 |
| F9 cleanup | fail-cleanup | 5 · ok | 5 · ok | — | — | **yes** | orphaned_dir_removed (both) | 0 / 0 |
| F10 newline path | plain | 3 · conflict | 3 · conflict | 2 | `line\nbreak.txt` | n/a | kept / kept | 0 / 1 |

Verdict distribution: `ok` 12, `conflict` 8, `stopped` 2, `unrelated_histories` 2,
`check_failed` 2, `harness_refused` 2.

Notes on the two "kept" states:

- **stop-after1** simulates a crash. A leaves nothing; B leaves one worktree, which is the measured
  cost of the worktree pipeline. Neither leaves refs.
- **conflict** deliberately keeps B's worktree so markers, stages and `MERGE_HEAD` survive for
  inspection (`keep-conflict` asserts the same for A, which simply never creates one). Cleanup at
  the end of a conflict run is `ok` because the worktree is *intentionally* retained; the scratch
  root is wiped before the next run.

`fail-cleanup` demonstrates the SG-11 failure: `git worktree remove` deletes the admin entry, fails
on the directory, and the harness then removes the owned orphan and records
`orphaned_dir_removed` with **exit 5** — the run is not reported as clean.

Removed from the matrix: `ignore-drivers`. The harness no longer has a mode that executes a
configured driver; that behaviour is characterized only by probe P4 in a throwaway repo.

---

## 2. What each case asserts

`expected.json` stores, per case: `verdict`, `exit`, `conflict_step`, `conflict_paths`,
`check_rc`, `cleanup`, `candidate`, `tree`, `content` (path → sha256 for every file in the
candidate), `base`, ordered `sources`, `user_state_unchanged`, `refs_left`, `worktrees_left`,
`drivers_ran`.

The checker additionally requires:

| Rule | Failure mode |
|---|---|
| Every expected row present | missing row |
| No unexpected rows | extra row |
| Each line is valid JSON | malformed output |
| Required fields present | schema drift |
| Verdict ∈ known set | typo/regression in classification |
| All compared fields equal | any behavioural change |
| Numeric fields are integers | string/number drift |

---

## 3. Negative tests (the checker must be able to fail)

| Test | Injection | Expected | Result (2.55.0 / 2.38.1) |
|---|---|---|---|
| P1.1 | drop the last row | exit 1, "missing row" | PASS / PASS |
| P1.2 | append `{not json}` | exit 1, "malformed JSON" | PASS / PASS |
| P1.3 | replace a candidate SHA with zeros | exit 1, field diff | PASS / PASS |
| P1.4 | unmodified matrix | exit 0 | PASS / PASS |
| — | `SP1_GIT_BIN=/bin/false` | exit 1, 28 missing rows | PASS (manual, both) |

The `/bin/false` case is the audit's V-1 reproduction: before remediation it exited 0 with zero
rows; now it exits 1.

---

## 4. Repeated runs

The full matrix was run twice from a wiped scratch root on each Git version. Candidate/tree SHAs,
content hashes and cleanup states were identical. `expected.json` was recorded once and both later
runs matched it without regeneration.

That is repeatability **within the stated environment** (same Git version and OS family). It is not
a cross-version determinism claim beyond the two versions tested here — although the two versions
did produce identical candidate SHAs for every clean case.

---

## 5. Reproduction

```bash
cd docs/experiments/scripts
mkdir -m700 /tmp/sp1-scratch
SP1_SCRATCH=/tmp/sp1-scratch ./sp1.sh matrix
SP1_SCRATCH=/tmp/sp1-scratch ./tests.sh
```

Git 2.38.1:

```bash
docker build -f Dockerfile.git238 -t ilvuno-git238r .
docker run --rm -u "$(id -u)":"$(id -g)" -v "$PWD":/sp1r -w /sp1r \
  -e SP1_GIT_BIN=/usr/local/bin/git -e SP1_SCRATCH=/tmp/sp1-c \
  ilvuno-git238r bash -lc 'mkdir -m700 -p "$SP1_SCRATCH"; ./sp1.sh matrix && ./tests.sh'
```
