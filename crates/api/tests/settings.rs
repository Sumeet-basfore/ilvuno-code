//! `GET /settings` and `PUT /settings`: the defaults, the round-trip, and every refusal.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{get, harness, send};
use serde_json::json;
use tower::ServiceExt;

fn defaults() -> serde_json::Value {
    json!({ "budget_rounds": 3, "budget_tokens": 150_000, "message_policy": "round", "theme": "dark", "human_gate": true })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_untouched_home_reads_as_the_defaults() {
    let h = harness();
    let reply = get(&h.app, "/settings").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json, defaults());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_put_answers_with_the_full_object_and_a_get_reads_it_back() {
    let h = harness();
    let reply = send(&h.app, "PUT", "/settings", Some(r#"{"theme":"light","budget_rounds":5}"#)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.json,
        json!({ "budget_rounds": 5, "budget_tokens": 150_000, "message_policy": "round", "theme": "light", "human_gate": true })
    );
    assert_eq!(get(&h.app, "/settings").await.json, reply.json);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_partial_put_leaves_the_other_keys_alone() {
    let h = harness();
    send(&h.app, "PUT", "/settings", Some(r#"{"message_policy":"never","human_gate":false}"#)).await;
    let reply = send(&h.app, "PUT", "/settings", Some(r#"{"budget_tokens":5000}"#)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json["message_policy"], json!("never"));
    assert_eq!(reply.json["human_gate"], json!(false));
    assert_eq!(reply.json["budget_tokens"], json!(5000));
    assert_eq!(reply.json["theme"], json!("dark"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_bad_value_is_a_400_and_nothing_is_written() {
    let h = harness();
    for body in [
        r#"{"budget_rounds":0}"#,
        r#"{"budget_rounds":21}"#,
        r#"{"budget_tokens":999}"#,
        r#"{"budget_tokens":10000001}"#,
        r#"{"message_policy":"later"}"#,
        r#"{"theme":"sepia"}"#,
        r#"{"human_gate":"yes"}"#,
        r#"{"budget_rounds":"3"}"#,
        r#"{"budget_rounds":3,"watch_every_step":true}"#,
        r#"{"budget_rounds":3,"theme":"light","message_policy":"later"}"#,
        r#"not json"#,
        r#"[]"#,
    ] {
        let reply = send(&h.app, "PUT", "/settings", Some(body)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{body} should be a 400, got {}", reply.json);
        assert_eq!(reply.json["kind"], json!("bad_request"));
    }
    assert_eq!(get(&h.app, "/settings").await.json, defaults(), "a refused write must not have written anything");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_put_without_a_json_content_type_is_refused_and_the_window_can_still_read() {
    let h = harness();
    let request = Request::builder()
        .method("PUT")
        .uri("/settings")
        .header("content-type", "text/plain")
        .body(Body::from(r#"{"theme":"light"}"#))
        .unwrap();
    let response = h.app.clone().oneshot(request).await.expect("put response");
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let request = Request::builder().method("GET").uri("/settings").header("origin", "tauri://localhost").body(Body::empty()).unwrap();
    let response = h.app.clone().oneshot(request).await.expect("get response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["access-control-allow-origin"], "tauri://localhost");
}
