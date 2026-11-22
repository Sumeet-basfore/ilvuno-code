//! Claude Code adapter: stream-json on stdio plus the control protocol (research/09-spike.md §1–2).
//!
//! The `initialize` control request is always the first frame. Without it the CLI silently denies
//! every tool and the turn completes having done nothing (docs/provider-contract.md).

use provider_core::{Capabilities, Decision, ErrorKind, Event, Provider, ProviderError, Session, SpawnSpec};
use std::collections::VecDeque;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub mod wire;

const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);
const STDERR_LOG: &str = "provider-stderr.log";

#[derive(Debug, Clone)]
pub struct ClaudeProvider {
    pub binary: String,
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self { binary: "claude".to_string() }
    }
}

pub struct ClaudeSession {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Lines<BufReader<ChildStdout>>,
    session_id: String,
    caps: Capabilities,
    pending: VecDeque<Event>,
    interrupt_sent: bool,
    eof_reported: bool,
    counter: u64,
}

impl ClaudeSession {
    fn next_request_id(&mut self, prefix: &str) -> String {
        self.counter += 1;
        format!("{prefix}-{}", self.counter)
    }

    async fn write_frame(&mut self, frame: &serde_json::Value) -> Result<(), ProviderError> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| ProviderError::new(ErrorKind::Protocol, "stdin already closed"))?;
        let mut line = frame.to_string();
        line.push('\n');
        let io = async {
            stdin.write_all(line.as_bytes()).await?;
            stdin.flush().await
        };
        io.await
            .map_err(|e| ProviderError::new(ErrorKind::TurnFailed, format!("adapter_died: {e}")))
    }
}

impl Provider for ClaudeProvider {
    type Session = ClaudeSession;

    async fn open(&self, spec: SpawnSpec) -> Result<ClaudeSession, ProviderError> {
        uuid::Uuid::parse_str(&spec.session_id)
            .map_err(|_| ProviderError::new(ErrorKind::Protocol, "session_id must be a UUID"))?;
        std::fs::create_dir_all(&spec.cwd)
            .map_err(|e| ProviderError::new(ErrorKind::SpawnFailed, format!("cwd: {e}")))?;
        let stderr = std::fs::File::create(spec.cwd.join(STDERR_LOG))
            .map_err(|e| ProviderError::new(ErrorKind::SpawnFailed, format!("stderr log: {e}")))?;

        let mut cmd = Command::new(&self.binary);
        cmd.args([
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-prompt-tool",
            "stdio",
            "--permission-mode",
            "default",
        ]);
        cmd.arg("--max-budget-usd").arg(format!("{:.2}", spec.max_budget_usd));
        cmd.arg(if spec.resume { "--resume" } else { "--session-id" }).arg(&spec.session_id);
        if let Some(model) = &spec.model {
            cmd.args(["--model", model]);
        }
        cmd.env_clear()
            .envs(spec.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .current_dir(&spec.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr))
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| ProviderError::new(ErrorKind::SpawnFailed, format!("{}: {e}", self.binary)))?;
        let pid = child.id();
        let stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProviderError::new(ErrorKind::SpawnFailed, "no stdout pipe"))?;

        let mut session = ClaudeSession {
            child,
            stdin,
            lines: BufReader::new(stdout).lines(),
            session_id: spec.session_id.clone(),
            caps: wire::claude_capabilities(),
            pending: VecDeque::from([Event::Spawned { pid }]),
            interrupt_sent: false,
            eof_reported: false,
            counter: 0,
        };
        let init_id = session.next_request_id("init");
        session.write_frame(&wire::initialize_frame(&init_id)).await?;
        Ok(session)
    }
}

impl Session for ClaudeSession {
    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }

    async fn send_turn(&mut self, text: &str) -> Result<(), ProviderError> {
        self.interrupt_sent = false;
        let frame = wire::user_frame(&self.session_id, text);
        self.write_frame(&frame).await
    }

    async fn next_event(&mut self) -> Result<Option<Event>, ProviderError> {
        loop {
            if let Some(mut event) = self.pending.pop_front() {
                if let Event::TurnFinished { interrupted, .. } = &mut event {
                    *interrupted = self.interrupt_sent;
                }
                return Ok(Some(event));
            }
            if self.eof_reported {
                return Ok(None);
            }
            let line = match self.lines.next_line().await {
                Ok(Some(line)) => line,
                Ok(None) => {
                    self.eof_reported = true;
                    return Ok(Some(Event::SessionEnded { reason: "eof".to_string() }));
                }
                Err(e) => return Err(ProviderError::new(ErrorKind::Protocol, format!("read: {e}"))),
            };
            if line.trim().is_empty() {
                continue;
            }
            let frame: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
                ProviderError::new(ErrorKind::Protocol, format!("unparseable frame ({e}): {line}"))
            })?;
            self.pending.extend(wire::parse_frame(&frame));
        }
    }

    async fn reply_permission(&mut self, request_id: &str, decision: Decision) -> Result<(), ProviderError> {
        let frame = wire::permission_reply_frame(request_id, &decision);
        self.write_frame(&frame).await
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        self.interrupt_sent = true;
        let id = self.next_request_id("int");
        self.write_frame(&wire::interrupt_frame(&id)).await
    }

    async fn shutdown(&mut self) {
        // Closing stdin asks the CLI to exit; kill only if it does not within the grace period.
        self.stdin = None;
        if tokio::time::timeout(SHUTDOWN_GRACE, self.child.wait()).await.is_err() {
            let _ = self.child.kill().await;
        }
    }
}
