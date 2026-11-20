# Ilvuno M0 entry points. Scope: docs/tasks/T-0001.md, T-0002 (forge-verify), T-0005 (reference-*).

GO     ?= go
BIN    ?= ilvuno

# Tests and builds never read the developer's global or system git configuration.
export GIT_CONFIG_GLOBAL := /dev/null
export GIT_CONFIG_NOSYSTEM := 1

.PHONY: build test lint docs-check forge-verify reference-matrix reference-tests bench gate G0

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

# Every docs/ path cited by the M0 authority documents must exist.
docs-check:
	@fail=0; \
	for f in $$(grep -rhoE 'docs/[A-Za-z0-9_./-]+\.(md|txt|json|sh|go|py|yml)' \
	    docs/tasks/T-0001.md docs/tasks/T-0002.md docs/tasks/T-0003.md docs/tasks/T-0004.md \
	    docs/tasks/T-0005.md docs/readiness/06-m0-readiness-checklist.md \
	    docs/readiness/07-m0-reconciliation.md AGENTS.md CLAUDE.md README.md | grep -v "T-NNNN" | sort -u); do \
	  if [ ! -e "$$f" ]; then echo "docs-check: missing $$f"; fail=1; fi; \
	done; \
	exit $$fail

# T-0002: forge self-tests, and the fixture corpus that is built on the forge (T-0003).
forge-verify:
	$(GO) test -count=1 -v ./internal/testkit/forge/... ./internal/testkit/fixtures/... ./internal/testkit/safety/...

# T-0005 (R3-1): each run gets a fresh private base, created here and removed here, and the
# exit status of the wrapper is preserved. The wrapper refuses any other base.
reference-matrix:
	@scratch=$$(mktemp -d) && scratch=$$(cd -P "$$scratch" && pwd -P) && chmod 700 "$$scratch" && \
	rc=0; TMPDIR="$$scratch" testdata/reference/check-reference.sh matrix || rc=$$?; \
	rm -rf "$$scratch"; exit $$rc

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

G0:
	@:
