//! CORS for the desktop window: preflight answers, and every response (errors included) carries the origin
//! header. `/health` must still answer, since the window polls it to decide whether the runner is up.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{get, harness};
use tower::ServiceExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn preflight_is_answered_with_204_and_the_cors_headers() {
    let h = harness();
    let request = Request::builder()
        .method("OPTIONS")
        .uri("/runs")
        .header("origin", "tauri://localhost")
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "content-type")
        .body(Body::empty())
        .unwrap();
    let response = h.app.clone().oneshot(request).await.expect("preflight response");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let headers = response.headers();
    assert_eq!(headers["access-control-allow-origin"], "*");
    assert_eq!(headers["access-control-allow-methods"], "GET, POST, OPTIONS");
    assert_eq!(headers["access-control-allow-headers"], "content-type, last-event-id");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn error_responses_carry_the_origin_header_so_the_window_can_read_them() {
    let h = harness();
    let request = Request::builder().method("GET").uri("/runs/no-such-run").body(Body::empty()).unwrap();
    let response = h.app.clone().oneshot(request).await.expect("404 response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.headers()["access-control-allow-origin"], "*");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_still_answers_with_the_origin_header() {
    let h = harness();
    let request = Request::builder().method("GET").uri("/health").header("origin", "tauri://localhost").body(Body::empty()).unwrap();
    let response = h.app.clone().oneshot(request).await.expect("health response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["access-control-allow-origin"], "*");
    let reply = get(&h.app, "/health").await;
    assert_eq!(reply.json, serde_json::json!({ "ok": true, "version": "0.0.1" }));
}
