//! SQLite calls block, so every handler moves its database work onto a blocking thread.

use crate::error::{ApiError, ApiResult};
use runner::{query, Home, Store};

/// Run database work off the async threads.
pub async fn blocking<T, F>(work: F) -> ApiResult<T>
where
    F: FnOnce() -> ApiResult<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(work).await.map_err(|e| ApiError::Internal(format!("worker thread: {e}")))?
}

/// Open this request's own connection. Connections are never shared between requests or tasks.
pub fn open_store(home: &Home) -> ApiResult<Store> {
    Ok(Store::open(&home.db_path())?)
}

/// The run's status, or `NotFound` when no run has this id.
pub fn require_run(store: &Store, run_id: &str) -> ApiResult<String> {
    query::run_status(store.conn(), run_id)?.ok_or_else(|| ApiError::NotFound(format!("no run with id {run_id}")))
}
