//! Runs one task on its thread: start it, open a session, pump events through the policy and the journal,
//! nudge once if the agent does not report, then keep the files it reported as artifacts.

use crate::commands;
use crate::execution;
use crate::goal::{GoalPlan, MAX_USD_PER_CALL};
use crate::home::Home;
use crate::policy::{decision_payload, Policy, PolicyKind};
use crate::prompts::{self, OrchestratorPrompt, WorkerPrompt, ORCHESTRATOR_PREAMBLE_VERSION, WORKER_PREAMBLE_VERSION};
use crate::run_status;
use crate::store::{new_id, Outcome, Store};
use anyhow::{anyhow, Context, Result};
use provider_core::{ErrorKind, Event, Provider, ProviderError, Session, SpawnSpec};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

const MAX_TURNS_PER_DISPATCH: u32 = 2;
const NUDGE: &str = "You have not reported yet. Run your `ilvuno report` command now, exactly once.";
const TRUNCATE_TOOL_INPUT: usize = 2000;

pub struct ExecCtx<'a> {
    pub home: &'a Home,
    pub plan: &'a GoalPlan,
    /// Directory prepended to the agent's PATH so `ilvuno` resolves.
    pub ilvuno_bin_dir: Option<&'a Path>,
    pub max_usd: f64,
}

/// What the dispatcher needs to know about one task.
pub struct TaskRow {
    pub round_id: String,
    pub thread_id: String,
    pub thread: String,
    pub agent: String,
    pub is_orchestrator: bool,
    pub status: String,
    pub prompt: String,
    pub scope: Vec<String>,
    pub standing: String,
}

/// Run one open task. The caller has already chosen it and knows its round index.
pub async fn execute<P: Provider>(
    provider: &P,
    store: &mut Store,
    cx: &ExecCtx<'_>,
    run_id: &str,
    round_index: u32,
    task_id: &str,
) -> Result<()> {
    let task = task_row(store.conn(), task_id)?;
    if task.status == "running" {
        store.transact("dispatch.requeue", None, |tx, p| {
            execution::requeue_dispatch(tx, p, run_id, task_id)?;
            Ok(Outcome::Applied(()))
        })?;
    }
    crate::ledger::render_board(store.conn(), run_id, &cx.home.board_path(run_id))?;

    let start = match store.transact("dispatch.start", None, |tx, p| commands::start_dispatch(tx, p, run_id, task_id))? {
        Outcome::Applied(start) => start,
        Outcome::Refused(reason) => {
            fail(store, run_id, task_id, &reason)?;
            return Ok(());
        }
        Outcome::Duplicate(_) => return Ok(()),
    };

    let session_id = new_id();
    let thread_dir = start.thread_dir.clone();
    std::fs::create_dir_all(&thread_dir).with_context(|| format!("create {}", thread_dir.display()))?;
    let (thread_id, bind) = (start.thread_id.clone(), session_id.clone());
    store.transact("thread.bind", None, |tx, p| {
        execution::bind_session(tx, p, run_id, &thread_id, &bind)?;
        Ok(Outcome::Applied(()))
    })?;

    let bin = cx
        .ilvuno_bin_dir
        .map(|d| d.join("ilvuno").to_string_lossy().to_string())
        .unwrap_or_else(|| "ilvuno".to_string());
    let mut env = child_env(cx.home, cx.ilvuno_bin_dir);
    if task.is_orchestrator {
        // The orchestrator's `ilvuno task` calls name this task, which is how the runner knows who writes.
        env.push(("ILVUNO_TASK".to_string(), task_id.to_string()));
    }
    let spec = SpawnSpec {
        cwd: thread_dir.clone(),
        env,
        max_budget_usd: cx.max_usd.min(MAX_USD_PER_CALL),
        model: cx.plan.provider_spec(&task.agent).and_then(|s| s.model.clone()),
        session_id,
        resume: false,
    };
    let mut session = match provider.open(spec).await {
        Ok(session) => session,
        Err(e) => {
            fail(store, run_id, task_id, &format!("{:?}: {}", e.kind, e.message))?;
            return Ok(());
        }
    };

    let kind = if task.is_orchestrator { PolicyKind::Orchestrator } else { PolicyKind::Worker };
    let policy = Policy {
        kind,
        thread_dir: thread_dir.clone(),
        read_roots: read_roots(cx.home, run_id, &task.scope),
        task_id: task_id.to_string(),
        ilvuno_bin: bin.clone(),
    };
    let version = if task.is_orchestrator { ORCHESTRATOR_PREAMBLE_VERSION } else { WORKER_PREAMBLE_VERSION };
    let target = Target { run_id, task: &task, task_id, round_index, thread_dir: &thread_dir, bin: &bin };
    let prompt = render_prompt(store.conn(), cx, &target)?;
    store.transact("dispatch.prompt", None, |_tx, p| {
        p.push(
            run_id,
            Some(&task.round_id),
            Some(&task.thread_id),
            "prompt.injected",
            json!({ "dispatch_id": task_id, "version": version, "text": prompt }),
        );
        Ok(Outcome::Applied(()))
    })?;

    let mut turn_text = prompt;
    for _ in 0..MAX_TURNS_PER_DISPATCH {
        if let Err(e) = session.send_turn(&turn_text).await {
            fail(store, run_id, task_id, &describe(&e))?;
            break;
        }
        let turn = pump_turn(&mut session, store, &policy, run_id, &task.round_id, &task.thread_id).await?;
        if status_of(store.conn(), task_id)? != "running" {
            break;
        }
        if let TurnEnd::Died(reason) = turn {
            fail(store, run_id, task_id, &format!("turn_failed: {reason}"))?;
            break;
        }
        turn_text = NUDGE.to_string();
    }
    session.shutdown().await;

    if status_of(store.conn(), task_id)? == "running" {
        fail(store, run_id, task_id, "no_report: the thread did not report after two turns")?;
    }
    if status_of(store.conn(), task_id)? == "reported" {
        keep_reported_files(store, cx, run_id, round_index, &task, task_id, &thread_dir)?;
    }
    Ok(())
}

