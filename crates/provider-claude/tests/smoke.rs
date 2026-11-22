//! Manual smoke test against the real `claude` CLI. It spends quota, so it is ignored by default:
//!   cargo test -p provider-claude --test smoke -- --ignored --nocapture
//! Each process is capped at --max-budget-usd 0.10.

use provider_claude::ClaudeProvider;
use provider_core::{Decision, Event, Provider, Session, SpawnSpec};
use serde_json::json;
use std::path::PathBuf;

fn spec(cwd: PathBuf, session_id: &str, resume: bool) -> SpawnSpec {
    let env = vec![
        ("HOME".to_string(), std::env::var("HOME").unwrap_or_default()),
        ("PATH".to_string(), std::env::var("PATH").unwrap_or_default()),
    ];
    SpawnSpec { cwd, env, max_budget_usd: 0.10, model: None, session_id: session_id.to_string(), resume }
}

/// Drain one turn: answer permissions with allow, return (text, cost, tokens_in, tokens_out).
async fn run_turn<S: Session>(s: &mut S) -> (String, f64, u64, u64) {
    let mut text = String::new();
    let (mut cost, mut tin, mut tout) = (0.0, 0, 0);
    loop {
        match s.next_event().await.expect("event") {
            Some(Event::PermissionRequest { request_id, tool, input, .. }) => {
                println!("permission: {tool} {input}");
                s.reply_permission(&request_id, Decision::Allow { updated_input: input }).await.expect("reply");
            }
            Some(Event::TextDelta { text: t }) => text.push_str(&t),
            Some(Event::Usage { tokens_in, tokens_out, cost_usd, .. }) => {
                cost = cost_usd.unwrap_or(0.0);
                tin = tokens_in;
                tout = tokens_out;
            }
            Some(Event::TurnFinished { error, .. }) => {
                assert!(error.is_none(), "turn error: {error:?}");
                break;
            }
            Some(Event::SessionEnded { reason }) => panic!("session ended mid-turn: {reason}"),
            Some(_) => {}
            None => panic!("stream closed mid-turn"),
        }
    }
    (text, cost, tin, tout)
}

#[tokio::test]
#[ignore = "spends real quota"]
async fn real_claude_permission_turn_then_resume() {
    let dir = std::env::temp_dir().join(format!("ilvuno-smoke-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("dir");
    let sid = uuid::Uuid::new_v4().to_string();
    let provider = ClaudeProvider::default();

    let mut s = provider.open(spec(dir.clone(), &sid, false)).await.expect("open");
    s.send_turn("Use the Write tool to create hello.txt in the current directory with exactly the content: ok . Then reply DONE.")
        .await
        .expect("send");
    let (text1, cost1, tin1, tout1) = run_turn(&mut s).await;
    println!("turn1 text={text1:?} cost=${cost1:.4} tokens_in={tin1} tokens_out={tout1}");
    s.shutdown().await;
    assert_eq!(std::fs::read_to_string(dir.join("hello.txt")).expect("file").trim(), "ok");

    let mut r = provider.open(spec(dir.clone(), &sid, true)).await.expect("resume");
    r.send_turn("What exact word did you write into hello.txt? Reply with just that word.").await.expect("send");
    let (text2, cost2, tin2, tout2) = run_turn(&mut r).await;
    println!("resume text={text2:?} cost=${cost2:.4} tokens_in={tin2} tokens_out={tout2}");
    r.shutdown().await;
    assert!(text2.to_lowercase().contains("ok"), "resumed session lost context: {text2:?}");
    println!("{}", json!({"session": sid, "turn1_cost": cost1, "resume_cost": cost2}));
}

#[tokio::test]
#[ignore = "spends real quota"]
async fn real_claude_interrupt_mid_turn_reports_interrupted() {
    let dir = std::env::temp_dir().join(format!("ilvuno-smoke-int-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("dir");
    let sid = uuid::Uuid::new_v4().to_string();
    let mut s = ClaudeProvider::default().open(spec(dir, &sid, false)).await.expect("open");
    s.send_turn("Write the numbers from 1 to 300, one per line, as plain text. Do not use tools.")
        .await
        .expect("send");
    let mut interrupted_sent = false;
    loop {
        match s.next_event().await.expect("event") {
            Some(Event::TextDelta { .. }) | Some(Event::ThinkingDelta { .. }) if !interrupted_sent => {
                s.interrupt().await.expect("interrupt");
                interrupted_sent = true;
            }
            Some(Event::TurnFinished { interrupted, .. }) => {
                println!("interrupted={interrupted}");
                assert!(interrupted_sent, "turn finished before we interrupted");
                assert!(interrupted, "turn must report interrupted");
                break;
            }
            Some(Event::PermissionRequest { request_id, input, .. }) => {
                s.reply_permission(&request_id, Decision::Allow { updated_input: input }).await.expect("reply");
            }
            Some(_) => {}
            None => panic!("stream closed"),
        }
    }
    s.shutdown().await;
}
