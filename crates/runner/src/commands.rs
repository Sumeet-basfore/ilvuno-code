//! Commands the runner issues: run creation, lookups, the agent's note and report, and the start of a
//! dispatch. Each takes an open transaction, writes projections and pushes journal rows. Invariant checks
//! run before the first write, so a refusal never leaves a half-written round.

use crate::goal::ORCHESTRATOR;
use crate::home::Home;
use crate::ledger;
use crate::state::{check_dispatch, RunStatus, Verdict};
use crate::store::{new_id, now_ms, Outcome, Pending};
use crate::tasks::{self, CreatedBy, NewTask};
use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
use std::path::PathBuf;

/// The scope of a planning task. It names no file: the task is the goal itself.
pub const RUN_SCOPE: &str = "run";

pub struct NewGoal {
    pub title: String,
    pub goal_text: String,
    pub done: Option<String>,
    /// The checks as written in the goal file (a JSON array).
    pub checks: Value,
    pub message_policy: String,
    pub budget_rounds: u32,
    pub budget_tokens: u64,
    pub budget_usd: Option<f64>,
    /// JSON object: agent id → { driver, command, model, mode }.
    pub providers: Value,
    pub threads: Vec<NewThread>,
}

pub struct NewThread {
    pub name: String,
    pub agent: String,
    pub prompt: String,
}

#[derive(Debug, Clone)]
pub struct RunRow {
    pub id: String,
    pub goal_id: String,
    pub status: RunStatus,
    pub rounds_used: u32,
    pub tokens_used: u64,
    pub budget_rounds: u32,
    pub budget_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct RoundRow {
    pub id: String,
    pub index: u32,
    pub verdict: Option<Verdict>,
}

#[derive(Debug, Clone)]
pub struct DispatchStart {
    pub round_id: String,
    pub thread_id: String,
    pub thread_dir: PathBuf,
    pub scope: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportStatus {
    Ok,
    Failed,
}

/// Create the run, its threads and, when the goal has an orchestrator, its first task (the goal itself).
pub fn create_run(tx: &Transaction<'_>, pending: &mut Pending, goal: &NewGoal, home: &Home) -> Result<String> {
    let goal_id = new_id();
    let run_id = new_id();
    let done_spec = json!({ "done": goal.done, "checks": goal.checks });
    tx.execute(
        "INSERT INTO goals (id, created_at, title, goal_text, done_spec, message_policy, budget_rounds, budget_tokens,
                            budget_usd, providers)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            goal_id,
            now_ms(),
            goal.title,
            goal.goal_text,
            done_spec.to_string(),
            goal.message_policy,
            goal.budget_rounds,
            goal.budget_tokens as i64,
            goal.budget_usd,
            goal.providers.to_string(),
        ],
    )?;
    tx.execute(
        "INSERT INTO runs (id, goal_id, status, started_at, rounds_used, tokens_used) VALUES (?1, ?2, 'working', ?3, 0, 0)",
        params![run_id, goal_id, now_ms()],
    )?;
    for thread in &goal.threads {
        let role = if thread.name == ORCHESTRATOR { "orchestrator" } else { "worker" };
        let folder = home.threads_dir(&run_id).join(&thread.name);
        tx.execute(
            "INSERT INTO threads (id, run_id, role, provider, name, prompt, worktree_path, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'idle', ?8)",
            params![new_id(), run_id, role, thread.agent, thread.name, thread.prompt, folder.to_string_lossy(), now_ms()],
        )?;
    }
    let names: Vec<&str> = goal.threads.iter().map(|t| t.name.as_str()).collect();
    pending.push(
        &run_id,
        None,
        None,
        "run.created",
        json!({ "goal_id": goal_id, "budget_rounds": goal.budget_rounds, "budget_tokens": goal.budget_tokens, "threads": names }),
    );
    if goal.threads.iter().any(|t| t.name == ORCHESTRATOR) {
        let prompt = kickoff_prompt(&goal.goal_text, goal.done.as_deref());
        let scope = [RUN_SCOPE.to_string()];
        let task = NewTask { run_id: &run_id, thread: ORCHESTRATOR, prompt: &prompt, scope: &scope, created_by: CreatedBy::User, author_thread: None };
        match tasks::create_task(tx, pending, &task)? {
            Outcome::Applied(_) => {}
            Outcome::Refused(reason) | Outcome::Duplicate(reason) => bail!("the first task was refused: {reason}"),
        }
    }
    Ok(run_id)
}

fn kickoff_prompt(goal: &str, done: Option<&str>) -> String {
    match done {
        Some(done) => format!("Goal:\n{goal}\n\nDone looks like:\n{done}"),
        None => format!("Goal:\n{goal}\n\nNo done text was written. The human decides when it is finished."),
    }
}

