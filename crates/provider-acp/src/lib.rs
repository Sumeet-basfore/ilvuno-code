//! ACP adapter: any agent that speaks the Agent Client Protocol over stdio (docs/provider-contract.md).
//!
//! The ACP connection is push-based (callbacks); the runner pulls events. A task owns the connection:
//! callbacks push into a channel, `next_event` pulls from it, and commands (prompt, cancel, shutdown)
//! travel the other way. The connection task never waits on a prompt, so a cancel is always sent.

mod map;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::v1::schema::{
    CancelNotification, ContentBlock, InitializeRequest, LoadSessionRequest, NewSessionRequest, PromptRequest,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
    SessionId, SessionNotification, TextContent,
};
use agent_client_protocol::{
    v1::{ConnectionTo, Responder},
    Agent, AcpAgent, AcpAgentConfig, Client,
};
use map::CostTracker;
use provider_core::{Capabilities, Decision, ErrorKind, Event, Provider, ProviderError, Session, SpawnSpec};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub use map::select_option;

const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

/// One ACP agent, launched from a command line such as `npx -y @agentclientprotocol/codex-acp@2.9.0`.
#[derive(Debug, Clone)]
pub struct AcpProvider {
    pub command: String,
}

impl AcpProvider {
    pub fn new(command: impl Into<String>) -> Self {
        Self { command: command.into() }
    }

    /// The agent's command, run under `env -i` so the child gets exactly the variables the runner
    /// passed and nothing inherited. The ACP crate itself would pass the whole parent environment.
    fn launch(&self, env: &[(String, String)]) -> Result<AcpAgentConfig, ProviderError> {
        let inner = AcpAgent::from_str(&self.command)
            .map_err(|e| ProviderError::new(ErrorKind::SpawnFailed, format!("command {:?}: {e}", self.command)))?
            .into_config();
        // The config has no args getter; its serialized form is the public description of it.
        let described = serde_json::to_value(&inner).map_err(|e| ProviderError::new(ErrorKind::SpawnFailed, e.to_string()))?;
        let program = described["command"].as_str().unwrap_or_default().to_string();
        let args: Vec<String> = described["args"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default();
        let mut config = AcpAgentConfig::new("env").arg("-i");
        for (name, value) in env {
            config = config.arg(format!("{name}={value}"));
        }
        Ok(config.arg(program).args(args))
    }
}

enum Command {
    Prompt(String),
    Cancel,
    Shutdown,
}

enum Startup {
    Ready(Capabilities),
    Failed(String),
}

struct PendingPermission {
    responder: Responder<RequestPermissionResponse>,
    options: Vec<agent_client_protocol::v1::schema::PermissionOption>,
}

/// State the connection task and the session share.
#[derive(Clone)]
struct Shared {
    events: mpsc::UnboundedSender<Event>,
    commands: mpsc::UnboundedSender<Command>,
    pending: Arc<Mutex<HashMap<String, PendingPermission>>>,
    cancel_requested: Arc<AtomicBool>,
    /// Set when the call passed its USD cap; the turn then ends with an error, not a plain stop.
    cap_hit: Arc<Mutex<Option<String>>>,
    cost: Arc<Mutex<CostTracker>>,
    usd_cap: f64,
    permission_seq: Arc<AtomicU64>,
}

/// The session holds commands and the permission map, never the event sender: once the connection is
/// gone, the event channel closes and `next_event` can say so.
pub struct AcpSession {
    commands: mpsc::UnboundedSender<Command>,
    pending: Arc<Mutex<HashMap<String, PendingPermission>>>,
    cancel_requested: Arc<AtomicBool>,
    events: mpsc::UnboundedReceiver<Event>,
    caps: Capabilities,
    task: JoinHandle<()>,
}

impl Provider for AcpProvider {
    type Session = AcpSession;

