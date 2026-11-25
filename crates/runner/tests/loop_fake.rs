//! The run loop end to end on the scripted provider: zero quota. The fake agents play their part by calling
//! the same functions the `ilvuno` commands call (`run_task`, `run_report`), so the behaviour under test is
//! the behaviour a real agent gets.

use provider_core::fake::FakeProvider;
use provider_core::{Decision, Event};
use runner::commands::{self, ReportStatus};
use runner::driver::{Driver, RunEnd};
use runner::goal::{self, GoalPlan};
use runner::replay;
use runner::report::run_report;
use runner::run_status::{self, GateDecision};
use runner::store::Outcome;
use runner::tasks::run_task;
use runner::{Home, RunStatus, Store};
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const GOOD_BRIEF: &str = "<!doctype html><html><body><p>Both notes were read.</p></body></html>";

/// Plans for the orchestrator, one entry per orchestrator turn: the tasks it creates as (thread, prompt, scope).
type Plan = Vec<(String, String, Vec<String>)>;

#[derive(Clone, Default)]
struct Script {
    plans: Arc<Mutex<VecDeque<Plan>>>,
    /// Content for each write of brief.html, in order. Once empty, GOOD_BRIEF is written.
    briefs: Arc<Mutex<VecDeque<String>>>,
    /// Optional content for other files, consumed in write order.
    contents: Arc<Mutex<VecDeque<String>>>,
}

impl Script {
    fn with_plans(plans: Vec<Plan>) -> Self {
        Self { plans: Arc::new(Mutex::new(plans.into())), ..Self::default() }
    }

    fn with_briefs(self, briefs: &[&str]) -> Self {
        *self.briefs.lock().unwrap() = briefs.iter().map(|b| b.to_string()).collect();
        self
    }

    fn with_contents(self, contents: &[&str]) -> Self {
        *self.contents.lock().unwrap() = contents.iter().map(|b| b.to_string()).collect();
        self
    }
}

