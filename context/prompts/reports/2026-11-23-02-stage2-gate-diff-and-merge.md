# Stage 2-C · gate diff and merge

## Changed

- Added per-thread diff API, branch/commit/stat in run detail, and a gate with folded patches.
- Accept now refuses dirty projects, merges checked-out branches in task order, journals each result, and stops on conflicts. Repeated accept is safe. Folder mode still records acceptance only.
- Excluded Claude's stderr capture from thread commits. Commits: `e430433` (code), `fde5a52` (project record).

## Verified

- `cargo test --workspace`: 149 passed, 3 live-provider tests ignored. `node --check crates/desktop/ui/app.js` passed.
- Five scripted API cases cover landing, conflict, dirty refusal, repeat accept, and folder mode. Patch cap and adapter-log exclusion have runner tests.
- Real run `ae186839-1120-4401-a1fc-77225af4bb11` used 62,623 tokens. Before accept, the throwaway project had no `hello.txt`; after accept, it had the file and merge commit `010723c` named the run id.
- Read the real 1440×900 gate screenshot: `/tmp/ilvuno-stage2-live.HZtImU/gate-patch.png`.

## Could not verify

- Did not run `tools/shot.sh`: it would kill the already-running desktop process (`./target/debug/desktop`, PID 1144781). Captured the real Tauri window on a separate display instead.
- `cargo fmt` was unavailable because Rust 1.100.0 has no `rustfmt` component installed. No toolchain component was installed.
- An initial live attempt was blocked after the orchestrator gave the worker an absolute project path and the 120,000-token budget was exceeded. The corrected one-worker run above passed.

## Open

- Two concurrent runs on the same project are still not serialized.
- The stage 3 UI truth work and dogfooding remain.

## Result

Stage 2-C is complete on this branch. No changes landed in the Ilvuno repository itself.
