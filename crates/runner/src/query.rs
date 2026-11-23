//! Read-only views for the API: the run list, existence checks, and the journal after a sequence number.
//! Nothing here writes. Shapes match what the CLI and the export already report.

use crate::goal;
use anyhow::Result;
use rusqlite::{params, params_from_iter, types::Value as SqlValue, Connection, OptionalExtension};
use serde_json::{json, Value};

/// Journal kinds a caller may filter on. Unknown kinds are rejected by the API instead of silently
/// returning an empty page.
pub const JOURNAL_KINDS: &[&str] = &[
    "artifact.missing",
    "artifact.registered",
    "dispatch.failed",
    "dispatch.message",
    "dispatch.queued",
    "dispatch.refused",
    "dispatch.reported",
    "dispatch.requeued",
    "dispatch.started",
    "gate.decided",
    "git.commit",
    "git.commit_failed",
    "git.fallback",
    "git.worktree",
    "ledger.appended",
    "merge.conflict",
    "merge.landed",
    "merge.refused",
    "merge.requested",
    "note.refused",
    "permission.decided",
    "prompt.injected",
    "round.planned",
    "round.verdict",
    "run.created",
    "run.git",
    "run.transition",
    "task.refused",
    "thread.agent",
    "thread.spawned",
    "tool.call",
    "usage.recorded",
];

/// One journal row, as a live reader sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalEntry {
    pub seq: i64,
    pub at: i64,
    pub kind: String,
    pub round: Option<i64>,
    pub thread: Option<String>,
    /// Present for tool activity rows and the dispatch ids needed to place them under one turn.
    pub payload: Option<Value>,
}

/// A journal row for history views. `round` and `thread` are display names, resolved without reading
/// any task result text.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalRecord {
    pub seq: i64,
    pub at: i64,
    pub kind: String,
    pub round: Option<i64>,
    pub thread: Option<String>,
    pub payload: Value,
}

/// The status of a run, or `None` when no run has this id.
pub fn run_status(conn: &Connection, run_id: &str) -> Result<Option<String>> {
    Ok(conn.query_row("SELECT status FROM runs WHERE id = ?1", params![run_id], |r| r.get(0)).optional()?)
}

/// Every run, newest first. Each row is the summary the run list shows.
pub fn list_runs(conn: &Connection) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.status, g.title, r.rounds_used, r.tokens_used, r.started_at
         FROM runs r JOIN goals g ON g.id = r.goal_id
         ORDER BY r.started_at DESC, r.rowid DESC",
    )?;
    let rows: Vec<(String, String, String, i64, i64, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::with_capacity(rows.len());
    for (id, status, title, rounds_used, tokens_used, started_at) in rows {
        // Same folder the export's `run.project` reports.
        let project = goal::load_snapshot(conn, &id).ok().map(|p| p.base().to_string_lossy().to_string());
        out.push(json!({
            "id": id, "status": status, "title": title, "project": project,
            "rounds_used": rounds_used, "tokens_used": tokens_used, "started_at": started_at,
        }));
    }
    Ok(out)
}

/// The newest journal sequence number of a run, or 0 when it has none.
pub fn last_seq(conn: &Connection, run_id: &str) -> Result<i64> {
    Ok(conn.query_row("SELECT COALESCE(MAX(seq), 0) FROM journal WHERE run_id = ?1", params![run_id], |r| r.get(0))?)
}

/// Journal rows of a run with a sequence number after `after`, oldest first.
pub fn journal_after(conn: &Connection, run_id: &str, after: i64) -> Result<Vec<JournalEntry>> {
    let mut stmt = conn.prepare(
        "SELECT j.seq, j.at, j.kind, r.\"index\", t.name,
                CASE WHEN j.kind IN (
                    'tool.call', 'permission.decided', 'dispatch.failed',
                    'dispatch.queued', 'dispatch.started', 'dispatch.reported'
                ) THEN j.payload END
         FROM journal j
         LEFT JOIN rounds r ON r.id = j.round_id
         LEFT JOIN threads t ON t.id = j.thread_id
         WHERE j.run_id = ?1 AND j.seq > ?2
         ORDER BY j.seq",
    )?;
    let rows = stmt
        .query_map(params![run_id, after], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows
        .into_iter()
        .map(|(seq, at, kind, round, thread, payload)| {
            let payload = payload.and_then(|text| serde_json::from_str::<Value>(&text).ok());
            let payload = if matches!(kind.as_str(), "dispatch.queued" | "dispatch.started" | "dispatch.reported") {
                payload.map(|value| json!({ "dispatch_id": value.get("dispatch_id").cloned().unwrap_or(Value::Null) }))
            } else {
                payload
            };
            JournalEntry { seq, at, kind, round, thread, payload }
        })
        .collect())
}

/// Read a bounded journal page in sequence order. The query reads the journal payload and light round
/// and thread labels only; it never joins dispatch rows or their result text.
pub fn journal_page(
    conn: &Connection,
    run_id: &str,
    after: i64,
    kinds: &[String],
    limit: usize,
) -> Result<Vec<JournalRecord>> {
    let kind_clause = if kinds.is_empty() {
        String::new()
    } else {
        let placeholders = (0..kinds.len()).map(|i| format!("?{}", i + 3)).collect::<Vec<_>>().join(", ");
        format!(" AND j.kind IN ({placeholders})")
    };
    let limit_index = kinds.len() + 3;
    let sql = format!(
        "SELECT j.seq, j.at, j.kind, r.\"index\", t.name, j.payload
         FROM journal j
         LEFT JOIN rounds r ON r.id = j.round_id
         LEFT JOIN threads t ON t.id = j.thread_id
         WHERE j.run_id = ?1 AND j.seq > ?2{kind_clause}
         ORDER BY j.seq ASC LIMIT ?{limit_index}"
    );
    let mut values = vec![SqlValue::Text(run_id.to_string()), SqlValue::Integer(after)];
    values.extend(kinds.iter().cloned().map(SqlValue::Text));
    values.push(SqlValue::Integer(limit.min(i64::MAX as usize) as i64));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(values), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, String>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(rows
        .into_iter()
        .map(|(seq, at, kind, round, thread, payload)| JournalRecord {
            seq,
            at,
            kind,
            round,
            thread,
            payload: serde_json::from_str(&payload).unwrap_or(Value::Null),
        })
        .collect())
}
