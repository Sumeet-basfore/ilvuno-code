//! Stage 2: a thread works in a git worktree of the project, on its own branch, and the project's own
//! HEAD never moves. Folder mode is the fallback and still works exactly as before.

mod common;

use common::{get, harness, post, start_run, two_task_script, wait_status, GOAL_WITH_ORCHESTRATOR};
use runner::{git, Home, Store};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

fn git_cmd(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").current_dir(dir).args(args).output().expect("run git");
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn make_repo(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).expect("repo dir");
    let dir = std::fs::canonicalize(dir).expect("canonical repo");
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.name", "Test"],
        vec!["config", "user.email", "test@example.com"],
    ] {
        git_cmd(&dir, &args);
    }
    std::fs::write(dir.join("readme.md"), "start\n").expect("readme");
    git_cmd(&dir, &["add", "-A"]);
    git_cmd(&dir, &["commit", "-q", "-m", "start"]);
    dir
}

fn goal(base: &Path) -> String {
    json!({
        "base": base.to_string_lossy(),
        "goal": "Write notes for one source.",
        "done": "notes.md exists.",
        "providers": { "claude-main": { "driver": "claude" } },
        "threads": [
            { "name": "orchestrator", "agent": "claude-main", "prompt": "Plan the work." },
            { "name": "reader-1", "agent": "claude-main", "prompt": "You read the source and write notes." }
        ],
        "budget": { "rounds": 3, "tokens": 600000, "max_usd_per_call": 0.2 }
    })
    .to_string()
}

fn thread<'a>(run: &'a Value, name: &str) -> &'a Value {
    run["threads"].as_array().expect("threads").iter().find(|t| t["name"] == name).expect("thread")
}

fn commits_since(worktree: &Path, base: &str) -> usize {
    git_cmd(worktree, &["rev-list", "--count", &format!("{base}..HEAD")]).parse().expect("count")
}

