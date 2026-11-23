//! Run actions: add a task, and decide at the gate. The gate test runs a whole scripted run through the API.

mod common;

use axum::http::StatusCode;
use common::{get, harness, post, put, start_run, wait_idle, wait_status, GOAL_WITH_ORCHESTRATOR, GOAL_WORKERS};

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stopping_a_run_pauses_it_and_keeps_the_reason() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    let reply = post(&h.app, &format!("/runs/{id}/stop"), r#"{"note":"wrong direction"}"#).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.json);
    assert_eq!(reply.json["status"], "human_paused");

    let run = get(&h.app, &format!("/runs/{id}")).await.json;
    assert_eq!(run["run"]["status"], "human_paused");
    let reason = run["run"]["blocked_reason"].as_str().unwrap_or_default();
    assert!(reason.contains("stopped by you"), "reason was {reason:?}");
    assert!(reason.contains("wrong direction"), "reason was {reason:?}");

    // A second stop is refused rather than silently ignored.
    let again = post(&h.app, &format!("/runs/{id}/stop"), r#"{}"#).await;
    assert_eq!(again.status, StatusCode::CONFLICT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_finished_run_cannot_be_stopped() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WITH_ORCHESTRATOR).await;
    wait_status(&h.app, &id, "reviewed").await;
    wait_idle(&h.state).await;

    let at_gate = post(&h.app, &format!("/runs/{id}/stop"), "").await;
    assert_eq!(at_gate.status, StatusCode::CONFLICT, "{}", at_gate.json);
    assert!(at_gate.json["error"].as_str().unwrap_or_default().contains("nothing to stop"), "{}", at_gate.json);

    let accepted = post(&h.app, &format!("/runs/{id}/gate"), r#"{"decision":"accept"}"#).await;
    assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.json);
    let after_accept = post(&h.app, &format!("/runs/{id}/stop"), "").await;
    assert_eq!(after_accept.status, StatusCode::CONFLICT, "{}", after_accept.json);
}

/// Stop, then resume: the run has to leave the paused state before the loop reads it, or the button is a
/// no-op. This is the path the "Resume run" button takes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stopped_run_can_be_resumed() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    let stopped = post(&h.app, &format!("/runs/{id}/stop"), r#"{"note":"checking resume"}"#).await;
    assert_eq!(stopped.status, StatusCode::OK, "{}", stopped.json);
    assert_eq!(stopped.json["status"], "human_paused");

    let resumed = post(&h.app, &format!("/runs/{id}/resume"), "{}").await;
    assert_eq!(resumed.status, StatusCode::ACCEPTED, "{}", resumed.json);

    // The point of a resume: the run leaves the paused state and the loop reads it again. Where it lands
    // after that depends on the round (here: no open tasks, no checks), so only the exit is asserted.
    let mut status = String::from("human_paused");
    for _ in 0..50 {
        let run = get(&h.app, &format!("/runs/{id}")).await.json;
        status = run["run"]["status"].as_str().unwrap_or_default().to_string();
        if status != "human_paused" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert_ne!(status, "human_paused", "resume left the run paused");
}

/// A goal that names two agents, so a thread can move from one to the other.
const GOAL_TWO_AGENTS: &str = r#"{
  "goal": "Write notes for one source.",
  "done": "notes.md exists.",
  "providers": { "claude-main": { "driver": "claude" }, "claude-alt": { "driver": "claude" } },
  "threads": [
    { "name": "reader-1", "agent": "claude-main", "prompt": "You read the source you are given." }
  ],
  "budget": { "rounds": 3, "tokens": 600000, "max_usd_per_call": 0.2 }
}"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_agent_of_a_thread_can_be_changed() {
    let h = harness();
    let id = start_run(&h.app, GOAL_TWO_AGENTS).await;
    wait_idle(&h.state).await;

    let url = format!("/runs/{id}/threads/reader-1/agent");
    let reply = put(&h.app, &url, r#"{"agent":"claude-alt"}"#).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.json);

    let run = get(&h.app, &format!("/runs/{id}")).await.json;
    assert_eq!(run["threads"][0]["agent"], "claude-alt");
    // The window needs the list to offer a choice, so the detail has to carry it.
    assert_eq!(run["goal"]["providers"].as_array().map(Vec::len), Some(2));

    let unknown_agent = put(&h.app, &url, r#"{"agent":"ghost"}"#).await;
    assert_eq!(unknown_agent.status, StatusCode::CONFLICT, "{}", unknown_agent.json);
    let unknown_thread = put(&h.app, &format!("/runs/{id}/threads/nobody/agent"), r#"{"agent":"claude-alt"}"#).await;
    assert_eq!(unknown_thread.status, StatusCode::CONFLICT, "{}", unknown_thread.json);
}