fn task_row(conn: &Connection, task_id: &str) -> Result<TaskRow> {
    let (round_id, thread_id, thread, agent, role, status, prompt, scope, standing): (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT d.round_id, d.thread_id, t.name, t.provider, t.role, d.status, d.task, d.scope, t.prompt
             FROM dispatch d JOIN threads t ON t.id = d.thread_id WHERE d.id = ?1",
            params![task_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
        )
        .map_err(|e| anyhow!("task {task_id}: {e}"))?;
    Ok(TaskRow {
        round_id,
        thread_id,
        thread,
        agent,
        is_orchestrator: role == "orchestrator",
        status,
        prompt,
        scope: serde_json::from_str(&scope)?,
        standing,
    })
}

/// The text a task's thread receives under "=== TASK ===": its standing job, the task, and the scope.
pub fn task_text(standing: &str, scope: &[String], prompt: &str) -> String {
    let mut text = String::new();
    if !standing.trim().is_empty() {
        text.push_str(&format!("Standing job for your thread:\n{}\n\n", standing.trim()));
    }
    text.push_str(&format!(
        "Task:\n{prompt}\n\nScope (the only files and sources this task may touch): {}",
        scope.join(", ")
    ));
    text
}

/// Where a task runs and what it is told about itself.
struct Target<'a> {
    run_id: &'a str,
    task: &'a TaskRow,
    task_id: &'a str,
    round_index: u32,
    thread_dir: &'a Path,
    bin: &'a str,
}

/// The first prompt of a task: the preamble for its kind of thread, with the task text filled in.
fn render_prompt(conn: &Connection, cx: &ExecCtx<'_>, target: &Target<'_>) -> Result<String> {
    let task = target.task;
    let text = task_text(&task.standing, &task.scope, &task.prompt);
    let folder = target.thread_dir.to_string_lossy().to_string();
    let run = cx.home.run_dir(target.run_id).to_string_lossy().to_string();
    if task.is_orchestrator {
        let workers = worker_names(conn, target.run_id)?;
        let workers = if workers.is_empty() { "none".to_string() } else { workers.join(", ") };
        let goal_folder = cx.plan.base().to_string_lossy().to_string();
        return Ok(prompts::render_orchestrator(&OrchestratorPrompt {
            thread: &task.thread,
            task_id: target.task_id,
            round_index: target.round_index,
            folder: &folder,
            run: &run,
            goal_folder: &goal_folder,
            workers: &workers,
            bin: target.bin,
            task: &text,
        }));
    }
    let coordinator = match commands::orchestrator_name(conn, target.run_id)? {
        Some(name) => format!("the orchestrator thread {name}"),
        None => "the user".to_string(),
    };
    Ok(prompts::render_worker(&WorkerPrompt {
        thread: &task.thread,
        coordinator: &coordinator,
        task_id: target.task_id,
        round_index: target.round_index,
        folder: &folder,
        run: &run,
        message_policy: &cx.plan.file.message_policy,
        bin: target.bin,
        task: &text,
    }))
}

