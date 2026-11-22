//! Commands: each takes an open transaction, writes projections, pushes journal rows. Invariant checks
//! run before the first write, so a refusal never leaves a half-written round.

use crate::ledger;
use crate::state::{check_dispatch, check_merge, check_round_start, RunStatus, Verdict};
use crate::store::{new_id, Outcome, Pending};
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub struct NewGoal {
    pub title: String,
    pub goal_text: String,
    pub template: String,
    pub deliverable: Value,
    pub done_spec: Value,
    pub budget_rounds: u32,
    pub budget_tokens: u64,
    pub budget_usd: Option<f64>,
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
    pub template: String,
}

#[derive(Debug, Clone)]
pub struct SlotSpec {
    pub slot: String,
    pub role: String,
    pub scope: Vec<String>,
    pub intent: String,
}

#[derive(Debug, Clone)]
pub struct DispatchRef {
    pub dispatch_id: String,
    pub thread_id: String,
    pub slot: String,
    pub thread_dir: PathBuf,
    pub scope: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RoundRef {
    pub round_id: String,
    pub index: u32,
    pub dispatches: Vec<DispatchRef>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    Accept,
    SendBack { note: String },
}

pub fn create_run(tx: &Transaction<'_>, pending: &mut Pending, goal: &NewGoal) -> Result<String> {
    let goal_id = new_id();
    let run_id = new_id();
    tx.execute(
        "INSERT INTO goals (id, created_at, title, goal_text, template, deliverable, done_spec,
                            budget_rounds, budget_tokens, budget_usd)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            goal_id,
            crate::store::now_ms(),
            goal.title,
            goal.goal_text,
            goal.template,
            goal.deliverable.to_string(),
            goal.done_spec.to_string(),
            goal.budget_rounds,
            goal.budget_tokens as i64,
            goal.budget_usd,
        ],
    )?;
    tx.execute(
        "INSERT INTO runs (id, goal_id, status, started_at, rounds_used, tokens_used) VALUES (?1, ?2, 'planned', ?3, 0, 0)",
        params![run_id, goal_id, crate::store::now_ms()],
    )?;
    pending.push(
        &run_id,
        None,
        None,
        "run.created",
        json!({ "goal_id": goal_id, "template": goal.template, "budget_rounds": goal.budget_rounds,
                "budget_tokens": goal.budget_tokens }),
    );
    Ok(run_id)
}

pub fn load_run(conn: &Connection, run_id: &str) -> Result<RunRow> {
    conn.query_row(
        "SELECT r.id, r.goal_id, r.status, r.rounds_used, r.tokens_used, g.budget_rounds, g.budget_tokens, g.template
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
                template: r.get(7)?,
            })
        },
    )
    .map_err(|e| anyhow!("run {run_id}: {e}"))
}

