//! Shared state: the home directory, the builder for agent instances, and the one-run-at-a-time drive slot.

use crate::agents::AgentCache;
use crate::error::{ApiError, ApiResult};
use runner::goal::GoalPlan;
use runner::Home;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

/// Builds the agent instances a goal names. The binary passes the real adapters; tests pass a scripted one.
pub type ProviderBuilder<P> = Arc<dyn Fn(&GoalPlan) -> anyhow::Result<BTreeMap<String, P>> + Send + Sync>;

pub struct AppState<P> {
    pub home: Arc<Home>,
    pub build: ProviderBuilder<P>,
    /// The last `GET /agents` answer, so the window does not spawn six processes each time.
    pub agents: Arc<AgentCache>,
    slot: Arc<DriveSlot>,
}

impl<P> Clone for AppState<P> {
    fn clone(&self) -> Self {
        Self {
            home: self.home.clone(),
            build: self.build.clone(),
            agents: self.agents.clone(),
            slot: self.slot.clone(),
        }
    }
}

impl<P> AppState<P> {
    pub fn new(home: Home, build: ProviderBuilder<P>) -> Self {
        Self {
            home: Arc::new(home),
            build,
            agents: Arc::new(AgentCache::default()),
            slot: Arc::new(DriveSlot::default()),
        }
    }

    /// Take the right to drive a run. Only one run is driven per process. `holder` says which run, for the
    /// refusal message. The right is given back when the returned claim is dropped.
    pub fn claim_drive(&self, holder: &str) -> ApiResult<DriveClaim> {
        let mut current = lock(&self.slot.holder);
        if let Some(other) = current.as_ref() {
            return Err(ApiError::Conflict(format!(
                "another run is already being driven by this process ({other}); wait for it to reach the gate or stop, then try again"
            )));
        }
        *current = Some(holder.to_string());
        Ok(DriveClaim { slot: self.slot.clone() })
    }

    /// Run startup work only while the slot is empty, keeping the lock for the full action. This closes
    /// the gap between checking the claim and changing run state.
    pub fn with_drive_unclaimed<T>(
        &self,
        action: impl FnOnce() -> anyhow::Result<T>,
    ) -> anyhow::Result<Option<T>> {
        let holder = lock(&self.slot.holder);
        if holder.is_some() {
            return Ok(None);
        }
        action().map(Some)
    }
}

#[derive(Default)]
struct DriveSlot {
    holder: Mutex<Option<String>>,
}

/// Held while a run is driven in the background. Dropping it frees the slot.
pub struct DriveClaim {
    slot: Arc<DriveSlot>,
}

impl DriveClaim {
    /// Name the run that now holds the slot, once its id is known.
    pub fn set_holder(&self, run_id: &str) {
        *lock(&self.slot.holder) = Some(format!("run {run_id}"));
    }
}

impl Drop for DriveClaim {
    fn drop(&mut self) {
        *lock(&self.slot.holder) = None;
    }
}

/// A poisoned lock only means another request panicked; the slot's value is still valid to read.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
