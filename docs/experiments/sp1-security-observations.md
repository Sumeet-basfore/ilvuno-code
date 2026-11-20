# SP-1 · Security Observations

Everything here was executed. Probe IDs (`P1…P5`) map to `sp1.sh probes`; versioned transcripts
are printed by that command and summarised below.

**Scope:** custom merge drivers, write classes, cleanup failure modes, the declared floor, and the
"worktree is not a sandbox" claim. No exploit was developed; the drivers used are benign markers.

> **SP-1R update (2026-11-19).** This report describes the original SP-1 run. The harness has since been
> remediated: the matrix is an enforcing test (`expected.json`, 28 cases), output goes to a validated
> private scratch root, Git runs under a controlled environment, driver refusal is config-based, and
> classification uses repository state rather than stderr. Numbers below that refer to "8 clean
> cases" remain the original corpus result; current enforced results are in
> `sp1r-regression-matrix.md`. See `sp1r-remediation-report.md` for the finding-by-finding status.

---

## 1. Custom merge drivers execute in both designs — and in `merge-tree`

A driver is selected by `.gitattributes` (`path merge=<name>`) and defined in Git config
(`merge.<name>.driver = <shell command>`). With a benign driver that writes a marker outside the
repository:

| Operation | Driver executed? | Result |
|---|---|---|
| `git merge-tree --write-tree` | **yes** | marker written; exit 0 |
| `git merge` in a worktree | **yes** | marker written; exit 0 |

So the object pipeline is **not** a lower-risk path: "we only compute a tree" is not true once a
driver is configured. The control must be identical in both designs: **detect, then refuse.**

## 2. The review's proposed neutralisation does not work

The red-team review proposed overriding drivers with `-c merge.default=merge` and
`-c merge.<driver>.driver=2`. Tested (P4, both versions):

| Override | Git 2.55.0 | Git 2.38.1 | Safe? |
|---|---|---|---|
| `-c merge.default=merge` | driver **still runs**, exit 0 | driver still runs | **no** — `merge.default` does not override an explicit attribute |
| `-c merge.upper.driver=2` | `error: cannot run 2`, exit 1 | `error: cannot run 2`, **exit 0** | **no** — spawns `2` as a command; on 2.38 the failure is reported as success |
| `-c merge.upper.driver=true` | exit 0, content = **ours** | exit 0, content = **ours** | **no** — silently drops the other side's change; a wrong merge disguised as clean |
| `-c merge.upper.driver=text` | `merge-tree` exit 1 (`cannot run text`), `merge` exit 1 → `UU` conflict | `merge-tree` **exit 0**, `merge` exit 2 | **no** — inconsistent across commands and versions |
| `-c merge.upper.driver=` (empty) | `error: cannot run`, exit 1 | same | fails closed, but by error, not by design |

Two conclusions:

1. **Neutralising a driver from the outside is not supported.** The only reliable behaviours are
   "run it" or "refuse to run".
2. **`driver=true` is the most dangerous option tested.** Exit 0, no conflict, one side's change
   gone. Any future suggestion to "just disable drivers with `-c`" must be rejected with this
   table.

## 3. Detection must have a fallback on the declared floor

`git check-attr --source=<tree>` resolves attributes against a tree without a checkout. It exists
on 2.55; on **2.38.1 it exits 129**. Because the first implementation relied on it:

```
Git 2.38.1, no fallback:   F8 driver -> verdict=ok  exit=0  drivers_ran=YES   (fail open)
Git 2.38.1, with fallback: F8 driver -> verdict=driver_refused exit=4 drivers_ran=no
```

The fallback reads `.gitattributes` blobs from the merge inputs (and `.git/info/attributes`) and
refuses if any in-tree selection names a configured non-builtin driver. It is conservative: it may
refuse a run whose changed paths do not actually use the driver. That is the correct direction to
be wrong.

**Design consequence:** capability-probe the command, never assume it. The probe is part of
`doctor`, and the capability is recorded in the run.

## 4. Write classes, measured

Per operation, with `find .git -newer <marker>` and object counts. Identical in both designs except
where noted.

