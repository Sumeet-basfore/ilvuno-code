//! The run loop. Takes the open round, runs each of its open tasks on its thread, then closes the round when
//! none is open. Closing runs the user's checks and records the verdict. A pass stops at the human gate. A
//! failure wakes the orchestrator with the failing detail, if the run has one, and otherwise waits for the
//! next task from the user.

use crate::checks::{self, CheckResult, Locations};
use crate::commands::{self, load_run, open_round, RoundRow};
use crate::dispatch::{self, ExecCtx};
use crate::goal::GoalPlan;
use crate::home::Home;
use crate::run_status::{self, set_run_status};
use crate::state::{is_no_progress, RunStatus, Verdict};
use crate::store::{Outcome, Store};
use crate::tasks::{self, CreatedBy, NewTask};
use anyhow::{Context, Result};
use provider_core::Provider;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEnd {
    /// Checks passed; waiting for the human.
    AtGate,
    Accepted,
    Paused,
    /// No open round: the run waits for a task (`ilvuno task`, then `ilvuno run --resume`).
    Idle,
    Stopped { status: RunStatus, reason: String },
}

pub struct Driver<'a, P: Provider> {
    /// Agent instances by id. Each task runs on the agent its thread names.
    pub providers: &'a BTreeMap<String, P>,
    pub home: &'a Home,
    pub ilvuno_bin_dir: Option<&'a Path>,
    pub max_usd: f64,
}

/// A task still waiting to run, with the agent its thread uses.
struct OpenTask {
    id: String,
    thread: String,
    agent: String,
}

impl<P: Provider> Driver<'_, P> {
    pub async fn drive(&self, store: &mut Store, plan: &GoalPlan, run_id: &str) -> Result<RunEnd> {
        loop {
            let run = load_run(store.conn(), run_id)?;
            match run.status {
                RunStatus::Reviewed => return Ok(RunEnd::AtGate),
                RunStatus::Accepted | RunStatus::Merged => return Ok(RunEnd::Accepted),
                RunStatus::HumanPaused => return Ok(RunEnd::Paused),
                status if status.is_terminal() => {
                    return Ok(RunEnd::Stopped { status, reason: blocked_reason(store.conn(), run_id)? });
                }
                _ => {}
            }
            if run.tokens_used >= run.budget_tokens {
                let reason = format!("token budget exhausted: {} of {} tokens", run.tokens_used, run.budget_tokens);
                store.transact("run.budget", None, |tx, p| {
                    set_run_status(tx, p, run_id, RunStatus::BudgetExhausted, Some(&reason))?;
                    Ok(Outcome::Applied(()))
                })?;
                continue;
            }
            let Some(round) = open_round(store.conn(), run_id)? else {
                return Ok(RunEnd::Idle);
            };
            self.run_open_tasks(store, plan, run_id, &round).await?;
            if let Some(end) = self.close_round(store, plan, run_id, &round)? {
                return Ok(end);
            }
        }
    }

    /// Run tasks until none of the round is open. Tasks added during a turn (an orchestrator's) are picked
    /// up in the same loop. A task is attempted once per pass, so a task that cannot start cannot spin.
    async fn run_open_tasks(&self, store: &mut Store, plan: &GoalPlan, run_id: &str, round: &RoundRow) -> Result<()> {
        let mut attempted = BTreeSet::new();
        while let Some(task) = next_open_task(store.conn(), &round.id, &attempted)? {
            // Checked per task, not per round: one round of three tasks can blow the run's budget.
            if self.budget_exhausted(store, run_id)? {
                break;
            }
            attempted.insert(task.id.clone());
            let provider = self.providers.get(&task.agent).with_context(|| {
                format!("thread {}: agent {:?} is not available in this process", task.thread, task.agent)
            })?;
            let cx = ExecCtx { home: self.home, plan, ilvuno_bin_dir: self.ilvuno_bin_dir, max_usd: self.max_usd };
            dispatch::execute(provider, store, &cx, run_id, round.index, &task.id).await?;
        }
        Ok(())
    }

    /// True when the run has spent its token budget. The outer loop turns this into a stop; a task in
    /// flight is never killed, because half a turn leaves nothing worth keeping.
    fn budget_exhausted(&self, store: &Store, run_id: &str) -> Result<bool> {
        let (used, limit): (i64, i64) = store.conn().query_row(
            "SELECT r.tokens_used, g.budget_tokens FROM runs r JOIN goals g ON g.id = r.goal_id WHERE r.id = ?1",
            rusqlite::params![run_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        Ok(used as u64 >= limit as u64)
    }

    /// Close the round if no task is open: run the checks, record the verdict, and either stop, or wake the
    /// orchestrator with what failed. Returns `Some` when the run stops here.
    fn close_round(&self, store: &mut Store, plan: &GoalPlan, run_id: &str, round: &RoundRow) -> Result<Option<RunEnd>> {
        if open_task_count(store.conn(), &round.id)? > 0 {
            return Ok(None);
        }
        let failed = failed_threads(store.conn(), &round.id)?;
        let artifacts = self.home.artifacts_dir(run_id);
        // Checks run outside the write transaction: a command check may take a while, and a user's `ilvuno
        // task` must not wait on it.
        let checks: Vec<CheckResult> = if failed.is_empty() {
            checks::evaluate(plan.checks(), &Locations { artifacts: &artifacts, goal_dir: plan.base() })
        } else {
            Vec::new()
        };
        let mut failing: Vec<String> = checks.iter().filter(|c| !c.passed).map(|c| c.name.clone()).collect();
        failing.extend(failed.iter().map(|thread| format!("task-failed:{thread}")));
        let stalled = is_no_progress(previous_failing(store.conn(), run_id, round.index)?.as_deref(), &failing);
        let verdict = if failing.is_empty() { Verdict::Pass } else { Verdict::Block };
        let readiness = json!({
            "checks": checks.iter().map(|c| json!({ "name": c.name, "passed": c.passed, "detail": c.detail })).collect::<Vec<_>>(),
            "failing": failing,
            "failed_tasks": failed,
        });
        let plan_json = round_plan(store.conn(), &round.id)?;
        let wake = wake_prompt(round.index, &checks, &failed);
        let round_id = round.id.clone();

        let closed = store.transact("round.close", None, |tx, p| {
            if open_task_count(tx, &round_id)? > 0 {
                return Ok(Outcome::Refused("tasks were added while the round was checked".into()));
            }
            run_status::record_verdict(tx, p, run_id, &round_id, verdict, &readiness)?;
            tx.execute("UPDATE rounds SET plan = ?2 WHERE id = ?1", params![round_id, plan_json])?;
            if stalled {
                set_run_status(tx, p, run_id, RunStatus::NoProgress, Some("same failing checks two rounds running"))?;
                let reason = "same failing checks two rounds running".to_string();
                return Ok(Outcome::Applied(Some(RunEnd::Stopped { status: RunStatus::NoProgress, reason })));
            }
            if verdict == Verdict::Block {
                if let Some(orchestrator) = commands::orchestrator_name(tx, run_id)? {
                    let scope = [commands::RUN_SCOPE.to_string()];
                    let task = NewTask {
                        run_id,
                        thread: &orchestrator,
                        prompt: &wake,
                        scope: &scope,
                        created_by: CreatedBy::Runner,
                        author_thread: None,
                    };
                    if let Outcome::Refused(reason) = tasks::create_task(tx, p, &task)? {
                        set_run_status(tx, p, run_id, RunStatus::Blocked, Some(&reason))?;
                        return Ok(Outcome::Applied(Some(RunEnd::Stopped { status: RunStatus::Blocked, reason })));
                    }
                }
            }
            Ok(Outcome::Applied(None))
        })?;
        match closed {
            Outcome::Applied(end) => Ok(end),
            Outcome::Refused(_) | Outcome::Duplicate(_) => Ok(None),
        }
    }
}

fn next_open_task(conn: &Connection, round_id: &str, attempted: &BTreeSet<String>) -> Result<Option<OpenTask>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, t.name, t.provider FROM dispatch d JOIN threads t ON t.id = d.thread_id
         WHERE d.round_id = ?1 AND d.status IN ('queued', 'running') ORDER BY d.rowid",
    )?;
    let open: Vec<OpenTask> = stmt
        .query_map(params![round_id], |r| Ok(OpenTask { id: r.get(0)?, thread: r.get(1)?, agent: r.get(2)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(open.into_iter().find(|t| !attempted.contains(&t.id)))
}

fn open_task_count(conn: &Connection, round_id: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM dispatch WHERE round_id = ?1 AND status IN ('queued', 'running')",
        params![round_id],
        |r| r.get(0),
    )?)
}