struct Env {
    dir: PathBuf,
    home: Home,
    store: Store,
    run_id: String,
    plan: GoalPlan,
    provider: FakeProvider,
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A run from a goal file written into a scratch folder. Creating the run starts its orchestrator task.
fn setup(name: &str, goal_json: &str, script: Script) -> Env {
    let dir = std::env::temp_dir().join(format!("ilvuno-loop-{name}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let goal_path = dir.join("goal.json");
    std::fs::write(&goal_path, goal_json).expect("goal file");
    let plan = GoalPlan::load(&goal_path).expect("goal");
    let home = Home::new(dir.join("home"));
    let mut store = Store::open(&home.db_path()).expect("store");
    let raw = goal_json.to_string();
    let base = dir.clone();
    let new_goal = plan.new_goal().expect("new goal");
    let run_id = match store
        .transact("run.create", None, |tx, p| {
            let id = commands::create_run(tx, p, &new_goal, &home)?;
            goal::snapshot(tx, &id, &raw, &base)?;
            Ok(Outcome::Applied(id))
        })
        .expect("create run")
    {
        Outcome::Applied(id) => id,
        other => panic!("{other:?}"),
    };
    let provider = FakeProvider::new();
    provider.set_prompt_hook(agent(home.clone(), script));
    // Enough finished turns for every task the scenario starts. Each turn ends at once: the agent reports
    // from inside the hook, before the events are read.
    for _ in 0..30 {
        provider.push_turn(vec![said("I did the task in my folder."), usage(), finished()]);
    }
    Env { dir, home, store, run_id, plan, provider }
}

fn git_in(dir: &std::path::Path, args: &[&str]) {
    let out = std::process::Command::new("git").args(args).current_dir(dir).output().expect("run git");
    assert!(out.status.success(), "git {args:?} failed: {}", String::from_utf8_lossy(&out.stderr));
}

/// A run whose project is a clean git repository, so every thread gets a worktree. The goal file sits
/// beside the repository, never inside it, or the repository would be dirty and the run would fall back
/// to folder mode.
fn setup_git(name: &str, goal_json: &str, script: Script) -> Env {
    let dir = std::env::temp_dir().join(format!("ilvuno-git-{name}-{}", uuid::Uuid::new_v4()));
    let repo = dir.join("repo");
    std::fs::create_dir_all(&repo).expect("repo dir");
    git_in(&repo, &["init", "-q"]);
    std::fs::write(repo.join("README.md"), "the project\n").expect("readme");
    git_in(&repo, &["add", "-A"]);
    git_in(&repo, &["-c", "user.name=test", "-c", "user.email=test@example.invalid", "commit", "-qm", "first"]);

    let goal_path = dir.join("goal.json");
    std::fs::write(&goal_path, goal_json).expect("goal file");
    let plan = GoalPlan::load(&goal_path).expect("goal");
    assert_eq!(plan.base(), repo.as_path(), "the goal's base is the repository");
    let home = Home::new(dir.join("home"));
    let mut store = Store::open(&home.db_path()).expect("store");
    let raw = goal_json.to_string();
    let new_goal = plan.new_goal().expect("new goal");
    let project = runner::git::project_git(plan.base());
    assert!(matches!(project, runner::git::ProjectGit::Eligible { .. }), "the run has to be a git run");
    let run_id = match store
        .transact("run.create", None, |tx, p| {
            let id = commands::create_run(tx, p, &new_goal, &home)?;
            commands::record_run_git(tx, p, &id, &new_goal.threads, &project)?;
            goal::snapshot(tx, &id, &raw, &repo)?;
            Ok(Outcome::Applied(id))
        })
        .expect("create run")
    {
        Outcome::Applied(id) => id,
        other => panic!("{other:?}"),
    };
    let provider = FakeProvider::new();
    provider.set_prompt_hook(agent(home.clone(), script));
    for _ in 0..10 {
        provider.push_turn(vec![said("I did the task in my folder."), usage(), finished()]);
    }
    Env { dir, home, store, run_id, plan, provider }
}

/// The scripted agent. It reads the prompt it is given, as a real agent would, and acts on it.
fn agent(home: Home, script: Script) -> Box<dyn FnMut(&str) + Send> {
    Box::new(move |prompt: &str| {
        let task_id = between(prompt, "Your task id is ", ".");
        if prompt.contains("You are the orchestrator") {
            let next = script.plans.lock().unwrap().pop_front().unwrap_or_default();
            for (thread, text, scope) in next {
                let outcome = run_task(&home, Some(&task_id), &thread, &text, &scope).expect("task command");
                assert!(matches!(outcome, Outcome::Applied(_)), "orchestrator task refused: {outcome:?}");
            }
            report(&home, &task_id, ReportStatus::Ok, &[]);
            return;
        }
        let folder = PathBuf::from(between(prompt, "Work only inside your folder: ", ". You may read"));
        let scope: Vec<String> = line_after(prompt, "(the only files and sources this task may touch): ")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        std::fs::create_dir_all(&folder).expect("folder");
        if scope.iter().any(|f| f == "fail-me.md") {
            report(&home, &task_id, ReportStatus::Failed, &[]);
            return;
        }
        for file in &scope {
            let body = if file == "brief.html" {
                script.briefs.lock().unwrap().pop_front().unwrap_or_else(|| GOOD_BRIEF.to_string())
            } else {
                script.contents.lock().unwrap().pop_front().unwrap_or_else(|| format!("notes for {file}\n"))
            };
            std::fs::write(folder.join(file), body).expect("write");
        }
        report(&home, &task_id, ReportStatus::Ok, &scope);
    })
}

fn report(home: &Home, task_id: &str, status: ReportStatus, files: &[String]) {
    let outcome = run_report(home, task_id, status, "three sentences of summary.", files).expect("report command");
    assert!(matches!(outcome, Outcome::Applied(())), "report refused: {outcome:?}");
}

fn between(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).expect("start marker") + start.len();
    let to = text[from..].find(end).expect("end marker") + from;
    text[from..to].to_string()
}

fn line_after(text: &str, marker: &str) -> String {
    let from = text.find(marker).expect("marker") + marker.len();
    text[from..].lines().next().unwrap_or_default().to_string()
}

fn usage() -> Event {
    Event::Usage { tokens_in: 900, tokens_out: 150, cache_read_tokens: 0, context_used: None, cost_usd: Some(0.01) }
}

/// What the agent says. Only the final text per task is stored; deltas are summed into it.
fn said(text: &str) -> Event {
    Event::TextDelta { text: text.to_string() }
}

fn finished() -> Event {
    Event::TurnFinished { stop_reason: Some("end_turn".into()), interrupted: false, error: None }
}

fn instances(provider: &FakeProvider, plan: &GoalPlan) -> BTreeMap<String, FakeProvider> {
    plan.providers().keys().map(|id| (id.clone(), provider.clone())).collect()
}

async fn drive(env: &mut Env) -> RunEnd {
    let providers = instances(&env.provider, &env.plan);
    let driver = Driver { providers: &providers, home: &env.home, ilvuno_bin_dir: None, max_usd: 0.2 };
    driver.drive(&mut env.store, &env.plan, &env.run_id).await.expect("drive")
}

fn count(store: &Store, sql: &str) -> i64 {
    store.conn().query_row(sql, [], |r| r.get(0)).expect("count")
}

fn status(store: &Store, run: &str) -> String {
    store.conn().query_row("SELECT status FROM runs WHERE id = ?1", [run], |r| r.get(0)).expect("status")
}

fn verdicts(store: &Store, run: &str) -> Vec<Option<String>> {
    let replayed = replay::replay(store.conn(), run).expect("replay");
    replayed.rounds.iter().map(|r| r.verdict.clone()).collect()
}

fn readiness(store: &Store, round_index: u32) -> serde_json::Value {
    let raw: String = store
        .conn()
        .query_row("SELECT readiness FROM rounds WHERE \"index\" = ?1", [round_index], |r| r.get(0))
        .expect("readiness");
    serde_json::from_str(&raw).expect("readiness JSON")
}

fn assert_projection_matches(env: &Env) {
    let replayed = replay::replay(env.store.conn(), &env.run_id).expect("replay");
    let problems = replay::compare_with_projection(env.store.conn(), &env.run_id, &replayed).expect("compare");
    assert_eq!(problems, Vec::<String>::new(), "journal and projection disagree");
}

/// The orchestrator, two readers, and two checks: brief.html must exist and say nothing unfinished.
const TWO_READERS_GOAL: &str = r#"{
  "goal": "read two sources and write one short brief",
  "done": "brief.html exists and has no placeholder text",
  "checks": [
    {"type": "file_exists", "path": "brief.html"},
    {"type": "text_absent", "path": "brief.html", "needles": ["TODO", "TBD"]}
  ],
  "providers": {"claude-main": {"driver": "claude"}},
  "threads": [
    {"name": "orchestrator", "agent": "claude-main", "prompt": "Plan the work. Give each reader one source."},
    {"name": "reader-1", "agent": "claude-main", "prompt": "Read your source and write notes."},
    {"name": "reader-2", "agent": "claude-main", "prompt": "Read your source and write notes."}
  ],
  "budget": {"rounds": 3, "tokens": 600000}
}"#;

const NO_ORCHESTRATOR_GOAL: &str = r#"{
  "goal": "read a source",
  "providers": {"claude-main": {"driver": "claude"}},
  "threads": [{"name": "reader-1", "agent": "claude-main", "prompt": "Read and write notes."}],
  "budget": {"rounds": 2, "tokens": 600000}
}"#;