fn worker_names(conn: &Connection, run_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM threads WHERE run_id = ?1 AND role = 'worker' ORDER BY rowid")?;
    let names = stmt.query_map(params![run_id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    Ok(names)
}

enum TurnEnd {
    Finished,
    Died(String),
}

async fn pump_turn<S: Session>(
    session: &mut S,
    store: &mut Store,
    policy: &Policy,
    run_id: &str,
    round_id: &str,
    thread_id: &str,
) -> Result<TurnEnd> {
    loop {
        let event = match session.next_event().await {
            Ok(Some(event)) => event,
            Ok(None) => return Ok(TurnEnd::Died("stream closed".into())),
            Err(e) => return Ok(TurnEnd::Died(describe(&e))),
        };
        match event {
            Event::PermissionRequest { request_id, tool, input, .. } => {
                let decision = policy.decide(&tool, &input);
                let payload = decision_payload(&tool, &decision);
                let (rid, tid) = (round_id.to_string(), thread_id.to_string());
                store.transact("permission.record", None, |_tx, p| {
                    execution::record_permission(p, run_id, &rid, &tid, payload);
                    Ok(Outcome::Applied(()))
                })?;
                if let Err(e) = session.reply_permission(&request_id, decision).await {
                    return Ok(TurnEnd::Died(describe(&e)));
                }
            }
            Event::ToolCall { name, input, .. } => {
                let payload = json!({ "tool": name, "input": truncate_value(&input, TRUNCATE_TOOL_INPUT) });
                let (rid, tid) = (round_id.to_string(), thread_id.to_string());
                store.transact("tool.record", None, |_tx, p| {
                    execution::record_tool_call(p, run_id, &rid, &tid, payload);
                    Ok(Outcome::Applied(()))
                })?;
            }
            Event::Usage { tokens_in, tokens_out, cost_usd, .. } => {
                let (rid, tid) = (round_id.to_string(), thread_id.to_string());
                store.transact("usage.record", None, |tx, p| {
                    run_status::record_usage(tx, p, run_id, &rid, &tid, tokens_in + tokens_out, cost_usd)?;
                    Ok(Outcome::Applied(()))
                })?;
            }
            Event::TurnFinished { error, .. } => {
                return Ok(match error {
                    Some(e) if !e.is_empty() => TurnEnd::Died(e),
                    _ => TurnEnd::Finished,
                });
            }
            Event::SessionEnded { reason } => return Ok(TurnEnd::Died(reason)),
            _ => {}
        }
    }
}

fn fail(store: &mut Store, run_id: &str, task_id: &str, reason: &str) -> Result<()> {
    store.transact("dispatch.fail", None, |tx, p| execution::fail_dispatch(tx, p, run_id, task_id, reason))?;
    Ok(())
}

/// Keep what a task reported. Each file goes to the round's snapshot folder for that thread, and the newest
/// copy goes to the run's artifacts folder, where the checks look. Paths that leave the thread's folder
/// are never copied.
fn keep_reported_files(
    store: &mut Store,
    cx: &ExecCtx<'_>,
    run_id: &str,
    round_index: u32,
    task: &TaskRow,
    task_id: &str,
    thread_dir: &Path,
) -> Result<()> {
    let result: String = store.conn().query_row(
        "SELECT result_json FROM dispatch WHERE id = ?1",
        params![task_id],
        |r| r.get(0),
    )?;
    let files: Vec<String> = serde_json::from_str::<Value>(&result)?
        .get("files")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    for reported in files {
        let Some(file) = folder_relative(&reported, thread_dir) else {
            continue;
        };
        let Ok(bytes) = std::fs::read(thread_dir.join(&file)) else {
            store.transact("artifact.missing", None, |_tx, p| {
                p.push(run_id, Some(&task.round_id), Some(&task.thread_id), "artifact.missing", json!({ "file": file }));
                Ok(Outcome::Applied(()))
            })?;
            continue;
        };
        let snapshot = snapshot_dir(cx.home, run_id, round_index, &task.thread).join(&file);
        write_file(&snapshot, &bytes)?;
        write_file(&cx.home.artifacts_dir(run_id).join(&file), &bytes)?;
        let sha = hex(&Sha256::digest(&bytes));
        let kind = if file.ends_with(".html") { "report" } else { "note" };
        let path = snapshot.to_string_lossy().to_string();
        let (rid, tid) = (task.round_id.clone(), task.thread_id.clone());
        store.transact("artifact.register", None, |tx, p| {
            execution::register_artifact(tx, p, run_id, &rid, &tid, kind, &path, bytes.len() as u64, &sha)?;
            Ok(Outcome::Applied(()))
        })?;
    }
    Ok(())
}

/// The path of a reported file relative to the thread's folder. An absolute path counts only when it points
/// inside that folder; anything else (a source elsewhere, `..`, an empty name) is not kept.
fn folder_relative(file: &str, thread_dir: &Path) -> Option<String> {
    let path = Path::new(file.trim());
    let relative = if path.is_absolute() { path.strip_prefix(thread_dir).ok()? } else { path };
    let plain = !relative.as_os_str().is_empty() && relative.components().all(|c| matches!(c, Component::Normal(_)));
    plain.then(|| relative.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reported_paths_are_kept_only_when_they_name_a_file_in_the_thread_folder() {
        let dir = Path::new("/runs/r1/threads/reader-1");
        assert_eq!(folder_relative("brief.html", dir).as_deref(), Some("brief.html"));
        assert_eq!(folder_relative("/runs/r1/threads/reader-1/brief.html", dir).as_deref(), Some("brief.html"));
        assert_eq!(folder_relative("notes/a.md", dir).as_deref(), Some("notes/a.md"));
        assert_eq!(folder_relative("/repo/docs/security.md", dir), None);
        assert_eq!(folder_relative("../reader-2/x.md", dir), None);
        assert_eq!(folder_relative("/runs/r1/threads/reader-1/../reader-2/x.md", dir), None);
        assert_eq!(folder_relative("  ", dir), None);
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(path, bytes).with_context(|| format!("write {}", path.display()))
}

/// What a task's thread may read beyond its own folder: the run folder, and the task's absolute scope
/// entries (sources handed to it by name).
fn read_roots(home: &Home, run_id: &str, scope: &[String]) -> Vec<PathBuf> {
    let mut roots = vec![home.run_dir(run_id)];
    roots.extend(scope.iter().map(PathBuf::from).filter(|p| p.is_absolute()));
    roots
}

/// Where a round's reported files are snapshotted: `artifacts/rounds/r<index>/<thread>/`.
pub fn snapshot_dir(home: &Home, run_id: &str, round_index: u32, thread: &str) -> PathBuf {
    home.artifacts_dir(run_id).join("rounds").join(format!("r{round_index}")).join(thread)
}

pub fn status_of(conn: &Connection, task_id: &str) -> Result<String> {
    Ok(conn.query_row("SELECT status FROM dispatch WHERE id = ?1", params![task_id], |r| r.get(0))?)
}

fn child_env(home: &Home, bin_dir: Option<&Path>) -> Vec<(String, String)> {
    let inherited = std::env::var("PATH").unwrap_or_default();
    let path = match bin_dir {
        Some(dir) => format!("{}:{inherited}", dir.display()),
        None => inherited,
    };
    let mut env = vec![
        ("PATH".to_string(), path),
        ("ILVUNO_HOME".to_string(), home.root().to_string_lossy().to_string()),
    ];
    // Only what the CLI needs to run and find its own login. No API keys are added by us.
    for key in ["HOME", "USER", "LANG", "LC_ALL", "TERM", "TMPDIR"] {
        if let Ok(value) = std::env::var(key) {
            env.push((key.to_string(), value));
        }
    }
    env
}

fn describe(e: &ProviderError) -> String {
    match e.kind {
        ErrorKind::AuthRequired => format!("auth_required: {}", e.message),
        ErrorKind::UsageLimit => format!("usage_limit: {}", e.message),
        _ => format!("{:?}: {}", e.kind, e.message),
    }
}

fn truncate_value(v: &Value, limit: usize) -> Value {
    let text = v.to_string();
    if text.len() <= limit {
        v.clone()
    } else {
        Value::String(format!("{}…", text.chars().take(limit).collect::<String>()))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
