//! Shared test harness: a temp `ILVUNO_HOME`, the scripted provider, and in-process requests (no socket).

#![allow(dead_code)]

use api::state::{AppState, ProviderBuilder};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use provider_core::fake::FakeProvider;
use runner::commands::ReportStatus;
use runner::goal::GoalPlan;
use runner::report::run_report;
use runner::store::Outcome;
use runner::tasks::run_task;
use runner::Home;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

/// A goal with one worker and no orchestrator: starting it opens no round and calls no agent.
pub const GOAL_WORKERS: &str = r#"{
  "goal": "Write notes for one source.",
  "done": "notes.md exists.",
  "providers": { "claude-main": { "driver": "claude" } },
  "threads": [
    { "name": "reader-1", "agent": "claude-main", "prompt": "You read the source you are given and write short notes." }
  ],
  "budget": { "rounds": 3, "tokens": 600000, "max_usd_per_call": 0.2 }
}"#;

/// A goal with an orchestrator, so starting it runs the scripted agent. The fake agent writes `notes.md`.
pub const GOAL_WITH_ORCHESTRATOR: &str = r#"{
  "goal": "Write notes for one source.",
  "done": "notes.md exists.",
  "providers": { "claude-main": { "driver": "claude" } },
  "threads": [
    { "name": "orchestrator", "agent": "claude-main", "prompt": "Plan the work." },
    { "name": "reader-1", "agent": "claude-main", "prompt": "You read the source you are given and write short notes." }
  ],
  "budget": { "rounds": 3, "tokens": 600000, "max_usd_per_call": 0.2 }
}"#;

