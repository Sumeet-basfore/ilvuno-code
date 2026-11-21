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

Local changes to the copied harness. The C-5 change (approved, 2026-11-20) makes `tests.sh` use `$T/r3-cl`
instead of `/tmp/r3-cl`, on three lines; see `docs/research/09-git-floor.md` §5. T-0005 also made portability
and fail-loud changes so that the same harness runs on macOS (BSD userland, bash 3.2). Each one, with its
reason, is in the table in `README.md` ("Adaptations in this copy"). No assertion was weakened. The
negative-test and matrix results are in `docs/research/09-git-floor.md` §9.

Exit status and output: the wrapper exits 0 only when every step succeeded and the comparison matched. It
prints its `PASS` line from its exit handler, and only when cleanup also succeeded. A cleanup failure after a
passing run therefore exits nonzero with no `PASS` line. Harness-internal output from `sp1.sh` and `tests.sh`
still appears before any later cleanup failure, because the wrapper does not own that output.

CI runs `check-reference-test.sh` on the runner's own Git (negative tests only). It runs `matrix` and `tests`
only in the pinned Git 2.38.1 image, because `tests.sh` P1.4 asserts a supported Git version.
