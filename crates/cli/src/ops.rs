//! The run operations behind `ilvuno run`, `gate` and the API: create a run from goal text, and drive it.

use crate::providers;
use anyhow::{bail, Result};
use provider_core::Provider;
use runner::commands;
use runner::driver::{Driver, RunEnd};
use runner::goal::{self, GoalPlan};
use runner::store::Outcome;
use runner::{Home, Store};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where runs are kept: `ILVUNO_HOME`, or `~/.local/share/ilvuno`.
pub fn home_from_env() -> Home {
    let root = std::env::var_os("ILVUNO_HOME").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share/ilvuno")
    });
    Home::new(root)
}

/// Directory of this binary, so agents can run `ilvuno` by name.
fn own_bin_dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf))
}

/// Create a run from goal JSON text and snapshot it. `base_for` names the folder that relative paths in the
/// goal resolve from; it may depend on the new run's id. Returns the run id and the plan read back from the
/// snapshot, so later steps use exactly what was recorded.
pub fn create_run(store: &mut Store, home: &Home, raw: &str, base_for: impl Fn(&str) -> PathBuf) -> Result<(String, GoalPlan)> {
    // Validate, and refuse a driver this build lacks, before any run row is written.
    let probe = GoalPlan::from_raw(raw, home.root())?;
    providers::build(&probe)?;
    let new_goal = probe.new_goal()?;
    let run_id = match store.transact("run.create", None, |tx, p| {
        let run_id = commands::create_run(tx, p, &new_goal, home)?;
        goal::snapshot(tx, &run_id, raw, &base_for(&run_id))?;
        Ok(Outcome::Applied(run_id))
    })? {
        Outcome::Applied(id) => id,
        other => bail!("run not created: {other:?}"),
    };
    let plan = GoalPlan::from_raw(raw, &base_for(&run_id))?;
    Ok((run_id, plan))
}

/// Drive a run with the agents its goal names. This is `ilvuno run`.
pub async fn drive(home: &Home, store: &mut Store, plan: &GoalPlan, run_id: &str) -> Result<RunEnd> {
    let instances = providers::build(plan)?;
    drive_with(&instances, home, store, plan, run_id).await
}

/// Drive a run with the given agent instances. The API passes its own builder through here, so tests can
/// run the same loop on the scripted provider.
pub async fn drive_with<P: Provider>(
    providers: &BTreeMap<String, P>,
    home: &Home,
    store: &mut Store,
    plan: &GoalPlan,
    run_id: &str,
) -> Result<RunEnd> {
    reopen_if_stopped(store, run_id)?;
    let bin_dir = own_bin_dir();
    let driver = Driver { providers, home, ilvuno_bin_dir: bin_dir.as_deref(), max_usd: plan.usd_per_call() };
    driver.drive(store, plan, run_id).await
}

/// Driving is also resuming: a run that was stopped goes back to working first, or the loop would read the
/// very state it is being resumed from and end at once. Recorded like any other transition.
fn reopen_if_stopped(store: &mut Store, run_id: &str) -> Result<()> {
    use runner::RunStatus;
    let status = commands::load_run(store.conn(), run_id)?.status;
    let stopped = matches!(
        status,
        RunStatus::Blocked | RunStatus::BudgetExhausted | RunStatus::NoProgress | RunStatus::HumanPaused
    );
    if !stopped {
        return Ok(());
    }
    store.transact("run.reopen", None, |tx, p| {
        // The run is open again: no end time (it would freeze "Ran for"), and no stale stop reason.
        tx.execute("UPDATE runs SET ended_at = NULL, blocked_reason = NULL WHERE id = ?1", [run_id])?;
        runner::run_status::set_run_status(tx, p, run_id, RunStatus::Working, Some("resumed by you"))?;
        Ok(Outcome::Applied(()))
    })?;
    Ok(())
}
