//! `ilvuno status` shows the second-opinion warning recorded in a round's readiness report.

use runner::commands::{self, NewGoal, SlotSpec};
use runner::second_opinion;
use runner::state::Verdict;
use runner::store::Outcome;
use runner::{run_status, Home, Store};
use serde_json::json;
use std::process::Command;

fn slot(name: &str, provider: &str) -> SlotSpec {
    SlotSpec {
        slot: name.into(),
        provider: provider.into(),
        role: "worker".into(),
        scope: vec!["notes.md".into()],
        intent: format!("{name} works on notes.md"),
    }
}

#[test]
fn status_prints_the_warning_stored_in_the_round_readiness() {
    let dir = std::env::temp_dir().join(format!("ilvuno-status-{}", uuid::Uuid::new_v4()));
    let home = Home::new(&dir);
    let mut store = Store::open(&home.db_path()).expect("store");
    let goal = NewGoal {
        title: "t".into(),
        goal_text: "g".into(),
        template: "research".into(),
        deliverable: json!({}),
        done_spec: json!({"checks": []}),
        budget_rounds: 3,
        budget_tokens: 150_000,
        budget_usd: None,
        providers: json!({"claude-main": {"driver": "claude"}}),
        assignments: json!({}),
    };
    let run_id = match store
        .transact("run.create", None, |tx, p| Ok(Outcome::Applied(commands::create_run(tx, p, &goal)?)))
        .expect("create")
    {
        Outcome::Applied(id) => id,
        other => panic!("{other:?}"),
    };
    let slots = [slot("builder", "claude-main"), slot("reviewer", "claude-main")];
    let round = match store
        .transact("round.plan", None, |tx, p| commands::plan_round(tx, p, &run_id, &slots, &home.threads_dir(&run_id)))
        .expect("plan")
    {
        Outcome::Applied(round) => round,
        other => panic!("planning must not refuse a second opinion: {other:?}"),
    };

    let warnings = second_opinion::for_round(store.conn(), &round.round_id).expect("warnings");
    let readiness = json!({ "checks": [], "skipped": [], "failing": [], "failed_dispatches": [], "warnings": warnings });
    store
        .transact("round.verdict", None, |tx, p| {
            run_status::record_verdict(tx, p, &run_id, &round.round_id, Verdict::Pass, &readiness)?;
            Ok(Outcome::Applied(()))
        })
        .expect("verdict");

    let stored: String = store
        .conn()
        .query_row("SELECT readiness FROM rounds WHERE id = ?1", [&round.round_id], |r| r.get(0))
        .expect("readiness");
    let line = "reviewer uses the same provider as builder (claude-main): no second opinion";
    assert!(stored.contains(line), "rounds.readiness: {stored}");

    let output = Command::new(env!("CARGO_BIN_EXE_ilvuno"))
        .args(["status", &run_id])
        .env("ILVUNO_HOME", &dir)
        .output()
        .expect("run ilvuno status");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "status failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains(&format!("[warn] {line}")), "ilvuno status output:\n{stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}
