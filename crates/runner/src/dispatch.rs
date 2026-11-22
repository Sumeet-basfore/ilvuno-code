//! Executes one dispatch: start it, open a worker session, pump events through the policy and the
//! journal, nudge once if the worker does not report, then copy the reported files as artifacts.

use crate::commands::{self, DispatchStart};
use crate::execution;
use crate::goal::{GoalPlan, MAX_USD_PER_CALL};
use crate::home::Home;
use crate::policy::{decision_payload, WorkerPolicy};
use crate::prompts::{render_worker_prompt, WorkerPrompt, WORKER_PREAMBLE_VERSION};
use crate::research::{self, Handoff, BRIEF_FILE, SYNTH_SLOT};
use crate::run_status;
use crate::store::{new_id, Outcome, Store};
use anyhow::{Context, Result};
use provider_core::{ErrorKind, Event, Provider, ProviderError, Session, SpawnSpec};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const HANDOFF_CHAR_LIMIT: usize = 20_000;
const MAX_TURNS_PER_DISPATCH: u32 = 2;
const NUDGE: &str = "You have not reported yet. Run your `ilvuno report` command now, exactly once.";
const PROVIDER_NAME: &str = "claude-main";

pub struct ExecCtx<'a> {
    pub home: &'a Home,
    pub plan: &'a GoalPlan,
    /// Directory prepended to the worker's PATH so `ilvuno report` resolves.
    pub ilvuno_bin_dir: Option<&'a Path>,
    pub max_usd: f64,
}

pub fn provider_name() -> &'static str {
    PROVIDER_NAME
}

