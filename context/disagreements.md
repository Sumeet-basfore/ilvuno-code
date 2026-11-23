# Where we disagreed

A project's record that only lists agreements is marketing. This page lists the times the user corrected the
agent, the times the agent pushed back, and the times the agent was simply wrong. The last section is the
most useful one.

## The user corrected the agent

| The agent proposed | The user said | What happened |
|---|---|---|
| A design where Claude codes and Codex reviews | *"If I have only one subscription, it must still work."* | Roles were removed. Agents became choices per thread (D7), and the per-thread picker came later (`d36828b`). |
| Research mode and feature mode | *"We don't need separate modes. The app gives abilities; the user decides via prompts."* | The PRD was rewritten around abilities, not workflows (D8). |
| A browser UI | *"Nobody codes in a browser. It should be a real app."* | Rust engine plus Tauri window (D1, D3). |
| A dense, developer-looking interface | *"The UI is nothing like the aim"* (second time), after a first rating of **5/10** | The layout was rebuilt twice; the third attempt was researched from shipped apps instead of invented (`481d10c`, `cf3a083`). |
| Reports written as Markdown | *"All future reports must be HTML"* | `tools/render-report.py`; the rule went into `AGENTS.md`. Later refined: HTML for reading, Markdown in git. |
| Long technical prose | *"Dummify it. English is my second language."* | The plain-English rule, applied to docs, commits and UI copy. |
| A plan to rebuild parts of the app | *"Do not rebuild from scratch. Only adopt what is genuinely useful."* | The old Go repo was read for evidence only; one document was adopted (`research/14-git-floor.md`), no code. |
| Generated HTML and mocks kept in the repository | *"We don't need to push the HTML and the assets."* | Sources kept, generated pages ignored; the agent argued for keeping the screenshots and fonts, and the user's direction was applied to the rest. |

## The agent pushed back

| The agent argued | Against | Outcome |
|---|---|---|
| Three disagreements with the product thesis (`0916141`) | the thesis itself | The user settled two; one — safety badges — became part of the window. |
| Git before dogfooding | an earlier instinct to dogfood next | Vindicated by finding that agents write to a scratch folder (R5). The order changed. |
| Keep the screenshots and the fonts in git | dropping all "assets" | Kept: fonts are build inputs; the screenshots are the only record of runs that no longer exist. |
| Refuse reference features the PRD did not approve (terminal, file explorer, PRs) | the appeal of matching T3 Code and Orca | Listed in `docs/ui-checklist.md` as *not approved*, rather than silently built or silently dropped. |
| Do not rewrite git history without a yes | repository size | Still open (O3). |

## Where the agent was wrong

Written down because it is the most expensive kind of error to repeat.

| Claim | Reality | How it was caught |
|---|---|---|
| "Resume works" | It never did. Nothing set a stopped run back to `working`, so the driver read the pause it was being resumed from and returned at once. | Stopping and resuming a real run (`40d1009`). |
| "The menu is broken" | The menu was fine; the button's own native tooltip was painted over it. | Dumping the menu's DOM instead of guessing a third time. |
| "The picker is broken" | The picker was fine; `xdotool` typed `/tmp/statete-a.json` into the GTK location bar. | Printing the path the app received. |
| "Tool activity needs a provider-contract change" | It does not: `tool.call` and `permission.decided` are already journaled and streamed. | Reading the journal of a real run (R4). |
| "Dogfooding is next" | Impossible: agents write in `run/threads/<name>/`, not in the project. | Reading `dispatch.rs` (R5). |
| Coordinates read off a scaled screenshot | Three clicks landed on the wrong control, twice blamed the app. | Measuring the element's rectangle from an unscaled capture. |

## Where we agreed, and why it stuck

- **The board is the product.** Transcripts are commodity; a written state that survives a handoff is not.
- **The gate is the product.** Nothing lands without a person, and the merge is the runner's job, not the
  agent's.
- **Honest `unknown`.** A state we cannot read is shown as unknown, never guessed.
- **Abilities, not workflows.** The app never decides who does what.
- **One subscription must work.** No feature may require two providers.
- **Verify, don't assert.** Every serious defect of the day was found by running the thing: a live stop, a
  real screenshot, a measured contrast ratio, a DOM dump.
