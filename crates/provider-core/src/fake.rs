//! Scripted provider: replays a fixed list of events per turn. All state-machine tests run on this,
//! so unit tests cost zero quota.

use crate::{Capabilities, Decision, Event, Provider, ProviderError, Session, SpawnSpec};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub type PromptHook = Box<dyn FnMut(&str) + Send>;

#[derive(Default)]
struct Shared {
    script: Mutex<VecDeque<Vec<Event>>>,
    hook: Mutex<Option<PromptHook>>,
    decisions: Mutex<Vec<(String, Decision)>>,
    prompts: Mutex<Vec<String>>,
    spawns: Mutex<Vec<SpawnSpec>>,
}

/// Cheap to clone; all clones share one script and one record of what happened.
#[derive(Clone, Default)]
pub struct FakeProvider {
    shared: Arc<Shared>,
}

impl FakeProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue the events for the next `send_turn`. Scripts are consumed in order.
    pub fn push_turn(&self, events: Vec<Event>) {
        if let Ok(mut s) = self.shared.script.lock() {
            s.push_back(events);
        }
    }

    /// Put one scripted turn before the queued turns. Tests use this to add events to the next run.
    pub fn push_turn_front(&self, events: Vec<Event>) {
        if let Ok(mut s) = self.shared.script.lock() {
            s.push_front(events);
        }
    }

    /// Called synchronously with each prompt text, before its events are yielded. Tests use this to
    /// play the part of the agent (e.g. invoke `ilvuno report`).
    pub fn set_prompt_hook(&self, hook: PromptHook) {
        if let Ok(mut h) = self.shared.hook.lock() {
            *h = Some(hook);
        }
    }

    pub fn decisions(&self) -> Vec<(String, Decision)> {
        self.shared.decisions.lock().map(|d| d.clone()).unwrap_or_default()
    }

    pub fn prompts(&self) -> Vec<String> {
        self.shared.prompts.lock().map(|p| p.clone()).unwrap_or_default()
    }

    pub fn spawns(&self) -> Vec<SpawnSpec> {
        self.shared.spawns.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

pub struct FakeSession {
    shared: Arc<Shared>,
    pending: VecDeque<Event>,
    caps: Capabilities,
}

impl Provider for FakeProvider {
    type Session = FakeSession;

    async fn open(&self, spec: SpawnSpec) -> Result<FakeSession, ProviderError> {
        if let Ok(mut s) = self.shared.spawns.lock() {
            s.push(spec);
        }
        Ok(FakeSession {
            shared: self.shared.clone(),
            pending: VecDeque::new(),
            caps: Capabilities { resume: true, interrupt: true, approvals: true, usage: true, ..Default::default() },
        })
    }
}

impl Session for FakeSession {
    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }

    async fn send_turn(&mut self, text: &str) -> Result<(), ProviderError> {
        if let Ok(mut p) = self.shared.prompts.lock() {
            p.push(text.to_string());
        }
        if let Ok(mut h) = self.shared.hook.lock() {
            if let Some(hook) = h.as_mut() {
                hook(text);
            }
        }
        let turn = self
            .shared
            .script
            .lock()
            .map_err(|_| ProviderError::new(crate::ErrorKind::Unknown, "script lock poisoned"))?
            .pop_front()
            .ok_or_else(|| ProviderError::new(crate::ErrorKind::Unknown, "fake script exhausted"))?;
        self.pending = turn.into();
        Ok(())
    }

    async fn next_event(&mut self) -> Result<Option<Event>, ProviderError> {
        Ok(self.pending.pop_front())
    }

    async fn reply_permission(&mut self, request_id: &str, decision: Decision) -> Result<(), ProviderError> {
        if let Ok(mut d) = self.shared.decisions.lock() {
            d.push((request_id.to_string(), decision));
        }
        Ok(())
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        self.pending.clear();
        Ok(())
    }

    async fn shutdown(&mut self) {
        self.pending.clear();
    }
}