| Operation | Writes | Class |
|---|---|---|
| `merge-tree --write-tree` | loose objects (`objects/xx/…`), **including on conflict** | B (Git-managed) |
| `commit-tree` | one commit object | B |
| `update-ref refs/ilvuno/*` | `refs/ilvuno/**`, `logs/refs/ilvuno/**` | B |
| `worktree add --detach` | `.git/worktrees/<n>/{commondir,gitdir,HEAD,index,logs/HEAD,ORIG_HEAD}` | B |
| `git merge` (B only) | same as above + objects | B |
| `worktree remove` + `prune` | removes `.git/worktrees/<n>/**` | B |
| driver command | **anything the command does**, anywhere | **C** |
| verification child | anything the command does, including network | **C** |
| `init` | `ilvuno.toml`, one `.gitignore` line | D |
| key init | `~/.config/ilvuno/keys/**` | D |

Measured deltas: median **+3.5 objects** per clean 2-source run for both approaches; identical
object counts after the same fixture. **No refs outside `refs/ilvuno/*` were created in any
scenario.** The main index was never written (A never touches it; B writes only
`.git/worktrees/<n>/index`).

This confirms the four-class model in `docs/readiness/03-security-boundary-correction.md` §2 and
refutes the "writes only `.ilvuno/` and `refs/ilvuno/`" wording in `SEC-101`.

## 5. Worktrees are workspace separation, not isolation

Nothing in SP-1 changes this. A verification child runs with the user's privileges and can write
outside the candidate directory, including to `$HOME` and the main checkout. The candidate worktree
shares `.git` (objects, refs, config, hooks). The experiment deliberately did **not** run any
project verification commands beyond the fixture-local `check.sh`; the claim is established by the
mechanism, not by a destructive test.

Consequences carried forward:

- the report must state `sandbox: none` and `child_network: unconstrained`;
- `SEC-101` must be restated as four effect classes;
- no document may call a worktree "isolated" or "disposable isolation" without qualification.

## 6. Two cleanup failure modes, both real

**P3 — remove fails after the admin entry is deleted.**

```
git worktree remove --force  -> rc=255 "failed to delete …: Permission denied"
worktree list entries        -> 1  (main only)
.git/worktrees entries       -> 0
directory still present      -> yes
```

Result: an orphan directory that `worktree list` and `git worktree prune` cannot find. Cleanup
logic that only inspects `worktree list` will report success while leaving disk behind.

**P5 — directory deleted under Git (e.g. `git clean -xdf`).**

```
worktree list entries  -> 2   (main + phantom)
admin entries          -> 1
after git worktree prune -> 0
```

Result: a stale admin entry; `prune` fixes it. This is the failure the review described, and it is
the *benign* one.

**Design consequence:** cleanup must be verified by checking the directory path *and* the admin
entries, and must journal the outcome. `abort`/`gc` must handle three states: removed, orphaned
directory, stale admin entry.

## 7. Repository ignore line is a security-adjacent requirement

Without `.ilvuno/` in `.gitignore` (P2):

```
with ignore:     git status -> clean
without ignore:  git status -> ?? .ilvuno/          (the candidate worktree is visible to the user)
```

Two consequences: the user's `git status` no longer reflects their work, and a subsequent
`git add -A` by the user or an agent can stage the candidate directory. `init` must write the line
and `doctor` must assert it before the first integration.

## 8. Floor summary

| Behaviour | 2.38.1 | 2.55.0 | Design impact |
|---|---|---|---|
| `merge-tree --write-tree` | works | works | A is floor-compatible |
| `-z --name-only --no-messages` | works | works | parsing is stable |
| `check-attr --source` | **absent (129)** | present | fallback mandatory |
| driver override `=2` | error, **exit 0** | error, exit 1 | never use |
| driver override `=text` | merge-tree exit 0 | merge-tree exit 1 | never use |
| `worktree add --detach` | works | works | B is floor-compatible |
| `worktree remove` orphan-on-failure | yes | yes | cleanup must verify directories |

## 9. Open security questions SP-1 did not answer

1. Whether a repository can defeat the attribute fallback (e.g. attributes in a submodule, a
   `.gitattributes` symlink, `core.attributesFile` pointing outside the repo).
2. Whether a driver can be made to run *after* refusal (e.g. via `post-merge` hooks — hooks are
   disabled in candidate commits, but `git merge` in B runs no hook for merges; verify).
3. Whether `git merge` in B can be influenced by `merge.ff=only` / `merge.conflictStyle` in the
   user's config in ways that change the verdict (strategy options were not tested).
4. LFS smudge/filter interaction: a candidate tree verified under A is materialized afterwards; if a
   filter changes content, the checked tree differs from the merged tree (untested).
