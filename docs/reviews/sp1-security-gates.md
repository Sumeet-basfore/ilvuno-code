# SP-1 Security Gates

This review addresses execution behavior in Pipeline B and the experiment harness. Git worktrees isolate a checkout and index; they do not sandbox Git, repository configuration, hooks, filters, merge drivers, or commands run in the worktree. Tests were run in disposable repositories on Git 2.38.1 and Git 2.55.0. All payloads wrote only inside the audit temporary root.

## Threat boundary

The candidate repository and its configuration are potentially adversarial inputs. Any Git operation that checks out files or merges branches may invoke configured programs. A human consent prompt can authorize an effect; it cannot contain the effect. Reliable containment of arbitrary process execution requires an OS-level sandbox with constrained filesystem, process, and network access. M0 may defer production-grade sandboxing only if its fixtures are trusted, controlled, temporary, and contain no untrusted project commands or executable Git extensions.

## Findings

### SG-1 — `post-checkout`, `pre-merge-commit`, and `post-merge` hooks execute

- **Severity:** High
- **Evidence:** A marker `post-checkout` hook ran during `git worktree add --detach`; `pre-merge-commit` and `post-merge` hooks ran during successful `git merge --no-ff` on both Git versions. `pre-merge-commit` returning 42 caused merge to fail with rc 1 and no unmerged paths. Pipeline B classified this as `conflict` with an empty path set.
- **Reproduction:** In a disposable repository configure `core.hooksPath` to a temporary hook directory with executable marker scripts named `post-checkout`, `pre-merge-commit`, and `post-merge`; add a detached worktree and perform a clean non-fast-forward merge. Separately make `pre-merge-commit` exit 42 and inspect Pipeline B's report.
- **Impact:** A repository can execute code during setup/merge; hook failure is misreported as a content conflict. Worktree creation itself may execute `post-checkout` before the candidate merge starts.
- **Recommended correction:** For controlled harnesses, run Git with an explicit empty hooks directory configured through command-scoped config/environment and assert no hooks ran. Production must define which hooks, if any, are intentionally supported. Classify hook/program failures separately from content conflicts using structured state and postconditions; never infer conflict from exit 1 alone.
- **Required milestone:** M0 must neutralize hooks in fixtures; M2 must have an approved hook/effect policy and robust classification. Arbitrary untrusted repositories require sandboxing.

### SG-2 — Runtime Git configuration overrides defeat repository-level neutralization

- **Severity:** High
- **Evidence:** `GIT_CONFIG_COUNT=1`, `GIT_CONFIG_KEY_0=core.hooksPath`, and `GIT_CONFIG_VALUE_0=<marker-dir>` overrode a repository's `core.hooksPath=/dev/null`; the hook executed on both versions. Clearing global/system config does not clear runtime config injection. Git documents environment config pairs and `-c` precedence.
- **Reproduction:** Set those three environment variables while invoking Git in a temporary repo whose local config points `core.hooksPath` to `/dev/null`; add a worktree. The marker hook executes.
- **Impact:** Local config or global-config reset alone is not a fail-closed execution boundary. Inherited environment can alter merge strategy, hooks, attributes, and other behavior.
- **Recommended correction:** Construct a sanitized environment for every Git subprocess, remove `GIT_CONFIG_*` and other Git control variables unless explicitly needed, and pass controlled command-scoped config. Record the effective configuration relevant to the operation and reject unsupported overrides.
- **Required milestone:** M0 harness wrapper; M2 production process boundary.

### SG-3 — Custom merge-driver gate misses configured default driver

- **Severity:** High
- **Evidence:** With a configured custom driver and `merge.default=spy`, but no explicit `merge` attribute, the SP-1 detector saw configured driver text while `driver_sel(BASE)` was empty. Its refusal condition requires both values, so the driver ran during `merge-tree --write-tree` on both Git versions.
- **Reproduction:** Configure `merge.spy.name`, `merge.spy.driver` to a marker script, and `merge.default=spy`; leave `.gitattributes` without a merge attribute; perform a conflicting merge with the detector enabled.
- **Impact:** A configured external program runs despite the advertised “refuse custom drivers” gate. Pipeline A is also exposed; object-only merging is not execution-free.
- **Recommended correction:** Determine the effective driver for every path and effective configuration, including `merge.default`, then fail closed whenever a non-built-in driver can be selected. Prefer a controlled isolated Git config and explicit allowlist over partial parsing.
- **Required milestone:** M0 test harness must exclude/neutralize custom drivers; M2 driver policy and enforcement.

