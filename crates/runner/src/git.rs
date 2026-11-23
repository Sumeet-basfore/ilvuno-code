//! Git, as stage 2 needs it: a worktree per thread, a commit after each turn, a diff the gate can show, and
//! a merge the runner performs only when a person accepts.
//!
//! Nothing here pushes, force-pushes, resets or touches a branch it did not create. A merge into the
//! project refuses unless the project's working tree is clean, and a conflict is aborted and reported.
//!
//! Traps taken from research/14-git-floor.md: the version floor, and a custom merge driver running inside
//! our merge. The floor check is here; the driver check belongs with the policy that refuses it.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The oldest git this supports. Below it, flags we rely on have been seen to differ.
pub const FLOOR: (u32, u32, u32) = (2, 38, 1);

/// A git command with prompts off, so a credential prompt can never hang the runner.
fn git(dir: &Path, args: &[&str]) -> Result<Output> {
    Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "true")
        .output()
        .with_context(|| format!("run git {}", args.join(" ")))
}

/// Same, but a non-zero exit is an error carrying git's own words.
fn git_ok(dir: &Path, args: &[&str]) -> Result<String> {
    let out = git(dir, args)?;
    if !out.status.success() {
        bail!("git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn version() -> Result<(u32, u32, u32)> {
    let text = git_ok(Path::new("."), &["--version"])?;
    let number = text.split_whitespace().nth(2).unwrap_or_default();
    let mut parts = number.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    Ok((parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0)))
}

pub fn meets_floor(version: (u32, u32, u32)) -> bool {
    version >= FLOOR
}

/// The top of the repository that contains `path`, or `None` when it is not in one.
pub fn root_of(path: &Path) -> Option<PathBuf> {
    let out = git(path, &["rev-parse", "--show-toplevel"]).ok()?;
    if !out.status.success() {
        return None;
    }
    Some(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

/// True when there is nothing to lose: no staged, unstaged or untracked changes.
pub fn is_clean(root: &Path) -> Result<bool> {
    Ok(git_ok(root, &["status", "--porcelain"])?.is_empty())
}

pub fn head(root: &Path) -> Result<String> {
    git_ok(root, &["rev-parse", "HEAD"])
}

/// A branch name for one thread of one run: `run/<run8>/<thread>`.
pub fn branch_name(run_id: &str, thread: &str) -> String {
    let short: String = run_id.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
    format!("run/{short}/{}", thread.replace(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_', "-"))
}

/// Create the thread's worktree on a new branch at `base`. The folder must not exist yet.
pub fn add_worktree(root: &Path, path: &Path, branch: &str, base: &str) -> Result<()> {
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let path = path.to_string_lossy().to_string();
    git_ok(root, &["worktree", "add", "-b", branch, &path, base])?;
    Ok(())
}

/// Commit everything the thread changed. `None` when the turn changed nothing.
///
/// Hooks are skipped: this is a snapshot of what the agent did, not a curated commit, and the human sees
/// the diff at the gate. A hanging hook would otherwise freeze the runner.
pub fn commit_all(worktree: &Path, message: &str) -> Result<Option<String>> {
    git_ok(worktree, &["add", "-A"])?;
    let staged = git_ok(worktree, &["diff", "--cached", "--name-only"])?;
    if staged.is_empty() {
        return Ok(None);
    }
    git_ok(
        worktree,
        &[
            "-c", "user.name=Ilvuno",
            "-c", "user.email=ilvuno@localhost",
            "commit", "--no-verify", "-m", message,
        ],
    )?;
    Ok(Some(git_ok(worktree, &["rev-parse", "--short", "HEAD"])?))
}

/// What this thread changed since it started: one line per file with counts.
pub fn diff_stat(worktree: &Path, base: &str) -> Result<String> {
    git_ok(worktree, &["diff", "--stat", &format!("{base}...HEAD")])
}

/// The full patch, for a gate that wants to show it.
pub fn diff(worktree: &Path, base: &str) -> Result<String> {
    git_ok(worktree, &["diff", &format!("{base}...HEAD")])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    /// The project moved to a new commit.
    Merged(String),
    /// The branch had nothing the project did not already have.
    UpToDate,
    /// Files that conflicted. The merge was aborted, so the project is exactly as it was.
    Conflict(Vec<String>),
}

/// Merge `branch` into the project's current branch. Refuses to start on a dirty tree, and leaves the
/// project untouched when the merge conflicts.
pub fn merge(root: &Path, branch: &str, message: &str) -> Result<MergeOutcome> {
    if !is_clean(root)? {
        bail!("the project has uncommitted changes; commit or stash them first");
    }
    let out = git(
        root,
        &[
            "-c", "user.name=Ilvuno",
            "-c", "user.email=ilvuno@localhost",
            "merge", "--no-ff", "--no-edit", "-m", message, branch,
        ],
    )?;
    if out.status.success() {
        let head = head(root)?;
        let short: String = head.chars().take(8).collect();
        return Ok(MergeOutcome::Merged(short));
    }
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stderr.contains("Already up to date") {
        return Ok(MergeOutcome::UpToDate);
    }
    // A conflicted merge leaves the index half-written. Put it back before reporting.
    let conflicts = git_ok(root, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
    let _ = git(root, &["merge", "--abort"]);
    if conflicts.is_empty() {
        bail!("git merge {branch}: {}", stderr.trim());
    }
    Ok(MergeOutcome::Conflict(conflicts.lines().map(str::to_string).collect()))
}

/// Remove a worktree and forget the branch. Safe to call for one that is already gone.
pub fn remove_worktree(root: &Path, path: &Path) -> Result<()> {
    let p = path.to_string_lossy().to_string();
    let _ = git(root, &["worktree", "remove", "--force", &p]);
    // The registration survives a manually deleted folder; prune it so `git worktree list` stays honest.
    let _ = git(root, &["worktree", "prune"]);
    let _ = std::fs::remove_dir_all(path);
    Ok(())
}

pub fn delete_branch(root: &Path, branch: &str) -> Result<()> {
    let _ = git(root, &["branch", "-D", branch]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Repo {
        dir: PathBuf,
    }

    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn repo(name: &str) -> Repo {
        let dir = std::env::temp_dir().join(format!("ilvuno-git-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("mk");
        for args in [
            vec!["init", "-q", "-b", "main"],
            vec!["config", "user.name", "Test"],
            vec!["config", "user.email", "test@example.com"],
        ] {
            git_ok(&dir, &args).expect("git setup");
        }
        std::fs::write(dir.join("readme.md"), "one\n").expect("write");
        git_ok(&dir, &["add", "-A"]).expect("add");
        git_ok(&dir, &["commit", "-q", "-m", "start"]).expect("commit");
        Repo { dir }
    }

    #[test]
    fn the_floor_is_the_one_the_previous_project_measured() {
        // Research 14: 2.38.1 is the floor. Anything older is refused, not guessed at.
        assert!(meets_floor((2, 38, 1)));
        assert!(meets_floor((2, 55, 0)));
        assert!(!meets_floor((2, 38, 0)));
        assert!(!meets_floor((1, 9, 9)));
    }

    #[test]
    fn a_worktree_is_a_real_checkout_and_its_commit_stays_on_its_branch() {
        let r = repo("worktree");
        let base = head(&r.dir).expect("head");
        let path = r.dir.join("..").join(format!("wt-{}", uuid::Uuid::new_v4()));
        let branch = branch_name("0123abcd-rest", "reader-1");
        assert_eq!(branch, "run/0123abcd/reader-1");

        add_worktree(&r.dir, &path, &branch, &base).expect("worktree");
        assert!(path.join("readme.md").is_file(), "the worktree is a checkout of the project");

        std::fs::write(path.join("readme.md"), "one\ntwo\n").expect("write");
        std::fs::write(path.join("new.md"), "hello\n").expect("write");
        let sha = commit_all(&path, "reader-1: two files").expect("commit").expect("a commit");
        assert_eq!(sha.len(), 7);

        // The project has not moved; only the branch has.
        assert_eq!(head(&r.dir).expect("head"), base);
        let stat = diff_stat(&path, &base).expect("stat");
        assert!(stat.contains("new.md"), "stat was {stat:?}");
        assert!(stat.contains("readme.md"), "stat was {stat:?}");

        remove_worktree(&r.dir, &path).expect("remove");
        assert!(!path.exists());
        delete_branch(&r.dir, &branch).expect("delete");
    }

    #[test]
    fn a_turn_that_changed_nothing_makes_no_commit() {
        let r = repo("empty-turn");
        let base = head(&r.dir).expect("head");
        let path = r.dir.join("..").join(format!("wt-{}", uuid::Uuid::new_v4()));
        add_worktree(&r.dir, &path, &branch_name("x", "t"), &base).expect("worktree");
        assert_eq!(commit_all(&path, "nothing").expect("commit"), None);
        remove_worktree(&r.dir, &path).expect("remove");
    }

    #[test]
    fn the_merge_lands_the_branch_and_refuses_a_dirty_project() {
        let r = repo("merge");
        let base = head(&r.dir).expect("head");
        let path = r.dir.join("..").join(format!("wt-{}", uuid::Uuid::new_v4()));
        let branch = branch_name("feedface-1", "writer");
        add_worktree(&r.dir, &path, &branch, &base).expect("worktree");
        std::fs::write(path.join("notes.md"), "notes\n").expect("write");
        commit_all(&path, "writer: notes").expect("commit").expect("sha");

        // A dirty project is refused before git can half-merge anything.
        std::fs::write(r.dir.join("scratch.txt"), "x\n").expect("write");
        let refused = merge(&r.dir, &branch, "land writer").expect_err("dirty tree must refuse");
        assert!(refused.to_string().contains("uncommitted"), "{refused}");
        std::fs::remove_file(r.dir.join("scratch.txt")).expect("clean");

        let outcome = merge(&r.dir, &branch, "land writer").expect("merge");
        assert!(matches!(outcome, MergeOutcome::Merged(_)), "{outcome:?}");
        assert!(r.dir.join("notes.md").is_file(), "the file landed in the project");
        remove_worktree(&r.dir, &path).expect("remove");
    }

    #[test]
    fn a_conflict_is_aborted_and_named() {
        let r = repo("conflict");
        let base = head(&r.dir).expect("head");

        let a = r.dir.join("..").join(format!("wt-a-{}", uuid::Uuid::new_v4()));
        let b = r.dir.join("..").join(format!("wt-b-{}", uuid::Uuid::new_v4()));
        let ba = branch_name("aaa", "one");
        let bb = branch_name("bbb", "two");
        add_worktree(&r.dir, &a, &ba, &base).expect("a");
        add_worktree(&r.dir, &b, &bb, &base).expect("b");
        std::fs::write(a.join("readme.md"), "from one\n").expect("write");
        std::fs::write(b.join("readme.md"), "from two\n").expect("write");
        commit_all(&a, "one").expect("c").expect("sha");
        commit_all(&b, "two").expect("c").expect("sha");

        assert!(matches!(merge(&r.dir, &ba, "land one").expect("first merge"), MergeOutcome::Merged(_)));
        let second = merge(&r.dir, &bb, "land two").expect("second merge");
        match second {
            MergeOutcome::Conflict(files) => assert_eq!(files, vec!["readme.md".to_string()]),
            other => panic!("expected a conflict, got {other:?}"),
        }
        // Aborted, not half-merged: the project still holds the first branch's version, and is clean.
        assert_eq!(std::fs::read_to_string(r.dir.join("readme.md")).expect("read"), "from one\n");
        assert!(is_clean(&r.dir).expect("clean"), "the project is clean after an aborted merge");

        remove_worktree(&r.dir, &a).expect("rm a");
        remove_worktree(&r.dir, &b).expect("rm b");
    }
}