/// A check that must see what the agent wrote. The project is a real git repository, so the thread works
/// in its own worktree and the check has to run in the tree the gate would merge.
const GIT_PROJECT_GOAL: &str = r#"{
  "goal": "write the brief in the project",
  "done": "brief.md is in the project",
  "base": "repo",
  "checks": [{"type": "command_exit_zero", "command": "test -f brief.md"}],
  "providers": {"claude-main": {"driver": "claude"}},
  "threads": [{"name": "writer", "agent": "claude-main", "prompt": "Write brief.md."}],
  "budget": {"rounds": 2, "tokens": 600000}
}"#;

const TWO_WRITERS_GOAL: &str = r#"{
  "goal": "write two files in the project",
  "done": "both files exist",
  "base": "repo",
  "checks": [{"type": "command_exit_zero", "command": "test -f a.md -a -f b.md"}],
  "providers": {"claude-main": {"driver": "claude"}},
  "threads": [
    {"name": "writer-a", "agent": "claude-main", "prompt": "Write a.md."},
    {"name": "writer-b", "agent": "claude-main", "prompt": "Write b.md."}
  ],
  "budget": {"rounds": 2, "tokens": 600000}
}"#;

const IDLE_PLANNED_WRITER_GOAL: &str = r#"{
  "goal": "write one file in the project",
  "done": "the file exists",
  "base": "repo",
  "checks": [{"type": "command_exit_zero", "command": "test -f a.md"}],
  "providers": {"claude-main": {"driver": "claude"}},
  "threads": [
    {"name": "writer", "agent": "claude-main", "prompt": "Write a.md."},
    {"name": "idle", "agent": "claude-main", "prompt": "Do nothing."}
  ],
  "budget": {"rounds": 2, "tokens": 600000}
}"#;