fn run_git(home: &Home, id: &str) -> (Option<String>, Option<String>) {
    let store = Store::open(&home.db_path()).expect("store");
    store
        .conn()
        .query_row("SELECT git_root, git_base FROM runs WHERE id = ?1", rusqlite::params![id], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .expect("git columns")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_scripted_agent_commits_on_its_branch_and_the_project_does_not_move() {
    let h = harness();
    let repo = make_repo(&h.dir.join("repo"));
    let base = git::head(&repo).expect("base");
    let id = start_run(&h.app, &goal(&repo)).await;
    wait_status(&h.app, &id, "reviewed").await;

    let worktree = h.home.worktree_dir(&id, "reader-1");
    assert!(worktree.is_dir(), "the checkout was created under the home");
    assert!(worktree.join("notes.md").is_file(), "the agent wrote in its checkout");
    assert_eq!(git::head(&repo).expect("project head"), base, "the project's HEAD did not move");
    assert_eq!(commits_since(&worktree, &base), 1, "one commit for one task");
    assert!(git_cmd(&worktree, &["log", "--format=%s", "-1"]).contains("reader-1: task"));
    assert!(!repo.join("notes.md").exists(), "nothing landed in the project");

    let run = get(&h.app, &format!("/runs/{id}")).await.json;
    let reader = thread(&run, "reader-1");
    let branch = git::branch_name(&id, "reader-1");
    assert_eq!(reader["branch"].as_str(), Some(branch.as_str()));
    assert_eq!(reader["has_checkout"], true, "{reader}");
    assert!(h.fake.spawns().iter().any(|s| s.cwd == worktree), "the provider ran in the worktree");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_second_task_reuses_the_checkout_and_adds_a_commit() {
    let h = harness();
    h.fake.set_prompt_hook(two_task_script(h.home.clone()));
    let repo = make_repo(&h.dir.join("repo"));
    let base = git::head(&repo).expect("base");
    let id = start_run(&h.app, &goal(&repo)).await;
    wait_status(&h.app, &id, "reviewed").await;

    let worktree = h.home.worktree_dir(&id, "reader-1");
    assert!(worktree.join("notes.md").is_file(), "the first task's file survived");
    assert!(worktree.join("second.md").is_file(), "the second task wrote in the same checkout");
    assert_eq!(commits_since(&worktree, &base), 2, "one commit per task");
    assert_eq!(git::head(&repo).expect("project head"), base, "the project's HEAD did not move");
    let subjects = git_cmd(&worktree, &["log", "--format=%s", &format!("{base}..HEAD")]);
    assert_eq!(subjects.lines().count(), 2, "{subjects}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_non_git_base_stays_in_folder_mode() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WITH_ORCHESTRATOR).await;
    wait_status(&h.app, &id, "reviewed").await;

    let run = get(&h.app, &format!("/runs/{id}")).await.json;
    let reader = thread(&run, "reader-1");
    assert!(reader["branch"].is_null(), "{reader}");
    assert_eq!(reader["has_checkout"], false);
    assert!(h.home.threads_dir(&id).join("reader-1").join("notes.md").is_file(), "the folder is the checkout");
    let (root, base) = run_git(&h.home, &id);
    assert!(root.is_none() && base.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_dirty_project_falls_back_to_folder_mode_and_the_journal_says_why() {
    let h = harness();
    let repo = make_repo(&h.dir.join("repo"));
    std::fs::write(repo.join("scratch.txt"), "dirty\n").expect("dirty file");
    let base = git::head(&repo).expect("base");
    let id = start_run(&h.app, &goal(&repo)).await;
    wait_status(&h.app, &id, "reviewed").await;

    let run = get(&h.app, &format!("/runs/{id}")).await.json;
    let reader = thread(&run, "reader-1");
    assert!(reader["branch"].is_null(), "{reader}");
    assert_eq!(reader["has_checkout"], false);
    assert!(h.home.threads_dir(&id).join("reader-1").join("notes.md").is_file(), "the folder is the checkout");
    assert!(!repo.join("notes.md").exists(), "nothing landed in the project");
    assert!(repo.join("scratch.txt").is_file(), "the dirty file was not touched");
    assert_eq!(git::head(&repo).expect("project head"), base);

    let (root, gitbase) = run_git(&h.home, &id);
    assert!(root.is_none() && gitbase.is_none(), "no git columns on a fallback run");
    let store = Store::open(&h.home.db_path()).expect("store");
    let payload: String = store
        .conn()
        .query_row(
            "SELECT payload FROM journal WHERE run_id = ?1 AND kind = 'git.fallback'",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .expect("the fallback is journaled");
    assert!(payload.contains("uncommitted"), "the journal says why: {payload}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_merges_the_thread_change_and_records_the_run_in_git() {
    let h = harness();
    let repo = make_repo(&h.dir.join("merge-repo"));
    let id = start_run(&h.app, &goal(&repo)).await;
    wait_status(&h.app, &id, "reviewed").await;

    let diff = get(&h.app, &format!("/runs/{id}/threads/reader-1/diff")).await;
    assert_eq!(diff.status, axum::http::StatusCode::OK, "{}", diff.json);
    assert!(diff.json["branch"].as_str().unwrap().contains("reader-1"));
    assert!(diff.json["base"].is_string());
    assert!(diff.json["stat"].as_str().unwrap().contains("notes.md"));
    assert!(diff.json["patch"].as_str().unwrap().contains("notes"));
    assert_eq!(diff.json["truncated"], false);
    let thread = thread(&get(&h.app, &format!("/runs/{id}")).await.json, "reader-1").clone();
    assert!(thread["commit"].is_string());
    assert!(thread["stat"].as_str().unwrap().contains("notes.md"));

    let accepted = post(&h.app, &format!("/runs/{id}/gate"), r#"{"decision":"accept"}"#).await;
    assert_eq!(accepted.status, axum::http::StatusCode::OK, "{}", accepted.json);
    assert_eq!(accepted.json["status"], "merged");
    assert_eq!(std::fs::read_to_string(repo.join("notes.md")).expect("landed notes"), "notes\n");
    let log = git_cmd(&repo, &["log", "--format=%s"]);
    assert!(log.contains(&id), "merge commit should name run {id}: {log}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_conflict_aborts_and_blocks_with_the_file_name() {
    let h = harness();
    let repo = make_repo(&h.dir.join("conflict-repo"));
    let id = start_run(&h.app, &goal(&repo)).await;
    wait_status(&h.app, &id, "reviewed").await;

    std::fs::write(repo.join("notes.md"), "project version\n").expect("project edit");
    git_cmd(&repo, &["add", "notes.md"]);
    git_cmd(&repo, &["commit", "-q", "-m", "project edit after run start"]);
    let head_before = git::head(&repo).expect("head before merge");
    let body_before = std::fs::read_to_string(repo.join("notes.md")).expect("project notes");

    let reply = post(&h.app, &format!("/runs/{id}/gate"), r#"{"decision":"accept"}"#).await;
    assert_eq!(reply.status, axum::http::StatusCode::OK, "{}", reply.json);
    assert_eq!(reply.json["status"], "blocked");
    let detail = get(&h.app, &format!("/runs/{id}")).await.json;
    assert!(detail["run"]["blocked_reason"].as_str().unwrap_or_default().contains("notes.md"));
    assert_eq!(git::head(&repo).expect("head after abort"), head_before);
    assert_eq!(std::fs::read_to_string(repo.join("notes.md")).expect("project notes"), body_before);
    assert!(git::is_clean(&repo).expect("clean after abort"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dirty_project_refuses_accept_before_any_merge() {
    let h = harness();
    let repo = make_repo(&h.dir.join("dirty-repo"));
    let id = start_run(&h.app, &goal(&repo)).await;
    wait_status(&h.app, &id, "reviewed").await;
    let base = git::head(&repo).expect("base");
    std::fs::write(repo.join("local.txt"), "keep me\n").expect("dirty project");

    let reply = post(&h.app, &format!("/runs/{id}/gate"), r#"{"decision":"accept"}"#).await;
    assert_eq!(reply.status, axum::http::StatusCode::CONFLICT, "{}", reply.json);
    assert!(reply.json["error"].as_str().unwrap_or_default().contains("local.txt"));
    assert_eq!(git::head(&repo).expect("head unchanged"), base);
    assert!(!repo.join("notes.md").exists());
    assert_eq!(get(&h.app, &format!("/runs/{id}")).await.json["run"]["status"], "reviewed");
    let store = Store::open(&h.home.db_path()).expect("store");
    let landed: i64 = store.conn().query_row(
        "SELECT COUNT(*) FROM journal WHERE run_id = ?1 AND kind = 'merge.landed'", rusqlite::params![id], |r| r.get(0),
    ).expect("merge event count");
    assert_eq!(landed, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accepting_twice_lands_a_thread_branch_only_once() {
    let h = harness();
    let repo = make_repo(&h.dir.join("twice-repo"));
    let id = start_run(&h.app, &goal(&repo)).await;
    wait_status(&h.app, &id, "reviewed").await;
    for _ in 0..2 {
        let reply = post(&h.app, &format!("/runs/{id}/gate"), r#"{"decision":"accept"}"#).await;
        assert_eq!(reply.status, axum::http::StatusCode::OK, "{}", reply.json);
        assert_eq!(reply.json["status"], "merged");
    }
    let log = git_cmd(&repo, &["log", "--format=%s", &format!("--grep=Ilvuno run {id}: merge reader-1")]);
    assert_eq!(log.lines().count(), 1, "{log}");
    let store = Store::open(&h.home.db_path()).expect("store");
    let landed: i64 = store.conn().query_row(
        "SELECT COUNT(*) FROM journal WHERE run_id = ?1 AND kind = 'merge.landed'", rusqlite::params![id], |r| r.get(0),
    ).expect("merge event count");
    assert!(landed >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn folder_mode_accept_records_the_decision_without_merging() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WITH_ORCHESTRATOR).await;
    wait_status(&h.app, &id, "reviewed").await;
    let diff = get(&h.app, &format!("/runs/{id}/threads/reader-1/diff")).await;
    assert_eq!(diff.status, axum::http::StatusCode::OK);
    assert_eq!(diff.json, json!({ "branch": null }));
    let accepted = post(&h.app, &format!("/runs/{id}/gate"), r#"{"decision":"accept"}"#).await;
    assert_eq!(accepted.status, axum::http::StatusCode::OK, "{}", accepted.json);
    assert_eq!(accepted.json["status"], "accepted");
    let store = Store::open(&h.home.db_path()).expect("store");
    let landed: i64 = store.conn().query_row(
        "SELECT COUNT(*) FROM journal WHERE run_id = ?1 AND kind IN ('merge.landed', 'merge.conflict')", rusqlite::params![id], |r| r.get(0),
    ).expect("merge event count");
    assert_eq!(landed, 0);
}
