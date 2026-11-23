//! Write the board every thread is told to read (`<run>/board.md`).
//!
//! The board is for humans first. Two rules follow from that and were learned from the first experiment:
//! paths are shown relative to the run, and long entries are trimmed — the full text is in the journal.

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

/// Text that is readable without opening a transcript: what the run is for, where it stands, then the
/// entries. Each entry names its round and who wrote it.
pub fn render_board(conn: &Connection, run_id: &str, out: &Path) -> Result<()> {
    let run_root = out.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let head: (String, String, String, i64, i64) = conn.query_row(
        "SELECT g.goal_text, g.done_spec, r.status, r.rounds_used, r.tokens_used
         FROM runs r JOIN goals g ON g.id = r.goal_id WHERE r.id = ?1",
        params![run_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    let (goal, done_spec, status, rounds, tokens) = head;
    let done = serde_json::from_str::<Value>(&done_spec)
        .ok()
        .and_then(|v| v["done"].as_str().map(str::to_string))
        .unwrap_or_default();

    let mut text = String::new();
    text.push_str(&format!("# Board — {}\n\n", trim(&goal, 200)));
    if !done.is_empty() {
        text.push_str(&format!("**Done looks like:** {}\n\n", trim(&done, 200)));
    }
    text.push_str(&format!(
        "**Status:** {status} · {rounds} round(s) · {tokens} tokens used\n\n\
         Newest last. Full text of every entry: `ilvuno replay {run_id}`.\n\n"
    ));

    let mut stmt = conn.prepare(
        "SELECT l.id, l.kind, l.body, l.scope, r.\"index\", t.name, l.evidence
         FROM ledger l
         LEFT JOIN rounds r ON r.id = l.round_id
         LEFT JOIN threads t ON t.id = l.thread_id
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
            r.get::<_, Option<String>>(6)?,
        ))
    })?;
    for row in rows {
        let (id, kind, body, scope, round, thread, evidence) = row?;
        let round = round.map(|i| format!("round {i}")).unwrap_or_else(|| "run".into());
        let author = thread.or_else(|| author_without_thread(evidence.as_deref())).unwrap_or_else(|| "runner".into());
        text.push_str(&format!(
            "- **#{id} {kind}** ({round}, by {author}) {}\n  {}\n",
            shrink_paths(&scope, &run_root),
            trim(&shrink_paths(&body, &run_root), 400)
        ));
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, text)?;
    Ok(())
}

/// Absolute paths are meaningless to a reader: show them relative to the run, and name thread folders.
fn shrink_paths(s: &str, run_root: &Path) -> String {
    let root = run_root.to_string_lossy();
    if root.is_empty() {
        return s.to_string();
    }
    let mut out = s.replace(&format!("{root}/threads/"), "threads/");
    out = out.replace(&format!("{root}/"), "");
    out.replace(&root.to_string(), ".")
}

fn trim(s: &str, max: usize) -> String {
    let one: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() <= max {
        one
    } else {
        let mut cut: String = one.chars().take(max).collect();
        cut.push('…');
        cut
    }
}

fn author_without_thread(evidence: Option<&str>) -> Option<String> {
    let v: Value = serde_json::from_str(evidence?).ok()?;
    v["created_by"].as_str().or_else(|| v["author"].as_str()).map(str::to_string)
}
