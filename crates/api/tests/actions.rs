//! Run actions: add a task, and decide at the gate. The gate test runs a whole scripted run through the API.

mod common;

use axum::http::StatusCode;
use common::{get, harness, post, start_run, wait_idle, wait_status, GOAL_WITH_ORCHESTRATOR, GOAL_WORKERS};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_task_is_written_to_the_round_and_the_board() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    let body = r#"{"thread":"reader-1","prompt":"Read the security note.","scope":["notes.md"]}"#;
    let reply = post(&h.app, &format!("/runs/{id}/tasks"), body).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert!(reply.json["task_id"].as_str().is_some_and(|t| !t.is_empty()));

    let run = get(&h.app, &format!("/runs/{id}")).await.json;
    let tasks = run["tasks"].as_array().expect("tasks");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["prompt"], "Read the security note.");
    assert_eq!(tasks[0]["thread"], "reader-1");
    assert_eq!(tasks[0]["created_by"], "user");
    assert_eq!(tasks[0]["status"], "queued");

    let board = get(&h.app, &format!("/runs/{id}/board")).await.json;
    assert!(board["markdown"].as_str().unwrap_or_default().contains("Read the security note."));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_task_is_a_409_with_the_runner_reason() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    let reply = post(&h.app, &format!("/runs/{id}/tasks"), r#"{"thread":"nobody","prompt":"x","scope":["a"]}"#).await;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert_eq!(reply.json["kind"], "conflict");
    assert_eq!(reply.json["error"], "unknown thread nobody");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bad_task_body_is_a_400() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;
    let reply = post(&h.app, &format!("/runs/{id}/tasks"), r#"{"thread":"reader-1"}"#).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.json["kind"], "bad_request");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_gate_decision_needs_a_known_decision_and_a_note_for_send_back() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;
    let url = format!("/runs/{id}/gate");
    for body in [r#"{"decision":"maybe"}"#, r#"{"decision":"send_back"}"#, r#"{"decision":"send_back","note":"  "}"#] {
        let reply = post(&h.app, &url, body).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "body {body}");
        assert_eq!(reply.json["kind"], "bad_request", "body {body}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_gate_refuses_a_run_that_is_not_at_it() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;
    let reply = post(&h.app, &format!("/runs/{id}/gate"), r#"{"decision":"accept"}"#).await;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert!(reply.json["error"].as_str().unwrap_or_default().contains("not at the gate"), "{}", reply.json);
}

/// A whole run through the API: start, the scripted orchestrator plans, the worker writes, the checks pass,
/// the run reaches the gate, and a human accepts it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_scripted_run_reaches_the_gate_and_is_accepted_through_the_api() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WITH_ORCHESTRATOR).await;
    wait_status(&h.app, &id, "reviewed").await;
    wait_idle(&h.state).await;

    let run = get(&h.app, &format!("/runs/{id}")).await.json;
    assert_eq!(run["rounds"].as_array().map(Vec::len), Some(1));
    assert_eq!(run["rounds"][0]["verdict"], "pass");
    let kinds: Vec<&str> = run["tasks"].as_array().unwrap().iter().map(|t| t["created_by"].as_str().unwrap()).collect();
    assert_eq!(kinds, vec!["user", "orchestrator"], "the kickoff is the user's task; the plan is the orchestrator's");

    let accepted = post(&h.app, &format!("/runs/{id}/gate"), r#"{"decision":"accept"}"#).await;
    assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.json);
    assert_eq!(accepted.json, serde_json::json!({ "run_id": id, "status": "accepted" }));
    let after = get(&h.app, &format!("/runs/{id}")).await.json;
    assert_eq!(after["run"]["status"], "accepted");
}