/// The dispatches of a round still waiting for an outcome, in plan order.
pub fn open_dispatches(conn: &Connection, round_id: &str) -> Result<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, slot, status FROM dispatch WHERE round_id = ?1 AND status IN ('queued', 'running') ORDER BY rowid",
    )?;
    let rows = stmt.query_map(params![round_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub async fn execute<P: Provider>(
    provider: &P,
    store: &mut Store,
    cx: &ExecCtx<'_>,
    run_id: &str,
    round_id: &str,
    round_index: u32,
    dispatch_id: &str,
) -> Result<()> {
    let slot = slot_of(store.conn(), dispatch_id)?;
    if status_of(store.conn(), dispatch_id)? == "running" {
        store.transact("dispatch.requeue", None, |tx, p| {
            execution::requeue_dispatch(tx, p, run_id, dispatch_id)?;
            Ok(Outcome::Applied(()))
        })?;
    }
    let task = task_text(store, cx, run_id, round_index, &slot)?;
    crate::ledger::render_board(store.conn(), run_id, &cx.home.board_path(run_id))?;

    let start = store.transact("dispatch.start", None, |tx, p| {
        commands::start_dispatch(tx, p, run_id, dispatch_id, &task)
    })?;
    let start: DispatchStart = match start {
        Outcome::Applied(s) => s,
        Outcome::Refused(reason) => {
            fail(store, run_id, dispatch_id, &reason)?;
            return Ok(());
        }
        Outcome::Duplicate(_) => return Ok(()),
    };

    let session_id = new_id();
    let thread_dir = start.thread_dir.clone();
    std::fs::create_dir_all(&thread_dir).with_context(|| format!("create {}", thread_dir.display()))?;
    let thread_id = start.thread_id.clone();
    let bind = session_id.clone();
    store.transact("thread.bind", None, |tx, p| {
        execution::bind_session(tx, p, run_id, &thread_id, &bind)?;
        Ok(Outcome::Applied(()))
    })?;

    let spec = SpawnSpec {
        cwd: thread_dir.clone(),
        env: child_env(cx.home, cx.ilvuno_bin_dir),
        max_budget_usd: cx.max_usd.min(MAX_USD_PER_CALL),
        model: None,
        session_id,
        resume: false,
    };
    let mut session = match provider.open(spec).await {
        Ok(s) => s,
        Err(e) => {
            fail(store, run_id, dispatch_id, &format!("{:?}: {}", e.kind, e.message))?;
            return Ok(());
        }
    };

    let policy = WorkerPolicy {
        thread_dir: thread_dir.clone(),
        read_sources: vec![cx.plan.source_abs.clone(), cx.home.board_path(run_id)],
        task_id: dispatch_id.to_string(),
        ilvuno_bin: "ilvuno".to_string(),
    };
    let prompt = render_worker_prompt(&WorkerPrompt {
        coordinator: "runner",
        task_id: dispatch_id,
        round_index,
        branch: "none (research run, no branch)",
        worktree: &thread_dir.to_string_lossy(),
        run_board: &cx.home.run_dir(run_id).to_string_lossy(),
        message_policy: &cx.plan.file.message_policy,
        task: &task,
    });
    store.transact("dispatch.prompt", None, |tx, p| {
        let _ = tx;
        p.push(
            run_id,
            Some(round_id),
            Some(&start.thread_id),
            "prompt.injected",
            json!({ "dispatch_id": dispatch_id, "version": WORKER_PREAMBLE_VERSION }),
        );
        Ok(Outcome::Applied(()))
    })?;

    let mut text = prompt;
    for _ in 0..MAX_TURNS_PER_DISPATCH {
        if let Err(e) = session.send_turn(&text).await {
            fail(store, run_id, dispatch_id, &describe(&e))?;
            break;
        }
        let turn = pump_turn(&mut session, store, &policy, run_id, &start, round_id).await?;
        if status_of(store.conn(), dispatch_id)? != "running" {
            break;
        }
        if let TurnEnd::Died(reason) = turn {
            fail(store, run_id, dispatch_id, &format!("turn_failed: {reason}"))?;
            break;
        }
        text = NUDGE.to_string();
    }
    session.shutdown().await;

    if status_of(store.conn(), dispatch_id)? == "running" {
        fail(store, run_id, dispatch_id, "no_report: worker did not report after two turns")?;
    }
    if status_of(store.conn(), dispatch_id)? == "reported" {
        copy_reported_files(store, cx, run_id, round_id, round_index, &start, dispatch_id)?;
    }
    Ok(())
}

enum TurnEnd {
    Finished,
    Died(String),
}

async fn pump_turn<S: Session>(
    session: &mut S,
    store: &mut Store,
    policy: &WorkerPolicy,
    run_id: &str,
    start: &DispatchStart,
    round_id: &str,
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
                let (rid, tid) = (round_id.to_string(), start.thread_id.clone());
                store.transact("permission.record", None, |_tx, p| {
                    execution::record_permission(p, run_id, &rid, &tid, payload);
                    Ok(Outcome::Applied(()))
                })?;
                if let Err(e) = session.reply_permission(&request_id, decision).await {
                    return Ok(TurnEnd::Died(describe(&e)));
                }
            }
            Event::ToolCall { name, input, .. } => {
                let payload = json!({ "tool": name, "input": truncate_value(&input, 2000) });
                let (rid, tid) = (round_id.to_string(), start.thread_id.clone());
                store.transact("tool.record", None, |_tx, p| {
                    execution::record_tool_call(p, run_id, &rid, &tid, payload);
                    Ok(Outcome::Applied(()))
                })?;
            }
            Event::Usage { tokens_in, tokens_out, cost_usd, .. } => {
                let (rid, tid) = (round_id.to_string(), start.thread_id.clone());
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

fn fail(store: &mut Store, run_id: &str, dispatch_id: &str, reason: &str) -> Result<()> {
    store.transact("dispatch.fail", None, |tx, p| execution::fail_dispatch(tx, p, run_id, dispatch_id, reason))?;
    Ok(())
}

fn copy_reported_files(
    store: &mut Store,
    cx: &ExecCtx<'_>,
    run_id: &str,
    round_id: &str,
    round_index: u32,
    start: &DispatchStart,
    dispatch_id: &str,
) -> Result<()> {
    let result: String = store.conn().query_row(
        "SELECT result_json FROM dispatch WHERE id = ?1",
        params![dispatch_id],
        |r| r.get(0),
    )?;
    let files: Vec<String> = serde_json::from_str::<Value>(&result)?
        .get("files")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    // Snapshots are keyed by round and slot, so two workers writing the same file name cannot collide.
    let slot = slot_of(store.conn(), dispatch_id)?;
    let snapshot_dir = snapshot_dir(cx.home, run_id, round_index, &slot);
    std::fs::create_dir_all(&snapshot_dir)?;
    for file in files {
        let src = start.thread_dir.join(&file);
        let Ok(bytes) = std::fs::read(&src) else { continue };
        let snapshot = snapshot_dir.join(&file);
        std::fs::write(&snapshot, &bytes)?;
        if file == BRIEF_FILE {
            std::fs::write(cx.home.artifacts_dir(run_id).join(&file), &bytes)?;
        }
        let sha = hex(&Sha256::digest(&bytes));
        let kind = if file.ends_with(".html") { "report" } else { "note" };
        let path = snapshot.to_string_lossy().to_string();
        let (rid, tid) = (round_id.to_string(), start.thread_id.clone());
        store.transact("artifact.register", None, |tx, p| {
            execution::register_artifact(tx, p, run_id, &rid, &tid, kind, &path, bytes.len() as u64, &sha)?;
            Ok(Outcome::Applied(()))
        })?;
    }
    Ok(())
}

/// The task text the worker receives. For the synthesizer this also records the handoff: what was
/// included, and what was dropped, so the ledger shows any context that did not make it.
fn task_text(store: &mut Store, cx: &ExecCtx<'_>, run_id: &str, round_index: u32, slot: &str) -> Result<String> {
    if slot == SYNTH_SLOT {
        let (handoffs, dropped) = collect_handoffs(store.conn(), run_id)?;
        let included: Vec<String> = handoffs.iter().map(|h| format!("{}:{}", h.slot, h.file)).collect();
        let notes = crate::driver::revision_notes(store.conn(), run_id, round_index)?;
        let round_id: String = store.conn().query_row(
            "SELECT id FROM rounds WHERE run_id = ?1 AND \"index\" = ?2",
            params![run_id, round_index],
            |r| r.get(0),
        )?;
        let body = format!("handoff to {SYNTH_SLOT}: {} included, {} dropped", included.len(), dropped.len());
        let evidence = json!({ "included": included, "dropped": dropped });
        store.transact("handoff.record", None, |tx, p| {
            crate::ledger::append(tx, p, run_id, Some(&round_id), None, "decision", &[], &body, evidence)?;
            Ok(Outcome::Applied(()))
        })?;
        return Ok(research::synth_task(cx.plan, &handoffs, &notes));
    }
    let group = cx
        .plan
        .file
        .groups
        .iter()
        .find(|g| g.slot == slot)
        .with_context(|| format!("no researcher group for slot {slot}"))?;
    Ok(research::researcher_task(cx.plan, &group.sections))
}

/// Read every finding file of the run. Unreadable files are dropped, not silently skipped.
fn collect_handoffs(conn: &Connection, run_id: &str) -> Result<(Vec<Handoff>, Vec<String>)> {
    let mut stmt = conn.prepare(
        "SELECT d.slot, t.worktree_path, l.scope FROM ledger l
         JOIN dispatch d ON d.thread_id = l.thread_id
         JOIN threads t ON t.id = l.thread_id
         WHERE l.run_id = ?1 AND l.kind = 'finding' ORDER BY l.id",
    )?;
    let rows: Vec<(String, String, String)> = stmt
        .query_map(params![run_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut handoffs = Vec::new();
    let mut dropped = Vec::new();
    for (slot, dir, scope) in rows {
        let files: Vec<String> = serde_json::from_str(&scope)?;
        for file in files {
            let path: PathBuf = Path::new(&dir).join(&file);
            match std::fs::read_to_string(&path) {
                Ok(text) if text.len() <= HANDOFF_CHAR_LIMIT => handoffs.push(Handoff { slot: slot.clone(), file, content: text }),
                Ok(_) => dropped.push(format!("{slot}:{file} (over {HANDOFF_CHAR_LIMIT} chars)")),
                Err(e) => dropped.push(format!("{slot}:{file} ({e})")),
            }
        }
    }
    Ok((handoffs, dropped))
}

/// Where a round's reported files are snapshotted: `artifacts/rounds/r<index>/<slot>/`.
pub fn snapshot_dir(home: &Home, run_id: &str, round_index: u32, slot: &str) -> PathBuf {
    home.artifacts_dir(run_id).join("rounds").join(format!("r{round_index}")).join(slot)
}

pub fn slot_of(conn: &Connection, dispatch_id: &str) -> Result<String> {
    Ok(conn.query_row("SELECT slot FROM dispatch WHERE id = ?1", params![dispatch_id], |r| r.get(0))?)
}

pub fn status_of(conn: &Connection, dispatch_id: &str) -> Result<String> {
    Ok(conn.query_row("SELECT status FROM dispatch WHERE id = ?1", params![dispatch_id], |r| r.get(0))?)
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
