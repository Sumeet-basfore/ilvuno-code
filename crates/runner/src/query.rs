//! Read-only views for the API: the run list, existence checks, and the journal after a sequence number.
//! Nothing here writes. Shapes match what the CLI and the export already report.

use crate::goal;
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

/// One journal row, as a live reader sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub seq: i64,
    pub at: i64,
    pub kind: String,
}

pub fn run_exists(conn: &Connection, run_id: &str) -> Result<bool> {
    Ok(conn
        .query_row("SELECT 1 FROM runs WHERE id = ?1", params![run_id], |r| r.get::<_, i64>(0))
        .optional()?
        .is_some())
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
    let mut stmt = conn.prepare("SELECT seq, at, kind FROM journal WHERE run_id = ?1 AND seq > ?2 ORDER BY seq")?;
    let rows = stmt
        .query_map(params![run_id, after], |r| Ok(JournalEntry { seq: r.get(0)?, at: r.get(1)?, kind: r.get(2)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}
