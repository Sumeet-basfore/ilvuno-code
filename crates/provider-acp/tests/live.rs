//! Live check against a real ACP agent. It spends real money and needs that agent's own login, so it
//! is ignored by default:
//!
//! ```text
//! ILVUNO_ACP_LIVE_COMMAND="npx -y @agentclientprotocol/codex-acp@2.9.0" \
//!   cargo test -p provider-acp --test live -- --ignored --nocapture
//! ```
//!
//! Set `ILVUNO_ACP_TRACE=<file>` to record every wire frame for the mapping tests.

use provider_acp::AcpProvider;
use provider_core::{Decision, Event, Provider, Session, SpawnSpec};
use std::time::Duration;

const DEFAULT_COMMAND: &str = "npx -y @agentclientprotocol/codex-acp@2.9.0";
const CAP_USD: f64 = 0.05;

fn child_env() -> Vec<(String, String)> {
    ["PATH", "HOME", "USER", "LANG", "TERM", "TMPDIR"]
        .iter()
        .filter_map(|key| std::env::var(key).ok().map(|value| (key.to_string(), value)))
        .collect()
}

#[tokio::test]
#[ignore = "live: spends money and needs the agent's login"]
async fn one_prompt_round_trip_on_a_real_agent() {
    let command = std::env::var("ILVUNO_ACP_LIVE_COMMAND").unwrap_or_else(|_| DEFAULT_COMMAND.to_string());
    let cwd = std::env::temp_dir().join(format!("ilvuno-acp-live-{}", uuid::Uuid::new_v4()));
    let spec = SpawnSpec {
        cwd,
        env: child_env(),
        max_budget_usd: CAP_USD,
        model: None,
        session_id: uuid::Uuid::new_v4().to_string(),
        resume: false,
    };
    let mut session = AcpProvider::new(command).open(spec).await.expect("open session");
    println!("capabilities: {:?}", session.capabilities());
    session.send_turn("Reply with exactly one word: ok").await.expect("send turn");

    let mut text = String::new();
    let mut finished = None;
    loop {
        let next = tokio::time::timeout(Duration::from_secs(300), session.next_event())
            .await
            .expect("agent went silent for 300 s")
            .expect("event");
        let Some(event) = next else { break };
        println!("event: {event:?}");
        match event {
            Event::TextDelta { text: chunk } => text.push_str(&chunk),
            Event::PermissionRequest { request_id, .. } => {
                // This probe asks for no tools: any permission request is refused.
                let deny = Decision::Deny { message: "live probe runs no tools".into() };
                session.reply_permission(&request_id, deny).await.expect("reply");
            }
            done @ Event::TurnFinished { .. } => {
                finished = Some(done);
                break;
            }
            _ => {}
        }
    }
    session.shutdown().await;
    assert!(text.to_lowercase().contains("ok"), "reply text: {text:?}");
    assert!(
        matches!(finished, Some(Event::TurnFinished { error: None, .. })),
        "turn did not finish cleanly: {finished:?}"
    );
}
