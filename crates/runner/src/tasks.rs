//! Tasks: the one unit of work. A task is a prompt, a target thread and a declared scope. The first task
//! of a round opens the round; later tasks join the open one. The runner refuses only bounds: the run's
//! state, an unknown thread, an empty prompt or scope, the per-round cap, and the round and token budgets.
//! It never looks at what the task asks for.

use crate::commands::{load_run, newest_run, latest_round, open_round};
use crate::home::Home;
use crate::ledger;
use crate::state::{check_round_start, RunStatus};
use crate::store::{new_id, now_ms, Outcome, Pending, Store};
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::json;

/// The most tasks one round may hold. A bound, not a judgement.
pub const TASKS_PER_ROUND_CAP: i64 = 8;

/// Who wrote a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatedBy {
    User,
    Orchestrator,
    /// The runner, when a failed round wakes the orchestrator with the failing detail.
    Runner,
}

impl CreatedBy {
    pub fn as_str(self) -> &'static str {
        match self {
            CreatedBy::User => "user",
            CreatedBy::Orchestrator => "orchestrator",
            CreatedBy::Runner => "runner",
        }
    }
}

pub struct NewTask<'a> {
    pub run_id: &'a str,
    /// Name of the target thread in this run.
    pub thread: &'a str,
    pub prompt: &'a str,
    pub scope: &'a [String],
    pub created_by: CreatedBy,
    /// The thread that wrote the task, when an orchestrator did. Shown on the board as the author.
    pub author_thread: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRef {
    pub task_id: String,
    pub round_id: String,
    pub round_index: u32,
    pub thread: String,
}

/// Write one task, and the round it belongs to if that round is new. Refusals are journaled and change
/// nothing else.
pub fn create_task(tx: &Transaction<'_>, pending: &mut Pending, task: &NewTask<'_>) -> Result<Outcome<TaskRef>> {
    let run = load_run(tx, task.run_id)?;
    let prompt = task.prompt.trim();
    let scope: Vec<String> = task.scope.iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    if run.status != RunStatus::Working {
        return Ok(refuse(pending, task.run_id, format!("run is {}, not working", run.status.as_str())));
    }
    if prompt.is_empty() {
        return Ok(refuse(pending, task.run_id, "task prompt is empty".to_string()));
    }
    if scope.is_empty() {
        return Ok(refuse(pending, task.run_id, "task has no scope".to_string()));
    }
    let Some(thread_id) = thread_id_by_name(tx, task.run_id, task.thread)? else {
        return Ok(refuse(pending, task.run_id, format!("unknown thread {}", task.thread)));
    };
    let round = match open_round(tx, task.run_id)? {
        Some(round) => {
            let count: i64 = tx.query_row("SELECT COUNT(*) FROM dispatch WHERE round_id = ?1", params![round.id], |r| r.get(0))?;
            if count >= TASKS_PER_ROUND_CAP {
                let reason = format!("round {} already has {TASKS_PER_ROUND_CAP} tasks: the cap per round", round.index);
                return Ok(refuse(pending, task.run_id, reason));
            }
            round
        }
        None => {
            let previous = latest_round(tx, task.run_id)?.map(|r| r.verdict);
            let budget = check_round_start(previous, run.rounds_used, run.budget_rounds, run.tokens_used, run.budget_tokens);
            if let Err(refusal) = budget {
                return Ok(refuse(pending, task.run_id, refusal.0));
            }
            open_new_round(tx, pending, task.run_id, run.rounds_used)?
        }
    };

    let task_id = new_id();
    tx.execute(
        "INSERT INTO dispatch (id, round_id, thread_id, task, scope, status, created_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'queued', ?6, ?7)",
        params![task_id, round.id, thread_id, prompt, serde_json::to_string(&scope)?, task.created_by.as_str(), now_ms()],
    )?;
    ledger::append(
        tx,
        pending,
        task.run_id,
        Some(&round.id),
        task.author_thread,
        "intent",
        &scope,
        prompt,
        json!({ "task_id": task_id, "thread": task.thread, "created_by": task.created_by.as_str() }),
    )?;
    pending.push(
        task.run_id,
        Some(&round.id),
        Some(&thread_id),
        "dispatch.queued",
        json!({ "dispatch_id": task_id, "thread": task.thread, "scope": scope, "created_by": task.created_by.as_str() }),
    );
    Ok(Outcome::Applied(TaskRef {
        task_id,
        round_id: round.id,
        round_index: round.index,
        thread: task.thread.to_string(),
    }))
}

