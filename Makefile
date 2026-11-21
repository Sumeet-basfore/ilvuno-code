# Ilvuno M0 entry points at the T-0003 stage. Scope: docs/tasks/T-0001.md through T-0005.md.
# forge-verify (T-0002) covers the forge, the fixture corpus (T-0003) and the safety package (T-0004).
# reference-matrix and reference-tests (T-0005) run the SP-1R3 reference harness.

GO     ?= go
BIN    ?= ilvuno

# Tests and builds never read the developer's global or system git configuration.
export GIT_CONFIG_GLOBAL := /dev/null
export GIT_CONFIG_NOSYSTEM := 1

.PHONY: build test lint docs-check forge-verify reference-matrix reference-tests bench gate

build:
	CGO_ENABLED=0 $(GO) build -trimpath -o $(BIN) ./cmd/ilvuno

test:
	$(GO) test -count=1 ./...

# D-16: gofmt, go vet and the in-repository import-rule test only. No staticcheck, no golangci-lint.
lint:
	@files="$$(git ls-files --cached --others --exclude-standard -- '*.go')"; \
	bad="$$(gofmt -l $$files)"; \
	if [ -n "$$bad" ]; then echo "gofmt: these files need formatting:"; echo "$$bad"; exit 1; fi
	$(GO) vet ./...
	$(GO) test -count=1 ./internal/lint/importrule/...

# Strict (T-0005): every docs/ path cited by the M0 authority documents, and every testdata/reference path
# cited by T-0005 or the Git-floor note, must exist. There is no deferral list.
docs-check:
	@fail=0; \
	for f in $$(grep -rhoE 'docs/[A-Za-z0-9_./-]+\.(md|txt|json|sh|go|py|yml)' \
	    docs/tasks/T-0001.md docs/tasks/T-0002.md docs/tasks/T-0003.md docs/tasks/T-0004.md \
	    docs/tasks/T-0005.md docs/readiness/06-m0-readiness-checklist.md \
	    docs/readiness/07-m0-reconciliation.md AGENTS.md CLAUDE.md README.md | grep -v "T-NNNN" | sort -u) \
	  $$(grep -rhoE 'testdata/reference/[A-Za-z0-9_./-]+\.(md|txt|json|sh|py)' \
	    docs/tasks/T-0005.md docs/research/09-git-floor.md | sort -u); do \
	  if [ ! -e "$$f" ]; then echo "docs-check: missing $$f"; fail=1; fi; \
	done; \
	exit $$fail

# T-0002 (forge), T-0003 (fixture corpus, including the oracle cross-check) and T-0004 (safety fingerprint),
# with their self-tests.
forge-verify:
	$(GO) test -count=1 -v ./internal/testkit/forge/... ./internal/testkit/fixtures/... ./internal/testkit/safety/...

# T-0005 (R3-1): each run gets a fresh private base, created here and removed here, and the exit status of
# the wrapper is preserved. The wrapper refuses any other base. Both targets need a POSIX shell and Git.
reference-matrix:
	@scratch=$$(mktemp -d) && scratch=$$(cd -P "$$scratch" && pwd -P) && chmod 700 "$$scratch" && \
	rc=0; TMPDIR="$$scratch" testdata/reference/check-reference.sh matrix || rc=$$?; \
	rm -rf "$$scratch"; exit $$rc

# The negative tests (AC-9 to AC-13) run on any host with Git and a SHA-256 tool. tests.sh (P1.4) asserts that
# the run used a supported Git (2.55.0 or 2.38.1), so it fails by design on any other Git. CI therefore runs
# tests.sh only in the pinned Git 2.38.1 image, and runs the negative tests alone on the runner's own Git.
reference-tests:
	@scratch=$$(mktemp -d) && scratch=$$(cd -P "$$scratch" && pwd -P) && chmod 700 "$$scratch" && \
	rc=0; TMPDIR="$$scratch" bash testdata/reference/check-reference-test.sh || rc=$$?; \
	if [ $$rc -eq 0 ]; then TMPDIR="$$scratch" testdata/reference/check-reference.sh tests || rc=$$?; fi; \
	rm -rf "$$scratch"; exit $$rc

bench:
	@echo "bench: not implemented in this build (lands with T-0305)"; exit 2

# Gate runners are not implemented in M0. `make gate G0` exits 2 rather than passing.
gate:
	@echo "gate: not implemented in this build (G0 runner is outside T-0001..T-0005)"; exit 2
