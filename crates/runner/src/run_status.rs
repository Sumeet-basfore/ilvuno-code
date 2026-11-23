//! Run-level transitions: status, token accounting, verdicts, the human gate, merge and freeze.

use crate::commands::{self, latest_round, load_run};
use crate::home::Home;
use crate::ledger;
use crate::state::{check_merge, RunStatus, Verdict};
use crate::store::{Outcome, Pending, Store};
use crate::tasks::{self, CreatedBy, NewTask};
use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction};
use serde_json::{json, Value};

/// What the human decided at the gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    Accept,
    SendBack { note: String },
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
                         blocked_reason = CASE WHEN ?4 IS NOT NULL AND ?2 IN ('blocked', 'no_progress', 'budget_exhausted', 'human_paused')
                                          THEN ?4 ELSE blocked_reason END
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

/// A person stopped the run. The turn in flight finishes, because half a turn leaves nothing worth
/// keeping; no new task and no new round start. `resume` picks the run up where it stopped.
pub fn stop(store: &mut Store, run_id: &str, note: Option<&str>) -> Result<Outcome<()>> {
    let run = load_run(store.conn(), run_id)?;
    if run.status == RunStatus::HumanPaused {
        return Ok(Outcome::Refused("this run is already stopped".into()));
    }
    if run.status.is_terminal() || run.status == RunStatus::Reviewed {
        return Ok(Outcome::Refused(format!(
            "this run is {}; there is nothing to stop",
            run.status.as_str()
        )));
    }
    let reason = match note.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => format!("stopped by you: {n}"),
        None => "stopped by you".to_string(),
    };
    store.transact("run.stop", None, |tx, p| {
        set_run_status(tx, p, run_id, RunStatus::HumanPaused, Some(&reason))?;
        Ok(Outcome::Applied(()))
    })
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

/// The human gate as the CLI and the API run it: decide, then refresh the board so its status is not stale.
/// A refusal changes nothing and is returned as `Refused`.
pub fn decide_gate_and_render(home: &Home, store: &mut Store, run_id: &str, decision: &GateDecision) -> Result<Outcome<()>> {
    let outcome = store.transact("gate.decide", None, |tx, p| gate_decide(tx, p, run_id, decision))?;
    if let Outcome::Applied(_) = outcome {
        ledger::render_board(store.conn(), run_id, &home.board_path(run_id))?;
    }
    Ok(outcome)
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
            ledger::append(tx, pending, run_id, Some(&round.id), None, "constraint", &[], note, json!({"source": "human"}))?;
            set_run_status(tx, pending, run_id, RunStatus::Working, Some("sent back by human"))?;
            if let Some(orchestrator) = commands::orchestrator_name(tx, run_id)? {
                let prompt = format!(
                    "The human sent the run back with this note:\n{note}\n\nPlan the next step: create the tasks that answer \
                     the note. Do not write the deliverable yourself."
                );
                let scope = [commands::RUN_SCOPE.to_string()];
                let task = NewTask {
                    run_id,
                    thread: &orchestrator,
                    prompt: &prompt,
                    scope: &scope,
                    created_by: CreatedBy::User,
                    author_thread: None,
                    kind: "task",
                };
                if let Outcome::Refused(reason) = tasks::create_task(tx, pending, &task)? {
                    set_run_status(tx, pending, run_id, RunStatus::Blocked, Some(&reason))?;
                }
            }
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
