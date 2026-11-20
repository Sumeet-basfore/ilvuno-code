# 03 — Security Boundary Correction

Status: proposal. No approved document modified. Experiments referenced as `X-nn` are in
`02-git-integration-correction.md` §2.

---

## 1. The problem with `SEC-101` as written

Current text (PRD): *"Tool never writes outside `.ilvuno/`, `refs/ilvuno/*`, and the configured
task directory … Path audit shows no other writes, including on failure paths."*

Four things make this unenforceable:

| # | Fact | Consequence |
|---|---|---|
| 1 | `merge-tree --write-tree` writes loose objects (X-06; object count 30→32 on a conflicting merge) | The audit must either allow `objects/**` (weakening the sentence) or fail a legitimate operation |
| 2 | `worktree add` writes `.git/worktrees/<n>/**` (X-06) | Same |
| 3 | `init` writes `ilvuno.toml` and edits `.gitignore`; `key init` writes under `~/.config/ilvuno/` | Same, plus a path outside the repo |
| 4 | A custom merge driver is arbitrary code and **runs** during `merge-tree`, writing anywhere (X-05) | No path audit of Ilvuno's own code can cover it |

An invariant that cannot be enforced is worse than a narrower one that can: the test either passes
while the claim is false, or fails on normal operation and gets muted. Both outcomes destroy the
credibility the product depends on.

---

## 2. Replacement model: four effect classes

**PROPOSED replacement for `SEC-101` / `INV-1`:**

> **Effect classes.** Ilvuno's own code performs effects only in classes A and B. Class B is
> enumerated, observable, and reported. Class C is executed under an explicit consent gate and is
> **not** constrained by Ilvuno. Class D happens once, at the user's explicit command.
>
> | Class | Owner | Examples | Constrained by us? |
> |---|---|---|---|
> | **A — Ilvuno-owned** | Ilvuno | `.ilvuno/**` (journals, bundles, approvals, locks, worktrees dir), `docs/ilvuno/tasks/**` (passports) | Yes: allowlist per subcommand |
> | **B — Git-managed** | Git, on our instruction | `objects/**` (merge, commit, checkout), `refs/ilvuno/**` + `logs/refs/ilvuno/**`, `.git/worktrees/<n>/**`, `FETCH_HEAD`, `packed-refs` if the user packs | Yes: only the enumerated operations; all use anchored `refs/ilvuno/` targets |
> | **C — Child process** | the project's own commands | verification and repair children: writes to `$HOME`, `node_modules`, caches, network | **No.** Declared, allowlisted env, process-group bounded |
> | **D — One-time consent edits** | the user | `ilvuno.toml`, one `.gitignore` line, `~/.config/ilvuno/keys/**` | Only on the user's explicit command, never automatically |

Rule that replaces "no writes outside": **no writes outside classes A, B and D by Ilvuno's own code,
and every class-B write is derived from a declared Git operation.** Class C is out of scope by
design and is named in the report.

### 2.1 Why this is stronger, not weaker

The old sentence was a claim about everything. The new model is a claim that can be *tested*:

- A path audit instruments Ilvuno's own syscalls/commands and asserts the set ⊆ A ∪ B ∪ D for every
  scenario, including failures and crashes.
- Class B is compared against a **declared operation table** (2 §6), so an unexpected Git write (a
  new Git version writing a new file) fails the test instead of being silently allowed.
- Class C is not audited for containment; it is audited for **consent and declaration** — the run
  must have recorded the consent, and the report must say the child was not isolated.

---

## 3. Consent model

Three distinct consents, all interactive, none bypassable by a flag-only invocation:

| Consent | Trigger | What is shown | Recorded as |
|---|---|---|---|
| **C1 — executable configuration** | first `integrate`/`verify` with a config digest not yet accepted | every command that will run, verbatim, with cwd and timeout | `config_accepted:<digest>` in the journal; changing any command changes the digest |
| **C2 — merge drivers** | a non-builtin `merge.<name>.driver` selected for a changed path (X-05) | driver name, command, affected paths, and the determinism consequence | `driver_consent:<name>:<digest>` + `driver_used: true` |
| **C3 — repo edits** | `init` | the exact `ilvuno.toml` content and the `.gitignore` line | the file diff itself; `init` refuses to overwrite |

Why C1 exists at all: `ilvuno.toml` is executable configuration committed to a repository. Reviewing
it is the user's job (same as `package.json` scripts), but a user who clones and runs must see what
will execute. The digest makes the consent specific to a file version, not to a habit.

Explicit non-consent: `--yes`, `--auto-approve`, `--no-verify` do **not** exist. Non-interactive
sessions may pre-accept C1 only via `--accept-config=<digest>`, which the user must obtain from a
previous interactive run.

---

## 4. What "no network" may mean

Current documents say both "no network calls" (good, about Ilvuno) and list verification children
that obviously use the network (npm install, etc.).

**Proposed wording:** *"Ilvuno makes no network requests of its own. Verification and repair
children run with the user's network access; the report states `child_network: unconstrained`."*

This matters for the platform-team persona: a compliance reviewer must not read "offline" as "the
test suite cannot exfiltrate".

---

## 5. Absolute paths in child output (MED-05)

`INV-9` says persisted artifacts contain no absolute paths; `SEC-103` made path redaction an *export*
option; the redactor's classes cover secrets only. A test command that prints `$PWD` therefore writes
a machine path into evidence.

**Proposed:** add a fourth redaction class, applied at write time like the others:

| Class | Rule | Example output |
|---|---|---|
| `path` | absolute path under the repo root → repo-relative | `/home/u/r/src/a.ts` → `src/a.ts` |
| `path` | absolute path under `$HOME` → `~/…` | `/home/u/.cache/x` → `~/.cache/x` |
| `path` | other absolute paths → `«abs-path»/<basename>` | `/opt/tool/bin/x` → `«abs-path»/x` |

Raw output remains available only through `--keep-raw`, which is already refused for bundles and
recorded in the journal. Test: a fixture command printing `$PWD` and `$HOME`, then inspect every
persisted file and one exported bundle (X-07).

---

## 6. Evidence assurance levels (MED-03)

Digests prove **internal consistency**, not authorship, execution environment, or survival of a
wholesale replacement. Three levels, used verbatim in docs, report footers and the approval UI:

| Level | Guarantee | Requires | Test |
|---|---|---|---|
| **integrity** | the bundle is internally consistent: the manifest matches the files it lists. Without a trusted external anchor this does **not** prove the bundle is unchanged since sealing — an actor can replace the bundle and recompute its manifest and digests | digests + self-contained manifest | tamper one byte without recomputing → detected; tamper and recompute the manifest → **not** detected (expected, see note) |
| **attributed** | a specific key approved this exact `(candidate_sha, evidence_digest)` | Ed25519 approval, local record, passport ref | sign, verify, rotate/revoke, copy across runs |
| **provenanced** | a trusted executor ran it in a known environment | **not offered** | — |

A stronger integrity claim requires a trusted external digest or signature — the digest recorded
somewhere the bundle cannot rewrite (approval record, passport, CI log) — or the `attributed` level.
Self-contained bundles alone cannot distinguish "unchanged" from "replaced and re-sealed".

Rules: unsigned approvals are always rendered `integrity only`; team/cross-user use requires
`attributed`; no document may use "verified" for anything above integrity unless the signature
exists. `T-0602` AC-5 already labels unsigned; this extends the labelling to the report and export.

---

## 7. Threat-model deltas

| Threat | Before | After |
|---|---|---|
| Malicious `.gitattributes` + user-configured driver | "recorded", no control | refused by default; consent shows command; run marked non-reproducible (`X-05`, §3 C2) |
| Malicious `ilvuno.toml` | reviewable, no gate | C1 consent on a digest; every command shown once, per digest |
| Path leak through logs | export-time optional | write-time class `path` |
| "Verified" overclaim in team review | note in security doc | three named levels; unsigned explicitly weaker |
| Audit test passing while claim is false | single "no writes outside" assertion | declared operation table; unexpected Git writes fail the test |
| Driver consent misread as sandbox | — | report line: `merge_driver: <name> (consented) — effects unconstrained` |

---

## 8. Test matrix (what replaces BG-3 as written)

| # | Scenario | Assertion |
|---|---|---|
| S-01 | `integrate` clean, 3 sources | effects ⊆ A ∪ B; declared Git writes match the operation table |
| S-02 | conflict at step 2 | same; no ref, no worktree; objects present and accounted for |
| S-03 | SIGKILL during `worktree add` | same after `prune`; no orphan |
| S-04 | SIGKILL during cleanup | same; `orphaned` state recorded if removal failed |
| S-05 | verification child writes `$HOME` | class C: allowed, declared in report; **no** claim of containment |
| S-05b | verification child makes an outbound request | same; `child_network: unconstrained` present |
| S-06 | `init` on a fresh repo | writes only `ilvuno.toml` + `.gitignore`, both shown (C3) |
| S-07 | custom driver, no consent | refuse; marker file **absent** |
| S-08 | custom driver, consent | marker present; `driver_used`, driver digest recorded; run excluded from reproducibility claim |
| S-09 | child prints `$PWD` / `$HOME` | no absolute path in any persisted artifact or export |
| S-10 | tampered bundle / recomputed unsigned chain / signed approval / missing evidence | documented outcome per assurance level |
| S-11 | anchored ref check | `refs/heads/refs/ilvuno/x` is **not** accepted as ours (regression test for X-04) |

S-01…S-04 instrument the process, not just inspect the filesystem: a write to any path outside the
declared set fails the test with the path and the operation that caused it.

---

## 9. Claims we may and may not make

| Allowed | Not allowed |
|---|---|
| "Ilvuno writes only its own state and the Git objects its declared operations require" | "Ilvuno never writes outside `.ilvuno/`" |
| "Verification runs your project's commands as you, with your access; it is not a sandbox" | "Verification is isolated" |
| "Bundles are integrity-verifiable offline" | "Bundles prove who ran what" |
| "Approvals are attributed when signed" | "Approvals are authenticated" (without keys) |
| "Custom merge drivers are refused unless you accept them explicitly" | "Merges are deterministic" (without the fingerprint caveat) |
| "No Ilvuno service calls are made" | "Ilvuno works offline" (children may not) |

The banned-claims test (`T-0305`, `VER-101`) gains these strings as forbidden
(`never writes outside`, `is isolated`, `proves`, `fully offline`).

---

## 10. Approval dependencies

| Change | Decision | Why the owner must decide |
|---|---|---|
| Four effect classes replacing one invariant | **D-07** | Narrows a published security claim; affects the product's trust story |
| Merge-driver refusal default | **D-04** | Changes whether a user's merges behave as they configured |
| Consent gates C1–C3 | **D-07** | Adds friction to first use; must be accepted as a product trade-off |
| Path redaction class + narrowing of `INV-9` | **D-05** | Changes evidence content; affects debugging |
| Assurance-level vocabulary | **D-11** | Claim policy; affects marketing and team sales |
