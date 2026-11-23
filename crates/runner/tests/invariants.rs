//! M1 acceptance for the five invariants, exercised against real SQLite (temp ILVUNO_HOME, never ~).

use runner::commands::{self, NewGoal, ReportStatus, SlotSpec};
use runner::run_status::{self, GateDecision};
use runner::store::{Outcome, Pending};
use runner::{Home, RunStatus, Store, Verdict};
use rusqlite::{params, Transaction};
use serde_json::json;
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
    let dir = std::env::temp_dir().join(format!("ilvuno-test-{name}-{}", uuid::Uuid::new_v4()));
    let home = Home::new(&dir);
    let mut store = Store::open(&home.db_path()).expect("open store");
    let goal = NewGoal {
        title: "t".into(),
        goal_text: "g".into(),
        template: "research".into(),
        deliverable: json!({"format": ["html"]}),
        done_spec: json!({"checks": []}),
        budget_rounds: 3,
        budget_tokens: 150_000,
        budget_usd: None,
        providers: json!({"claude-main": {"driver": "claude"}}),
        assignments: json!({}),
    };
    let run_id = match store
        .transact("run.create", None, |tx, p| Ok(Outcome::Applied(commands::create_run(tx, p, &goal)?)))
        .expect("create run")
    {
        Outcome::Applied(id) => id,
        other => panic!("unexpected {other:?}"),
    };
    Fixture { dir, home, store, run_id }
}

fn slots(scope: &[&str]) -> Vec<SlotSpec> {
    vec![SlotSpec {
        slot: "researcher-a".into(),
        provider: "claude-main".into(),
        role: "worker".into(),
        scope: scope.iter().map(|s| s.to_string()).collect(),
        intent: "read the source".into(),
    }]
}

fn plan(f: &mut Fixture, scope: &[&str]) -> Outcome<commands::RoundRef> {
    let root = f.home.threads_dir(&f.run_id);
    let run_id = f.run_id.clone();
    f.store
        .transact("round.plan", None, |tx, p| commands::plan_round(tx, p, &run_id, &slots(scope), &root))
        .expect("plan round")
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
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM schema_migrations"), 2, "0001_init and 0002_providers");
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM journal WHERE kind = 'run.created'"), 1);
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM receipts"), 0, "no request_id, so no receipt");
}

#[test]
fn invariant1_refuses_round_while_previous_round_has_no_verdict() {
    let mut f = fixture("inv1");
    let r0 = match plan(&mut f, &["research/09-spike.md"]) {
        Outcome::Applied(r) => r,
        other => panic!("{other:?}"),
    };
    let second = plan(&mut f, &["research/09-spike.md"]);
    assert_eq!(second.refusal(), Some("previous round has no verdict"));
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM rounds"), 1, "refused round must not exist");
    assert!(journal_kinds(&f.store, &f.run_id).contains(&"round.refused".to_string()));
    decide_verdict(&mut f, &r0.round_id, Verdict::Pass);
}

#[test]
fn invariant2_refuses_dispatch_without_scope_before_writing_anything() {
    let mut f = fixture("inv2-scope");
    let outcome = plan(&mut f, &[]);
    assert_eq!(outcome.refusal(), Some("dispatch has no scope"));
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM rounds"), 0);
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM dispatch"), 0);
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM ledger"), 0);
}

#[test]
fn invariant2_refuses_dispatch_when_no_intent_covers_scope() {
    let mut f = fixture("inv2-intent");
    let round = match plan(&mut f, &["research/09-spike.md"]) {
        Outcome::Applied(r) => r,
        other => panic!("{other:?}"),
    };
    // Simulate a bypass: the intent row disappears after planning.
    f.store
        .conn()
        .execute("DELETE FROM ledger WHERE round_id = ?1 AND kind = 'intent'", params![round.round_id])
        .expect("delete intent");
    let dispatch_id = round.dispatches[0].dispatch_id.clone();
    let run_id = f.run_id.clone();
    let outcome = f
        .store
        .transact("dispatch.start", None, |tx, p| commands::start_dispatch(tx, p, &run_id, &dispatch_id, "task"))
        .expect("start");
    assert_eq!(outcome.refusal(), Some("no ledger intent names this dispatch's scope"));
    let status: String = f
        .store
        .conn()
        .query_row("SELECT status FROM dispatch WHERE id = ?1", params![dispatch_id], |r| r.get(0))
        .expect("status");
    assert_eq!(status, "queued", "refused dispatch must not start");
}

#[test]
fn invariant3_merge_refused_until_accepted_by_human() {
    let mut f = fixture("inv3");
    let round = match plan(&mut f, &["research/09-spike.md"]) {
        Outcome::Applied(r) => r,
        other => panic!("{other:?}"),
    };
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
    let round = match plan(&mut f, &["research/09-spike.md", "findings.md"]) {
        Outcome::Applied(r) => r,
        other => panic!("{other:?}"),
    };
    let dispatch_id = round.dispatches[0].dispatch_id.clone();
    let run_id = f.run_id.clone();
    let start = f
        .store
        .transact("dispatch.start", None, |tx, p| commands::start_dispatch(tx, p, &run_id, &dispatch_id, "task"))
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
    let round = match plan(&mut f, &["research/09-spike.md", "findings.md"]) {
        Outcome::Applied(r) => r,
        other => panic!("{other:?}"),
    };
    let dispatch_id = round.dispatches[0].dispatch_id.clone();
    let run_id = f.run_id.clone();
    f.store
        .transact("dispatch.start", None, |tx, p| commands::start_dispatch(tx, p, &run_id, &dispatch_id, "task"))
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
        let round = match plan(&mut f, &["research/09-spike.md"]) {
            Outcome::Applied(r) => r,
            other => panic!("{other:?}"),
        };
        decide_verdict(&mut f, &round.round_id, Verdict::Block);
    }
    // Keep a fake artifact on disk to prove refusal does not clean up.
    let thread_dir = f.home.threads_dir(&f.run_id).join("r0-researcher-a");
    std::fs::create_dir_all(&thread_dir).expect("mkdir");
    std::fs::write(thread_dir.join("findings.md"), "kept").expect("write");

    let fourth = plan(&mut f, &["research/09-spike.md"]);
    match fourth {
        Outcome::Refused(reason) => assert!(reason.contains("3 of 3"), "reason: {reason}"),
        other => panic!("fourth round must be refused, got {other:?}"),
    }
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM rounds"), 3);
    assert_eq!(count(&f.store, "SELECT COUNT(*) FROM threads"), 3);
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
    let round = match plan(&mut f, &["research/09-spike.md"]) {
        Outcome::Applied(r) => r,
        other => panic!("{other:?}"),
    };
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
