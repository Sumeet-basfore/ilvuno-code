//! `ilvuno export`: one run as plain files.
//!
//! This is the fixture a prototype reads, and the shape the real UI will read. It is a plain directory:
//! `run.json` (the whole picture), `board.md`, `journal.jsonl`, `artifacts/`. Nothing is invented here;
//! every field comes from the database.

use crate::goal;
use crate::home::Home;
use crate::store::Store;
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub fn export(store: &Store, home: &Home, run_id: &str, out: &Path) -> Result<PathBuf> {
    let conn = store.conn();
    std::fs::create_dir_all(out).with_context(|| format!("create {}", out.display()))?;

    let run = run_json(conn, run_id)?;
    std::fs::write(out.join("run.json"), serde_json::to_string_pretty(&run)?)?;

    let board = home.board_path(run_id);
    if board.is_file() {
        std::fs::copy(&board, out.join("board.md"))?;
    }
    journal_jsonl(conn, run_id, &out.join("journal.jsonl"))?;

    let artifacts = home.artifacts_dir(run_id);
    if artifacts.is_dir() {
        copy_dir(&artifacts, &out.join("artifacts"))?;
    }
    Ok(out.to_path_buf())
}

/// One run as the `run.json` of an export. The API serves this same shape.
pub fn run_json(conn: &Connection, run_id: &str) -> Result<Value> {
    let (status, rounds_used, tokens, started, ended, blocked): (String, i64, i64, i64, Option<i64>, Option<String>) = conn
        .query_row(
            "SELECT status, rounds_used, tokens_used, started_at, ended_at, blocked_reason FROM runs WHERE id = ?1",
            params![run_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .with_context(|| format!("run {run_id}"))?;

    let goal_row: (String, String, String, String, i64, i64) = conn.query_row(
        "SELECT g.goal_text, g.done_spec, g.message_policy, g.id, g.budget_rounds, g.budget_tokens
         FROM runs r JOIN goals g ON g.id = r.goal_id WHERE r.id = ?1",
        params![run_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
    )?;
    let (goal_text, done_spec, policy, goal_id, budget_rounds, budget_tokens) = goal_row;
    let done: Value = serde_json::from_str(&done_spec).unwrap_or(json!({}));
    // The project is the folder the goal file was written in: no separate table yet.
    let project = goal::load_snapshot(conn, run_id).ok().map(|p| p.base().to_string_lossy().to_string());

    let threads = rows(conn, "SELECT name, provider, role, prompt, status, native_session_id FROM threads WHERE run_id = ?1 ORDER BY name", run_id, |r| {
        Ok(json!({
            "name": r.get::<_, String>(0)?, "agent": r.get::<_, String>(1)?, "role": r.get::<_, String>(2)?,
            "prompt": r.get::<_, String>(3)?, "status": r.get::<_, String>(4)?, "native_session": r.get::<_, Option<String>>(5)?,
        }))
    })?;

    let rounds = rows(conn, "SELECT \"index\", status, verdict, readiness, tokens_delta, cost_delta FROM rounds WHERE run_id = ?1 ORDER BY \"index\"", run_id, |r| {
        let readiness: Option<String> = r.get(3)?;
        Ok(json!({
            "index": r.get::<_, i64>(0)?, "status": r.get::<_, String>(1)?, "verdict": r.get::<_, Option<String>>(2)?,
            "readiness": readiness.map(|s| serde_json::from_str::<Value>(&s).unwrap_or(Value::Null)),
            "tokens": r.get::<_, Option<i64>>(4)?, "cost_usd": r.get::<_, Option<f64>>(5)?,
        }))
    })?;

    let tasks = rows(conn, "SELECT d.task, d.status, d.created_by, d.scope, t.name, d.result_summary, d.result_text, r.\"index\", d.kind
         FROM dispatch d JOIN threads t ON t.id = d.thread_id JOIN rounds r ON r.id = d.round_id
         WHERE r.run_id = ?1 ORDER BY r.\"index\", d.rowid", run_id, |r| {
        let scope: String = r.get(3)?;
        Ok(json!({
            "prompt": r.get::<_, String>(0)?, "status": r.get::<_, String>(1)?, "created_by": r.get::<_, String>(2)?,
            "scope": serde_json::from_str::<Value>(&scope).unwrap_or(Value::Null),
            "thread": r.get::<_, String>(4)?, "result": r.get::<_, Option<String>>(5)?,
            "message": r.get::<_, Option<String>>(6)?, "round": r.get::<_, i64>(7)?,
            "kind": r.get::<_, String>(8)?,
        }))
    })?;

    let ledger = rows(conn, "SELECT l.id, l.kind, l.body, l.scope, r.\"index\", t.name, l.evidence
         FROM ledger l LEFT JOIN rounds r ON r.id = l.round_id LEFT JOIN threads t ON t.id = l.thread_id
         WHERE l.run_id = ?1 ORDER BY l.id", run_id, |r| {
        let scope: String = r.get(3)?;
        let thread: Option<String> = r.get(5)?;
        let evidence: Option<String> = r.get(6)?;
        // Who wrote it: the thread, or, for entries without one, the recorded creator (user / runner).
        // Without this the UI had to guess from board.md, which only carries a rendering of the same fact.
        let author = thread.clone().or_else(|| {
            evidence.as_deref()
                .and_then(|e| serde_json::from_str::<Value>(e).ok())
                .and_then(|v| v["created_by"].as_str().map(str::to_string))
        }).unwrap_or_else(|| "runner".to_string());
        Ok(json!({
            "id": r.get::<_, i64>(0)?, "kind": r.get::<_, String>(1)?, "body": r.get::<_, String>(2)?,
            "scope": serde_json::from_str::<Value>(&scope).unwrap_or(Value::Null),
            "round": r.get::<_, Option<i64>>(4)?, "thread": thread, "author": author,
        }))
    })?;

    let artifacts = rows(conn, "SELECT a.kind, a.path, a.bytes, a.sha256, t.name, a.round_id IS NOT NULL
         FROM artifacts a LEFT JOIN threads t ON t.id = a.thread_id WHERE a.run_id = ?1 ORDER BY a.rowid", run_id, |r| {
        Ok(json!({
            "kind": r.get::<_, String>(0)?, "path": r.get::<_, String>(1)?, "bytes": r.get::<_, Option<i64>>(2)?,
            "sha256": r.get::<_, Option<String>>(3)?, "thread": r.get::<_, Option<String>>(4)?,
        }))
    })?;

    Ok(json!({
        "run": { "id": run_id, "status": status, "rounds_used": rounds_used, "tokens_used": tokens,
                 "started_at": started, "ended_at": ended, "blocked_reason": blocked,
                 "project": project },
        "goal": { "id": goal_id, "text": goal_text, "done": done.get("done").cloned().unwrap_or(Value::Null),
                  "checks": done.get("checks").cloned().unwrap_or(Value::Array(vec![])),
                  "message_policy": policy, "budget": { "rounds": budget_rounds, "tokens": budget_tokens } },
        "threads": threads, "rounds": rounds, "tasks": tasks, "ledger": ledger, "artifacts": artifacts,
    }))
}

fn journal_jsonl(conn: &Connection, run_id: &str, out: &Path) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT seq, at, kind, round_id, thread_id, payload FROM journal WHERE run_id = ?1 ORDER BY seq",
    )?;
    let mut text = String::new();
    let rows = stmt.query_map(params![run_id], |r| {
        Ok(json!({
            "seq": r.get::<_, i64>(0)?, "at": r.get::<_, i64>(1)?, "kind": r.get::<_, String>(2)?,
            "round_id": r.get::<_, Option<String>>(3)?, "thread_id": r.get::<_, Option<String>>(4)?,
            "payload": serde_json::from_str::<Value>(&r.get::<_, String>(5)?).unwrap_or(Value::Null),
        }))
    })?;
    for row in rows {
        text.push_str(&serde_json::to_string(&row?)?);
        text.push('\n');
    }
    std::fs::write(out, text)?;
    Ok(())
}

fn rows<F>(conn: &Connection, sql: &str, run_id: &str, mut f: F) -> Result<Vec<Value>>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<Value>,
{
    let mut stmt = conn.prepare(sql)?;
    let mapped = stmt.query_map(params![run_id], |r| f(r))?;
    let mut out = Vec::new();
    for row in mapped {
        out.push(row?);
    }
    Ok(out)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        let meta = entry.metadata()?;
        if meta.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
