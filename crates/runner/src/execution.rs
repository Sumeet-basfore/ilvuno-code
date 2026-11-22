//! Commands the dispatch executor issues while a worker runs: failure, requeue after a crash, session
//! binding, artifact registration, and journal-only evidence (tool calls, permission decisions).

use crate::store::{new_id, now_ms, Outcome, Pending};
use anyhow::Result;
use rusqlite::{params, Transaction};
use serde_json::{json, Value};

/// Mark a dispatch failed (the runner's decision, e.g. after no report). Journaled as `dispatch.failed`.
pub fn fail_dispatch(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    run_id: &str,
    dispatch_id: &str,
    reason: &str,
) -> Result<Outcome<()>> {
    let (round_id, thread_id): (String, String) = tx.query_row(
        "SELECT round_id, thread_id FROM dispatch WHERE id = ?1",
        params![dispatch_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    tx.execute(
        "UPDATE dispatch SET status = 'failed', result_summary = ?2 WHERE id = ?1",
        params![dispatch_id, reason],
    )?;
    tx.execute(
        "UPDATE threads SET status = 'idle', last_active_at = ?2 WHERE id = ?1",
        params![thread_id, now_ms()],
    )?;
    pending.push(
        run_id,
        Some(&round_id),
        Some(&thread_id),
        "dispatch.failed",
        json!({ "dispatch_id": dispatch_id, "reason": reason }),
    );
    Ok(Outcome::Applied(()))
}

/// A dispatch left `running` by a crash goes back to `queued` so it can be started again. Reports
/// already committed are never touched.
pub fn requeue_dispatch(tx: &Transaction<'_>, pending: &mut Pending, run_id: &str, dispatch_id: &str) -> Result<()> {
    let round_id: String = tx.query_row("SELECT round_id FROM dispatch WHERE id = ?1", params![dispatch_id], |r| r.get(0))?;
    tx.execute("UPDATE dispatch SET status = 'queued' WHERE id = ?1 AND status = 'running'", params![dispatch_id])?;
    pending.push(run_id, Some(&round_id), None, "dispatch.requeued", json!({ "dispatch_id": dispatch_id }));
    Ok(())
}

/// Record the native session id before the process starts, so a crash can resume it.
pub fn bind_session(tx: &Transaction<'_>, pending: &mut Pending, run_id: &str, thread_id: &str, native_id: &str) -> Result<()> {
    tx.execute(
        "UPDATE threads SET native_session_id = ?2 WHERE id = ?1",
        params![thread_id, native_id],
    )?;
    pending.push(run_id, None, Some(thread_id), "thread.spawned", json!({ "native_session_id": native_id }));
    Ok(())
}

/// Register a file the run produced. Pointer and hash only; the bytes stay on disk.
#[allow(clippy::too_many_arguments)]
pub fn register_artifact(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    run_id: &str,
    round_id: &str,
    thread_id: &str,
    kind: &str,
    path: &str,
    bytes: u64,
    sha256: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO artifacts (id, run_id, round_id, thread_id, kind, path, mime, bytes, sha256, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'text/html', ?7, ?8, ?9)",
        params![new_id(), run_id, round_id, thread_id, kind, path, bytes as i64, sha256, now_ms()],
    )?;
    pending.push(
        run_id,
        Some(round_id),
        Some(thread_id),
        "artifact.registered",
        json!({ "kind": kind, "path": path, "bytes": bytes, "sha256": sha256 }),
    );
    Ok(())
}

/// Journal-only evidence: a tool call the worker made (persisted per the provider contract).
pub fn record_tool_call(pending: &mut Pending, run_id: &str, round_id: &str, thread_id: &str, payload: Value) {
    pending.push(run_id, Some(round_id), Some(thread_id), "tool.call", payload);
}

/// Journal-only evidence: the policy's answer to a permission request.
pub fn record_permission(pending: &mut Pending, run_id: &str, round_id: &str, thread_id: &str, payload: Value) {
    pending.push(run_id, Some(round_id), Some(thread_id), "permission.decided", payload);
}
