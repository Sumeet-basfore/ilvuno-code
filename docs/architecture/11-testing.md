# 11 — Testing Strategy

Status: specification. The testing strategy is part of the architecture because determinism is a
product promise (`NFR-103`, `INV-2`) and because half the failure modes are only reachable by
injection.

---

## 1. Test pyramid for this product

Classic unit-heavy pyramids fail here: the interesting behaviour is *git plus processes plus
crash timing*. So the shape is:

```
        ┌──────────────────────────────┐
        │ crash / signal matrix  (~40) │   slow, serial, per-OS
        ├──────────────────────────────┤
        │ scenario tests on fixtures   │   the core suite (~120)
        ├──────────────────────────────┤
        │ property & determinism (~30) │   fast, generative
        ├──────────────────────────────┤
        │ unit (pure funcs, parsers)   │   fast, plentiful
        └──────────────────────────────┘
```

Budget: the whole suite ≤ 6 minutes on CI, excluding the crash matrix (≤ 12 minutes, nightly +
on PRs touching `engine`/`journal`).

---

## 2. The fixture forge

Every scenario is built from code, not checked in as binary git repos. One helper:

```
forge.New(t, forge.Spec{
    Base:          "main",
    Files:         map[string]string{...},   // committed on base
    Branches:      []forge.Branch{{Name, From, Changes: map[string]string{}, Delete: [...]}},
    MergeOrder:    []string{},
    Env:           forge.DeterministicEnv,   // fixed identity/date/tz
})
```

Rules that make fixtures trustworthy:

| Rule | Reason |
|---|---|
| Fixed commit dates, identity, `TZ=UTC`, `umask 022` | otherwise candidate SHAs are not comparable across runs or machines |
| Fixtures are created in `t.TempDir()` | no shared state, parallel-safe |
| Each fixture records its own git version in the test log | a git upgrade that changes merge output is diagnosable |
| No network, no global git config (`GIT_CONFIG_GLOBAL=/dev/null`), hooks dir emptied | reproducibility |
| Fixture builders are themselves tested (`forge_test.go`) | a broken forge makes every scenario lie |

A fixture is ~15 lines. The canonical set below is the specification's real deliverable: these
are the nine failure classes the product claims to handle.

---

## 3. Required fixtures (the nine classes)

| ID | Fixture | Shape | Asserts |
|---|---|---|---|
| `T-CONF-1` | Textual conflict, pair | Both branches edit the same hunk of `a.ts` | Preflight predicts, real merge confirms, exit 3, zero side effects, user tree fingerprint unchanged |
| `T-CONF-2` | Triple-only conflict | A∩B clean, A∩C clean, B∩C clean; `{A,B,C}` conflicts | Preflight **tuple** check catches it; pairwise-only would not (`06` §4.1) |
| `T-CONF-3` | Adjacent-but-clean | Edits 2 lines apart | Merge clean; `hunk_proximity` warning when enabled |
| `T-CONF-4` | Rename vs edit | A renames `x.ts→y.ts`, B edits `x.ts` | Recorded as a real conflict; prediction matches |
| `T-API-1` | Interface incompatibility, clean merge | A renames exported `parse()`, B adds a caller of `parse()` | Each branch green alone; candidate fails typecheck; `combination_only: true` |
| `T-API-2` | Index-driven symbol touch | Same as above with `index_command` configured | `interface_symbol_touched` deterministic warning naming both SHAs |
| `T-DEP-1` | Dependency disagreement | A adds `left-pad@1`, B adds `left-pad@2` in `package.json` | `dependency_disagreement` blocking when configured; per-source constraints in data |
| `T-DEP-2` | Lockfile churn only | Both touch `pnpm-lock.yaml`, no declaration change | `lockfile_churn` info; not blocking |
| `T-API-3` | Schema/migration collision | A and B both add migration `007_*.sql` | File-overlap + conflict; report names both |
| `T-AGENT-1` | Agent crash mid-run (post-MVP adapter) | Adapter process exits 137 during a turn | `AgentRun.state = failed`, no `passed`, no approval path |
| `T-AGENT-2` | Agent lies about completion | `reported_files` includes a file it never touched; no commit produced | `claim_divergence` warning; `produced_refs` empty; task state unchanged |
| `T-CMD-1` | Failed command | A check exits 1 with output | `failed`, evidence has log + exit + duration, exit code 1 |
| `T-CMD-2` | Command timeout | Check sleeps past its timeout | Group killed, `timed_out`, no orphans, evidence records TERM vs KILL rung |
| `T-CMD-3` | Command mutates candidate | Check commits on the candidate | `candidate_mutated` failure; result not attributable |
| `T-CMD-4` | Command writes files | Check creates untracked artifacts | Warning + `outputs` list; result still bound to `candidate_sha` |
| `T-EVD-1` | Stale evidence | Approve, then change a source SHA | `stale_approval`, gate closed |
| `T-EVD-2` | Tampered log | Edit a byte in a bundle log | `bundle verify` fails; approval invalidated |
| `T-EVD-3` | Missing bundle | Delete bundle dir, then `task show` | `evidence_missing`; state downgrades to `unverified`, never `passed` |
| `T-EVD-4` | Pruned bundle | `bundle prune` then approve | `verifiable-as-pruned` tombstone; approval refused (or accepted per config — decided: **refused**) |
| `T-INT-1` | Interrupted verification | SIGKILL during a check | `interrupted`, uncertainty `verify_outcome_unknown`, `resume --decide=rerun` produces `attempt: 2` and keeps attempt-1 evidence |
| `T-INT-2` | Interrupted merge | SIGKILL between merge intent and result | uncertainty `merge_outcome_unknown`; `--decide=remerge` reproduces the **same** candidate SHA |
| `T-INT-3` | Interrupted seal | SIGKILL during bundle seal | `bundle.tmp` present, no partial `bundle/`; reseal is idempotent |
| `T-COMP-1` | Wrong completion state: no checks | Config with zero commands | `unverified`, exit 4, confidence low block present |
| `T-COMP-2` | Wrong completion state: advisory only | One advisory check failing | Outcome unaffected, advisory failure still visible |
| `T-COMP-3` | Wrong completion state: `passed` with a skipped command | `needs:` prerequisite failed | `inconclusive`, not `passed` |
| `T-COMP-4` | **Architectural**: agent completion ≠ integration | Assert no code path maps `AgentRun.completed` → approval/passed | Static check over the `model` package + a scenario that tries to obtain one |
| `T-SAFE-1` | Unauthorized write | Config `cwd = "../.."`, ref `refs/heads/main` as a write target, journal edited to target a user worktree | All refused; repo fingerprint (refs + tree + index + stash) unchanged |
| `T-SAFE-2` | User-work preservation | Dirty candidate, cleanup runs | Patch archived, worktree removed, user tree untouched |
| `T-SAFE-3` | Hook suppression | A repo hook writes a marker file | No marker after candidate build (`INV-11`) |
| `T-SEC-1..11` | §9 of doc `09` | enumerated there | enumerated there |

