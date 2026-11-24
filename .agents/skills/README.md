# pstack skills, vendored

These 59 skills are **not ours**. They are copied verbatim from
[`cursor/plugins` → `pstack/skills`](https://github.com/cursor/plugins/tree/main/pstack/skills).

| | |
|---|---|
| Source | `https://github.com/cursor/plugins`, directory `pstack/skills` |
| Commit | `005d8f6e8ec057b738adcd08a08b30db0478402d` |
| Copied | 2026-11-24 |
| Licence | MIT, Copyright (c) 2026 Lauren Tan — full text in `LICENSE` beside this file |

Nothing here was edited. If a skill needs a change for this project, copy it out under a new name and say
why, rather than editing in place — an edit here is lost the next time these are refreshed.

## How to refresh

```bash
cd /tmp && rm -rf cursor-plugins
git clone --depth 1 --filter=blob:none --sparse https://github.com/cursor/plugins cursor-plugins
cd cursor-plugins && git sparse-checkout set pstack
rm -rf /home/sumeet/personal-repo/projects/ilvuno-second/.agents/skills
mkdir -p /home/sumeet/personal-repo/projects/ilvuno-second/.agents/skills
cp -r pstack/skills/. /home/sumeet/personal-repo/projects/ilvuno-second/.agents/skills/
cp pstack/LICENSE /home/sumeet/personal-repo/projects/ilvuno-second/.agents/skills/LICENSE
```

`.claude/skills` is a symlink to this folder, so Claude Code and the other agents read the same copy.

## One thing that conflicts with this project

pstack ships its own **model routing** — code work to `opus 5.5`, prose and judgment the same, a review panel
of `fable 5.2 / opus 5.5 / gemini 4 neon`, all governed by `/setup-pstack`, which writes
`~/.claude/rules/pstack-models.md`.

**This project does not use those models.** `AGENTS.md` and `context/prompts/README.md` say that any
sub-agent runs on `gpt-6-luna` with `reasoningEffort: "xhigh"`, and nothing else. If you use `poteto-mode` or
`swarm`, the project rule wins.

`/setup-pstack` has since been run, and its rule now names `gpt-6-luna` for every role, in
`~/.cursor/rules/pstack-models.mdc` and `~/.claude/rules/pstack-models.md`. That is the same rule, not a
second opinion: editing either file to name another model would break the project's constraint. Reasoning
effort stays `xhigh`, and pstack's rule format cannot express it — effort is pinned in `AGENTS.md` and
`context/prompts/README.md`.

Which skills are safe to use as written: `unslop` (prose), `technical-writing`, `teach`, `why`, `recall`,
`reflect`, `sharpen`, `reframe`, the `principle-*` family, `tdd`, `typescript-best-practices`, `statechart`.
The orchestration ones — `poteto-mode`, `swarm`, `setup-pstack` — assume their own routing and a Cursor or
Claude Code harness; read them before running them here.
