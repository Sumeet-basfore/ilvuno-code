//! `ilvuno status` shows each check of a round, with its result and detail.

use runner::commands::{self, NewGoal, NewThread};
use runner::run_status;
use runner::store::Outcome;
use runner::tasks::{self, CreatedBy, NewTask};
use runner::{Home, Store, Verdict};
use serde_json::json;
use std::process::Command;

#[test]
fn status_prints_each_check_with_its_detail() {
    let dir = std::env::temp_dir().join(format!("ilvuno-status-{}", uuid::Uuid::new_v4()));
    let home = Home::new(&dir);
    let mut store = Store::open(&home.db_path()).expect("store");
    let goal = NewGoal {
        title: "t".into(),
        goal_text: "g".into(),
        done: None,
        checks: json!([]),
        message_policy: "round".into(),
        budget_rounds: 3,
        budget_tokens: 150_000,
        budget_usd: None,
        providers: json!({"claude-main": {"driver": "claude"}}),
        threads: vec![NewThread { name: "reader-1".into(), agent: "claude-main".into(), prompt: String::new() }],
    };
    let run_id = match store
        .transact("run.create", None, |tx, p| Ok(Outcome::Applied(commands::create_run(tx, p, &goal, &home)?)))
        .expect("create")
    {
        Outcome::Applied(id) => id,
        other => panic!("{other:?}"),
    };
    let scope = vec!["brief.html".to_string()];
    let task = match store
        .transact("task.create", None, |tx, p| {
            tasks::create_task(
                tx,
                p,
                &NewTask { run_id: &run_id, thread: "reader-1", prompt: "write it", scope: &scope, created_by: CreatedBy::User, author_thread: None, kind: "task" },
            )
        })
        .expect("task")
    {
        Outcome::Applied(task) => task,
        other => panic!("planning must not refuse: {other:?}"),
    };

    let readiness = json!({
        "checks": [{ "name": "text_absent brief.html", "passed": false, "detail": "found: TODO" }],
        "failing": ["text_absent brief.html"],
        "failed_tasks": [],
    });
    store
        .transact("round.verdict", None, |tx, p| {
            run_status::record_verdict(tx, p, &run_id, &task.round_id, Verdict::Block, &readiness)?;
            Ok(Outcome::Applied(()))
        })
        .expect("verdict");

    let output = Command::new(env!("CARGO_BIN_EXE_ilvuno"))
        .args(["status", &run_id])
        .env("ILVUNO_HOME", &dir)
        .output()
        .expect("run ilvuno status");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "status failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("[FAIL] text_absent brief.html: found: TODO"), "ilvuno status output:\n{stdout}");
    assert!(stdout.contains("task on reader-1 (user)"), "ilvuno status output:\n{stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}