/// Names of the threads whose task in this round failed, in task order.
fn failed_threads(conn: &Connection, round_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT t.name FROM dispatch d JOIN threads t ON t.id = d.thread_id
         WHERE d.round_id = ?1 AND d.status = 'failed' ORDER BY d.rowid",
    )?;
    let names = stmt.query_map(params![round_id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    Ok(names)
}

/// The round's tasks as the user will read them: thread, scope, and who wrote each one.
fn round_plan(conn: &Connection, round_id: &str) -> Result<String> {
    let mut stmt = conn.prepare(
        "SELECT t.name, d.scope, d.created_by FROM dispatch d JOIN threads t ON t.id = d.thread_id
         WHERE d.round_id = ?1 ORDER BY d.rowid",
    )?;
    let rows: Vec<(String, String, String)> = stmt
        .query_map(params![round_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut tasks = Vec::new();
    for (thread, scope, created_by) in rows {
        tasks.push(json!({ "thread": thread, "scope": serde_json::from_str::<Value>(&scope)?, "created_by": created_by }));
    }
    Ok(Value::Array(tasks).to_string())
}

/// What the orchestrator is told when a round did not pass: which checks failed, and which tasks failed.
fn wake_prompt(round_index: u32, checks: &[CheckResult], failed: &[String]) -> String {
    let mut text = format!(
        "Round {round_index} did not pass. Plan the next step: create the tasks that fix this. \
         Do not write the deliverable yourself.\n\nFailing checks:\n"
    );
    let mut any = false;
    for check in checks.iter().filter(|c| !c.passed) {
        text.push_str(&format!("- {}: {}\n", check.name, check.detail));
        any = true;
    }
    for thread in failed {
        text.push_str(&format!("- the task on thread {thread} failed\n"));
        any = true;
    }
    if !any {
        text.push_str("- (none listed)\n");
    }
    text
}

fn previous_failing(conn: &Connection, run_id: &str, index: u32) -> Result<Option<Vec<String>>> {
    if index == 0 {
        return Ok(None);
    }
    let readiness: Option<Option<String>> = conn
        .query_row(
            "SELECT readiness FROM rounds WHERE run_id = ?1 AND \"index\" = ?2",
            params![run_id, index - 1],
            |r| r.get(0),
        )
        .optional()?;
    let Some(Some(text)) = readiness else { return Ok(None) };
    let value: Value = serde_json::from_str(&text)?;
    Ok(Some(
        value["failing"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default(),
    ))
}

fn blocked_reason(conn: &Connection, run_id: &str) -> Result<String> {
    Ok(conn.query_row(
        "SELECT COALESCE(blocked_reason, status) FROM runs WHERE id = ?1",
        params![run_id],
        |r| r.get(0),
    )?)
}
