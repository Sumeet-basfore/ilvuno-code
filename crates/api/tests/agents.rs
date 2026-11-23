//! `GET /agents`: the list the settings screen shows, and that the window may read it.

mod common;

use api::agents::Agent;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{get, harness};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_route_answers_with_the_id_name_path_found_version_and_safety() {
    let h = harness();
    // Pinned rather than detected, so the test never looks at this machine's PATH or runs a process.
    h.state.agents.put(vec![
        Agent {
            id: "claude",
            name: "Claude Code",
            path: Some("/home/u/.local/bin/claude".into()),
            found: true,
            version: Some("2.1.329".into()),
            safety: "asks_before_each_tool",
        },
        Agent { id: "codex", name: "Codex", path: None, found: false, version: None, safety: "works_in_folder_asks_outside" },
    ]);

    let reply = get(&h.app, "/agents").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.json,
        json!([
            { "id": "claude", "name": "Claude Code", "path": "/home/u/.local/bin/claude", "found": true,
              "version": "2.1.329", "safety": "asks_before_each_tool" },
            { "id": "codex", "name": "Codex", "path": null, "found": false, "version": null,
              "safety": "works_in_folder_asks_outside" }
        ])
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_window_can_read_it_and_a_hung_version_call_is_reported_as_no_version() {
    let h = harness();
    // A found agent whose `--version` said nothing in time: still found, version unknown, not guessed.
    h.state.agents.put(vec![Agent {
        id: "pi",
        name: "Pi",
        path: Some("/usr/local/bin/pi".into()),
        found: true,
        version: None,
        safety: "works_in_folder_asks_outside",
    }]);

    let request = Request::builder().method("GET").uri("/agents").header("origin", "tauri://localhost").body(Body::empty()).unwrap();
    let response = h.app.clone().oneshot(request).await.expect("agents response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["access-control-allow-origin"], "tauri://localhost");

    let reply = get(&h.app, "/agents").await;
    assert_eq!(reply.json[0]["found"], json!(true));
    assert_eq!(reply.json[0]["version"], json!(null));
}