#[derive(Debug, Clone)]
pub struct RoundRow {
    pub id: String,
    pub index: u32,
    pub verdict: Option<Verdict>,
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

/// Plan and open the next round. All checks happen before any write; a refusal is journaled.
pub fn plan_round(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    run_id: &str,
    slots: &[SlotSpec],
    thread_root: &Path,
    provider: &str,
) -> Result<Outcome<RoundRef>> {
    let run = load_run(tx, run_id)?;
    let previous = latest_round(tx, run_id)?;
    let refusal = if run.status.is_terminal() || run.status == RunStatus::HumanPaused {
        Some(format!("run is {}", run.status.as_str()))
    } else if slots.iter().any(|s| s.scope.is_empty()) {
        Some("dispatch has no scope".to_string())
    } else {
        check_round_start(
            previous.as_ref().map(|r| r.verdict),
            run.rounds_used,
            run.budget_rounds,
            run.tokens_used,
            run.budget_tokens,
        )
        .err()
        .map(|r| r.0)
    };
    if let Some(reason) = refusal {
        pending.push(run_id, None, None, "round.refused", json!({ "reason": reason }));
        return Ok(Outcome::Refused(reason));
    }

    let round_id = new_id();
    let index = run.rounds_used;
    let plan: Vec<Value> = slots
        .iter()
        .map(|s| json!({ "slot": s.slot, "role": s.role, "scope": s.scope }))
        .collect();
    tx.execute(
        "INSERT INTO rounds (id, run_id, \"index\", plan, status, started_at) VALUES (?1, ?2, ?3, ?4, 'planned', ?5)",
        params![round_id, run_id, index, Value::Array(plan.clone()).to_string(), crate::store::now_ms()],
    )?;
    tx.execute("UPDATE runs SET rounds_used = rounds_used + 1 WHERE id = ?1", params![run_id])?;
    if run.status != RunStatus::Working {
        set_run_status(tx, pending, run_id, RunStatus::Working, Some("round planned"))?;
    }

    let mut dispatches = Vec::new();
    for slot in slots {
        let thread_id = new_id();
        let dispatch_id = new_id();
        let thread_dir = thread_root.join(format!("r{index}-{}", slot.slot));
        tx.execute(
            "INSERT INTO threads (id, run_id, role, provider, worktree_path, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'idle', ?6)",
            params![thread_id, run_id, slot.role, provider, thread_dir.to_string_lossy(), crate::store::now_ms()],
        )?;
        ledger::append(
            tx,
            pending,
            run_id,
            Some(&round_id),
            Some(&thread_id),
            "intent",
            &slot.scope,
            &slot.intent,
            json!({ "slot": slot.slot }),
        )?;
        tx.execute(
            "INSERT INTO dispatch (id, round_id, thread_id, slot, task, scope, status, request_id, created_at)
             VALUES (?1, ?2, ?3, ?4, '', ?5, 'queued', ?6, ?7)",
            params![
                dispatch_id,
                round_id,
                thread_id,
                slot.slot,
                serde_json::to_string(&slot.scope)?,
                format!("plan:{round_id}:{}", slot.slot),
                crate::store::now_ms()
            ],
        )?;
        pending.push(
            run_id,
            Some(&round_id),
            Some(&thread_id),
            "dispatch.queued",
            json!({ "dispatch_id": dispatch_id, "slot": slot.slot, "role": slot.role, "scope": slot.scope }),
        );
        dispatches.push(DispatchRef {
            dispatch_id,
            thread_id,
            slot: slot.slot.clone(),
            thread_dir,
            scope: slot.scope.clone(),
        });
    }
    pending.push(
        run_id,
        Some(&round_id),
        None,
        "round.planned",
        json!({ "round_index": index, "dispatches": dispatches.iter().map(|d| json!({
            "dispatch_id": d.dispatch_id, "slot": d.slot })).collect::<Vec<_>>() }),
    );
    Ok(Outcome::Applied(RoundRef { round_id, index, dispatches }))
}

/// Invariant 2 at the moment of running: the round's ledger must hold an intent covering this scope.
pub fn start_dispatch(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    run_id: &str,
    dispatch_id: &str,
    task_text: &str,
) -> Result<Outcome<DispatchStart>> {
    let row = dispatch_row(tx, dispatch_id)?;
    if row.status != "queued" {
        let reason = format!("dispatch is {}, not queued", row.status);
        pending.push(run_id, Some(&row.round_id), Some(&row.thread_id), "dispatch.refused", json!({ "reason": reason }));
        return Ok(Outcome::Refused(reason));
    }
    let intents = ledger::intent_scopes(tx, &row.round_id)?;
    let covered = intents.iter().any(|intent| row.scope.iter().all(|s| intent.contains(s)));
    if let Err(refusal) = check_dispatch(covered, &row.scope) {
        pending.push(run_id, Some(&row.round_id), Some(&row.thread_id), "dispatch.refused", json!({ "reason": refusal.0 }));
        return Ok(Outcome::Refused(refusal.0));
    }
    tx.execute(
        "UPDATE dispatch SET status = 'running', task = ?2 WHERE id = ?1",
        params![dispatch_id, task_text],
    )?;
    tx.execute(
        "UPDATE threads SET status = 'working', last_active_at = ?2 WHERE id = ?1",
        params![row.thread_id, crate::store::now_ms()],
    )?;
    pending.push(
        run_id,
        Some(&row.round_id),
        Some(&row.thread_id),
        "dispatch.started",
        json!({ "dispatch_id": dispatch_id, "slot": row.slot }),
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

/// `ilvuno report`: the worker's single report. Files must sit inside the dispatch's declared scope.
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
        params![row.thread_id, crate::store::now_ms()],
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

pub fn record_usage(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    run_id: &str,
    round_id: &str,
    thread_id: &str,
    tokens: u64,
    cost_usd: Option<f64>,
) -> Result<()> {
    tx.execute("UPDATE runs SET tokens_used = tokens_used + ?2 WHERE id = ?1", params![run_id, tokens as i64])?;
    tx.execute(
        "UPDATE rounds SET tokens_delta = tokens_delta + ?2, cost_delta = cost_delta + ?3 WHERE id = ?1",
        params![round_id, tokens as i64, cost_usd.unwrap_or(0.0)],
    )?;
    pending.push(
        run_id,
        Some(round_id),
        Some(thread_id),
        "usage.recorded",
        json!({ "tokens": tokens, "cost_usd": cost_usd }),
    );
    Ok(())
}

pub fn set_run_status(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    run_id: &str,
    to: RunStatus,
    reason: Option<&str>,
) -> Result<()> {
    let from = load_run(tx, run_id)?.status;
    let ended = if to.is_terminal() { Some(crate::store::now_ms()) } else { None };
    tx.execute(
        "UPDATE runs SET status = ?2, ended_at = COALESCE(?3, ended_at),
                         blocked_reason = CASE WHEN ?2 = 'blocked' THEN ?4 ELSE blocked_reason END
         WHERE id = ?1",
        params![run_id, to.as_str(), ended, reason],
    )?;
    pending.push(
        run_id,
        None,
        None,
        "run.transition",
        json!({ "from": from.as_str(), "to": to.as_str(), "reason": reason }),
    );
    Ok(())
}

/// Record the round verdict and the readiness snapshot assembled before the gate.
pub fn record_verdict(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    run_id: &str,
    round_id: &str,
    verdict: Verdict,
    readiness: &Value,
) -> Result<()> {
    tx.execute(
        "UPDATE rounds SET verdict = ?2, status = 'reviewed', readiness = ?3, ended_at = ?4 WHERE id = ?1",
        params![round_id, verdict.as_str(), readiness.to_string(), crate::store::now_ms()],
    )?;
    pending.push(
        run_id,
        Some(round_id),
        None,
        "round.verdict",
        json!({ "verdict": verdict.as_str(), "readiness": readiness }),
    );
    if verdict.is_passing() {
        set_run_status(tx, pending, run_id, RunStatus::Reviewed, Some("gate opened"))?;
    }
    Ok(())
}

/// Human gate. Accept moves the run to `accepted`; send-back records a human block and resumes work.
pub fn gate_decide(
    tx: &Transaction<'_>,
    pending: &mut Pending,
    run_id: &str,
    decision: &GateDecision,
) -> Result<Outcome<()>> {
    let run = load_run(tx, run_id)?;
    if run.status != RunStatus::Reviewed {
        let reason = format!("run is {}, not at the gate", run.status.as_str());
        return Ok(Outcome::Refused(reason));
    }
    let round = latest_round(tx, run_id)?.ok_or_else(|| anyhow!("gate without a round"))?;
    match decision {
        GateDecision::Accept => {
            tx.execute("UPDATE runs SET accepted_by = 'human' WHERE id = ?1", params![run_id])?;
            set_run_status(tx, pending, run_id, RunStatus::Accepted, Some("accepted by human"))?;
            pending.push(run_id, Some(&round.id), None, "gate.decided", json!({ "decision": "accept" }));
        }
        GateDecision::SendBack { note } => {
            tx.execute(
                "UPDATE rounds SET verdict = 'block' WHERE id = ?1",
                params![round.id],
            )?;
            pending.push(
                run_id,
                Some(&round.id),
                None,
                "round.verdict",
                json!({ "verdict": "block", "source": "human", "note": note }),
            );
            set_run_status(tx, pending, run_id, RunStatus::Working, Some("sent back by human"))?;
            pending.push(run_id, Some(&round.id), None, "gate.decided", json!({ "decision": "send_back", "note": note }));
        }
    }
    Ok(Outcome::Applied(()))
}

/// Merge is not executed in v0. The check is still enforced so the refusal path is real and tested.
pub fn request_merge(tx: &Transaction<'_>, pending: &mut Pending, run_id: &str) -> Result<Outcome<()>> {
    let run = load_run(tx, run_id)?;
    let verdict = latest_round(tx, run_id)?.and_then(|r| r.verdict);
    match check_merge(verdict, run.status) {
        Ok(()) => {
            pending.push(run_id, None, None, "merge.requested", json!({ "by": "human" }));
            Ok(Outcome::Applied(()))
        }
        Err(refusal) => {
            pending.push(run_id, None, None, "merge.refused", json!({ "reason": refusal.0 }));
            Ok(Outcome::Refused(refusal.0))
        }
    }
}

pub fn freeze(tx: &Transaction<'_>, pending: &mut Pending, run_id: &str) -> Result<Outcome<()>> {
    let run = load_run(tx, run_id)?;
    if run.status.is_terminal() {
        return Ok(Outcome::Refused(format!("run is {}", run.status.as_str())));
    }
    set_run_status(tx, pending, run_id, RunStatus::HumanPaused, Some("frozen by human"))?;
    Ok(Outcome::Applied(()))
}

struct DispatchRowData {
    round_id: String,
    thread_id: String,
    slot: String,
    status: String,
    scope: Vec<String>,
}

fn dispatch_row(conn: &Connection, dispatch_id: &str) -> Result<DispatchRowData> {
    dispatch_row_opt(conn, dispatch_id)?.ok_or_else(|| anyhow!("unknown dispatch {dispatch_id}"))
}

fn dispatch_row_opt(conn: &Connection, dispatch_id: &str) -> Result<Option<DispatchRowData>> {
    let row = conn
        .query_row(
            "SELECT round_id, thread_id, slot, status, scope FROM dispatch WHERE id = ?1",
            params![dispatch_id],
            |r| Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get::<_, String>(4)?)),
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some((round_id, thread_id, slot, status, scope)) => Ok(Some(DispatchRowData {
            round_id,
            thread_id,
            slot,
            status,
            scope: serde_json::from_str(&scope)?,
        })),
    }
}

fn run_of_round(conn: &Connection, round_id: &str) -> Result<String> {
    conn.query_row("SELECT run_id FROM rounds WHERE id = ?1", params![round_id], |r| r.get(0))
        .map_err(|e| anyhow!("round {round_id}: {e}"))
}