---

## 4. Property-based tests

Generative, with shrinking. These catch the class of bug fixtures miss.

| ID | Property | Generator |
|---|---|---|
| `DET-1` | `integrate(spec)` twice ⇒ equal candidate SHA, equal tree, equal normalized report | random DAGs of file edits over a 20-file repo, 200 cases |
| `DET-2` | Candidate SHA independent of wall clock, cwd, HOME, TZ | run matrix over env perturbations |
| `DET-3` | `task_state` and `next_actions` are pure functions of the record set (permutation-invariant where the model says so) | shuffle record insertion order |
| `DET-4` | Preflight prediction agrees with actual merge outcome for every pair and tuple | generated branch sets; compare `merge-tree` vs real merge |
| `ORD-1` | Ordering is a total function: same source set + same policy ⇒ same order, for any input order | permutations |
| `INV-P1` | For any scenario, the set of written paths ⊆ declared write roots | instrumented FS |
| `INV-P2` | For any scenario, refs outside `refs/ilvuno/*` are unchanged | ref snapshot diff |
| `RED-1` | Any string matching a redaction class is absent from all artifacts | seeded corpus of 200 secrets + 200 benign lookalikes (asserts both directions) |
| `JRN-1` | Any prefix of a journal is parseable and yields a defined state | truncate at every byte boundary for a small run (bounded to 500 samples) |
| `JRN-2` | `resume` from any truncation either completes or refuses with a typed reason — never panics, never silently passes | same corpus |

`JRN-1`/`JRN-2` are the highest-value tests in the suite: they are a direct encoding of
"recoverable after interruption".

---

## 5. Safety fingerprint

One helper used by nearly every scenario test:

```
fp := safety.Fingerprint(repo)   // {HEAD, tree, index_digest, stash_list, refs{non-ilvuno}, config_digest}
... run command ...
require.Equal(fp, safety.Fingerprint(repo))
```

`Fingerprint` must be cheap and complete:

| Component | Method |
|---|---|
| HEAD / each ref | `git for-each-ref --format='%(refname) %(objectname)'` filtered to non-`refs/ilvuno/` |
| Working tree | `git status --porcelain=v2 --untracked-files=all` + content digest of modified files |
| Index | `git ls-files --stage` digest |
| Stashes | `git stash list --format=%H %s` |
| Config | digest of `.git/config` |

If any scenario changes that set, the test fails with a diff — which is exactly the message a
human needs (`INV-1`, `T-SAFE-1`).

---

## 6. Static and contract tests

