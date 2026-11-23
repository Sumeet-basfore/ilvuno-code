//! Bounded, filtered reads of the run journal.

mod common;

use axum::http::StatusCode;
use common::{get, harness, start_run, wait_idle, wait_status, GOAL_WITH_ORCHESTRATOR, GOAL_WORKERS};
use provider_core::Event;
use serde_json::json;

fn scripted_tail() -> Vec<Event> {
    vec![
        Event::TextDelta { text: "done".into() },
        Event::Usage {
            tokens_in: 10,
            tokens_out: 5,
            cache_read_tokens: 0,
            context_used: None,
            cost_usd: None,
        },
        Event::TurnFinished { stop_reason: Some("end_turn".into()), interrupted: false, error: None },
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn journal_pages_are_ordered_and_apply_after_kinds_and_limit() {
    let h = harness();
    let id = start_run(&h.app, GOAL_WORKERS).await;
    wait_idle(&h.state).await;

    let all = get(&h.app, &format!("/runs/{id}/journal")).await;
    assert_eq!(all.status, StatusCode::OK, "{}", all.json);
    let rows = all.json.as_array().expect("journal rows");
    assert!(rows.len() >= 3, "run rows: {}", all.json);
    assert!(
        rows.windows(2).all(|pair| pair[0]["seq"].as_i64() < pair[1]["seq"].as_i64()),
        "ordered: {}",
        all.json
    );
    assert!(
        rows.iter().all(|row| row["payload"].is_object() || row["payload"].is_array()),
        "parsed payloads: {}",
        all.json
    );
    assert!(
        rows.iter().all(|row| row["round"].is_null() || row["round"].is_i64()),
        "round labels: {}",
        all.json
    );
    assert!(
        rows.iter().all(|row| row["thread"].is_null() || row["thread"].is_string()),
        "thread labels: {}",
        all.json
    );

    let first_page = get(&h.app, &format!("/runs/{id}/journal?limit=2")).await;
    assert_eq!(first_page.json.as_array().unwrap().len(), 2, "limit: {}", first_page.json);
    let first_seq = first_page.json[0]["seq"].as_i64().unwrap();
    let after_page = get(&h.app, &format!("/runs/{id}/journal?after={first_seq}&limit=2")).await;
    assert_eq!(after_page.status, StatusCode::OK, "{}", after_page.json);
    assert_eq!(after_page.json, json!([rows[1].clone(), rows[2].clone()]), "after and limit");

    let filtered = get(&h.app, &format!("/runs/{id}/journal?kinds=run.created")).await;
    assert_eq!(filtered.status, StatusCode::OK, "{}", filtered.json);
    assert!(
        filtered.json.as_array().unwrap().iter().all(|row| row["kind"] == "run.created"),
        "kind filter: {}",
        filtered.json
    );
    assert_eq!(filtered.json.as_array().unwrap().len(), 1, "one creation row: {}", filtered.json);

    let empty_known_kind = get(&h.app, &format!("/runs/{id}/journal?kinds=dispatch.reported")).await;
    assert_eq!(empty_known_kind.status, StatusCode::OK, "{}", empty_known_kind.json);
    assert!(empty_known_kind.json.as_array().unwrap().is_empty(), "known kind with no rows: {}", empty_known_kind.json);

    let bad_kind = get(&h.app, &format!("/runs/{id}/journal?kinds=not-a-journal-kind")).await;
    assert_eq!(bad_kind.status, StatusCode::BAD_REQUEST, "{}", bad_kind.json);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scripted_run_returns_tool_calls_and_permission_decisions_for_a_turn() {
    let h = harness();
    let mut events = vec![
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
    ];
    events.extend(scripted_tail());
    h.fake.push_turn_front(events);

    let id = start_run(&h.app, GOAL_WITH_ORCHESTRATOR).await;
    wait_status(&h.app, &id, "reviewed").await;
    wait_idle(&h.state).await;

    let reply = get(
        &h.app,
        &format!("/runs/{id}/journal?kinds=tool.call,permission.decided&limit=1000"),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.json);
    let rows = reply.json.as_array().expect("journal rows");
    let call = rows.iter().find(|row| row["kind"] == "tool.call").expect("tool.call row");
    let permission = rows
        .iter()
        .find(|row| row["kind"] == "permission.decided")
        .expect("permission.decided row");
    assert_eq!(call["thread"], "orchestrator");
    assert_eq!(call["round"], 0);
    assert_eq!(permission["payload"]["tool"], "Write");
    assert_eq!(permission["payload"]["behavior"], "deny");
    assert_eq!(call["payload"]["tool"], "Bash");
    assert_eq!(call["payload"]["input"]["command"], "echo notes");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn journal_does_not_return_a_large_tool_input_whole() {
    let h = harness();
    let large = "sensitive-value".repeat(1000);
    let mut events = vec![Event::ToolCall {
        id: "tool-large".into(),
        name: "Bash".into(),
        input: json!({"command": large}),
        kind: "tool".into(),
        status: "requested".into(),
    }];
    events.extend(scripted_tail());
    h.fake.push_turn_front(events);

    let id = start_run(&h.app, GOAL_WITH_ORCHESTRATOR).await;
    wait_status(&h.app, &id, "reviewed").await;
    wait_idle(&h.state).await;

    let reply = get(&h.app, &format!("/runs/{id}/journal?kinds=tool.call&limit=1000")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.json);
    let row = reply.json.as_array().unwrap().iter()
        .find(|row| row["payload"]["tool"] == "Bash")
        .expect("large tool row");
    let input = row["payload"]["input"].as_str().expect("truncated serialized input");
    assert!(input.chars().count() <= 2001, "stored input is capped: {} chars", input.chars().count());
    assert!(input.chars().count() < large.chars().count(), "the complete input is not returned");
}