pub fn load_run(conn: &Connection, run_id: &str) -> Result<RunRow> {
    conn.query_row(
        "SELECT r.id, r.goal_id, r.status, r.rounds_used, r.tokens_used, g.budget_rounds, g.budget_tokens
         FROM runs r JOIN goals g ON g.id = r.goal_id WHERE r.id = ?1",
        params![run_id],
        |r| {
            let status: String = r.get(2)?;
            Ok(RunRow {
                id: r.get(0)?,
                goal_id: r.get(1)?,
                status: RunStatus::parse(&status).unwrap_or(RunStatus::Blocked),
                rounds_used: r.get::<_, i64>(3)? as u32,
                tokens_used: r.get::<_, i64>(4)? as u64,
                budget_rounds: r.get::<_, i64>(5)? as u32,
                budget_tokens: r.get::<_, i64>(6)? as u64,
            })
        },
    )
    .map_err(|e| anyhow!("run {run_id}: {e}"))
}

/// The newest run, the one `ilvuno task` writes to when no orchestrator task is calling.
pub fn newest_run(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT id FROM runs ORDER BY started_at DESC, rowid DESC LIMIT 1", [], |r| r.get(0))
        .optional()?)
}

pub fn latest_round(conn: &Connection, run_id: &str) -> Result<Option<RoundRow>> {
    let row = conn
        .query_row(
            "SELECT id, \"index\", verdict FROM rounds WHERE run_id = ?1 ORDER BY \"index\" DESC LIMIT 1",
            params![run_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, Option<String>>(2)?)),
        )
        .optional()?;
    Ok(row.map(|(id, index, verdict)| RoundRow {
        id,
        index: index as u32,
        verdict: verdict.as_deref().and_then(Verdict::parse),
    }))
}

/// The round still waiting for its verdict, if any. Tasks join this round; no other round is open.
pub fn open_round(conn: &Connection, run_id: &str) -> Result<Option<RoundRow>> {
    Ok(latest_round(conn, run_id)?.filter(|r| r.verdict.is_none()))
}

/// The thread named `orchestrator`, if the run has one.
pub fn orchestrator_name(conn: &Connection, run_id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT name FROM threads WHERE run_id = ?1 AND role = 'orchestrator'",
            params![run_id],
            |r| r.get(0),
        )
        .optional()?)
}

/// Invariant 2 at the moment of running: the round's ledger must hold an intent covering this scope.
pub fn start_dispatch(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    run_id: &str,
    dispatch_id: &str,
) -> Result<Outcome<DispatchStart>> {
    let row = dispatch_row(tx, dispatch_id)?;
    if row.status != "queued" {
        let reason = format!("task is {}, not queued", row.status);
        pending.push(run_id, Some(&row.round_id), Some(&row.thread_id), "dispatch.refused", json!({ "reason": reason }));
        return Ok(Outcome::Refused(reason));
    }
    let intents = ledger::intent_scopes(tx, &row.round_id)?;
    let covered = intents.iter().any(|intent| row.scope.iter().all(|s| intent.contains(s)));
    if let Err(refusal) = check_dispatch(covered, &row.scope) {
        pending.push(run_id, Some(&row.round_id), Some(&row.thread_id), "dispatch.refused", json!({ "reason": refusal.0 }));
        return Ok(Outcome::Refused(refusal.0));
    }
    tx.execute("UPDATE dispatch SET status = 'running' WHERE id = ?1", params![dispatch_id])?;
    tx.execute(
        "UPDATE threads SET status = 'working', last_active_at = ?2 WHERE id = ?1",
        params![row.thread_id, now_ms()],
    )?;
    pending.push(
        run_id,
        Some(&row.round_id),
        Some(&row.thread_id),
        "dispatch.started",
        json!({ "dispatch_id": dispatch_id }),
    );
    let thread_dir: String = tx.query_row(
        "SELECT worktree_path FROM threads WHERE id = ?1",
        params![row.thread_id],
        |r| r.get(0),
    )?;
    Ok(Outcome::Applied(DispatchStart {
        round_id: row.round_id,
        thread_id: row.thread_id,
        thread_dir: PathBuf::from(thread_dir),
        scope: row.scope,
    }))
}

