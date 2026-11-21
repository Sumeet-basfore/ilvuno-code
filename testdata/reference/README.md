# SP-1R3 reference harness (testdata/reference)

Reconciled against `docs/experiments/scripts/README.md` on `origin/main` (T-0005). The text below
keeps that README, with the adaptations listed in **Adaptations in this copy**. Those changes are
the only differences; the harness is the reference oracle for sequential two-way merges.

Remediated experiment harness for SP-1. Not production code. **It does not accept
caller-supplied repositories**: every repository is created by the invocation that uses it.

This directory is the **reference behaviour** for sequential two-way merges. It is not
`merge-tree` equivalence, and nothing here is compiled into the `ilvuno` binary.

## Entry points (T-0005)

```bash
make reference-matrix    # check-reference.sh matrix on a fresh private root (R3-1)
make reference-tests     # negative tests AC-9 to AC-13, then tests.sh (95 assertions)
bash testdata/reference/check-reference-owner.sh   # AC-13 wrong-owner test: needs Docker and the pinned image; exits 2 (NOT RUN) otherwise
```

The wrong-owner case is not in `check-reference-test.sh`. That suite prints an explicit NOT RUN line for it, and its
counts exclude it.

`check-reference.sh oracle|matrix|tests` is the wrapper that enforces R3-1 to R3-3. It needs
`TMPDIR` set to an absolute, canonical, existing, mode-700 directory, owned by you and outside the
work tree. It copies this directory into a private root and runs the harness there.

**Do not run `./sp1.sh oracle` in this directory.** It would overwrite the tracked `oracle.json`
(R3-3). Use `check-reference.sh oracle`, which regenerates the oracle in a fresh copy and compares it
with the tracked file.

## Commands

Run from this directory, with `SP1_SCRATCH` set to a private directory outside the work tree.

```bash
./sp1.sh fixtures        # 10 fixtures under the run-owned scratch root
./sp1.sh matrix          # 28 asserted cases; exits 1 on any missing/mismatched row
./sp1.sh probes          # P1..P5 characterization (informational)
./sp1.sh run F1-two-clean object plain
./tests.sh               # 95 negative/security/evidence assertions
# ./sp1.sh oracle        # do not run here: use check-reference.sh oracle (see above)
# ./sp1.sh matrix --update-expected   # rebuild expected.json from spec+oracle; refuses on disagreement
```

Case count: **10 fixtures × 2 approaches = 20 plain rows**, plus **4 variant pairs × 2 = 8 rows**,
total **28**. `spec.json` declares 28; `expected.py --update` refuses any other count.
Probes are separate from the matrix and are not counted.

## Scratch root

`SP1_SCRATCH` is a **parent**, never a data directory. Each invocation creates a new exclusive
child `run-<32 hex>` with a `.sp1r-owner` marker (created with `O_EXCL`; an existing marker is
never overwritten). Without `SP1_SCRATCH`, a `mktemp -d` root is used.

* The parent's existing contents are never read as scratch and never deleted.
* Only the run-owned child can be deleted, and only through `safe_rmrf`.
* `SP1_CLEAN=1` removes the child at exit; default is to keep it for inspection.

Threat model: **one harness process per fresh private mode-700 root, no concurrent same-UID
adversary.** `safe_rmrf` validates the path, then deletes; the window between those steps is not
atomic. Path validation alone is **not** TOCTOU-safe. Production hostile-filesystem cleanup needs
descriptor-relative or OS-level isolation and is a later security gate.

## Controlled Git execution

All Git calls go through `gitx`/`sp1_env`:

```
env -i PATH HOME TMPDIR XDG_CONFIG_HOME LANG=C LC_ALL=C TZ=UTC
       GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
       GIT_ATTR_NOSYSTEM=1 GIT_TERMINAL_PROMPT=0 GIT_CONFIG_COUNT=0
       fixed author/committer identity and dates
git -c core.hooksPath=<empty> -c core.attributesFile=/dev/null -c core.fsmonitor=false
    -c commit.gpgsign=false -c submodule.recurse=false ...
```

The fixture base commits use the identity `fixture <fixture@example.com>` and the date
`2020-01-01T00:00:00Z`. Integration commits made by the harness use `ilvuno <ilvuno@localhost>`.
The two are intentionally different: the base identity reproduces the oracle's base SHAs (T-0002 AC-7).

Before any write, and again before every merge step, a repository that configures an executable
feature is **refused** (verdict `harness_refused`, exit 4): `filter.*.smudge|clean|process`,
`merge.default`, non-builtin `merge.*.driver`, `diff.*.command|textconv`, enabled
`core.fsmonitor`. Detection is config-based, independent of attribute files.

**Not a sandbox.** A program that runs (in production, or a driver in probe P4) is not contained.

