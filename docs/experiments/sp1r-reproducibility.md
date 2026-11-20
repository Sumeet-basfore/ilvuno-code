# SP-1R — Reproducibility

What was run, on what, and what the recorded evidence can and cannot support.

---

## 1. Environments

| | Host | Container |
|---|---|---|
| Device | physical workstation | Docker |
| OS | CachyOS Linux, kernel 7.1.8-1-cachyos, x86_64 | Alpine Linux 3.17, x86_64, musl |
| Git | **2.55.0** (`/usr/bin/git`) | **2.38.1**, built from source |
| Python | 3.14.7 | 3.x (Alpine package) |
| User | uid 1000 (non-root) | uid 1000 via `-u`, non-root |

macOS, Windows, and any Git version other than the two above: **not tested**.

## 2. Container provenance

| Item | Value |
|---|---|
| Base image | `alpine:3.17@sha256:8fc3dacfb6d69da8d44e42390de777e48577085db99aa4e4af35f483eb08b989` |
| Git source | `https://www.kernel.org/pub/software/scm/git/git-2.38.1.tar.xz` |
| Source SHA-256 | `97ddf8ea58a2b9e0fbc2508e245028ca75911bd38d1551616b148c1aa5740ad9` |
| Build flags | `NO_TCLTK=1 NO_GETTEXT=1 NO_DOCS=1 NO_PERL=1 NO_PYTHON=1 NO_REGEX=YesPlease` |
| Built image ID | `sha256:14c2f3df639d947f47e8af64b59efcb0855d4aff1090f4a53debe790b230d394` |

The Dockerfile verifies the tarball checksum with `sha256sum -c` before building. The checksum was
recorded from the download used for these results; it was **not** verified against an upstream
signature, and the build is not bit-for-bit reproducible across machines (compiler, locale, and
timestamps vary). Treat the image as "built from a pinned input", not as "attested".

`NO_REGEX=YesPlease` is required because musl lacks `REG_STARTEND`; without it the build fails.

## 3. What every result record contains

Each matrix row is one JSON object with:

| Field | Meaning |
|---|---|
| `fixture`, `approach`, `variant` | case identity |
| `base`, `sources` | pinned base SHA and ordered source SHAs actually consumed |
| `verdict`, `exit` | outcome class and process exit |
| `conflict_step`, `conflict_paths` | machine-derived step index and NUL-parsed paths |
| `candidate`, `tree` | produced commit and tree (or empty) |
| `content` | `path=sha256` for every file in the candidate |
| `check_rc`, `cleanup`, `operational_detail` | verification and lifecycle outcome |
| `refs_left`, `worktrees_left` | residue after the run |
| `user_state_unchanged` | fingerprint over refs, config, hooks, stashes, HEAD, index, status, worktrees |
| `drivers_ran` | marker file produced by a configured merge driver |
| `git_version`, `os`, `merge_policy`, `metadata` | environment and policy actually used |

`expected.json` stores the reviewed expectation for the same fields; the checker compares both
directions (missing and extra rows).

## 4. Reproducibility evidence

| Observation | Result |
|---|---|
| Matrix re-run from a wiped scratch root | 28/28 identical; `expected.json` unchanged between runs |
| Candidate SHA, A vs B, same version | identical on every clean case |
| Candidate SHA, 2.55.0 vs 2.38.1 | identical on every clean case (F1, F2, F5, F7, F9) |
| Candidate content hashes | identical across approaches and versions |
| Conflict step and paths | identical across approaches and versions (F3, F4, F10) |
| Cleanup states | identical across versions |
| Test suite | 68/68 on both versions (SP-1R2) |
| Failing Git binary | matrix exits 1 with missing rows on both versions |

Scope of the determinism statement:

> Within the two recorded environments, with the fixed identity/date envelope and the recorded
> merge policy, the same pinned inputs produce the same candidate commit, tree and content.

Not claimed: equivalence across other Git versions, OSes, locales, filter/LFS configurations,
submodule content, case-insensitive filesystems, or repositories with `replace` refs, grafts,
alternates, or shallow history. The earlier SP-1 "8 clean cases" claim is retained but is now
enforced by a test rather than narrated.

## 5. Reproduce

```bash
cd docs/experiments/scripts
mkdir -m700 /tmp/sp1-scratch
SP1_SCRATCH=/tmp/sp1-scratch ./sp1.sh fixtures
SP1_SCRATCH=/tmp/sp1-scratch ./sp1.sh matrix      # asserts against expected.json
SP1_SCRATCH=/tmp/sp1-scratch ./sp1.sh probes      # P1..P5 characterization
SP1_SCRATCH=/tmp/sp1-scratch ./tests.sh           # 38 negative/security assertions
```

Floor version:

```bash
docker build -f Dockerfile.git238 -t ilvuno-git238r .
docker run --rm -u "$(id -u)":"$(id -g)" -v "$PWD":/sp1r -w /sp1r \
  -e SP1_GIT_BIN=/usr/local/bin/git -e SP1_SCRATCH=/tmp/sp1-c24836 \
  ilvuno-git238r bash -lc 'mkdir -m700 -p "$SP1_SCRATCH"; ./sp1.sh matrix && ./tests.sh'
```

Expected tail:

```
PASS: 28 cases, all fields match expectations
...
68 passed, 0 failed
```

## 6. Updating expectations

```bash
SP1_SCRATCH=/tmp/sp1-scratch ./sp1.sh matrix --update-expected
git diff --stat docs/experiments/scripts/expected.json
git diff docs/experiments/scripts/expected.json   # review every changed field
```

`expected.json` is a reviewed artefact. A change to it means a behaviour change: a new verdict, a
different conflict step, a different SHA, or a different cleanup state. Updating it without reading
the diff converts the matrix back into a recorder.

The fixture builders are deterministic (fixed identity, dates, `TZ=UTC`, empty global/system
config), so rebuilding a fixture yields the same base and source SHAs. That is why `base` and
`sources` can be asserted as constants.

## 7. Limits of this record

1. The **reports are not the artifacts** — raw `all.jsonl` lives in the scratch root during a run and
   is not committed. `expected.json` and the checker are committed; a reviewer re-runs the matrix
   rather than trusting a transcript.
2. The host is a rolling-release distribution; a future Git update changes `git_version` and may
   legitimately change results. Both would appear as a diff, by design.
3. The container image is **not** attested; only its inputs are pinned.
4. Timing is not asserted. Only outcomes are. This avoids flaky gates but means performance
   regressions are not caught here.
5. Two Git versions are a floor check, not a support matrix.