fn refuse<T>(pending: &mut Pending, run_id: &str, reason: String) -> Outcome<T> {
    pending.push(run_id, None, None, "task.refused", json!({ "reason": reason }));
    Outcome::Refused(reason)
}

fn open_new_round(tx: &Transaction<'_>, pending: &mut Pending, run_id: &str, index: u32) -> Result<crate::commands::RoundRow> {
    let round_id = new_id();
    tx.execute(
        "INSERT INTO rounds (id, run_id, \"index\", plan, status, started_at) VALUES (?1, ?2, ?3, '[]', 'open', ?4)",
        params![round_id, run_id, index, now_ms()],
    )?;
    tx.execute("UPDATE runs SET rounds_used = rounds_used + 1 WHERE id = ?1", params![run_id])?;
    pending.push(run_id, Some(&round_id), None, "round.planned", json!({ "round_index": index }));
    Ok(crate::commands::RoundRow { id: round_id, index, verdict: None })
}

fn thread_id_by_name(conn: &Connection, run_id: &str, name: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM threads WHERE run_id = ?1 AND name = ?2",
            params![run_id, name],
            |r| r.get(0),
        )
        .optional()?)
}

/// What `ilvuno task` does. An orchestrator task calls with its own task id (from `ILVUNO_TASK`) and the
/// task is written to that task's run. Without one the user is writing, and the task goes to the newest run.
pub fn run_task(home: &Home, caller_task: Option<&str>, thread: &str, prompt: &str, scope: &[String]) -> Result<Outcome<TaskRef>> {
    let mut store = Store::open(&home.db_path())?;
    let (run_id, created_by, author) = match caller_task {
        Some(task_id) => match caller_of(store.conn(), task_id)? {
            None => return Err(anyhow!("ILVUNO_TASK names {task_id}, which is not a task in this home")),
            Some(caller) if caller.role != "orchestrator" => {
                return Ok(Outcome::Refused("only the orchestrator thread may create tasks from a task".into()))
            }
            Some(caller) if caller.status != "running" => {
                return Ok(Outcome::Refused(format!("your task is {}, not running", caller.status)))
            }
            Some(caller) => (caller.run_id, CreatedBy::Orchestrator, Some(caller.thread_id)),
        },
        None => {
            let run_id = newest_run(store.conn())?
                .ok_or_else(|| anyhow!("no run yet: start one with `ilvuno run --goal <file>`"))?;
            (run_id, CreatedBy::User, None)
        }
    };
    submit_task(home, &mut store, &NewTask { run_id: &run_id, thread, prompt, scope, created_by, author_thread: author.as_deref() })
}

/// Write one task and refresh the board. This is the write behind `ilvuno task` and `POST /runs/:id/tasks`.
pub fn submit_task(home: &Home, store: &mut Store, task: &NewTask<'_>) -> Result<Outcome<TaskRef>> {
    let outcome = store.transact("task.create", None, |tx, p| create_task(tx, p, task))?;
    if let Outcome::Applied(_) = outcome {
        ledger::render_board(store.conn(), task.run_id, &home.board_path(task.run_id))?;
    }
    Ok(outcome)
}

struct Caller {
    run_id: String,
    thread_id: String,
    role: String,
    status: String,
}

fn caller_of(conn: &Connection, task_id: &str) -> Result<Option<Caller>> {
    Ok(conn
        .query_row(
            "SELECT r.run_id, d.thread_id, t.role, d.status
             FROM dispatch d JOIN threads t ON t.id = d.thread_id JOIN rounds r ON r.id = d.round_id
             WHERE d.id = ?1",
            params![task_id],
            |r| Ok(Caller { run_id: r.get(0)?, thread_id: r.get(1)?, role: r.get(2)?, status: r.get(3)? }),
        )
        .optional()?)
}
