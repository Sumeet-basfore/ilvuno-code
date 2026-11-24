//! Startup repair for runs left working by a process that stopped.

use crate::state::AppState;
use anyhow::{anyhow, Result};
use runner::run_status;
use runner::store::Outcome;
use runner::Store;

/// Pause runs without an active drive claim before the API starts serving requests.
pub fn reconcile_startup_runs<P>(state: &AppState<P>) -> Result<usize> {
    let recovered = state.with_drive_unclaimed(|| {
        let mut store = Store::open(&state.home.db_path())?;
        match run_status::reconcile_interrupted_runs(&mut store)? {
            Outcome::Applied(count) => Ok(count),
            Outcome::Refused(reason) | Outcome::Duplicate(reason) => {
                Err(anyhow!("startup reconciliation refused: {reason}"))
            }
        }
    })?;
    Ok(recovered.unwrap_or(0))
}
