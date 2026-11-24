# 07 · Land and package · 2026-11-24

## Goal

Prepare the packaging branch and the two review branches for PRs. Prompt 07 stopped when the AppImage
opened to a blank window.

## What changed

- Commit `d4895f6` adds Linux package targets, bundled `ilvuno` and `ilvunod`, runner startup, icons, and
  `docs/install.md`.
- The working tree was clean after that commit. This report and the appended context updates are the only
  uncommitted files.

## Branch table

| Branch | Relevant commits and branch age at inspection | Recorded test result | PR | Human decision or next step |
|---|---|---|---|---|
| `feat/linux-packaging` | `d4895f6` adds the packaging code. | Desktop release build passed. Tauri produced both packages. The AppImage API returned the real run, but its window capture was blank. The workspace test command's exit result was not recovered. | None | Fix and verify the blank AppImage window before opening a PR. |
| `feat/truth-in-the-ui` | `d3e1f68`, `7617f3f`; 2 ahead and 8 behind `main`. | Its report records 163 passed, 3 ignored, 0 failed, plus real restart recovery and screenshots. | None | Bring it up to date with `main` before opening a PR. |
| `audit/verify-and-harden` | `499c4b6`, `ab708a6`, `cc17054`, `7952862`; 4 ahead and 22 behind `main`. | Its report records 141 passed, 3 ignored, 0 failed, plus screenshot and contrast checks. | None | Resolve the report's `Cargo.toml` license metadata mismatch before opening a PR. |

## Verified

- The AppImage was copied to `/tmp` and launched there. Its bundled API returned run
  `64239160-0caa-456a-b3cf-3011a0f64bb1`.
- The 1440×900 AppImage screenshot was blank. Prompt 07 requires stopping on this defect.
- `main` and `origin/main` point to `cf18e1f`. The open PR list was empty.
- The `feat/truth-in-the-ui` and `audit/verify-and-harden` branches already match their remote branches.

## Could not verify and still open

- I did not relaunch the AppImage, extract or launch the `.deb`, or run the install-guide commands after the
  blank-window result.
- The `cargo test --workspace` session ended before its exit code and summary could be recovered. I did not
  rerun it, so this report does not claim it passed.
- No PRs were opened, and the other branches were not changed. The prompt's stop rule prevented those steps.
- The obsolete `docs/06-packaging-prerequisite` branch remains at `ed494ad` on the local and remote refs.
