//! `GET /runs/:id/events`: the SSE frame format, and replay from `Last-Event-ID`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{harness, post, start_run, wait_idle, wait_status, GOAL_WITH_ORCHESTRATOR, GOAL_WORKERS};
use provider_core::Event;
use serde_json::json;
use std::time::Duration;
use tower::ServiceExt;

/// Read SSE frames from a streaming body until `want` journal events have arrived. Each frame is the text
/// before a blank line. Gives up after five seconds.
async fn read_frames(body: Body, want: usize) -> Vec<String> {
    use tokio_stream::StreamExt;
    let mut stream = body.into_data_stream();
    let mut buffer = String::new();
    let mut frames = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while frames.len() < want {
        let chunk = tokio::time::timeout_at(deadline, stream.next()).await.expect("an event within five seconds");
        let bytes = chunk.expect("stream still open").expect("no body error");
        buffer.push_str(std::str::from_utf8(&bytes).expect("utf-8 frames"));
        while let Some(end) = buffer.find("\n\n") {
            let frame: String = buffer.drain(..end + 2).collect();
            if frame.contains("event: journal") {
                frames.push(frame);
            }
        }
    }
    frames
}

fn data_of(frame: &str) -> serde_json::Value {
    let data = frame.lines().find_map(|l| l.strip_prefix("data: ")).expect("a data line");
    serde_json::from_str(data).expect("data is JSON")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_new_journal_row_is_sent_as_a_journal_event() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    let request = Request::builder().uri(format!("/runs/{id}/events")).body(Body::empty()).unwrap();
    let response = h.app.clone().oneshot(request).await.expect("events response");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers()["content-type"].to_str().unwrap().starts_with("text/event-stream"));

    // Appended while the client is connected: the task writes journal rows the stream must send.
    let task = post(&h.app, &format!("/runs/{id}/tasks"), r#"{"thread":"reader-1","prompt":"Read it.","scope":["a.md"]}"#).await;
    assert_eq!(task.status, StatusCode::CREATED);

    // A task writes three journal rows: the round opens, the intent is noted, the task is queued.
    let frames = read_frames(response.into_body(), 3).await;
    for frame in &frames {
        assert!(frame.starts_with("event: journal\n"), "frame: {frame:?}");
        assert!(frame.lines().any(|l| l.starts_with("id: ")), "frame has an id line: {frame:?}");
        let data = data_of(frame);
        let keys: Vec<&str> = data.as_object().unwrap().keys().map(String::as_str).collect();
        if data["kind"] == "dispatch.queued" {
            assert_eq!(keys.len(), 6, "dispatch boundary carries turn labels and its id: {data}");
            assert_eq!(data["payload"].as_object().unwrap().len(), 1, "only the dispatch id is sent: {data}");
        } else {
            assert_eq!(keys.len(), 3, "ordinary rows keep seq, kind and at: {data}");
        }
        assert!(data["seq"].is_i64() && data["at"].is_i64() && data["kind"].is_string(), "{data}");
        let id_line: i64 = frame.lines().find_map(|l| l.strip_prefix("id: ")).unwrap().parse().unwrap();
        assert_eq!(id_line, data["seq"].as_i64().unwrap(), "the id line is the seq");
    }
    let kinds: Vec<String> = frames.iter().map(|f| data_of(f)["kind"].as_str().unwrap().to_string()).collect();
    assert_eq!(kinds, ["round.planned", "ledger.appended", "dispatch.queued"], "kinds in journal order");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn last_event_id_replays_everything_after_it() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;
    let task = post(&h.app, &format!("/runs/{id}/tasks"), r#"{"thread":"reader-1","prompt":"Read it.","scope":["a.md"]}"#).await;
    assert_eq!(task.status, StatusCode::CREATED);

    let store = runner::Store::open(&h.home.db_path()).expect("store");
    let all = runner::query::journal_after(store.conn(), &id, 0).expect("journal");
    assert!(all.len() >= 2, "the run has journal rows");
    let first = all[0].seq;

    let request = Request::builder()
        .uri(format!("/runs/{id}/events"))
        .header("last-event-id", "0")
        .body(Body::empty())
        .unwrap();
    let response = h.app.clone().oneshot(request).await.expect("events response");
    let frames = read_frames(response.into_body(), all.len()).await;
    let seqs: Vec<i64> = frames.iter().map(|f| data_of(f)["seq"].as_i64().unwrap()).collect();
    assert_eq!(seqs[0], first, "replay starts right after Last-Event-ID");
    assert_eq!(seqs, all.iter().map(|r| r.seq).collect::<Vec<_>>());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activity_events_include_their_turn_and_bounded_payload() {
    let h = harness();
    let events = vec![
        Event::ToolCall {
            id: "tool-1".into(),
            name: "Bash".into(),
            input: json!({"command": "echo notes"}),
            kind: "tool".into(),
            status: "requested".into(),
        },
        Event::PermissionRequest {
            request_id: "permission-1".into(),
            tool: "Write".into(),
            input: json!({"file_path": "/etc/ilvuno-test-file", "content": "x"}),
            options: vec!["allow".into(), "deny".into()],
        },
        Event::TextDelta { text: "done".into() },
        Event::Usage {
            tokens_in: 10,
            tokens_out: 5,
            cache_read_tokens: 0,
            context_used: None,
            cost_usd: None,
        },
        Event::TurnFinished { stop_reason: Some("end_turn".into()), interrupted: false, error: None },
    ];
    h.fake.push_turn_front(events);

    let id = start_run(&h.app, GOAL_WITH_ORCHESTRATOR).await;
    wait_status(&h.app, &id, "reviewed").await;
    wait_idle(&h.state).await;
    let store = runner::Store::open(&h.home.db_path()).expect("store");
    let all = runner::query::journal_after(store.conn(), &id, 0).expect("journal");

    let request = Request::builder()
        .uri(format!("/runs/{id}/events"))
        .header("last-event-id", "0")
        .body(Body::empty())
        .unwrap();
    let response = h.app.clone().oneshot(request).await.expect("events response");
    let frames = read_frames(response.into_body(), all.len()).await;
    let rows: Vec<serde_json::Value> = frames.iter().map(|frame| data_of(frame)).collect();
    let call = rows.iter().find(|row| row["kind"] == "tool.call").expect("tool call event");
    assert_eq!(call["thread"], "orchestrator");
    assert_eq!(call["round"], 0);
    assert_eq!(call["payload"]["tool"], "Bash");
    assert_eq!(call["payload"]["input"]["command"], "echo notes");

    let permission = rows.iter().find(|row| row["kind"] == "permission.decided").expect("permission event");
    assert_eq!(permission["thread"], "orchestrator");
    assert_eq!(permission["round"], 0);
    assert_eq!(permission["payload"]["tool"], "Write");
    assert_eq!(permission["payload"]["behavior"], "deny");

    for row in rows.iter().filter(|row| {
        !matches!(
            row["kind"].as_str(),
            Some("tool.call" | "permission.decided" | "dispatch.failed" | "dispatch.queued" | "dispatch.started" | "dispatch.reported")
        )
    }) {
        assert!(row.get("payload").is_none(), "non-activity events do not carry payload: {row}");
    }

    for row in rows.iter().filter(|row| {
        matches!(row["kind"].as_str(), Some("dispatch.queued" | "dispatch.started" | "dispatch.reported"))
    }) {
        assert!(row["thread"] == "orchestrator" || row["thread"] == "reader-1", "boundary has a thread label: {row}");
        assert_eq!(row["payload"].as_object().unwrap().len(), 1, "boundary payload only carries dispatch_id: {row}");
        assert!(row["payload"]["dispatch_id"].is_string(), "boundary dispatch id: {row}");
    }
}
