//! Acceptance for the five invariants, exercised against real SQLite (temp ILVUNO_HOME, never ~).

use runner::commands::{self, NewGoal, NewThread, ReportStatus};
use runner::run_status::{self, GateDecision};
use runner::store::{Outcome, Pending};
use runner::tasks::{self, CreatedBy, NewTask, TaskRef};
use runner::{Home, RunStatus, Store, Verdict};
use rusqlite::{params, Transaction};
use serde_json::json;
use std::fmt::Debug;
use std::path::PathBuf;

struct Fixture {
    dir: PathBuf,
    home: Home,
    store: Store,
    run_id: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn fixture(name: &str) -> Fixture {
    fixture_with(name, json!({ "claude-main": { "driver": "claude" } }))
}

fn fixture_with(name: &str, providers: serde_json::Value) -> Fixture {
    let dir = std::env::temp_dir().join(format!("ilvuno-test-{name}-{}", uuid::Uuid::new_v4()));
    let home = Home::new(&dir);
    let mut store = Store::open(&home.db_path()).expect("open store");
    // Production writes the goal snapshot beside the run; a command that reads the plan needs it.
    let raw = json!({
        "goal": "g", "done": "d", "checks": [],
        "providers": providers.clone(),
        "threads": [{ "name": "researcher-a", "agent": "claude-main", "prompt": "" }],
        "budget": { "rounds": 3, "tokens": 150_000 },
    })
    .to_string();
    let base = home.root().to_path_buf();
    let goal = NewGoal {
        title: "t".into(),
        goal_text: "g".into(),
        done: None,
        checks: json!([]),
        message_policy: "round".into(),
        budget_rounds: 3,
        budget_tokens: 150_000,
        budget_usd: None,
        providers,
        threads: vec![NewThread { name: "researcher-a".into(), agent: "claude-main".into(), prompt: String::new() }],
    };
    let run_id = match store
        .transact("run.create", None, |tx, p| {
            let id = commands::create_run(tx, p, &goal, &home)?;
            runner::goal::snapshot(tx, &id, &raw, &base)?;
            Ok(Outcome::Applied(id))
        })
        .expect("create run")
    {
        Outcome::Applied(id) => id,
        other => panic!("unexpected {other:?}"),
    };
    Fixture { dir, home, store, run_id }
}

fn applied<T: Debug>(outcome: Outcome<T>) -> T {
    match outcome {
        Outcome::Applied(value) => value,
        other => panic!("expected applied, got {other:?}"),
    }
}

/// A task on `researcher-a` with the given scope, through the same path `ilvuno task` uses.
fn plan(f: &mut Fixture, scope: &[&str]) -> Outcome<TaskRef> {
    let run_id = f.run_id.clone();
    let scope: Vec<String> = scope.iter().map(|s| s.to_string()).collect();
    f.store
        .transact("task.create", None, |tx, p| {
            tasks::create_task(
                tx,
                p,
                &NewTask {
                    run_id: &run_id,
                    thread: "researcher-a",
                    prompt: "read the source",
                    scope: &scope,
                    created_by: CreatedBy::User,
                    author_thread: None,
                    kind: "task",
                },
            )
        })
        .expect("create task")
}

/// A message: a task with no scope. It must be accepted, and it must not claim the board.
fn send_message(f: &mut Fixture, kind: &str, scope: &[&str]) -> Outcome<TaskRef> {
    let run_id = f.run_id.clone();
    let scope: Vec<String> = scope.iter().map(|s| s.to_string()).collect();
    f.store
        .transact("task.create", None, |tx, p| {
            tasks::create_task(
                tx,
                p,
                &NewTask {
                    run_id: &run_id,
                    thread: "researcher-a",
                    prompt: "what are you working on?",
                    scope: &scope,
                    created_by: CreatedBy::User,
                    author_thread: None,
                    kind,
                },
            )
        })
        .expect("message")
}

#[test]
fn a_message_needs_no_scope_and_claims_no_board_entry() {
    let mut f = fixture("message");
    let before = count(&f.store, "SELECT COUNT(*) FROM ledger WHERE kind = 'intent'");
    match send_message(&mut f, "message", &[]) {
        Outcome::Applied(_) => {}
        other => panic!("expected applied, got {other:?}"),
    }
    let after = count(&f.store, "SELECT COUNT(*) FROM ledger WHERE kind = 'intent'");
    assert_eq!(before, after, "a message must not write an intent row");
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM dispatch WHERE kind = 'message'"), 1);
}

#[test]
fn a_message_may_not_claim_files_and_a_task_still_needs_them() {
    let mut f = fixture("message-scope");
    assert!(matches!(send_message(&mut f, "message", &["notes.md"]), Outcome::Refused(_)));
    assert!(matches!(send_message(&mut f, "task", &[]), Outcome::Refused(_)));
    assert!(matches!(send_message(&mut f, "chatty", &[]), Outcome::Refused(_)));
}

/// The agent is a choice: a thread can move to another agent the goal names, and the journal says so.
#[test]
fn a_thread_can_move_to_another_agent_and_the_journal_names_both() {
    let mut f = fixture_with(
        "agent-switch",
        json!({ "claude-main": { "driver": "claude" }, "codex-main": { "driver": "claude" } }),
    );
    let run_id = f.run_id.clone();
    applied(run_status::set_thread_agent(&mut f.store, &run_id, "researcher-a", "codex-main").expect("switch"));
    let provider: String = f
        .store
        .conn()
        .query_row("SELECT provider FROM threads WHERE run_id = ?1 AND name = 'researcher-a'", params![run_id], |r| r.get(0))
        .expect("provider");
    assert_eq!(provider, "codex-main");
    let payload: String = f
        .store
        .conn()
        .query_row("SELECT payload FROM journal WHERE kind = 'thread.agent' ORDER BY seq DESC LIMIT 1", [], |r| r.get(0))
        .expect("journal row");
    assert!(payload.contains("claude-main"), "journal payload was {payload}");
    assert!(payload.contains("codex-main"), "journal payload was {payload}");

    // Unknown threads and agents the goal does not name are refused, not written.
    assert!(matches!(run_status::set_thread_agent(&mut f.store, &run_id, "nobody", "codex-main"), Ok(Outcome::Refused(_))));
    assert!(matches!(run_status::set_thread_agent(&mut f.store, &run_id, "researcher-a", "ghost"), Ok(Outcome::Refused(_))));
}

/// Switching while a turn is running would make the journal lie about who spoke, so it is refused.
#[test]
fn a_thread_with_a_running_turn_keeps_its_agent() {
    let mut f = fixture_with(
        "agent-busy",
        json!({ "claude-main": { "driver": "claude" }, "codex-main": { "driver": "claude" } }),
    );
    applied(plan(&mut f, &["notes.md"]));
    f.store.conn().execute("UPDATE dispatch SET status = 'running'", []).expect("mark running");
    let run_id = f.run_id.clone();
    match run_status::set_thread_agent(&mut f.store, &run_id, "researcher-a", "codex-main") {
        Ok(Outcome::Refused(reason)) => assert!(reason.contains("middle of a turn"), "reason was {reason}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn decide_verdict(f: &mut Fixture, round_id: &str, v: Verdict) {
    let run_id = f.run_id.clone();
    let round_id = round_id.to_string();
    f.store
        .transact("round.verdict", None, |tx, p| {
            run_status::record_verdict(tx, p, &run_id, &round_id, v, &json!({"checks": []}))?;
            Ok(Outcome::Applied(()))
        })
        .expect("verdict");
}

fn count(store: &Store, sql: &str) -> i64 {
    store.conn().query_row(sql, [], |r| r.get(0)).expect("count")
}

fn journal_kinds(store: &Store, run_id: &str) -> Vec<String> {
    let mut stmt = store
        .conn()
        .prepare("SELECT kind FROM journal WHERE run_id = ?1 ORDER BY seq")
        .expect("prepare");
    stmt.query_map(params![run_id], |r| r.get(0)).expect("query").map(|r| r.expect("row")).collect()
}

#[test]
fn schema_and_journal_are_created_on_open() {
    let f = fixture("schema");
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM schema_migrations"), 6, "0001_init, 0002_providers, 0003_tasks, 0004_messages, 0005_messages, 0006_git");
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM journal WHERE kind = 'run.created'"), 1);
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM receipts"), 0, "no request_id, so no receipt");
}

#[test]
fn invariant1_no_round_starts_before_the_previous_round_has_a_verdict() {
    let mut f = fixture("inv1");
    let first = applied(plan(&mut f, &["research/09-spike.md"]));
    // A task while the round is open joins that round: the old refusal became a join, not a new round.
    let second = applied(plan(&mut f, &["findings.md"]));
    assert_eq!(second.round_id, first.round_id, "the second task joins the open round");
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM rounds"), 1, "no second round before a verdict");
    // A blocking verdict keeps the run working. A passing one would send it to the gate, which refuses tasks.
    decide_verdict(&mut f, &first.round_id, Verdict::Block);
    let third = applied(plan(&mut f, &["findings.md"]));
    assert_eq!(third.round_index, 1, "after the verdict, the next task opens round 1");
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM rounds"), 2);
}

#[test]
fn invariant2_refuses_task_without_scope_before_writing_anything() {
    let mut f = fixture("inv2-scope");
    let outcome = plan(&mut f, &[]);
    assert_eq!(outcome.refusal(), Some("task has no scope"));
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM rounds"), 0);
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM dispatch"), 0);
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM ledger"), 0);
}

#[test]
fn invariant2_refuses_dispatch_when_no_intent_covers_scope() {
    let mut f = fixture("inv2-intent");
    let round = applied(plan(&mut f, &["research/09-spike.md"]));
    // Simulate a bypass: the intent row disappears after the task is written.
    f.store
        .conn()
        .execute("DELETE FROM ledger WHERE round_id = ?1 AND kind = 'intent'", params![round.round_id])
        .expect("delete intent");
    let dispatch_id = round.task_id.clone();
    let run_id = f.run_id.clone();
    let outcome = f
        .store
        .transact("dispatch.start", None, |tx, p| commands::start_dispatch(tx, p, &run_id, &dispatch_id))
        .expect("start");
    assert_eq!(outcome.refusal(), Some("no ledger intent names this dispatch's scope"));
    let status: String = f
        .store
        .conn()
        .query_row("SELECT status FROM dispatch WHERE id = ?1", params![dispatch_id], |r| r.get(0))
        .expect("status");
    assert_eq!(status, "queued", "refused task must not start");
}

#[test]
fn invariant3_merge_refused_until_accepted_by_human() {
    let mut f = fixture("inv3");
    let round = applied(plan(&mut f, &["research/09-spike.md"]));
    let run_id = f.run_id.clone();
    let merge = |f: &mut Fixture| {
        let rid = f.run_id.clone();
        f.store.transact("merge.request", None, |tx, p| run_status::request_merge(tx, p, &rid)).expect("merge")
    };
    assert!(matches!(merge(&mut f), Outcome::Refused(_)));
    decide_verdict(&mut f, &round.round_id, Verdict::Pass);
    assert!(matches!(merge(&mut f), Outcome::Refused(_)), "passing verdict alone is not enough");
    let gate = f
        .store
        .transact("gate.accept", None, |tx, p| run_status::gate_decide(tx, p, &run_id, &GateDecision::Accept))
        .expect("gate");
    assert_eq!(gate, Outcome::Applied(()));
    assert_eq!(merge(&mut f), Outcome::Applied(()));
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM journal WHERE kind = 'merge.refused'"), 2);
}

#[test]
fn invariant5_duplicate_request_id_is_one_dispatch_one_receipt() {
    let mut f = fixture("inv5");
    let round = applied(plan(&mut f, &["research/09-spike.md", "findings.md"]));
    let dispatch_id = round.task_id.clone();
    let run_id = f.run_id.clone();
    let start = f
        .store
        .transact("dispatch.start", None, |tx, p| commands::start_dispatch(tx, p, &run_id, &dispatch_id))
        .expect("start");
    assert!(matches!(start, Outcome::Applied(_)));

    let files = vec!["findings.md".to_string()];
    let report = |f: &mut Fixture| {
        let d = dispatch_id.clone();
        let files = files.clone();
        f.store
            .transact("ilvuno.report", Some("report-7"), move |tx, p| {
                commands::report_dispatch(tx, p, &d, ReportStatus::Ok, "three sentences.", &files)
            })
            .expect("report")
    };
    assert_eq!(report(&mut f), Outcome::Applied(()));
    assert!(matches!(report(&mut f), Outcome::Duplicate(_)), "second delivery must be deduplicated");
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM receipts WHERE request_id = 'report-7'"), 1);
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM ledger WHERE kind = 'finding'"), 1);
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM journal WHERE kind = 'dispatch.reported'"), 1);
}

#[test]
fn report_outside_declared_scope_is_refused_and_dispatch_stays_running() {
    let mut f = fixture("scope-report");
    let round = applied(plan(&mut f, &["research/09-spike.md", "findings.md"]));
    let dispatch_id = round.task_id.clone();
    let run_id = f.run_id.clone();
    f.store
        .transact("dispatch.start", None, |tx, p| commands::start_dispatch(tx, p, &run_id, &dispatch_id))
        .expect("start");
    let d = dispatch_id.clone();
    let outcome = f
        .store
        .transact("ilvuno.report", None, move |tx, p| {
            commands::report_dispatch(tx, p, &d, ReportStatus::Ok, "x", &["/etc/passwd".to_string()])
        })
        .expect("report");
    assert_eq!(outcome.refusal(), Some("file /etc/passwd is outside the declared scope"));
    let status: String = f
        .store
        .conn()
        .query_row("SELECT status FROM dispatch WHERE id = ?1", params![dispatch_id], |r| r.get(0))
        .expect("status");
    assert_eq!(status, "running");
}

#[test]
fn invariant4_refuses_fourth_round_and_keeps_threads_and_artifacts() {
    let mut f = fixture("inv4");
    for _ in 0..3 {
        let round = applied(plan(&mut f, &["research/09-spike.md"]));
        decide_verdict(&mut f, &round.round_id, Verdict::Block);
    }
    // Keep a fake artifact on disk to prove refusal does not clean up.
    let thread_dir = f.home.threads_dir(&f.run_id).join("researcher-a");
    std::fs::create_dir_all(&thread_dir).expect("mkdir");
    std::fs::write(thread_dir.join("findings.md"), "kept").expect("write");

    let fourth = plan(&mut f, &["research/09-spike.md"]);
    match fourth {
        Outcome::Refused(reason) => assert!(reason.contains("3 of 3"), "reason: {reason}"),
        other => panic!("fourth round must be refused, got {other:?}"),
    }
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM rounds"), 3);
    // Threads belong to the run, not to each round: the one thread from the goal is the only one.
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM threads"), 1);
    assert!(thread_dir.join("findings.md").exists());
}

#[test]
fn a_failing_body_rolls_back_projection_and_journal_together() {
    let mut f = fixture("rollback");
    let run_id = f.run_id.clone();
    let before_rounds = count(&f.store, "SELECT COUNT(*) FROM journal");
    let result: anyhow::Result<Outcome<()>> = f.store.transact("boom", None, |tx: &Transaction<'_>, p: &mut Pending| {
        tx.execute("UPDATE runs SET tokens_used = 999 WHERE id = ?1", params![run_id])?;
        p.push(&run_id, None, None, "should.not.exist", json!({}));
        anyhow::bail!("simulated crash between projection and journal");
    });
    assert!(result.is_err());
    let tokens: i64 = f
        .store
        .conn()
        .query_row("SELECT tokens_used FROM runs WHERE id = ?1", params![run_id], |r| r.get(0))
        .expect("tokens");
    assert_eq!(tokens, 0, "projection write must roll back");
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM journal"), before_rounds);
}

#[test]
fn journal_records_every_transition_in_order() {
    let mut f = fixture("order");
    let round = applied(plan(&mut f, &["research/09-spike.md"]));
    decide_verdict(&mut f, &round.round_id, Verdict::Pass);
    let run_id = f.run_id.clone();
    f.store
        .transact("gate.accept", None, |tx, p| run_status::gate_decide(tx, p, &run_id, &GateDecision::Accept))
        .expect("gate");
    let kinds = journal_kinds(&f.store, &f.run_id);
    let pos = |k: &str| kinds.iter().position(|x| x == k).unwrap_or_else(|| panic!("missing {k} in {kinds:?}"));
    assert!(pos("run.created") < pos("round.planned"));
    assert!(pos("round.planned") < pos("round.verdict"));
    assert!(pos("round.verdict") < pos("gate.decided"));
    let status: String = f
        .store
        .conn()
        .query_row("SELECT status FROM runs WHERE id = ?1", params![f.run_id], |r| r.get(0))
        .expect("status");
    assert_eq!(RunStatus::parse(&status), Some(RunStatus::Accepted));
}
