> Paste everything between the two markers as the goal. Nothing else.
> Run this between phases, or whenever the project has had a burst of changes.

---8<--- PROMPT STARTS ---8<---

**Goal: prove the build still works, fix what is broken, add nothing.**

Read `/home/sumeet/personal-repo/projects/ilvuno-second/context/prompts/README.md` and follow the house
rules in it. This goal is an audit. If it turns into a feature, you have left the goal.

## What to do, in this order

**1 · The suite.**
`RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace`. Record the count. If anything fails, fix the cause — not
the test — unless the test asserts something the project no longer promises, in which case say which promise
changed and who changed it.

**2 · The real thing, every screen.**
Run `tools/shots.sh` (it needs a runner and a run in `ILVUNO_HOME`). Then **read every image** in
`crates/desktop/shots/` and `crates/desktop/shots/states/`. For each: is it the UI we intend, is anything
clipped, is any text wrong, is any state lying? List what you find; fix the small things, report the large
ones.

**3 · The widths and the contrast.**
- `tools/shot.sh /tmp/x.png 1440 900`, then `1280`, `1060`, `960` (the window's minimum). Nothing may clip
  and nothing may scroll sideways.
- Recompute contrast for the text and control pairs from the tokens in `crates/desktop/ui/style.css`
  (text ≥ 4.5:1, control boundaries ≥ 3:1, both themes). Write the numbers into the report.

**4 · The engine's promises.**
- `ilvuno replay <run>` still matches its own journal on the newest run you can make;
- a `SIGKILL` of `ilvunod` mid-run, then a restart, does not leave a run claiming to be working (this is
  goal 05's job if it is not in yet — then just record it as still broken);
- `ilvuno export` produces `run.json`, `board.md`, `journal.jsonl`, `artifacts/`.

**5 · The documents.**
- every relative link in `context/`, `docs/` and `research/*.md` resolves;
- `context/timeline.md` covers the commits it should;
- nothing generated is tracked: `git ls-files | grep -E '\.html$'` should show only the app page, the two
  mock templates and the fixture artifacts.

**6 · The logs.**
Start the window, use it for a minute, and read `/tmp/desktop.log` for a JavaScript error, a
`ReferenceError`, or a `TypeError`. Any of those is a defect, not noise.

## Rules for this goal

- Fix regressions. Do not add features, options, or "while I was here" refactors.
- A test that fails because the project changed its promise: say which promise, in the report, and only then
  change the test.
- Do not commit generated pages, and do not regenerate the screenshot set unless you actually looked at the
  new images.
- Anything you cannot verify, list it under "could not verify". That list is the useful part of the report.

## Done when

- the suite is green and the count is in the report;
- every screenshot has been looked at, and each finding is either fixed or written down;
- the four widths are checked and the contrast numbers are recorded;
- the report file exists, with a section **"could not verify"** that is honest even if it is short;
- `context/timeline.md` has one line for this audit.

## Stop and report instead, if

- the suite fails in a way that needs a design decision (say which);
- a screenshot shows a state that cannot exist (that is a bug worth its own goal);
- you cannot make a real run because no agent CLI is logged in — say so, and audit everything else.

---8<--- PROMPT ENDS ---8<---

## For you, before you paste

- **Cost:** mostly local; no agent quota unless it makes a real run.
- **Use it** after any big merge, and before trusting a "done" from another prompt. This is the one that
  catches the claims that were never run.
