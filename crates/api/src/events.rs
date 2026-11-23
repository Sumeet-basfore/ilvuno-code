//! `GET /runs/:id/events`: one SSE event per journal row appended while the client is connected.

use crate::blocking::{blocking, open_store, require_run};
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use runner::{query, Store};
use serde::Serialize;
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

const POLL_EVERY: Duration = Duration::from_millis(250);
const HEARTBEAT_EVERY: Duration = Duration::from_secs(15);
const BUFFERED_EVENTS: usize = 64;

/// The data of one `journal` event. Field order is the wire order.
#[derive(Serialize)]
struct JournalEvent {
    seq: i64,
    kind: String,
    at: i64,
}

pub async fn stream<P>(
    State(state): State<AppState<P>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse>
where
    P: Send + Sync + 'static,
{
    // A reconnecting client sends the seq of the last event it saw; it gets everything after that.
    let resume_from: Option<i64> = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse().ok());
    let home = state.home.clone();
    let (store, after) = blocking({
        let run_id = run_id.clone();
        move || {
            let store = open_store(&home)?;
            require_run(&store, &run_id)?;
            let after = match resume_from {
                Some(seq) => seq,
                None => query::last_seq(store.conn(), &run_id)?,
            };
            Ok((store, after))
        }
    })
    .await?;

    let (tx, rx) = mpsc::channel(BUFFERED_EVENTS);
    tokio::spawn(poll_journal(store, run_id, after, tx));
    let sse = Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::new().interval(HEARTBEAT_EVERY));
    Ok(sse)
}

/// Reads the journal on a timer and sends each new row. Ends when the client goes away.
async fn poll_journal(mut store: Store, run_id: String, mut after: i64, tx: mpsc::Sender<Result<Event, Infallible>>) {
    let mut tick = tokio::time::interval(POLL_EVERY);
    loop {
        tokio::select! {
            _ = tx.closed() => return,
            _ = tick.tick() => {}
        }
        let id = run_id.clone();
        let read = tokio::task::spawn_blocking(move || {
            let rows = query::journal_after(store.conn(), &id, after);
            (store, rows)
        })
        .await;
        let (returned, rows) = match read {
            Ok(pair) => pair,
            Err(err) => {
                eprintln!("ilvunod: journal stream for run {run_id} stopped: {err}");
                return;
            }
        };
        store = returned;
        let rows = match rows {
            Ok(rows) => rows,
            Err(err) => {
                eprintln!("ilvunod: journal stream for run {run_id} stopped: {err:#}");
                return;
            }
        };
        for row in rows {
            after = row.seq;
            let payload = JournalEvent { seq: row.seq, kind: row.kind, at: row.at };
            let data = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            let event = Event::default().event("journal").id(row.seq.to_string()).data(data);
            if tx.send(Ok(event)).await.is_err() {
                return;
            }
        }
    }
}