/// `ilvuno report`: the task's single report. Files must sit inside the task's declared scope.
pub fn report_dispatch(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    dispatch_id: &str,
    status: ReportStatus,
    summary: &str,
    files: &[String],
) -> Result<Outcome<()>> {
    let row = match dispatch_row_opt(tx, dispatch_id)? {
        Some(row) => row,
        None => return Ok(Outcome::Refused(format!("unknown task {dispatch_id}"))),
    };
    let run_id = run_of_round(tx, &row.round_id)?;
    if row.status != "queued" && row.status != "running" {
        let reason = format!("task already {}", row.status);
        pending.push(&run_id, Some(&row.round_id), Some(&row.thread_id), "dispatch.refused", json!({ "reason": reason }));
        return Ok(Outcome::Refused(reason));
    }
    if let Some(outside) = files.iter().find(|f| !row.scope.contains(f)) {
        let reason = format!("file {outside} is outside the declared scope");
        pending.push(&run_id, Some(&row.round_id), Some(&row.thread_id), "dispatch.refused", json!({ "reason": reason }));
        return Ok(Outcome::Refused(reason));
    }
    let (new_status, kind) = match status {
        ReportStatus::Ok => ("reported", "dispatch.reported"),
        ReportStatus::Failed => ("failed", "dispatch.failed"),
    };
    tx.execute(
        "UPDATE dispatch SET status = ?2, result_summary = ?3, result_json = ?4 WHERE id = ?1",
        params![dispatch_id, new_status, summary, json!({ "files": files }).to_string()],
    )?;
    tx.execute(
        "UPDATE threads SET status = 'idle', last_active_at = ?2 WHERE id = ?1",
        params![row.thread_id, now_ms()],
    )?;
    ledger::append(
        tx,
        pending,
        &run_id,
        Some(&row.round_id),
        Some(&row.thread_id),
        "finding",
        files,
        summary,
        json!({ "task_id": dispatch_id, "status": new_status }),
    )?;
    pending.push(
        &run_id,
        Some(&row.round_id),
        Some(&row.thread_id),
        kind,
        json!({ "dispatch_id": dispatch_id, "files": files, "summary": summary }),
    );
    Ok(Outcome::Applied(()))
}

/// `ilvuno note`: a board entry from a task. The scope must stay inside the task's declared scope, and
/// the task may only write the kinds it is entitled to.
pub fn note(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    dispatch_id: &str,
    kind: &str,
    body: &str,
    scope: &[String],
) -> Result<Outcome<i64>> {
    const WRITABLE_KINDS: &[&str] = &["intent", "finding", "question", "answer"];
    let row = match dispatch_row_opt(tx, dispatch_id)? {
        Some(row) => row,
        None => return Ok(Outcome::Refused(format!("unknown task {dispatch_id}"))),
    };
    let run_id = run_of_round(tx, &row.round_id)?;
    if row.status != "queued" && row.status != "running" {
        let reason = format!("task already {}", row.status);
        pending.push(&run_id, Some(&row.round_id), Some(&row.thread_id), "note.refused", json!({ "reason": reason }));
        return Ok(Outcome::Refused(reason));
    }
    if !WRITABLE_KINDS.contains(&kind) {
        let reason = format!("kind {kind} is not writable by a task");
        pending.push(&run_id, Some(&row.round_id), Some(&row.thread_id), "note.refused", json!({ "reason": reason }));
        return Ok(Outcome::Refused(reason));
    }
    if body.trim().is_empty() {
        return Ok(Outcome::Refused("note body is empty".into()));
    }
    let scope: Vec<String> = if scope.is_empty() { row.scope.clone() } else { scope.to_vec() };
    if let Some(outside) = scope.iter().find(|s| !row.scope.contains(s)) {
        let reason = format!("scope entry {outside} is outside this task's scope");
        pending.push(&run_id, Some(&row.round_id), Some(&row.thread_id), "note.refused", json!({ "reason": reason }));
        return Ok(Outcome::Refused(reason));
    }
    let id = ledger::append(
        tx,
        pending,
        &run_id,
        Some(&row.round_id),
        Some(&row.thread_id),
        kind,
        &scope,
        body,
        json!({ "dispatch_id": dispatch_id }),
    )?;
    Ok(Outcome::Applied(id))
}

struct DispatchRowData {
    round_id: String,
    thread_id: String,
    status: String,
    scope: Vec<String>,
}

fn dispatch_row(conn: &Connection, dispatch_id: &str) -> Result<DispatchRowData> {
    dispatch_row_opt(conn, dispatch_id)?.ok_or_else(|| anyhow!("unknown task {dispatch_id}"))
}

fn dispatch_row_opt(conn: &Connection, dispatch_id: &str) -> Result<Option<DispatchRowData>> {
    let row = conn
        .query_row(
            "SELECT round_id, thread_id, status, scope FROM dispatch WHERE id = ?1",
            params![dispatch_id],
            |r| Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?, r.get::<_, String>(3)?)),
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some((round_id, thread_id, status, scope)) => Ok(Some(DispatchRowData {
            round_id,
            thread_id,
            status,
            scope: serde_json::from_str(&scope)?,
        })),
    }
}

pub fn run_of_round(conn: &Connection, round_id: &str) -> Result<String> {
    conn.query_row("SELECT run_id FROM rounds WHERE id = ?1", params![round_id], |r| r.get(0))
        .map_err(|e| anyhow!("round {round_id}: {e}"))
}
