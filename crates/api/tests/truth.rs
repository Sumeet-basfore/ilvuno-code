//! Startup recovery and the run facts the window reads from the API.

mod common;

use axum::http::StatusCode;
use common::{get, harness, post, start_run, wait_idle, GOAL_WORKERS};
use runner::store::{Outcome, Store};

const LOST_TURN: &str = "runner stopped; the turn was lost";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn startup_recovery_pauses_a_working_run_once_before_the_window_reads_it() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    // This is the stale list snapshot that could otherwise show `working` after a restart.
    let before = get(&h.app, "/runs").await.json;
    assert_eq!(status_in_list(&before, &id), Some("working"));

    let (app, recovered) = api::startup_router(h.home.clone(), h.state.build.clone()).unwrap();
    assert_eq!(recovered, 1);
    let (_, recovered_again) = api::startup_router(h.home.clone(), h.state.build.clone()).unwrap();
    assert_eq!(recovered_again, 0);

    let list = get(&app, "/runs").await.json;
    assert_eq!(status_in_list(&list, &id), Some("human_paused"));
    let detail = get(&app, &format!("/runs/{id}")).await.json;
    assert_eq!(detail["run"]["status"], "human_paused");
    assert_eq!(detail["run"]["blocked_reason"], LOST_TURN);
    assert!(detail["run"]["ended_at"].as_i64().is_some());

    let store = Store::open(&h.home.db_path()).unwrap();
    let transitions: i64 = store.conn().query_row(
        r#"SELECT COUNT(*) FROM journal WHERE run_id = ?1 AND kind = 'run.transition'
           AND json_extract(payload, '$.to') = 'human_paused'
           AND json_extract(payload, '$.reason') = ?2"#,
        rusqlite::params![id, LOST_TURN],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(transitions, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn startup_recovery_does_not_pause_a_run_while_its_drive_claim_is_held() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    let claim = h.state.claim_drive(&format!("run {id}")).unwrap();
    assert_eq!(api::recovery::reconcile_startup_runs(&h.state).unwrap(), 0);
    let active = get(&h.app, &format!("/runs/{id}")).await.json;
    assert_eq!(active["run"]["status"], "working");
    drop(claim);

    assert_eq!(api::recovery::reconcile_startup_runs(&h.state).unwrap(), 1);
    let recovered = get(&h.app, &format!("/runs/{id}")).await.json;
    assert_eq!(recovered["run"]["status"], "human_paused");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_task_after_the_token_budget_is_refused_and_the_run_is_stopped() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    let mut store = Store::open(&h.home.db_path()).unwrap();
    store.transact("test.spend_budget", None, |tx, _| {
        tx.execute(
            "UPDATE runs SET tokens_used = (SELECT budget_tokens FROM goals WHERE id = runs.goal_id) \
             WHERE id = ?1",
            [&id],
        )?;
        Ok(Outcome::Applied(()))
    })
    .unwrap();

    let reply = post(
        &h.app,
        &format!("/runs/{id}/tasks"),
        r#"{"thread":"reader-1","prompt":"Do one more task.","scope":["notes.md"]}"#,
    ).await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.json);
    assert!(reply.json["error"]
        .as_str()
        .unwrap_or_default()
        .contains("token budget exhausted"));

    let detail = get(&h.app, &format!("/runs/{id}")).await.json;
    assert_eq!(detail["run"]["status"], "budget_exhausted");
    assert!(detail["run"]["blocked_reason"]
        .as_str()
        .unwrap_or_default()
        .contains("token budget exhausted"));
    assert_eq!(detail["rounds"].as_array().map(Vec::len), Some(0), "the unaffordable round must not open");
    assert_eq!(detail["tasks"].as_array().map(Vec::len), Some(0), "the unaffordable task must not be queued");

    let rows = get(&h.app, &format!("/runs/{id}/journal?kinds=run.transition,task.refused")).await.json;
    assert!(rows.as_array().unwrap().iter().any(|row| {
        row["kind"] == "run.transition" && row["payload"]["to"] == "budget_exhausted"
    }));
    assert!(rows.as_array().unwrap().iter().any(|row| {
        row["kind"] == "task.refused"
            && row["payload"]["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("token budget exhausted")
    }));
    assert!(!rows.as_array().unwrap().iter().any(|row| row["kind"] == "round.planned" || row["kind"] == "dispatch.queued"));
}

fn status_in_list<'a>(rows: &'a serde_json::Value, id: &str) -> Option<&'a str> {
    rows.as_array()?.iter().find(|row| row["id"] == id)?["status"].as_str()
}
