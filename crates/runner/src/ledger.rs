//! Append-only ledger (the board). Every append also journals `ledger.appended`.

use crate::store::{now_ms, Pending};
use anyhow::Result;
use rusqlite::{params, Connection, Transaction};
use serde_json::{json, Value};
use std::path::Path;

pub fn append(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    run_id: &str,
    round_id: Option<&str>,
    thread_id: Option<&str>,
    kind: &str,
    scope: &[String],
    body: &str,
    evidence: Value,
) -> Result<i64> {
    tx.execute(
        "INSERT INTO ledger (run_id, round_id, thread_id, kind, scope, body, evidence, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            run_id,
            round_id,
            thread_id,
            kind,
            serde_json::to_string(scope)?,
            body,
            evidence.to_string(),
            now_ms()
        ],
    )?;
    let id = tx.last_insert_rowid();
    pending.push(
        run_id,
        round_id,
        thread_id,
        "ledger.appended",
        json!({ "ledger_id": id, "kind": kind, "body": body, "scope": scope }),
    );
    Ok(id)
}

/// Scopes of every `intent` row written for a round.
pub fn intent_scopes(conn: &Connection, round_id: &str) -> Result<Vec<Vec<String>>> {
    let mut stmt = conn.prepare("SELECT scope FROM ledger WHERE round_id = ?1 AND kind = 'intent' ORDER BY id")?;
    let rows = stmt.query_map(params![round_id], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(serde_json::from_str(&row?)?);
    }
    Ok(out)
}

/// Write the board every worker is told to read (`<run>/board.md`).
pub fn render_board(conn: &Connection, run_id: &str, out: &Path) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT l.id, l.kind, l.body, l.scope, r.\"index\", l.thread_id
         FROM ledger l LEFT JOIN rounds r ON r.id = l.round_id
         WHERE l.run_id = ?1 ORDER BY l.id",
    )?;
    let rows = stmt.query_map(params![run_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<i64>>(4)?,
            r.get::<_, Option<String>>(5)?,
        ))
    })?;
    let mut text = String::from("# Board\n\nAppend-only. Newest last.\n\n");
    for row in rows {
        let (id, kind, body, scope, round, thread) = row?;
        let round = round.map(|i| format!("round {i}")).unwrap_or_else(|| "run".into());
        let thread = thread.unwrap_or_else(|| "runner".into());
        text.push_str(&format!("- #{id} **{kind}** ({round}, thread {thread}) scope {scope}: {body}\n"));
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, text)?;
    Ok(())
}
