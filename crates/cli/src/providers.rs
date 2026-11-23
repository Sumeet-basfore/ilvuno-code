//! The concrete adapters this binary can run, behind one type. `Provider` returns `impl Future`, so it
//! cannot be a trait object; a closed enum lets one run mix drivers, one slot per instance.

use anyhow::{bail, Result};
use provider_acp::{AcpProvider, AcpSession};
use provider_claude::{ClaudeProvider, ClaudeSession};
use provider_core::{Capabilities, Decision, Event, Provider, ProviderError, Session, SpawnSpec};
use runner::goal::GoalPlan;
use std::collections::BTreeMap;

pub enum AnyProvider {
    Claude(ClaudeProvider),
    Acp(AcpProvider),
}

pub enum AnySession {
    Claude(Box<ClaudeSession>),
    Acp(Box<AcpSession>),
}

/// One adapter per provider instance the goal declares. A driver this build lacks is refused here,
/// before any agent starts.
pub fn build(plan: &GoalPlan) -> Result<BTreeMap<String, AnyProvider>> {
    let mut instances = BTreeMap::new();
    for (id, spec) in plan.providers() {
        let instance = match spec.driver.as_str() {
            "claude" => AnyProvider::Claude(ClaudeProvider { binary: spec.command.clone().unwrap_or_else(|| "claude".into()) }),
            "acp" => AnyProvider::Acp(AcpProvider::new(spec.command.clone().unwrap_or_default(), spec.mode.clone())),
            other => bail!("provider {id:?}: driver {other:?} is not available in this build"),
        };
        instances.insert(id.clone(), instance);
    }
    Ok(instances)
}

impl Provider for AnyProvider {
    type Session = AnySession;

    async fn open(&self, spec: SpawnSpec) -> Result<AnySession, ProviderError> {
        match self {
            AnyProvider::Claude(p) => Ok(AnySession::Claude(Box::new(p.open(spec).await?))),
            AnyProvider::Acp(p) => Ok(AnySession::Acp(Box::new(p.open(spec).await?))),
        }
    }
}

impl Session for AnySession {
    fn capabilities(&self) -> &Capabilities {
        match self {
            AnySession::Claude(s) => s.capabilities(),
            AnySession::Acp(s) => s.capabilities(),
        }
    }

    async fn send_turn(&mut self, text: &str) -> Result<(), ProviderError> {
        match self {
            AnySession::Claude(s) => s.send_turn(text).await,
            AnySession::Acp(s) => s.send_turn(text).await,
        }
    }

    async fn next_event(&mut self) -> Result<Option<Event>, ProviderError> {
        match self {
            AnySession::Claude(s) => s.next_event().await,
            AnySession::Acp(s) => s.next_event().await,
        }
    }

    async fn reply_permission(&mut self, request_id: &str, decision: Decision) -> Result<(), ProviderError> {
        match self {
            AnySession::Claude(s) => s.reply_permission(request_id, decision).await,
            AnySession::Acp(s) => s.reply_permission(request_id, decision).await,
        }
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        match self {
            AnySession::Claude(s) => s.interrupt().await,
            AnySession::Acp(s) => s.interrupt().await,
        }
    }

    async fn shutdown(&mut self) {
        match self {
            AnySession::Claude(s) => s.shutdown().await,
            AnySession::Acp(s) => s.shutdown().await,
        }
    }
}
