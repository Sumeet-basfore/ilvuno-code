# SP-1R3 reference: M0 wrapper (T-0005)


`check-reference.sh` runs this harness under operational conditions R3-1 to R3-4 (`docs/tasks/T-0005.md`).
It never modifies the tracked files in this directory.

```bash
make reference-matrix   # matrix in a fresh private copy; fails on any nonzero step
make reference-tests    # negative tests (check-reference-test.sh), then tests.sh
```

Pinned Git 2.38.1 image (the Dockerfile copies nothing, so any empty context works):

```bash
docker build --no-cache -f testdata/reference/Dockerfile.git238 -t ilvuno-git238r:<tag> <empty-context>
docker run --rm -u "$(id -u):$(id -g)" -v "$PWD:/src:ro" -v "$SCRATCH:/scratch" -w /src \
  -e TMPDIR=/scratch -e SP1_GIT_BIN=/usr/local/bin/git <tag> \
  bash -c 'bash testdata/reference/check-reference-test.sh && testdata/reference/check-reference.sh matrix && testdata/reference/check-reference.sh tests'
```

`$SCRATCH` must be a private, canonical, mode-700 directory that the caller owns and that lies outside `$PWD`.

Local change to the copied harness (approved as C-5, 2026-11-20): `tests.sh` lines 398, 399 and 401 use
`$T/r3-cl` instead of the hard-coded `/tmp/r3-cl`. No assertion changed. See `docs/research/09-git-floor.md` §5.