pub struct Harness {
    pub dir: PathBuf,
    pub home: Home,
    pub fake: FakeProvider,
    pub state: AppState<FakeProvider>,
    pub app: Router,
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A fresh home with the scripted provider behind every agent the goal names.
pub fn harness() -> Harness {
    let dir = std::env::temp_dir().join(format!("ilvunod-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let dir = std::fs::canonicalize(&dir).expect("canonical scratch dir");
    let home = Home::new(dir.join("home"));
    std::fs::create_dir_all(home.root()).expect("home dir");
    let fake = FakeProvider::new();
    fake.set_prompt_hook(scripted_agent(home.clone()));
    for _ in 0..20 {
        fake.push_turn(vec![said("done"), usage(), finished()]);
    }
    let builder: ProviderBuilder<FakeProvider> = {
        let fake = fake.clone();
        Arc::new(move |plan: &GoalPlan| Ok(plan.providers().keys().map(|id| (id.clone(), fake.clone())).collect()))
    };
    let state = AppState::new(home.clone(), builder);
    let app = api::router(state.clone());
    Harness { dir, home, fake, state, app }
}

/// The scripted agent. The orchestrator plans one task for `reader-1`; the worker writes `notes.md`. Each
/// reports through the same runner functions the `ilvuno` commands call.
fn scripted_agent(home: Home) -> Box<dyn FnMut(&str) + Send> {
    Box::new(move |prompt: &str| {
        let task_id = between(prompt, "Your task id is ", ".");
        if prompt.contains("You are the orchestrator") {
            let scope = vec!["notes.md".to_string()];
            let outcome = run_task(&home, Some(&task_id), "reader-1", "Write notes.md from the source.", &scope)
                .expect("orchestrator task command");
            assert!(matches!(outcome, Outcome::Applied(_)), "orchestrator task refused: {outcome:?}");
            report(&home, &task_id, &[]);
            return;
        }
        let folder = PathBuf::from(between(prompt, "Work only inside your folder: ", ". You may read"));
        std::fs::create_dir_all(&folder).expect("worker folder");
        std::fs::write(folder.join("notes.md"), "notes\n").expect("worker file");
        report(&home, &task_id, &["notes.md".to_string()]);
    })
}

/// A scripted orchestrator queues two tasks on reader-1; the worker writes one file per task and, on
/// the second, proves it can see the first task's file. Used by the worktree tests.
pub fn two_task_script(home: Home) -> Box<dyn FnMut(&str) + Send> {
    Box::new(move |prompt: &str| {
        let task_id = between(prompt, "Your task id is ", ".");
        if prompt.contains("You are the orchestrator") {
            let first = run_task(&home, Some(&task_id), "reader-1", "Write notes.md from the source.", &["notes.md".to_string()])
                .expect("first task command");
            assert!(matches!(first, Outcome::Applied(_)), "first task refused: {first:?}");
            let second = run_task(
                &home,
                Some(&task_id),
                "reader-1",
                "Add second.md. notes.md must already be there.",
                &["second.md".to_string()],
            )
            .expect("second task command");
            assert!(matches!(second, Outcome::Applied(_)), "second task refused: {second:?}");
            report(&home, &task_id, &[]);
            return;
        }
        let folder = PathBuf::from(between(prompt, "Work only inside your folder: ", ". You may read"));
        if prompt.contains("Add second.md") {
            assert!(folder.join("notes.md").is_file(), "the second task sees the first task's file");
            std::fs::write(folder.join("second.md"), "second\n").expect("second file");
            report(&home, &task_id, &["second.md".to_string()]);
        } else {
            std::fs::create_dir_all(&folder).expect("worker folder");
            std::fs::write(folder.join("notes.md"), "notes\n").expect("first file");
            report(&home, &task_id, &["notes.md".to_string()]);
        }
    })
}

fn report(home: &Home, task_id: &str, files: &[String]) {
    let outcome = run_report(home, task_id, ReportStatus::Ok, "three sentences of summary.", files).expect("report command");
    assert!(matches!(outcome, Outcome::Applied(())), "report refused: {outcome:?}");
}

fn between(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).expect("start marker") + start.len();
    let to = text[from..].find(end).expect("end marker") + from;
    text[from..to].to_string()
}

fn said(text: &str) -> provider_core::Event {
    provider_core::Event::TextDelta { text: text.to_string() }
}

fn usage() -> provider_core::Event {
    provider_core::Event::Usage { tokens_in: 900, tokens_out: 150, cache_read_tokens: 0, context_used: None, cost_usd: Some(0.01) }
}

fn finished() -> provider_core::Event {
    provider_core::Event::TurnFinished { stop_reason: Some("end_turn".into()), interrupted: false, error: None }
}

/// One response: its status and its body parsed as JSON (`Null` when the body is not JSON).
pub struct Reply {
    pub status: StatusCode,
    pub json: Value,
}

pub async fn send(app: &Router, method: &str, uri: &str, body: Option<&str>) -> Reply {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.map(|b| Body::from(b.to_string())).unwrap_or_else(Body::empty))
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("in-process request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body");
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Reply { status, json }
}

pub async fn get(app: &Router, uri: &str) -> Reply {
    send(app, "GET", uri, None).await
}

pub async fn post(app: &Router, uri: &str, body: &str) -> Reply {
    send(app, "POST", uri, Some(body)).await
}

pub async fn put(app: &Router, uri: &str, body: &str) -> Reply {
    send(app, "PUT", uri, Some(body)).await
}

/// Start a run from goal text and return its id. Panics unless the request was accepted.
pub async fn start_run(app: &Router, goal: &str) -> String {
    let reply = post(app, "/runs", goal).await;
    assert_eq!(reply.status, StatusCode::ACCEPTED, "start run: {}", reply.json);
    reply.json["run_id"].as_str().expect("run_id").to_string()
}

/// Wait until no background drive holds the slot. The slot is released when the drive task ends.
pub async fn wait_idle(state: &AppState<FakeProvider>) {
    for _ in 0..1000 {
        match state.claim_drive("probe") {
            Ok(claim) => {
                drop(claim);
                return;
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(10)).await,
        }
    }
    panic!("the background drive did not finish");
}

/// Poll the run until it reaches `status`. Panics after ten seconds.
pub async fn wait_status(app: &Router, run_id: &str, status: &str) {
    for _ in 0..1000 {
        let reply = get(app, &format!("/runs/{run_id}")).await;
        if reply.json["run"]["status"] == status {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("run {run_id} did not reach {status}");
}
