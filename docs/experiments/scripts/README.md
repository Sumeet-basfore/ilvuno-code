# SP-1R2 harness scripts (throwaway)

Remediated experiment harness for SP-1. Not production code. **It does not accept
caller-supplied repositories**: every repository is created by the invocation that uses it.

## Commands

```bash
./sp1.sh fixtures        # 10 fixtures under the run-owned scratch root
./sp1.sh matrix          # 28 asserted cases; exits 1 on any missing/mismatched row
./sp1.sh oracle          # writes oracle.json from direct, Git-native merges
./sp1.sh matrix --update-expected   # rebuild expected.json from spec+oracle; refuses on disagreement
./sp1.sh probes          # P1..P5 characterization (informational)
./sp1.sh run F1-two-clean object plain
./tests.sh               # 95 negative/security/evidence assertions
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

```bash
docker build -f Dockerfile.git238 -t ilvuno-git238r .
docker run --rm -u "$(id -u)":"$(id -g)" -v "$PWD":/sp1r -w /sp1r \
  -e SP1_GIT_BIN=/usr/local/bin/git -e SP1_SCRATCH=/tmp/sp1-c \
  ilvuno-git238r bash -lc 'mkdir -m700 -p "$SP1_SCRATCH"; ./sp1.sh matrix && ./tests.sh'
```

`SP1_SCRATCH` must be outside the mount. Run as your own UID; ownership checks and the
permission-failure probe depend on it.

The image pins the Alpine base digest and the Git source tarball SHA-256 (see
`sp1r-reproducibility.md`).

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
