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
    assert_eq!(headers["access-control-allow-origin"], "tauri://localhost");
    assert_eq!(headers["access-control-allow-methods"], "GET, POST, PUT, OPTIONS");
    assert_eq!(headers["access-control-allow-headers"], "content-type, last-event-id");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn error_responses_carry_the_origin_header_so_the_window_can_read_them() {
    let h = harness();
    let request = Request::builder()
        .method("GET")
        .uri("/runs/no-such-run")
        .header("origin", "http://tauri.localhost")
        .body(Body::empty())
        .unwrap();
    let response = h.app.clone().oneshot(request).await.expect("404 response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.headers()["access-control-allow-origin"], "http://tauri.localhost");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_still_answers_with_the_origin_header() {
    let h = harness();
    let request = Request::builder().method("GET").uri("/health").header("origin", "tauri://localhost").body(Body::empty()).unwrap();
    let response = h.app.clone().oneshot(request).await.expect("health response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["access-control-allow-origin"], "tauri://localhost");
    let reply = get(&h.app, "/health").await;
    assert_eq!(reply.json, serde_json::json!({ "ok": true, "version": "0.0.1" }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_foreign_origin_gets_no_cors_header_so_the_browser_blocks_the_read() {
    let h = harness();
    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .header("origin", "https://evil.example")
        .body(Body::empty())
        .unwrap();
    let response = h.app.clone().oneshot(request).await.expect("response");
    assert!(response.headers().get("access-control-allow-origin").is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_post_that_is_not_json_is_refused_before_it_reaches_a_handler() {
    let h = harness();
    let request = Request::builder()
        .method("POST")
        .uri("/runs")
        .header("origin", "https://evil.example")
        .header("content-type", "text/plain")
        .body(Body::from("{}"))
        .unwrap();
    let response = h.app.clone().oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}