    async fn open(&self, spec: SpawnSpec) -> Result<AcpSession, ProviderError> {
        std::fs::create_dir_all(&spec.cwd)
            .map_err(|e| ProviderError::new(ErrorKind::SpawnFailed, format!("cwd: {e}")))?;
        let config = self.launch(&spec.env)?;
        let (commands, command_rx) = mpsc::unbounded_channel();
        let (events, event_rx) = mpsc::unbounded_channel();
        let (startup_tx, mut startup_rx) = mpsc::unbounded_channel();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let cancel_requested = Arc::new(AtomicBool::new(false));
        let shared = Shared {
            events,
            commands: commands.clone(),
            pending: pending.clone(),
            cancel_requested: cancel_requested.clone(),
            cap_hit: Arc::new(Mutex::new(None)),
            cost: Arc::new(Mutex::new(CostTracker::default())),
            usd_cap: spec.max_budget_usd,
            permission_seq: Arc::new(AtomicU64::new(0)),
        };
        let task = tokio::spawn(run_connection(
            config,
            Connect { cwd: spec.cwd.clone(), session_id: spec.session_id.clone(), resume: spec.resume },
            shared,
            command_rx,
            startup_tx,
        ));
        match startup_rx.recv().await {
            Some(Startup::Ready(caps)) => {
                Ok(AcpSession { commands, pending, cancel_requested, events: event_rx, caps, task })
            }
            Some(Startup::Failed(message)) => Err(ProviderError::new(ErrorKind::SpawnFailed, message)),
            None => Err(ProviderError::new(ErrorKind::SpawnFailed, "agent connection ended before it started")),
        }
    }
}

struct Connect {
    cwd: PathBuf,
    session_id: String,
    resume: bool,
}

/// Owns the ACP connection for the life of the session. Startup outcome goes to `startup`; every later
/// event, including the final `SessionEnded`, goes to the event channel.
async fn run_connection(
    config: AcpAgentConfig,
    connect: Connect,
    shared: Shared,
    command_rx: mpsc::UnboundedReceiver<Command>,
    startup: mpsc::UnboundedSender<Startup>,
) {
    let ended = match serve(config, connect, shared.clone(), command_rx, startup.clone()).await {
        Ok(()) => "connection closed".to_string(),
        Err(e) => {
            let _ = startup.send(Startup::Failed(format!("{e}")));
            format!("agent error: {e}")
        }
    };
    let _ = shared.events.send(Event::SessionEnded { reason: ended });
}

async fn serve(
    config: AcpAgentConfig,
    connect: Connect,
    shared: Shared,
    mut command_rx: mpsc::UnboundedReceiver<Command>,
    startup: mpsc::UnboundedSender<Startup>,
) -> Result<(), agent_client_protocol::Error> {
    let updates = shared.clone();
    let requests = shared.clone();
    Client
        .v1()
        .name("ilvuno")
        .on_receive_notification(
            async move |note: SessionNotification, _cx| {
                let events = {
                    let mut cost = updates.cost.lock().unwrap_or_else(|p| p.into_inner());
                    let events = map::update_events(&note, &mut cost);
                    if cost.spent() > updates.usd_cap {
                        let mut hit = updates.cap_hit.lock().unwrap_or_else(|p| p.into_inner());
                        if hit.is_none() {
                            *hit = Some(format!("per-call cost cap of ${:.2} reached", updates.usd_cap));
                            let _ = updates.commands.send(Command::Cancel);
                        }
                    }
                    events
                };
                for event in events {
                    let _ = updates.events.send(event);
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |request: RequestPermissionRequest, responder: Responder<RequestPermissionResponse>, _cx| {
                let (tool, input) = map::translate_tool(&request.tool_call);
                let request_id = format!("perm-{}", requests.permission_seq.fetch_add(1, Ordering::Relaxed));
                let names = request.options.iter().map(|o| o.name.clone()).collect();
                requests
                    .pending
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .insert(request_id.clone(), PendingPermission { responder, options: request.options.clone() });
                let _ = requests.events.send(Event::PermissionRequest { request_id, tool, input, options: names });
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(trace_if_requested(AcpAgent::new(config)), async move |cx: ConnectionTo<Agent>| {
            let init = cx.send_request(InitializeRequest::new(ProtocolVersion::V1)).block_task().await?;
            let caps = Capabilities {
                resume: init.agent_capabilities.load_session,
                interrupt: true,
                approvals: true,
                usage: true,
                ..Default::default()
            };
            let cwd = connect.cwd.clone();
            let session = if connect.resume {
                let request = LoadSessionRequest::new(SessionId::new(connect.session_id.clone()), cwd);
                cx.send_request(request).block_task().await?;
                SessionId::new(connect.session_id.clone())
            } else {
                cx.send_request(NewSessionRequest::new(cwd)).block_task().await?.session_id
            };
            let _ = startup.send(Startup::Ready(caps));
            while let Some(command) = command_rx.recv().await {
                match command {
                    Command::Prompt(text) => {
                        shared.cancel_requested.store(false, Ordering::SeqCst);
                        let done = shared.clone();
                        cx.send_request(PromptRequest::new(
                            session.clone(),
                            vec![ContentBlock::Text(TextContent::new(text))],
                        ))
                        .on_receiving_result(async move |result| {
                            let cancel = done.cancel_requested.load(Ordering::SeqCst);
                            let cap = done.cap_hit.lock().unwrap_or_else(|p| p.into_inner()).take();
                            let events = match (result, cap) {
                                (_, Some(message)) => vec![map::turn_error(&message)],
                                (Ok(response), None) => map::turn_finished(&response, cancel),
                                (Err(e), None) => vec![map::turn_error(&e.to_string())],
                            };
                            for event in events {
                                let _ = done.events.send(event);
                            }
                            Ok(())
                        })?;
                    }
                    Command::Cancel => {
                        shared.cancel_requested.store(true, Ordering::SeqCst);
                        cx.send_notification(CancelNotification::new(session.clone()))?;
                    }
                    Command::Shutdown => break,
                }
            }
            Ok(())
        })
        .await
}

/// With `ILVUNO_ACP_TRACE=<file>` every wire line is appended to that file, tagged with its direction.
/// Used to record real frames for the mapping tests; off by default because it writes conversation text.
fn trace_if_requested(agent: AcpAgent) -> AcpAgent {
    let Some(path) = std::env::var_os("ILVUNO_ACP_TRACE") else { return agent };
    let file = match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => Arc::new(Mutex::new(file)),
        Err(_) => return agent,
    };
    agent.with_debug(move |line, direction| {
        if let Ok(mut out) = file.lock() {
            let _ = std::io::Write::write_all(&mut *out, format!("{direction:?}\t{line}\n").as_bytes());
        }
    })
}

impl Session for AcpSession {
    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }

    async fn send_turn(&mut self, text: &str) -> Result<(), ProviderError> {
        self.commands
            .send(Command::Prompt(text.to_string()))
            .map_err(|_| ProviderError::new(ErrorKind::TurnFailed, "adapter_died: connection closed"))
    }

    async fn next_event(&mut self) -> Result<Option<Event>, ProviderError> {
        Ok(self.events.recv().await)
    }

    async fn reply_permission(&mut self, request_id: &str, decision: Decision) -> Result<(), ProviderError> {
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(request_id)
            .ok_or_else(|| ProviderError::new(ErrorKind::Protocol, format!("no open permission request {request_id}")))?;
        let outcome = match select_option(&pending.options, &decision) {
            Some(option_id) => RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option_id)),
            None => RequestPermissionOutcome::Cancelled,
        };
        pending
            .responder
            .respond(RequestPermissionResponse::new(outcome))
            .map_err(|e| ProviderError::new(ErrorKind::Protocol, format!("permission reply: {e}")))
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        self.cancel_requested.store(true, Ordering::SeqCst);
        self.commands
            .send(Command::Cancel)
            .map_err(|_| ProviderError::new(ErrorKind::TurnFailed, "adapter_died: connection closed"))
    }

    async fn shutdown(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
        if tokio::time::timeout(SHUTDOWN_GRACE, &mut self.task).await.is_err() {
            self.task.abort();
        }
    }
}
