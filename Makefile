# Ilvuno M0 entry points at the T-0002 stage. Scope: docs/tasks/T-0001.md and T-0002.md.
# forge-verify is added by T-0002. fixtures and safety join it with T-0003/T-0004;
# reference-matrix and reference-tests are added by T-0005.

GO     ?= go
BIN    ?= ilvuno

# Tests and builds never read the developer's global or system git configuration.
export GIT_CONFIG_GLOBAL := /dev/null
export GIT_CONFIG_NOSYSTEM := 1

.PHONY: build test lint docs-check forge-verify bench gate

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

# Cited paths that a later task creates. Each is tolerated only while absent. The T-0005 commit
# removes its entry, and the check is strict from then on. Any other missing path fails.
DOCS_CHECK_DEFERRED := docs/research/09-git-floor.md

# Every docs/ path cited by the M0 authority documents must exist.
docs-check:
	@fail=0; \
	for f in $$(grep -rhoE 'docs/[A-Za-z0-9_./-]+\.(md|txt|json|sh|go|py|yml)' \
	    docs/tasks/T-0001.md docs/tasks/T-0002.md docs/tasks/T-0003.md docs/tasks/T-0004.md \
	    docs/tasks/T-0005.md docs/readiness/06-m0-readiness-checklist.md \
	    docs/readiness/07-m0-reconciliation.md AGENTS.md CLAUDE.md README.md | grep -v "T-NNNN" | sort -u); do \
	  if [ ! -e "$$f" ]; then \
	    case " $(DOCS_CHECK_DEFERRED) " in \
	      *" $$f "*) echo "docs-check: deferred until its creating task lands: $$f";; \
	      *) echo "docs-check: missing $$f"; fail=1;; \
	    esac; \
	  fi; \
	done; \
	exit $$fail

# T-0002 (forge): the deterministic repository forge and its self-tests. T-0003 and T-0004 add
# their packages to this list when they land.
forge-verify:
	$(GO) test -count=1 -v ./internal/testkit/forge/...

bench:
	@echo "bench: not implemented in this build (lands with T-0305)"; exit 2

# Gate runners are not implemented in M0. `make gate G0` exits 2 rather than passing.
gate:
	@echo "gate: not implemented in this build (G0 runner is outside T-0001..T-0005)"; exit 2