const CONFLICTING_WRITERS_GOAL: &str = r#"{
  "goal": "write the project README",
  "done": "the README exists",
  "base": "repo",
  "checks": [{"type": "command_exit_zero", "command": "test -f README.md"}],
  "providers": {"claude-main": {"driver": "claude"}},
  "threads": [
    {"name": "writer-a", "agent": "claude-main", "prompt": "Write the README."},
    {"name": "writer-b", "agent": "claude-main", "prompt": "Write the README."}
  ],
  "budget": {"rounds": 2, "tokens": 600000}
}"#;

const RETRY_CHECK_GOAL: &str = r#"{
  "goal": "write GOOD into brief.md",
  "done": "brief.md contains GOOD",
  "base": "repo",
  "checks": [{"type": "command_exit_zero", "command": "grep -q GOOD brief.md"}],
  "providers": {"claude-main": {"driver": "claude"}},
  "threads": [{"name": "writer", "agent": "claude-main", "prompt": "Write brief.md."}],
  "budget": {"rounds": 2, "tokens": 600000}
}"#;

fn plan_of(tasks: &[(&str, &str, &[&str])]) -> Plan {
    tasks
        .iter()
        .map(|(thread, text, scope)| (thread.to_string(), text.to_string(), scope.iter().map(|s| s.to_string()).collect()))
        .collect()
}

#[tokio::test]
async fn orchestrator_plans_checks_fail_then_it_is_woken_and_the_run_reaches_the_gate() {
    let script = Script::with_plans(vec![
        plan_of(&[("reader-1", "read source A", &["notes-a.md"]), ("reader-2", "read source B", &["notes-b.md"])]),
        plan_of(&[("reader-1", "write the brief from both notes", &["brief.html"])]),
    ]);
    let mut env = setup("happy", TWO_READERS_GOAL, script);
    assert_eq!(drive(&mut env).await, RunEnd::AtGate);

    assert_eq!(status(&env.store, &env.run_id), "reviewed");
    assert_eq!(verdicts(&env.store, &env.run_id), vec![Some("block".to_string()), Some("pass".to_string())]);
    // The failed round woke the orchestrator with the failing check named in its prompt.
    let wake = env.provider.prompts().into_iter().find(|p| p.contains("did not pass")).expect("wake prompt");
    assert!(wake.contains("file_exists brief.html: missing"), "wake prompt: {wake}");
    // Who wrote each task: the user (kickoff), the orchestrator (readers and brief), the runner (the wake).
    assert_eq!(count(&env.store, "SELECT COUNT(*) FROM dispatch WHERE created_by = 'user'"), 1);
    assert_eq!(count(&env.store, "SELECT COUNT(*) FROM dispatch WHERE created_by = 'orchestrator'"), 3);
    assert_eq!(count(&env.store, "SELECT COUNT(*) FROM dispatch WHERE created_by = 'runner'"), 1);
    assert!(env.home.artifacts_dir(&env.run_id).join("brief.html").is_file(), "the deliverable is kept");
    // Only the orchestrator's sessions carry ILVUNO_TASK; a worker cannot create tasks.
    let spawns = env.provider.spawns();
    assert_eq!(spawns.len(), 5, "orchestrator kickoff, two readers, orchestrator wake, brief writer");
    assert_eq!(spawns.iter().filter(|s| s.env.iter().any(|(k, _)| k == "ILVUNO_TASK")).count(), 2);
    // What a thread said is kept once per task, so a thread view has something to show.
    assert_eq!(count(&env.store, "SELECT COUNT(*) FROM dispatch WHERE result_text LIKE 'I did the task%'"), 5,
        "every finished task keeps its final message");
    assert_eq!(count(&env.store, "SELECT COUNT(*) FROM journal WHERE kind = 'dispatch.message'"), 5);
    // The board names each thread and each author, so a person can follow it without a transcript.
    let board = std::fs::read_to_string(env.home.board_path(&env.run_id)).expect("board");
    assert!(board.contains("by reader-1") && board.contains("by orchestrator") && board.contains("by user"), "{board}");

    let accepted = env
        .store
        .transact("gate.decide", None, |tx, p| run_status::gate_decide(tx, p, &env.run_id, &GateDecision::Accept))
        .expect("accept");
    assert_eq!(accepted, Outcome::Applied(()));
    assert_eq!(status(&env.store, &env.run_id), RunStatus::Accepted.as_str());
    assert_projection_matches(&env);
}

