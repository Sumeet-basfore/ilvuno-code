//! Provider boundary (docs/provider-contract.md).
//!
//! An adapter is a translator: it turns a CLI's wire protocol into [`Event`]s and turns the runner's
//! commands into wire frames. It never writes state, never judges a round, never retries a turn.

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;

pub mod fake;

/// Failure classes from the error taxonomy in docs/provider-contract.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    SpawnFailed,
    AuthRequired,
    UsageLimit,
    TurnFailed,
    Protocol,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderError {
    pub kind: ErrorKind,
    pub message: String,
}

impl ProviderError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ProviderError {}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub reasoning_levels: Vec<String>,
    pub context_window: Option<u64>,
}

/// Declared, not probed ad hoc. The runner greys out what is absent.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Capabilities {
    pub resume: bool,
    pub fork: bool,
    pub rewind: bool,
    pub interrupt: bool,
    pub steer: bool,
    pub approvals: bool,
    pub structured_output: bool,
    pub usage: bool,
    pub models: Vec<Model>,
}

/// Normalized adapter output. Anything not modelled travels in `Raw`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Spawned { pid: Option<u32> },
    Session { native_id: String, capabilities: Capabilities },
    TextDelta { text: String },
    ThinkingDelta { text: String },
    ToolCall { id: String, name: String, input: serde_json::Value, kind: String, status: String },
    ToolResult { id: String, output: String, status: String },
    PermissionRequest { request_id: String, tool: String, input: serde_json::Value, options: Vec<String> },
    Usage {
        tokens_in: u64,
        tokens_out: u64,
        cache_read_tokens: u64,
        context_used: Option<u64>,
        cost_usd: Option<f64>,
    },
    TurnFinished { stop_reason: Option<String>, interrupted: bool, error: Option<String> },
    SessionEnded { reason: String },
    Raw { payload: serde_json::Value },
}

/// The runner's answer to a `PermissionRequest`. The policy decides; the adapter only translates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "behavior", rename_all = "snake_case")]
pub enum Decision {
    Allow { updated_input: serde_json::Value },
    Deny { message: String },
}

/// What the runner asks an adapter to start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnSpec {
    pub cwd: PathBuf,
    /// Environment for the child. The runner builds this from an allowlist; adapters add nothing.
    pub env: Vec<(String, String)>,
    pub max_budget_usd: f64,
    pub model: Option<String>,
    /// UUID chosen by the runner so the native session id is known before the process starts.
    pub session_id: String,
    /// Reattach to `session_id` instead of creating it.
    pub resume: bool,
}

pub trait Provider {
    type Session: Session;

    fn open(&self, spec: SpawnSpec) -> impl Future<Output = Result<Self::Session, ProviderError>> + Send;
}

pub trait Session: Send {
    fn capabilities(&self) -> &Capabilities;

    fn send_turn(&mut self, text: &str) -> impl Future<Output = Result<(), ProviderError>> + Send;

    /// `Ok(None)` means the process ended.
    fn next_event(&mut self) -> impl Future<Output = Result<Option<Event>, ProviderError>> + Send;

    fn reply_permission(
        &mut self,
        request_id: &str,
        decision: Decision,
    ) -> impl Future<Output = Result<(), ProviderError>> + Send;

    fn interrupt(&mut self) -> impl Future<Output = Result<(), ProviderError>> + Send;

    fn shutdown(&mut self) -> impl Future<Output = ()> + Send;
}
