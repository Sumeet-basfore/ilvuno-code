//! The one error shape every route returns: `{"error": "...", "kind": "..."}`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    /// The request itself is malformed: bad JSON, a missing field, an invalid goal.
    BadRequest(String),
    NotFound(String),
    /// The runner refused the command, or the process is busy. The message is the runner's reason.
    Conflict(String),
    /// Something broke inside. The detail goes to the log; the client gets a generic message.
    Internal(String),
}

pub type ApiResult<T> = Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, kind, message) = match self {
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m),
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, "not_found", m),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, "conflict", m),
            ApiError::Internal(detail) => {
                eprintln!("ilvunod: internal error: {detail}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal", "internal error; see the ilvunod log".to_string())
            }
        };
        (status, Json(json!({ "error": message, "kind": kind }))).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(format!("{err:#}"))
    }
}