### SG-4 — Later candidate commits can introduce merge attributes after preflight

- **Severity:** High
- **Evidence:** Source 1 added `.gitattributes` selecting a custom driver and changed a file. Source 2 then conflicted on that file. Pipeline preflight inspected only the original base tree, yet the marker driver ran during the second merge on both versions.
- **Reproduction:** Build two pinned source commits in a temporary repo: first adds `.gitattributes` with `*.txt merge=spy` and edits `x.txt`; second independently edits `x.txt`. Configure `merge.spy.driver` to write a marker. Run sequential B integration.
- **Impact:** Checking only the base tree cannot prove the merge loop will not later execute a driver. Candidate content and attributes evolve at each step.
- **Recommended correction:** Re-evaluate effective attributes and configuration at every merge step and for all conflicted paths; still use a constrained process environment because partial attribute inspection cannot neutralize all execution sources.
- **Required milestone:** M2. M0 controlled fixtures only.

### SG-5 — `core.attributesFile` is missed on Git 2.38 fallback

- **Severity:** High
- **Evidence:** On Git 2.55, `check-attr --source` exposed the external attribute and the detector found it. On Git 2.38.1, the fallback missed `core.attributesFile`; the detector returned no driver while the configured driver executed during `merge-tree`.
- **Reproduction:** Set `core.attributesFile` to a temporary external attributes file selecting `merge=spy`; configure `merge.spy.driver` to write a marker; run the Git 2.38.1 detector/merge-tree path.
- **Impact:** The documented floor version fails open for an attributes source outside the base tree.
- **Recommended correction:** Treat `core.attributesFile` as an explicit source in the fallback or, preferably, disable uncontrolled attributes/config sources in the isolated harness and validate effective attributes path-by-path.
- **Required milestone:** M0 if the floor-version harness accepts repository-provided fixtures; otherwise M2 before any production driver gate claim.

### SG-6 — Linked-worktree `.git/info/attributes` fallback uses the wrong path

- **Severity:** High
- **Evidence:** In a linked worktree, `git rev-parse --git-path info/attributes` resolved to the common repository's attributes file. The SP-1 fallback hardcodes `$repo/.git/info/attributes`; in a linked worktree `.git` is a file, so the detector missed the selected custom driver. The driver executed on both versions.
- **Reproduction:** Create a linked worktree, write `*.txt merge=spy` to the common Git directory's `info/attributes`, configure the driver, then run detector and merge.
- **Impact:** Pipeline B's primary execution context is precisely where the fallback path assumption fails.
- **Recommended correction:** Resolve Git administrative paths using `git rev-parse --git-path` in the target worktree and test common-dir/worktree-dir differences. Do not treat path resolution alone as containment.
- **Required milestone:** M2. M0 harness should set a controlled common Git directory and assert no driver invocation.

### SG-7 — Checkout filters execute while materializing the disposable worktree

- **Severity:** High
- **Evidence:** A `.gitattributes` `filter=spy` attribute with a configured `filter.spy.smudge` command ran during `git worktree add` on both versions.
- **Reproduction:** In a disposable source repository configure `filter.spy.smudge` as a marker command, commit `.gitattributes` selecting `filter=spy`, then add a detached worktree.
- **Impact:** Driver checks that only inspect merge attributes do not prevent checkout-time process execution. This occurs before the first merge.
- **Recommended correction:** Use a sanitized environment/config and decide whether filters are disabled, allowlisted, or executed in a sandbox. If filter fidelity is a required product property, execute only within the approved isolation boundary and record the effective filter policy.
- **Required milestone:** M0 controlled-fixture neutralization; M2 explicit policy and containment.

### SG-8 — `.ilvuno` symlink redirects the candidate worktree outside the repository

