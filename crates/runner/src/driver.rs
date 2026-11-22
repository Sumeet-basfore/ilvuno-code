//! The round loop. Resumes from the journal's projection: an open round is continued (dispatches that
//! already reported are skipped), a closed round is followed by the next plan, the checks decide the
//! verdict, and a passing verdict stops at the human gate.

use crate::checks::{self, Check, SKIPPED};
use crate::commands::{self, load_run, RoundRow};
use crate::dispatch::{self, ExecCtx};
use crate::goal::GoalPlan;
use crate::home::Home;
use crate::research::{self, BRIEF_FILE, SYNTH_SLOT};
use crate::run_status::{self, set_run_status};
use crate::state::{is_no_progress, RunStatus, Verdict};
use crate::store::{Outcome, Store};
use anyhow::{anyhow, Result};
use provider_core::Provider;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEnd {
    /// Verdict passed; waiting for the human.
    AtGate,
    Accepted,
    Paused,
    Stopped { status: RunStatus, reason: String },
}

pub struct Driver<'a, P: Provider> {
    pub provider: &'a P,
    pub home: &'a Home,
    pub ilvuno_bin_dir: Option<&'a Path>,
    pub max_usd: f64,
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
            match open_round(store.conn(), run_id)? {
                Some(round) => self.continue_round(store, plan, run_id, &round).await?,
                None => {
                    if let Some(reason) = self.plan_next_round(store, plan, run_id, run.rounds_used)? {
                        return Ok(RunEnd::Stopped { status: RunStatus::Blocked, reason });
                    }
                }
            }
        }
    }

    async fn continue_round(&self, store: &mut Store, plan: &GoalPlan, run_id: &str, round: &RoundRow) -> Result<()> {
        let round_id = round.id.clone();
        for (dispatch_id, _slot, _status) in dispatch::open_dispatches(store.conn(), &round_id)? {
            let cx = ExecCtx {
                home: self.home,
                plan,
                ilvuno_bin_dir: self.ilvuno_bin_dir,
                max_usd: self.max_usd,
            };
            dispatch::execute(self.provider, store, &cx, run_id, &round_id, round.index, &dispatch_id).await?;
        }
        self.finish_round(store, plan, run_id, round)
    }

    /// Plan a new round. Returns `Some(reason)` when the round is refused and the run is blocked.
    fn plan_next_round(&self, store: &mut Store, plan: &GoalPlan, run_id: &str, index: u32) -> Result<Option<String>> {
        let notes = revision_notes(store.conn(), run_id, index)?;
        let slots = research::round_slots(plan, index, &notes);
        let root = self.home.threads_dir(run_id);
        let outcome = store.transact("round.plan", None, |tx, p| {
            commands::plan_round(tx, p, run_id, &slots, &root, dispatch::provider_name())
        })?;
        match outcome {
            Outcome::Applied(_) => Ok(None),
            Outcome::Refused(reason) | Outcome::Duplicate(reason) => {
                let stored = reason.clone();
                store.transact("run.block", None, |tx, p| {
                    set_run_status(tx, p, run_id, RunStatus::Blocked, Some(&stored))?;
                    Ok(Outcome::Applied(()))
                })?;
                Ok(Some(reason))
            }
        }
    }

    /// Checks decide the verdict. A failed dispatch is a failed check. Identical failures two rounds
    /// running are no progress, and the run stops rather than spending another round.
    fn finish_round(&self, store: &mut Store, plan: &GoalPlan, run_id: &str, round: &RoundRow) -> Result<()> {
        let failed_dispatches = failed_slots(store.conn(), &round.id)?;
        let mut checks: Vec<Check> = Vec::new();
        if failed_dispatches.is_empty() {
            let snapshot = dispatch::snapshot_dir(self.home, run_id, round.index, SYNTH_SLOT).join(BRIEF_FILE);
            let html = std::fs::read_to_string(snapshot).ok();
            checks = checks::evaluate(html.as_deref(), &plan.source_name, &plan.source_text, &plan.file.required_sections);
        }
        let skipped: Vec<Value> = SKIPPED
            .iter()
            .filter(|(name, _)| plan.file.checks.iter().any(|c| c == name))
            .map(|(name, reason)| json!({ "name": name, "reason": reason }))
            .collect();
        let mut failing: Vec<String> = checks.iter().filter(|c| !c.passed).map(|c| c.name.to_string()).collect();
        failing.extend(failed_dispatches.iter().map(|slot| format!("dispatch:{slot}")));
        let previous = previous_failing(store.conn(), run_id, round.index)?;
        let stalled = is_no_progress(previous.as_deref(), &failing);

        let verdict = if failing.is_empty() { Verdict::Pass } else { Verdict::Block };
        let readiness = json!({
            "checks": checks.iter().map(|c| json!({ "name": c.name, "passed": c.passed, "detail": c.detail })).collect::<Vec<_>>(),
            "skipped": skipped,
            "failing": failing,
            "failed_dispatches": failed_dispatches,
        });
        let round_id = round.id.clone();
        store.transact("round.verdict", None, |tx, p| {
            run_status::record_verdict(tx, p, run_id, &round_id, verdict, &readiness)?;
            Ok(Outcome::Applied(()))
        })?;
        if stalled {
            store.transact("run.no_progress", None, |tx, p| {
                set_run_status(tx, p, run_id, RunStatus::NoProgress, Some("same failing checks two rounds running"))?;
                Ok(Outcome::Applied(()))
            })?;
        }
        Ok(())
    }
}

/// The round that has no verdict yet, if any: the one to continue.
pub fn open_round(conn: &Connection, run_id: &str) -> Result<Option<RoundRow>> {
    let row = commands::latest_round(conn, run_id)?;
    Ok(row.filter(|r| r.verdict.is_none()))
}

/// Notes the next round must address: failed checks and human notes from the round before it.
pub fn revision_notes(conn: &Connection, run_id: &str, index: u32) -> Result<Vec<String>> {
    if index == 0 {
        return Ok(Vec::new());
    }
    let prev: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT id, readiness FROM rounds WHERE run_id = ?1 AND \"index\" = ?2",
            params![run_id, index - 1],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((prev_id, readiness)) = prev else { return Ok(Vec::new()) };
    let mut notes = Vec::new();
    if let Some(text) = readiness {
        let value: Value = serde_json::from_str(&text)?;
        for check in value["checks"].as_array().into_iter().flatten() {
            if check["passed"] == Value::Bool(false) {
                notes.push(format!("check {} failed: {}", check["name"].as_str().unwrap_or("?"), check["detail"].as_str().unwrap_or("")));
            }
        }
        for slot in value["failed_dispatches"].as_array().into_iter().flatten() {
            notes.push(format!("worker {} failed in the previous round", slot.as_str().unwrap_or("?")));
        }
    }
    let mut stmt = conn.prepare("SELECT body FROM ledger WHERE round_id = ?1 AND kind = 'constraint' ORDER BY id")?;
    for body in stmt.query_map(params![prev_id], |r| r.get::<_, String>(0))? {
        notes.push(format!("human note: {}", body?));
    }
    Ok(notes)
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

fn failed_slots(conn: &Connection, round_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT slot FROM dispatch WHERE round_id = ?1 AND status = 'failed' ORDER BY rowid")?;
    let slots = stmt.query_map(params![round_id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    Ok(slots)
}

fn blocked_reason(conn: &Connection, run_id: &str) -> Result<String> {
    conn.query_row("SELECT COALESCE(blocked_reason, status) FROM runs WHERE id = ?1", params![run_id], |r| r.get(0))
        .map_err(|e| anyhow!("run {run_id}: {e}"))
}
