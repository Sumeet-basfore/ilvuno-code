# Ilvuno

Ilvuno is planned as a standalone command-line tool. It takes independently produced agent changes,
checks whether they work together, and records evidence that a reviewer can inspect.

> **Not usable yet.** This repository currently contains M0 test infrastructure only. No `ilvuno`
> command performs integration, verification or evidence recording yet.

## Non-goals (V1)

- No graphical or web user interface, and no desktop shell.
- No background daemon and no hosted service.
- No Windows support in V1.
- No dispatch or control of coding agents.
- No telemetry by default.

## Build and test (M0)

Requires Go 1.27.0 and Git 2.38.1 or later.

    make build    # CGO_ENABLED=0 binary ./ilvuno
    make test
    make lint     # gofmt, go vet, import-rule test
    make docs-check

Governing documents: `AGENTS.md`, `docs/planning/01-master-roadmap.md`, `docs/readiness/06-m0-readiness-checklist.md`.