- **Severity:** High
- **Evidence:** With `.ilvuno` symlinked to a directory outside the repository, Pipeline B created `worktrees/candidate` under that external target on both Git versions.
- **Reproduction:** In a temporary repository create `.ilvuno -> /tmp/audit-outside`, then run `git worktree add --detach .ilvuno/worktrees/candidate <sha>`.
- **Impact:** A path that appears repository-local can redirect writes to an unintended location. A worktree is not a filesystem containment mechanism.
- **Recommended correction:** Create an owned private scratch root outside the attacker-controlled checkout, verify it with descriptor-based/no-follow path handling, reject symlinked components, and ensure all cleanup targets remain beneath that root.
- **Required milestone:** M0 script safety; M2 production path safety.

### SG-9 — Candidate worktree commands can execute arbitrary processes and escape

- **Severity:** High
- **Evidence:** A fixture/check command invoked in the candidate worktree wrote a marker outside the repository. No worktree mechanism prevented it.
- **Reproduction:** Put an executable fixture command in a temporary source tree that writes to a sibling temp directory; invoke it from the candidate worktree.
- **Impact:** Consent can authorize running a command but cannot restrict its filesystem or network effects. Any product claim of safety for untrusted repository commands requires OS process containment.
- **Recommended correction:** Use an OS sandbox/container with explicit filesystem, network, process, credential, and resource limits for arbitrary project verification. For M0, do not run project commands; use harness-owned deterministic checks against fixtures.
- **Required milestone:** M2 before arbitrary verification commands; M0 can defer only under the stated controlled-fixture restriction.

### SG-10 — Submodule checkout remains uninitialized by default, but is not a security guarantee

- **Severity:** Medium
- **Evidence:** A detached worktree containing a committed gitlink did not initialize the submodule; `git submodule status` showed `-<sha> deps/mod` on the host Git. This is a positive observation for the exact default command only.
- **Reproduction:** Commit a gitlink in a temporary superproject, add a detached worktree without `--recurse-submodules`, and inspect `git submodule status`.
- **Impact:** The tested command did not fetch/populate the submodule. A later command, config option, or recursive checkout can change this; do not claim general submodule isolation.
- **Recommended correction:** Explicitly disable recursive submodule behavior in the harness, assert no submodule worktrees or network access, and document submodules as unsupported until a policy exists.
- **Required milestone:** M0 harness assertion; M2 support decision.

### SG-11 — Worktree removal can orphan a directory after deleting admin metadata

- **Severity:** Medium
- **Evidence:** In the P3 cleanup fixture, `git worktree remove --force` returned 255 and removed administrative registration but left the worktree directory on both versions. `git worktree list` showed only main, and `worktree prune` could not discover the orphan directory.
- **Reproduction:** Run the existing P3 probe in an isolated temp repository and inspect both the directory and `git worktree list --porcelain` after removal failure.
- **Impact:** Registration-only cleanup checks report success while an on-disk candidate residue remains; a future run may collide with or overwrite it.
- **Recommended correction:** Verify directory existence and admin metadata separately, never recursively delete a path until its canonical ownership is proven, and journal `removed`, `orphaned_dir`, or `stale_admin` states. Make repeated cleanup idempotent.
- **Required milestone:** M0 harness cleanup; M2 recovery implementation.

## Security disposition by milestone

- **M0:** No production sandbox is required for a harness that operates only on newly created, controlled temporary fixtures and invokes no project verification commands. The SP-1 script itself must be made path-safe, fail closed, and use controlled hooks/config; fixtures must assert no unexpected process markers.
- **M2:** Before integrating arbitrary user branches or running repository commands, resolve hook/filter/driver/attribute policy, sanitize process environment, harden scratch paths and cleanup, classify process failures distinctly, and sandbox arbitrary commands. Approval alone is not containment.

## Official references

- [Git hooks](https://git-scm.com/docs/githooks)
- [Git attributes and filters](https://git-scm.com/docs/gitattributes)
- [Git configuration precedence and environment](https://git-scm.com/docs/git-config)
- [Git worktrees](https://git-scm.com/docs/git-worktree)