#[tokio::test]
async fn identical_failures_in_two_rounds_stop_the_run_as_no_progress() {
    let script = Script::with_plans(vec![
        plan_of(&[("reader-1", "read source A", &["notes-a.md"])]),
        plan_of(&[("reader-1", "read source A again", &["notes-b.md"])]),
    ]);
    let mut env = setup("stall", TWO_READERS_GOAL, script);
    let end = drive(&mut env).await;
    assert_eq!(end, RunEnd::Stopped { status: RunStatus::NoProgress, reason: "same failing checks two rounds running".into() });
    assert_projection_matches(&env);
}

#[tokio::test]
async fn a_round_past_the_budget_is_refused_and_the_run_says_why() {
    let goal = TWO_READERS_GOAL.replace("\"rounds\": 3", "\"rounds\": 2");
    let script = Script::with_plans(vec![
        plan_of(&[("reader-1", "read source A", &["notes-a.md"])]),
        plan_of(&[("reader-1", "write the brief", &["brief.html"])]),
    ])
    .with_briefs(&["<p>TODO: finish this</p>"]);
    let mut env = setup("budget", &goal, script);
    match drive(&mut env).await {
        RunEnd::Stopped { status, reason } => {
            assert_eq!(status, RunStatus::Blocked);
            assert!(reason.contains("2 of 2"), "reason: {reason}");
        }
        other => panic!("a third round must be refused, got {other:?}"),
    }
    assert_eq!(count(&env.store, "SELECT COUNT(*) FROM rounds"), 2);
    assert!(env.home.artifacts_dir(&env.run_id).join("brief.html").is_file(), "work is kept after a refusal");
    assert_projection_matches(&env);
}

#[tokio::test]
async fn with_no_orchestrator_the_user_adds_tasks_and_a_send_back_waits_for_them() {
    let mut env = setup("user", NO_ORCHESTRATOR_GOAL, Script::default());
    // Nothing to do until the user writes a task: the run waits.
    assert_eq!(drive(&mut env).await, RunEnd::Idle);

    let notes = vec!["notes.md".to_string()];
    let first = run_task(&env.home, None, "reader-1", "read the source", &notes).expect("task");
    assert!(matches!(first, Outcome::Applied(_)), "{first:?}");
    assert_eq!(drive(&mut env).await, RunEnd::AtGate, "no checks, so the first round passes to the gate");

    let back = GateDecision::SendBack { note: "say more about the second section".into() };
    let run_id = env.run_id.clone();
    env.store.transact("gate.decide", None, |tx, p| run_status::gate_decide(tx, p, &run_id, &back)).expect("send back");
    assert_eq!(drive(&mut env).await, RunEnd::Idle, "the send-back waits for the user's next task");
    assert_eq!(count(&env.store, "SELECT COUNT(*) FROM ledger WHERE kind = 'constraint'"), 1);

    let second = run_task(&env.home, None, "reader-1", "say more about section two", &notes).expect("task");
    assert!(matches!(second, Outcome::Applied(ref t) if t.round_index == 1), "{second:?}");
    assert_eq!(drive(&mut env).await, RunEnd::AtGate);
    // The send-back recorded round 0 as blocked by the human, so the journal shows block, then pass.
    assert_eq!(verdicts(&env.store, &env.run_id), vec![Some("block".to_string()), Some("pass".to_string())]);

    let back_again = GateDecision::SendBack { note: "again".into() };
    let run_id = env.run_id.clone();
    env.store.transact("gate.decide", None, |tx, p| run_status::gate_decide(tx, p, &run_id, &back_again)).expect("send back");
    // Two rounds used of two: the next task is refused, and the refusal says so.
    match run_task(&env.home, None, "reader-1", "one more try", &notes).expect("task command") {
        Outcome::Refused(reason) => assert!(reason.contains("2 of 2"), "reason: {reason}"),
        other => panic!("a third round must be refused, got {other:?}"),
    }
    assert_projection_matches(&env);
}

#[tokio::test]
async fn a_reported_failure_fails_the_round_and_names_the_thread() {
    let mut env = setup("failed-task", NO_ORCHESTRATOR_GOAL, Script::default());
    let scope = vec!["fail-me.md".to_string()];
    assert!(matches!(run_task(&env.home, None, "reader-1", "try it", &scope).expect("task"), Outcome::Applied(_)));
    assert_eq!(drive(&mut env).await, RunEnd::Idle, "no orchestrator to wake: the user decides");
    let readiness: String = env
        .store
        .conn()
        .query_row("SELECT readiness FROM rounds WHERE \"index\" = 0", [], |r| r.get(0))
        .expect("readiness");
    assert!(readiness.contains("task-failed:reader-1"), "{readiness}");
    assert_eq!(verdicts(&env.store, &env.run_id), vec![Some("block".to_string())]);
}

#[tokio::test]
async fn a_restart_reruns_only_the_task_that_was_running_when_the_process_died() {
    let goal = r#"{
      "goal": "two readers", "providers": {"claude-main": {"driver": "claude"}},
      "threads": [{"name": "reader-a", "agent": "claude-main"}, {"name": "reader-b", "agent": "claude-main"}],
      "budget": {"rounds": 2, "tokens": 600000}
    }"#;
    let mut env = setup("resume", goal, Script::default());
    let a = vec!["a.md".to_string()];
    let b = vec!["b.md".to_string()];
    let task_a = applied(run_task(&env.home, None, "reader-a", "read a", &a).expect("task command"));
    let task_b = applied(run_task(&env.home, None, "reader-b", "read b", &b).expect("task command"));

    // Before the crash: task a finished and reported; task b had started and never reported.
    let run_id = env.run_id.clone();
    let a_id = task_a.task_id.clone();
    env.store.transact("dispatch.start", None, |tx, p| commands::start_dispatch(tx, p, &run_id, &a_id)).expect("start a");
    run_report(&env.home, &a_id, ReportStatus::Ok, "a done", &a).expect("report a");
    let b_id = task_b.task_id.clone();
    env.store.transact("dispatch.start", None, |tx, p| commands::start_dispatch(tx, p, &run_id, &b_id)).expect("start b");

    // After the restart, the loop finishes the round without running a again.
    assert_eq!(drive(&mut env).await, RunEnd::AtGate);
    let prompts = env.provider.prompts();
    assert_eq!(prompts.iter().filter(|p| p.contains("Your thread is reader-a.")).count(), 0, "reader-a must not re-run");
    assert_eq!(prompts.iter().filter(|p| p.contains("Your thread is reader-b.")).count(), 1);
    let replayed = replay::replay(env.store.conn(), &env.run_id).expect("replay");
    assert!(replayed.transitions.iter().any(|t| t.contains("dispatch.requeued")), "{:?}", replayed.transitions);
    assert_projection_matches(&env);
}

#[tokio::test]
async fn each_thread_runs_on_the_agent_it_names() {
    let goal = r#"{
      "goal": "route each thread", "providers": {"fake-a": {"driver": "claude"}, "fake-b": {"driver": "claude"}},
      "threads": [{"name": "reader-a", "agent": "fake-a"}, {"name": "reader-b", "agent": "fake-b"}],
      "budget": {"rounds": 2, "tokens": 600000}
    }"#;
    let mut env = setup("routing", goal, Script::default());
    let a = vec!["a.md".to_string()];
    let b = vec!["b.md".to_string()];
    applied(run_task(&env.home, None, "reader-a", "read a", &a).expect("task command"));
    applied(run_task(&env.home, None, "reader-b", "read b", &b).expect("task command"));

    let (fa, fb) = (FakeProvider::new(), FakeProvider::new());
    for p in [&fa, &fb] {
        p.set_prompt_hook(agent(env.home.clone(), Script::default()));
        for _ in 0..4 {
            p.push_turn(vec![usage(), finished()]);
        }
    }
    let providers = BTreeMap::from([("fake-a".to_string(), fa.clone()), ("fake-b".to_string(), fb.clone())]);
    let driver = Driver { providers: &providers, home: &env.home, ilvuno_bin_dir: None, max_usd: 0.2 };
    assert_eq!(driver.drive(&mut env.store, &env.plan, &env.run_id).await.expect("drive"), RunEnd::AtGate);

    let a_prompts = fa.prompts();
    assert_eq!(a_prompts.len(), 1, "fake-a runs reader-a's task only");
    assert!(a_prompts[0].contains("Your thread is reader-a."));
    let b_prompts = fb.prompts();
    assert_eq!(b_prompts.len(), 1, "fake-b runs reader-b's task only");
    assert!(b_prompts[0].contains("Your thread is reader-b."));
    assert_projection_matches(&env);
}

#[tokio::test]
async fn a_worker_is_refused_a_permission_outside_its_folder() {
    let mut env = setup("escape", NO_ORCHESTRATOR_GOAL, Script::default());
    let scope = vec!["notes.md".to_string()];
    applied(run_task(&env.home, None, "reader-1", "read", &scope).expect("task command"));
    // Replace the scripted turn with one that asks to write outside the folder before it finishes.
    let fresh = FakeProvider::new();
    fresh.set_prompt_hook(agent(env.home.clone(), Script::default()));
    fresh.push_turn(vec![
        Event::PermissionRequest {
            request_id: "perm-escape".into(),
            tool: "Write".into(),
            input: serde_json::json!({"file_path": "/etc/ilvuno-escape-test", "content": "x"}),
            options: vec!["allow".into(), "deny".into()],
        },
        usage(),
        finished(),
    ]);
    let providers = BTreeMap::from([("claude-main".to_string(), fresh.clone())]);
    let driver = Driver { providers: &providers, home: &env.home, ilvuno_bin_dir: None, max_usd: 0.2 };
    driver.drive(&mut env.store, &env.plan, &env.run_id).await.expect("drive");
    let decisions = fresh.decisions();
    assert!(
        decisions.iter().any(|(id, d)| id == "perm-escape" && matches!(d, Decision::Deny { .. })),
        "write outside the folder must be denied: {decisions:?}"
    );
}

/// Stage 6, the failing half. A command check has to test the tree the gate would merge, not the project
/// folder, or an agent's work can pass its checks without ever being looked at.
#[tokio::test]
async fn a_command_check_runs_in_the_tree_the_gate_would_merge() {
    let mut env = setup_git("check-tree", GIT_PROJECT_GOAL, Script::default());
    let scope = vec!["brief.md".to_string()];
    applied(run_task(&env.home, None, "writer", "write the brief", &scope).expect("task"));

    let end = drive(&mut env).await;
    let readiness: String = env
        .store
        .conn()
        .query_row("SELECT readiness FROM rounds WHERE \"index\" = 0", [], |r| r.get(0))
        .expect("readiness");
    assert!(readiness.contains("\"passed\":true"), "the check has to see the writer's file. readiness: {readiness}");
    assert_eq!(end, RunEnd::AtGate, "a passing check opens the gate");

    // The writer wrote it, and the turn committed it, in its own worktree.
    let worktree = env.home.worktree_dir(&env.run_id, "writer");
    assert!(worktree.join("brief.md").is_file(), "the agent's file is in its worktree: {}", worktree.display());
    // The project folder is untouched until a person accepts.
    assert!(!env.plan.base().join("brief.md").exists(), "the project folder keeps its own tree until the gate");
    assert_eq!(status(&env.store, &env.run_id), "reviewed");
}

#[tokio::test]
async fn a_command_check_sees_files_from_both_thread_branches() {
    let mut env = setup_git("check-two-branches", TWO_WRITERS_GOAL, Script::default());
    applied(run_task(&env.home, None, "writer-a", "write a.md", &["a.md".into()]).expect("task a"));
    applied(run_task(&env.home, None, "writer-b", "write b.md", &["b.md".into()]).expect("task b"));

    assert_eq!(drive(&mut env).await, RunEnd::AtGate);
    let result = &readiness(&env.store, 0)["checks"][0];
    assert_eq!(result["name"].as_str(), Some("command_exit_zero test -f a.md -a -f b.md"));
    assert_eq!(result["passed"].as_bool(), Some(true));
    assert!(!env.plan.base().join("a.md").exists());
    assert!(!env.plan.base().join("b.md").exists());
    assert!(!env.home.worktree_dir(&env.run_id, "round-0").exists());
}

#[tokio::test]
async fn an_unstarted_planned_thread_does_not_block_round_checks() {
    let mut env = setup_git("check-idle-thread", IDLE_PLANNED_WRITER_GOAL, Script::default());
    applied(run_task(&env.home, None, "writer", "write a.md", &["a.md".into()]).expect("writer task"));

    assert_eq!(drive(&mut env).await, RunEnd::AtGate);
    let result = &readiness(&env.store, 0)["checks"][0];
    assert_eq!(result["name"].as_str(), Some("command_exit_zero test -f a.md"));
    assert_eq!(result["passed"].as_bool(), Some(true));
}

#[tokio::test]
async fn conflicting_thread_branches_block_checks_and_leave_the_project_unchanged() {
    let script = Script::default().with_contents(&["from writer a\n", "from writer b\n"]);
    let mut env = setup_git("check-conflict", CONFLICTING_WRITERS_GOAL, script);
    let scope = vec!["README.md".to_string()];
    applied(run_task(&env.home, None, "writer-a", "write README", &scope).expect("task a"));
    applied(run_task(&env.home, None, "writer-b", "write README", &scope).expect("task b"));

    let end = drive(&mut env).await;
    assert_ne!(end, RunEnd::AtGate, "a conflict must not open the gate");
    assert_eq!(status(&env.store, &env.run_id), "working");
    assert_eq!(verdicts(&env.store, &env.run_id), vec![Some("block".into())]);
    let result = &readiness(&env.store, 0)["checks"][0];
    assert_eq!(result["name"].as_str(), Some("threads_do_not_merge"));
    assert_eq!(result["passed"].as_bool(), Some(false));
    assert_eq!(result["detail"].as_str(), Some("README.md"));
    assert_eq!(
        std::fs::read_to_string(env.plan.base().join("README.md")).expect("project README"),
        "the project\n"
    );
    assert!(!env.home.worktree_dir(&env.run_id, "round-0").exists());
}

#[tokio::test]
async fn a_failed_tree_check_can_pass_after_the_next_round_fixes_the_file() {
    let script = Script::default().with_contents(&["BAD\n", "GOOD\n"]);
    let mut env = setup_git("check-retry", RETRY_CHECK_GOAL, script);
    let scope = vec!["brief.md".to_string()];
    applied(run_task(&env.home, None, "writer", "write BAD", &scope).expect("first task"));

    let first_end = drive(&mut env).await;
    assert_ne!(first_end, RunEnd::AtGate, "BAD must fail the check");
    let first = &readiness(&env.store, 0)["checks"][0];
    assert_eq!(first["name"].as_str(), Some("command_exit_zero grep -q GOOD brief.md"));
    assert_eq!(first["passed"].as_bool(), Some(false));

    applied(run_task(&env.home, None, "writer", "write GOOD", &scope).expect("retry task"));
    assert_eq!(drive(&mut env).await, RunEnd::AtGate);
    let second = &readiness(&env.store, 1)["checks"][0];
    assert_eq!(second["name"].as_str(), Some("command_exit_zero grep -q GOOD brief.md"));
    assert_eq!(second["passed"].as_bool(), Some(true));
    assert_eq!(
        std::fs::read_to_string(env.home.worktree_dir(&env.run_id, "writer").join("brief.md")).expect("writer file"),
        "GOOD\n"
    );
    assert!(!env.plan.base().join("brief.md").exists());
}

fn applied<T: std::fmt::Debug>(outcome: Outcome<T>) -> T {
    match outcome {
        Outcome::Applied(value) => value,
        other => panic!("expected applied, got {other:?}"),
    }
}
