//! Background driving. A run is driven in its own task with its own SQLite connection, never a shared one.

use crate::state::{AppState, DriveClaim, ProviderBuilder};
use anyhow::Result;
use cli::ops;
use provider_core::Provider;
use runner::goal::GoalPlan;
use runner::{Home, Store};

/// Drive a run in the background. The claim is held until the drive ends, so no second run can start.
pub fn spawn_drive<P>(state: &AppState<P>, claim: DriveClaim, plan: GoalPlan, run_id: String)
where
    P: Provider + Send + Sync + 'static,
{
    let home = state.home.clone();
    let build = state.build.clone();
    tokio::spawn(async move {
        let _claim = claim;
        if let Err(err) = drive_run(&home, &build, &plan, &run_id).await {
            eprintln!("ilvunod: run {run_id} stopped with an error: {err:#}");
        }
    });
}

async fn drive_run<P>(home: &Home, build: &ProviderBuilder<P>, plan: &GoalPlan, run_id: &str) -> Result<()>
where
    P: Provider + Send + Sync,
{
    let mut store = Store::open(&home.db_path())?;
    let providers = build(plan)?;
    ops::drive_with(&providers, home, &mut store, plan, run_id).await?;
    Ok(())
}
