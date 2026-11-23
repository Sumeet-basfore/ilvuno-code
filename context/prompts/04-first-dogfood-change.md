> Paste everything between the two markers as the goal. Nothing else.
> **Run this only after 01 and 02 are merged and working.**

---8<--- PROMPT STARTS ---8<---

**Goal: Ilvuno changes Ilvuno once, for real, and a person accepts it.**

Read `/home/sumeet/personal-repo/projects/ilvuno-second/context/prompts/README.md` and follow the house
rules in it. This is the north star, first attempt: one small change to this repository, produced by a run,
reviewed by a second thread, landed by the runner when a human accepts.

## Why it is shaped like this

Checks run against `artifacts/` and the goal folder, **not** against a thread's worktree — so a `command`
check saying "the tests pass" would test the unchanged project, which is a lie. For this first run, the
gate is the check: a person reads the diff. Say so in the goal file's `done` text instead of pretending
otherwise.

## What to build

**1 · A real, small change.** Pick exactly one, and pick the smallest:

- `ilvuno version` — a subcommand that prints `ilvuno <version>` from `Cargo.toml`, plus a test in
  `crates/cli/tests/`. Touches `crates/cli/src/main.rs` and one new test file.
- a missing unit test for `git::branch_name` — the branch sanitiser has no test for odd thread names
  (`a b`, `A/B`, `ünïcode`). Touches `crates/runner/src/git.rs` only.

If the first is picked, the goal is one sentence and the done text is: *"`ilvuno version` prints the version
and the new test passes."*

**2 · A goal file for it**, in `examples/dogfood-version.json` (or the name that matches the change):

- `base` is this repository, **absolute path**;
- two threads: `writer` (does the change) and `reviewer` (reads the diff and reports problems, writes
  nothing);
- the writer's scope is the two files it may touch. The reviewer's scope is the same files;
- `budget.rounds` 2, `budget.tokens` 300000, `max_usd_per_call` 0.2;
- `message_policy`: `"round"`;
- `done` describes the change in one sentence and says the acceptance is a person reading the diff.

**3 · Run it and stop at the gate.** Do not accept. Do not merge. Bring it to the human with:
- the run id;
- the writer's diff (the stat and the patch);
- the reviewer's report;
- what you would check before accepting.

**4 · After the human accepts**, verify the landing: the change is in the project, `git log` names the run,
`RUSTUP_TOOLCHAIN=1.100.0 cargo test --workspace` is green including the new test. Then update
`context/timeline.md` with the run id and the commit.

## Rules for this goal

- One change. Not two. Not "while I was there".
- The writer must not run `git` itself: its worktree commit is the runner's job. If the preamble does not
  already say that, add the line to `crates/runner/src/prompts/mod.rs` **and** note it in the run report —
  a prompt change is a real change and deserves its own commit.
- Nothing in this goal may touch `crates/desktop/ui/` or the gate code. If the change you picked needs
  either, pick the other change instead.
- The human accepts. You never do.

## Done when

- the run reached the gate with a readable diff of a real change;
- the human accepted, and the change is in the project on `main` with the run id in the message;
- `cargo test --workspace` passes;
- `context/timeline.md` records the first self-built change, and `context/story.md` gains a sentence saying
  it happened;
- the report file exists.

## Stop and report instead, if

- 01 or 02 is not merged (then stop immediately and say which one is missing);
- the writer's diff is unreadable or touches files outside its scope — that is a finding, not a reason to
  accept;
- the merge conflicts — then the project is left clean and the run is `blocked`; report the files.

---8<--- PROMPT ENDS ---8<---

## For you, before you paste

- **Cost:** a real multi-thread run on this repo — expect a few hundred thousand tokens and a few minutes.
  This is the first run whose output is this project's own code.
- **Check when it reports:** read the diff yourself. You are the check. If it is not what you asked for,
  refuse at the gate — that refusal is exactly what the gate is for.
- **If it goes wrong:** the project's working tree is untouched until you accept. Reject, delete the
  worktree, and the repo is as it was.