## Fixture manifest

`.git/sp1-meta.env` is parsed without shell evaluation:

```
FORMAT=1
BASE=<40 or 64 hex>
SRCS=<oid> <oid> [<oid>]
CHECK_SHA256=<64 hex>      # optional
```

Unknown keys, duplicate keys, short OIDs, duplicate sources, a source equal to the base, and
malformed lines are rejected. No `source`/`.` is used on repository data anywhere.

## Docker: Git 2.38.1 floor

The image pins the Alpine base digest and the Git tarball SHA-256 in `Dockerfile.git238`.
CI builds it from an empty context; the wrapper and tests run inside it:

```bash
docker build --no-cache -f testdata/reference/Dockerfile.git238 -t ilvuno-git238r <empty-directory>
docker run --rm -u "$(id -u)":"$(id -g)" -v "$PWD":/src:ro -v "$SCRATCH":/scratch -w /src \
  -e TMPDIR=/scratch -e SP1_GIT_BIN=/usr/local/bin/git --entrypoint bash ilvuno-git238r -c \
  'set -euo pipefail; bash testdata/reference/check-reference-test.sh; testdata/reference/check-reference.sh matrix; testdata/reference/check-reference.sh tests'
```

`SCRATCH` is a private mode-700 directory outside the repository, and it is mounted read-write.
Run as your own UID; ownership checks depend on it.

## Fixtures

| ID | Shape | Expected |
|---|---|---|
| F1 | 2 independent clean changes | ok, exit 0 |
| F2 | 3 independent clean changes | ok, exit 0 |
| F3 | both edit the same line | conflict, step 2 |
| F4 | clean step 1, conflict step 2 | conflict, step 2 |
| F5 | one source already in the base | skip, exit 0 |
| F6 | orphan history | unrelated_histories |
| F7 | check fails only in combination | check_failed, exit 1 |
| F8 | custom merge driver configured | harness_refused, exit 4 |
| F9 | cleanup-failure injection | exit 5, orphan reported |
| F10 | conflict on `line<LF>break.txt` | byte-exact path and blob hash |

## Expectations

`expected.json` is generated from `spec.json` (hand-declared verdicts/exit/cleanup) plus
`oracle.json` (independent Git-native candidate, tree, content, conflict step/paths). It is never
derived from the pipeline being tested. `--update` validates the observed run strictly and refuses
if it disagrees with the oracle.

`tests.sh` (P1.4) asserts that the matrix ran on a supported Git (2.55.0 or 2.38.1). It therefore
fails by design on any other Git, so run it only on those versions. CI runs it in the pinned image.

## Adaptations in this copy (T-0005)

| Change | Where | Why |
|---|---|---|
| Location: `docs/experiments/scripts/` to `testdata/reference/` | the whole directory | the approved T-0005 scope |
| Historical-commands note removed; commands refer to this directory | this README | the note said the harness was absent from the checkout |
| C-5: `/tmp/r3-cl` to `$T/r3-cl` (3 lines) | `tests.sh` | approved; `docs/research/09-git-floor.md` §5 |
| `check-reference.sh`, `check-reference-test.sh` | new | the R3-1 to R3-3 wrapper and its AC-9 to AC-13 tests |
| No `--` after an operand (`chmod`, `stat`) | wrapper, `lib.sh` | BSD `getopt` reads that `--` as a file name (macOS failure) |
| SHA-256 through `sha256sum`, else `shasum -a 256` | `lib.sh`, `sp1.sh`, `tests.sh`, `check-reference-test.sh` | macOS has no `sha256sum` |
| In-place edits without `sed -i` (helper `sed_inplace`) | `sp1.sh`, `tests.sh` | BSD `sed -i` takes the next argument as a backup suffix |
| `stat -c` with a BSD `stat -f` fallback; no `realpath -m`; `rm --one-file-system` only where GNU `rm` is | `lib.sh`, `sp1.sh` | GNU-only options |
| No `mapfile -d` (`read_nul` is a read loop) | `lib.sh` | bash 3.2, the macOS default, has no `mapfile -d` |
| `read_nul` fails on a missing file (was silent); runners require their output directory | `lib.sh`, `sp1.sh` | a silent empty list hid missing output; the P4 test had relied on it |
| P4 creates its output directories (`mkout`) | `tests.sh` | the test wrote into directories it never made |
| `tests.sh` removes its `sp1r2-victim-<pid>` fixture on exit | `tests.sh` | R3-4 residue; the fixture is this process's own |

The portability changes keep the output unchanged: the oracle regenerated in a fresh copy is
byte-identical to the tracked `oracle.json` under both the GNU userland and the BSD-semantics shims
(`docs/research/09-git-floor.md` §9).
