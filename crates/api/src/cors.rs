//! CORS for the desktop window. The window's page is not served from 127.0.0.1:4737, so the browser only lets
//! it read an answer when these headers are present. Loopback only, no auth in v1: see docs/api.md.

use axum::body::Body;
use axum::http::{header, HeaderValue, Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Every response, errors included, so the window can read the `{"error": ...}` body of a 404 or 409.
pub async fn cors(request: Request<Body>, next: Next) -> Response {
    if request.method() == Method::OPTIONS {
        // The browser's preflight. It asks before a POST with a JSON body or an EventSource with a header.
        let mut response = StatusCode::NO_CONTENT.into_response();
        let headers = response.headers_mut();
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
        headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST, OPTIONS"));
        headers.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("content-type, last-event-id"));
        return response;
    }
    let mut response = next.run(request).await;
    response.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    response
}
