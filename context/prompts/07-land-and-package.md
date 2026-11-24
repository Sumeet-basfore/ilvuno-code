> Paste everything between the two markers as the goal. Nothing else.
> Run this only after the packaging agent has stopped. It is mid-work right now.

---8<--- PROMPT STARTS ---8<---

**Goal: every branch here is one click from landing, and the packaging branch is finished and proven.**

Read `.agents/skills/poteto-mode/SKILL.md` before anything else. Then read
`context/prompts/README.md` and follow the house rules in it.

## Today

Four branches carry work and none of them is merged into `main`:

- `feat/truth-in-the-ui`, 2 commits. Restart recovery in `crates/api/src/recovery.rs`, 114 lines of tests in
  `crates/api/tests/truth.rs`, a budget screenshot.
- `audit/verify-and-harden`, 4 commits. The audit report and two fixes to the screenshot tool.
- `feat/linux-packaging`, 5 commits plus about 22 uncommitted files. New: `docs/install.md`,
  `tools/package-linux.sh`, `crates/desktop/tauri.packaging.conf.json`, icons, and changes to the window.
- `docs/06-packaging-prerequisite`, a blocker note that the accepted dogfood run made obsolete.

`main` is 2 commits ahead of `origin/main` and has not been pushed.

## What to do, in this order

1. **Finish the branch you are on.** Commit the packaging work in flight with a message that says what it
   adds. If any of those files are not yours, stop and say which.
2. **Prove the package.** Build it. Install or extract the result. Start it as a user would. Confirm the
   window reads a real run from a real `ilvunod`. Run every command in `docs/install.md` and fix the ones
   that do not work.
3. **Push `feat/linux-packaging` and open a PR.** The body says what you verified and what you could not.
4. **Push the other two branches and open a PR each.** If a branch's tests fail, fix it or write why not. Do
   not merge anything, and do not push `main`.
5. **Report a table**: branch, commits, test result, PR link, and what the human must decide.

## Rules

- Do not merge to `main`, do not push `main`, do not force-push.
- A package that builds but does not start is not finished.
- No dependency beyond what Tauri already bundles.
- Delete `docs/06-packaging-prerequisite` once its note is superseded, and say so in the PR.
- Update `context/timeline.md` with the commits you made, and `context/features.md` if packaging now exists.

## Done when

- `git status --porcelain` is empty;
- each of the three branches is pushed with an open PR, or a written reason it is not;
- the AppImage starts on this machine and shows a real run, and you have looked at it;
- the report file exists with the branch table.

## Stop and report instead, if

- the uncommitted files are not yours;
- the AppImage opens a blank window. That is a defect, not a packaging detail;
- a branch cannot be made green without a design decision;
- a PR would need a force-push.

---8<--- PROMPT ENDS ---8<---

## For you, before you paste

- **Wait** until the packaging agent stops and reports. This prompt starts by committing its half-finished
  work, and running it mid-turn would fight it for the same files.
- **Cost:** build time only. No agent quota unless it starts a run to prove the window shows one.
- **What to check when it reports:** three PR links, an AppImage you can start yourself, and an empty
  `git status`. Then merge the PRs in the order: truth-in-the-ui, audit, packaging.
