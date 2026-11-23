//! CORS and request-origin policy for the desktop window.
//!
//! The window's page is not served from `127.0.0.1:4737`, so the browser needs these headers to let it
//! read an answer. Two rules, both deliberate:
//!
//! 1. **Only the window's origins are allowed.** `*` would let any web page open in the same browser read
//!    every run on the machine. The window uses Tauri's custom protocol, which reports as
//!    `tauri://localhost` on macOS and Linux, and `http://tauri.localhost` on Windows.
//! 2. **Every write must be `application/json`.** A cross-site form can POST `text/plain` or
//!    `x-www-form-urlencoded` without a preflight, so a page could start a run without ever passing the
//!    origin check. Requiring JSON forces a preflight, which the origin check then refuses.
//!
//! Loopback only, no authentication: see `docs/api.md`.

use axum::body::Body;
use axum::http::{header, HeaderValue, Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Origins the window may use. Anything else gets no CORS header, so the browser blocks the read.
const ALLOWED_ORIGINS: [&str; 2] = ["tauri://localhost", "http://tauri.localhost"];

fn allowed_origin(request: &Request<Body>) -> Option<HeaderValue> {
    let origin = request.headers().get(header::ORIGIN)?;
    ALLOWED_ORIGINS
        .iter()
        .find(|candidate| origin.as_bytes() == candidate.as_bytes())
        .map(|_| origin.clone())
}

/// True when the request carries a JSON body. A missing content type on a POST is a refusal, not a guess.
fn is_json(request: &Request<Body>) -> bool {
    request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or("").trim().eq_ignore_ascii_case("application/json"))
        .unwrap_or(false)
}

pub async fn cors(request: Request<Body>, next: Next) -> Response {
    let origin = allowed_origin(&request);

    // The browser's preflight: it asks before a JSON POST or an EventSource with headers.
    if request.method() == Method::OPTIONS {
        let mut response = StatusCode::NO_CONTENT.into_response();
        if let Some(origin) = origin {
            let headers = response.headers_mut();
            headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
            headers.insert(header::VARY, HeaderValue::from_static("origin"));
            headers.insert(
                header::ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_static("GET, POST, OPTIONS"),
            );
            headers.insert(
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_static("content-type, last-event-id"),
            );
        }
        return response;
    }

    // Writes must be JSON. This is what stops a foreign page from POSTing `text/plain` without a preflight.
    if request.method() == Method::POST && !is_json(&request) {
        let mut response = (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            axum::Json(serde_json::json!({
                "error": "POST bodies must be application/json",
                "kind": "bad_request",
            })),
        )
            .into_response();
        if let Some(origin) = origin {
            response.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
            response.headers_mut().insert(header::VARY, HeaderValue::from_static("origin"));
        }
        return response;
    }

    let mut response = next.run(request).await;
    if let Some(origin) = origin {
        let headers = response.headers_mut();
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        headers.insert(header::VARY, HeaderValue::from_static("origin"));
    }
    response
}
