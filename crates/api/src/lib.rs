//! `ilvunod`: a small HTTP door onto the runner, for a desktop app. Loopback only, no auth in v1.
//! Every action calls the same function the `ilvuno` CLI calls. See docs/api.md.

pub mod actions;
pub mod agents;
pub mod blocking;
pub mod cors;
pub mod drive;
pub mod error;
pub mod events;
pub mod runs;
pub mod settings;
pub mod state;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use error::ApiError;
use provider_core::Provider;
use serde_json::{json, Value};
use state::AppState;

pub fn router<P>(state: AppState<P>) -> Router
where
    P: Provider + Send + Sync + 'static,
{
    Router::new()
        .route("/health", get(health))
        .route("/runs", get(runs::list::<P>).post(runs::create::<P>))
        .route("/runs/{id}", get(runs::detail::<P>))
        .route("/runs/{id}/threads/{thread}/diff", get(runs::thread_diff::<P>))
        .route("/runs/{id}/board", get(runs::board::<P>))
        .route("/runs/{id}/events", get(events::stream::<P>))
        .route("/runs/{id}/tasks", post(actions::create_task::<P>))
        .route("/runs/{id}/gate", post(actions::gate::<P>))
        .route("/runs/{id}/threads/{thread}/agent", axum::routing::put(runs::set_thread_agent::<P>))
        .route("/runs/{id}/stop", post(runs::stop::<P>))
        .route("/runs/{id}/resume", post(runs::resume::<P>))
        .route("/agents", get(agents::list::<P>))
        .route("/settings", get(settings::read_route::<P>).put(settings::write_route::<P>))
        .fallback(no_such_route)
        .method_not_allowed_fallback(wrong_method)
        .with_state(state)
        .layer(axum::middleware::from_fn(cors::cors))
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

async fn no_such_route() -> impl IntoResponse {
    ApiError::NotFound("no such route".into())
}

async fn wrong_method() -> impl IntoResponse {
    (StatusCode::METHOD_NOT_ALLOWED, ApiError::BadRequest("this route does not accept that method".into()))
}