| Check | Tool | Fails the build when |
|---|---|---|
| Import rule (`engine` must not import `agent`) | `go/packages` analyzer or a `go vet`-style custom check | any violation |
| No `git push` / no write to `refs/heads/*` anywhere | grep over source + a runtime argv assertion in tests | any match |
| Banned claims in user-facing strings (PRD Appendix A) | string table scan | any match |
| No absolute paths in persisted documents | test over all fixture outputs | any match |
| Schema validation of every emitted artifact | validate against `ilvuno.dev/*/v1` | any failure |
| Exit-code matrix completeness | table test over every error code | a code without a test |
| Determinism of `--json` output | byte comparison across two runs | any diff (excluding volatile block) |
| Docs/CLI drift | parse `--help` and compare to `docs/product` command list | mismatch |

The banned-claims test deserves emphasis: it is the mechanism that keeps the product honest as
contributors add helpful-sounding output (`VER-101`). It scans string literals, not code.

---

## 7. Crash and signal matrix

Deterministic process killing, not "hope the race happens".

```
for each stage in {fetch, pin, preflight, worktree_add, merge(1..n), verify_start,
                   verify_command(1..n), verify_end, seal, cleanup}:
    for each signal in {SIGKILL, SIGTERM, SIGINT}:
        run with test hook: die at stage
        assert: journal parses, state ∈ {interrupted, settled}, no orphans
        then: resume() → completes or refuses with a typed reason
        assert: safety fingerprint unchanged; eligibility correct
```

Implementation: the engine exposes an internal `testhooks.DieAfter(stage)` only compiled with the
`testhooks` build tag; tests set it via env. This is deliberately not a public flag.

Additional assertions after every crash case:

| Assertion | Reason |
|---|---|
| No worktree path exists outside `.ilvuno/worktrees/` | orphan control |
| `git worktree list` has no entries for the crashed run (or they are `orphaned` and reported) | doc `05` §4 |
| No child process from the crashed run survives (pgid scan) | supervision |
| Evidence bundle is either absent or complete+verifiable (never partial-and-valid) | sealing |

---

## 8. Future-surface tests (designed now, run later)

| Area | Test |
|---|---|
| Adapters | Recorded provider transcripts replayed against the adapter (the t3code replay-tkit pattern, `T3-072`); no live model calls in CI |
| ACP dialects | Fixture frames per agent id; capability negotiation asserted |
| Relay/E2EE | Fake relay with recorded frames; assert plaintext never appears in relay-visible bytes |
| Pairing | Scripted host+device exchange; assert single-use code, TTL expiry, fingerprint match, revocation immediacy |
| Event replay | Property: any `after_seq` yields the same final state as a full replay |
| Remote approval | Assert host re-verifies the bundle and refuses a stale candidate |

Adapter fixtures are recorded, not live — a test suite that depends on a provider's availability
is a suite that will be disabled (`CR` docs' own warning that fixtures are not real-inference
evidence; we accept that limit explicitly).

---

## 9. What is deliberately not tested

| Not tested | Why |
|---|---|
| Real provider inference quality | Not our code; out of scope |
| Git itself | We assert against real git behaviour, not a reimplementation |
| Windows (v0.1) | Unsupported; CI matrix is macOS + Linux |
| Network filesystems | Documented as "works until it does not"; `doctor` warns |
| Availability of a hosted service | none exists |
| Performance at 1M-file repos | Out of the stated envelope (100k tracked files, `NFR-108`) |

Stating this list prevents a reader from assuming coverage that does not exist.

---

## 10. CI matrix

| Job | OS | Trigger | Contents |
|---|---|---|---|
| `unit` | linux, macOS | every push | unit + property (fast subset) |
| `scenario` | linux, macOS | every push | all fixtures in §3 |
| `determinism` | linux, macOS | every push | `DET-1..4`, twice per job |
| `crash` | linux, macOS | PRs touching engine/journal; nightly | full signal matrix |
| `git-versions` | linux | nightly | oldest supported git (2.38) + latest |
| `contract` | linux | every push | §6 static/contract checks |
| `race` | linux | nightly | `-race` over scenario + property suites |

Determinism runs on **two OSes** on purpose: if `DET-1` passes on macOS and fails on Linux, we
have found a real nondeterminism source (usually line endings or locale) before a user does.

---

## 11. Open questions

| ID | Question | Recommendation |
|---|---|---|
| `TS-1` | How do we test git-version drift without slow CI? | Nightly only, plus a `doctor` probe that warns on untested versions |
| `TS-2` | Should the crash matrix run on every PR? | Only PRs touching `engine`/`journal`/`verify`; nightly for the rest |
| `TS-3` | Do fixture repos need to be real projects (a 500-file TS monorepo)? | One, for the `NFR-102`/`NFR-108` budgets; the rest stay tiny for speed and clarity |
| `TS-4` | How do we keep the banned-claims list current? | It lives beside the report templates; adding a template string requires touching the list (review gate) |
| `TS-5` | Is a golden-file report test too brittle? | Normalize volatile fields into a separate block (doc `12` §4) and compare the rest byte-for-byte — that is the point |
