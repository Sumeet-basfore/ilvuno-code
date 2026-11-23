//! Run routes: health, list, detail, board, start, resume, and the error shape.

mod common;

use axum::http::StatusCode;
use common::{get, harness, post, send, start_run, wait_idle, GOAL_WORKERS};
use runner::{export, Store};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_reports_ok_and_version() {
    let h = harness();
    let reply = get(&h.app, "/health").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json, serde_json::json!({ "ok": true, "version": "0.0.1" }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runs_list_is_newest_first_with_summary_fields() {
    let h = harness();
    let first = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;
    let second = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    let reply = get(&h.app, "/runs").await;
    assert_eq!(reply.status, StatusCode::OK);
    let runs = reply.json.as_array().expect("a JSON array");
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0]["id"], second.as_str());
    assert_eq!(runs[1]["id"], first.as_str());
    let row = &runs[0];
    assert_eq!(row["status"], "working");
    assert_eq!(row["title"], "Write notes for one source.");
    assert_eq!(row["rounds_used"], 0);
    assert_eq!(row["tokens_used"], 0);
    assert!(row["started_at"].is_i64());
    assert!(row["project"].is_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_detail_is_the_export_run_json() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    let reply = get(&h.app, &format!("/runs/{id}")).await;
    assert_eq!(reply.status, StatusCode::OK);
    let store = Store::open(&h.home.db_path()).expect("store");
    let expected = export::run_json(store.conn(), &id).expect("run json");
    assert_eq!(reply.json, expected, "the API must return the export's run.json shape unchanged");
    for key in ["run", "goal", "threads", "rounds", "tasks", "ledger", "artifacts"] {
        assert!(reply.json.get(key).is_some(), "missing top-level key {key}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn board_is_markdown_for_the_run() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;
    let reply = get(&h.app, &format!("/runs/{id}/board")).await;
    assert_eq!(reply.status, StatusCode::OK);
    let markdown = reply.json["markdown"].as_str().expect("markdown string");
    assert!(markdown.starts_with("# Board"), "board: {markdown}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_run_and_route_are_json_404s() {
    let h = harness();
    for uri in ["/runs/no-such-run", "/runs/no-such-run/board", "/nothing-here"] {
        let reply = get(&h.app, uri).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(reply.json["kind"], "not_found", "{uri}");
        assert!(reply.json["error"].is_string(), "{uri}");
    }
    let reply = post(&h.app, "/runs/no-such-run/tasks", r#"{"thread":"reader-1","prompt":"x","scope":["a"]}"#).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.json["kind"], "not_found");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_goal_is_a_json_400() {
    let h = harness();
    for body in ["not json", r#"{"goal":"x"}"#, ""] {
        let reply = post(&h.app, "/runs", body).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "body {body:?}");
        assert_eq!(reply.json["kind"], "bad_request", "body {body:?}");
        assert!(reply.json["error"].is_string());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn start_writes_goal_json_into_the_run_folder() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;
    let written = std::fs::read_to_string(h.home.run_dir(&id).join("goal.json")).expect("goal.json");
    assert_eq!(written, GOAL_WORKERS);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_busy_process_refuses_a_second_run_with_409() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    let claim = h.state.claim_drive("test").expect("claim");
    let resume = post(&h.app, &format!("/runs/{id}/resume"), "").await;
    assert_eq!(resume.status, StatusCode::CONFLICT);
    assert_eq!(resume.json["kind"], "conflict");
    assert!(resume.json["error"].as_str().unwrap_or_default().contains("already being driven"), "{}", resume.json);

    let second = post(&h.app, "/runs", GOAL_WORKERS).await;
    assert_eq!(second.status, StatusCode::CONFLICT);
    assert_eq!(second.json["kind"], "conflict");
    drop(claim);

    let resumed = send(&h.app, "POST", &format!("/runs/{id}/resume"), None).await;
    assert_eq!(resumed.status, StatusCode::ACCEPTED);
    assert_eq!(resumed.json["run_id"], id.as_str());
    wait_idle(&h.state).await;
}
