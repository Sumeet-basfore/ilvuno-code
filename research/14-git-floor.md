# 14 — What git can actually do before we build stage 2

Stage 2 is "one branch and one worktree per thread, a merge the runner executes". Before we write that, here
is what the previous Ilvuno attempt measured about git itself. It is worth keeping because every one of
these is a trap that costs an afternoon to rediscover.

**Where this comes from.** The old Go repository at `projects/ilvuno`, document
`docs/research/09-git-floor.md` and its raw logs in `docs/research/09-git-floor-evidence/` (commit `a526c43`).
I did not copy the logs; this is the part that changes how we write the code. Numbers below are from that
run, not from this machine.

**The floor.** That project supported **Git 2.38.1** and newer, on Linux first (decision D-05 there). Tests
ran on 2.38.1 and on 2.55.0. Behaviour was the same on both except where noted.

## The five things that matter

1. **`merge-tree` is the merge.** It takes two commits and prints the result.
   - clean: stdout is `<tree-id>\0`, exit 0
   - conflict: stdout is `<tree-id>\0<path>\0<path>…`, exit 1
   - unrelated histories: exit 128, `fatal: refusing to merge unrelated histories`
   - `--write-tree --name-only -z --no-messages` are accepted on both versions, but they do **not** appear
     in `git merge-tree -h`. Do not decide support from the help text; run it.

2. **A configured merge driver runs under `merge-tree`.** If a user's `.gitattributes` names a custom merge
   driver, git executes it during our merge. That is someone else's program running inside the runner. We
   must detect custom drivers and refuse *before* the merge, in our own code.

3. **`worktree add -B refs/ilvuno/<x>` does not create `refs/ilvuno/<x>`.** It creates
   `refs/heads/refs/ilvuno/<x>`, on both versions. To keep our refs out of `refs/heads`, use
   `worktree add --detach` and then `update-ref`; that path leaves no residue once cleaned up.

4. **`check-attr --source` does not exist on 2.38.1** (exit 129, `unknown option`); it exists on 2.55.0.
   Anything that reads attributes must probe for the flag and refuse or fall back when it is missing.

5. **A conflicting merge writes objects.** A conflicting `merge-tree` left 2 extra loose objects behind
   (19 → 21). Conflicts are not free; either use them or clean up.

## What I would do with this

- Put the git version check at the door: refuse to start a run below the floor, and print the version.
- Write one small capability probe (`check-attr --source`, `merge-tree --write-tree`) and cache it.
- Refuse custom merge drivers explicitly, with a message that names the attribute.
- Worktree refs by `--detach` + `update-ref`, never `-B refs/…`.
- Test against a pinned older git, not just the machine's git. The floor is where the surprises live.
