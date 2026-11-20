# M0 independent review request — revision 4

Date: 2026-11-20 · Requested by: engineering (author) · Status: **request only.** This file approves
nothing. It does not authorize M0, and the author does not self-approve E-8 or E-9.

Prior verdict: `docs/reviews/m0-final-entry-review.md` (M0 NOT READY; E-3, E-8, E-9 OPEN). That file is
not edited. This request covers the corrections made against it.

## What changed since that verdict

1. **D-16 — M0 lint policy.** `gofmt`, `go vet` and the in-repository import-rule test only.
   `staticcheck` is deferred until separately approved as a dependency; `golangci-lint` is not used.
   Recorded in `docs/decisions/open-questions.md` §0, with the exception written into `AGENTS.md` §4.1.
2. **D-17 — canonical repository and module path.**
   `https://github.com/Sumeet-basfore/ilvuno-code`, module path
   `github.com/Sumeet-basfore/ilvuno-code`. No other path is approved.
3. **D-18 — Apache-2.0 text approved.** The exact draft
   `docs/readiness/LICENSE-draft-Apache-2.0.txt` (sha256
   `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`, 11,358 bytes). T-0001 copies
   it verbatim to root `LICENSE`; no copyright line is added.
4. **T-0001 corrected** for E-9: module path fixed, lint aligned with D-16, Linux/macOS dependency
   checks, T-0001/CI ownership split, stub criteria removed.
5. **T-0005 corrected** for E-8: R3-1…R3-4 conditions, fresh temporary oracle copy, checked exit codes
   and no stale-output read, exact Git versions and image digest, narrowed `merge-tree` check,
   T-0005-owned reference CI jobs.
6. **E-3 moved to PASS** on the documented evidence (text approved). The root `LICENSE` still does not
   exist; T-0001 creates it. E-8 and E-9 remain **OPEN** pending this review.

## Review scope — answer each with PASS/OPEN and evidence

| # | Item | What to check |
|---|---|---|
| 1 | **E-3** | D-18 records approval of the exact draft; sha256 and byte count match the file; the approval does not claim the root `LICENSE` exists. |
| 2 | **E-8** | `T-0005` uses R3-1…R3-4 (no `C-1…C-9` condition references); oracle runs in a fresh copy with the copied `oracle.json` removed first; every command's exit status is checked; Git 2.55.0/2.38.1 and the full image ID are pinned; AC-5 is scoped to production paths (`cmd`, `internal`, `Makefile`, `.github`) and cannot fail on historical docs or `testdata/reference/`. |
| 3 | **E-9** | `T-0001` uses `github.com/Sumeet-basfore/ilvuno-code`; `make lint` is exactly `gofmt` + `go vet` + import-rule test; Linux `ldd` and macOS `otool -L` checks are present; T-0001 creates the initial workflow with no reference job; no package-stub criteria remain. |
| 4 | **Lint consistency** | `AGENTS.md` §4.1, D-16, `T-0001` step 5 and `06` §7 state the same scope; `staticcheck` is preserved as a future requirement and is not silently dropped. |
| 5 | **Module path** | D-17, `T-0001` step 1/AC-7/checklist and `06` §7 agree; no `github.com/<org>/ilvuno` placeholder remains as an approved value. |
| 6 | **SP-1R3 conditions and CI ownership** | R3-1…R3-4 match the verdict's conditions; T-0001 owns the initial workflow, T-0005 owns the reference jobs; S-4 workflow review is preserved. |
| 7 | **Authorization** | `06` §7 is unsigned; the request does not claim M0 is authorized; no production code, `go.mod`, `LICENSE` or workflow exists. |

## Files to read

```
AGENTS.md §4.1
docs/decisions/open-questions.md §0
docs/readiness/05-owner-decision-register.md (status line)
docs/readiness/06-m0-readiness-checklist.md
docs/readiness/07-m0-reconciliation.md
docs/tasks/T-0001.md
docs/tasks/T-0005.md
docs/readiness/LICENSE-draft-Apache-2.0.txt
docs/reviews/m0-final-entry-review.md (prior verdict, unchanged)
```

## Read-only checks

```bash
git status --short
sha256sum docs/readiness/LICENSE-draft-Apache-2.0.txt   # expect cfc7749b…
git grep -n "D-16\|D-17\|D-18" -- docs AGENTS.md
git grep -n "merge-tree" -- cmd internal Makefile .github  # expect no matches once those paths exist
git grep -n "C-1…C-9\|C-1 to C-4" -- docs/tasks docs/readiness  # expect only historical labels
```

The third `git grep` has no matching files before T-0001 creates them; that is expected and is not a
pass. Do not run the harness's `oracle` or `matrix` targets as evidence for this review; their
M0 copy does not exist yet (`docs/tasks/T-0005.md` step 1).

## Do not assume

- No code, `go.mod`, `LICENSE`, `testdata/reference/` or `.github/workflows/ci.yml` exists.
- `06` §7 and `05` §8 are unsigned. M0 is not authorized.
- The author's statements are not evidence; check the files and the recorded hashes.
- `docs/experiments/` is a historical record and was not changed in this revision.

## Requested output

A gate-by-gate verdict (E-3, E-8, E-9, plus items 4–7), each with the file and line supporting it,
a list of remaining blockers to authorization, and any correction required before the owner signs
`06` §7. Marking E-8 or E-9 PASS is the reviewer's decision, not the author's.
