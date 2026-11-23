//! Run routes: list, detail, board, start, stop, resume.

use crate::blocking::{blocking, open_store, require_run};
use crate::drive::spawn_drive;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use cli::ops;
use provider_core::Provider;
use runner::goal::{self, GoalPlan};
use runner::{export, ledger, query, Home};
use rusqlite::OptionalExtension;
use serde_json::{json, Value};
use std::path::Path as FsPath;

pub async fn list<P>(State(state): State<AppState<P>>) -> ApiResult<Json<Vec<Value>>>
where
    P: Send + Sync + 'static,
{
    let home = state.home.clone();
    let runs = blocking(move || {
        let store = open_store(&home)?;
        Ok(query::list_runs(store.conn())?)
    })
    .await?;
    Ok(Json(runs))
}

pub async fn detail<P>(State(state): State<AppState<P>>, Path(run_id): Path<String>) -> ApiResult<Json<Value>>
where
    P: Send + Sync + 'static,
{
    let home = state.home.clone();
    let run = blocking(move || {
        let store = open_store(&home)?;
        require_run(&store, &run_id)?;
        Ok(export::run_json(store.conn(), &run_id)?)
    })
    .await?;
    Ok(Json(run))
}

/// `GET /runs/:id/threads/:name/diff`: the bounded patch and full diffstat for one checkout.
pub async fn thread_diff<P>(
    State(state): State<AppState<P>>,
    Path((run_id, thread)): Path<(String, String)>,
) -> ApiResult<Json<Value>>
where
    P: Send + Sync + 'static,
{
    let home = state.home.clone();
    let result = blocking(move || {
        let store = open_store(&home)?;
        require_run(&store, &run_id)?;
        let row: Option<(Option<String>, Option<String>, Option<String>)> = store.conn().query_row(
            "SELECT r.git_base, t.branch, t.worktree_path
             FROM runs r JOIN threads t ON t.run_id = r.id
             WHERE r.id = ?1 AND t.name = ?2",
            rusqlite::params![run_id, thread],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).optional().map_err(anyhow::Error::from)?;
        let Some((base, branch, worktree)) = row else {
            return Err(ApiError::NotFound(format!("no thread named {thread}")));
        };
        let (Some(base), Some(branch), Some(worktree)) = (base, branch, worktree) else {
            return Ok(json!({ "branch": null }));
        };
        let path = FsPath::new(&worktree);
        if !path.join(".git").exists() {
            return Ok(json!({ "branch": null }));
        }
        let stat = runner::git::diff_stat(path, &base)?;
        let (patch, truncated) = runner::git::diff_capped(path, &base, 128 * 1024)?;
        Ok(json!({ "branch": branch, "base": base, "stat": stat, "patch": patch, "truncated": truncated }))
    }).await?;
    Ok(Json(result))
}

pub async fn board<P>(State(state): State<AppState<P>>, Path(run_id): Path<String>) -> ApiResult<Json<Value>>
where
    P: Send + Sync + 'static,
{
    let home = state.home.clone();
    let markdown = blocking(move || {
        let store = open_store(&home)?;
        require_run(&store, &run_id)?;
        let path = home.board_path(&run_id);
        if !path.is_file() {
            // The board is written on every change; a run that has had none still gets one.
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| ApiError::Internal(format!("create {}: {e}", dir.display())))?;
            }
            ledger::render_board(store.conn(), &run_id, &path)?;
        }
        std::fs::read_to_string(&path).map_err(|e| ApiError::Internal(format!("read {}: {e}", path.display())))
    })
    .await?;
    Ok(Json(json!({ "markdown": markdown })))
}

/// `POST /runs`: the body is goal JSON. Writes the run, then drives it in the background.
pub async fn create<P>(State(state): State<AppState<P>>, body: Bytes) -> ApiResult<(StatusCode, Json<Value>)>
where
    P: Provider + Send + Sync + 'static,
{
    let raw = std::str::from_utf8(&body).map_err(|_| ApiError::BadRequest("the goal must be UTF-8 JSON".into()))?.to_string();
    let home = state.home.clone();
    // Validate before claiming the slot, so a bad goal is a 400 even while another run is busy.
    let probe = GoalPlan::from_raw(&raw, home.root()).map_err(|e| ApiError::BadRequest(format!("{e:#}")))?;
    (state.build)(&probe).map_err(|e| ApiError::BadRequest(format!("{e:#}")))?;
    let claim = state.claim_drive("a run being created")?;
    let (run_id, plan) = blocking({
        let home = home.clone();
        move || {
            let mut store = open_store(&home)?;
            let (run_id, plan) = ops::create_run(&mut store, &home, &raw, |id| home.run_dir(id))?;
            write_goal_file(&home, &run_id, &raw)?;
            Ok((run_id, plan))
        }
    })
    .await?;
    claim.set_holder(&run_id);
    spawn_drive(&state, claim, plan, run_id.clone());
    Ok((StatusCode::ACCEPTED, Json(json!({ "run_id": run_id, "status": "working" }))))
}

/// `PUT /runs/:id/threads/:name/agent`: choose the agent that thread's next task runs on.
pub async fn set_thread_agent<P>(
    State(state): State<AppState<P>>,
    Path((run_id, thread)): Path<(String, String)>,
    body: Bytes,
) -> ApiResult<Json<Value>>
where
    P: Provider + Send + Sync + 'static,
{
    let agent = serde_json::from_slice::<AgentBody>(&body)
        .map_err(|e| ApiError::BadRequest(format!("invalid request body: {e}")))?
        .agent;
    let home = state.home.clone();
    let (run, thread_for_call, agent_for_call) = (run_id.clone(), thread.clone(), agent.clone());
    blocking(move || {
        let mut store = open_store(&home)?;
        require_run(&store, &run)?;
        match runner::run_status::set_thread_agent(&mut store, &run, &thread_for_call, &agent_for_call)? {
            runner::Outcome::Applied(()) => Ok(()),
            runner::Outcome::Refused(reason) => Err(ApiError::Conflict(reason)),
            runner::Outcome::Duplicate(_) => Ok(()),
        }
    })
    .await?;
    Ok(Json(json!({ "run_id": run_id, "thread": thread, "agent": agent })))
}

#[derive(serde::Deserialize)]
struct AgentBody {
    agent: String,
}

/// `POST /runs/:id/stop`: the current turn finishes, then nothing else starts. `resume` continues.
pub async fn stop<P>(State(state): State<AppState<P>>, Path(run_id): Path<String>, body: Bytes) -> ApiResult<Json<Value>>
where
    P: Provider + Send + Sync + 'static,
{
    let note = if body.is_empty() {
        None
    } else {
        serde_json::from_slice::<StopBody>(&body)
            .map_err(|e| ApiError::BadRequest(format!("invalid request body: {e}")))?
            .note
    };
    let home = state.home.clone();
    let run = run_id.clone();
    let status = blocking(move || {
        let mut store = open_store(&home)?;
        require_run(&store, &run)?;
        match runner::run_status::stop(&mut store, &run, note.as_deref())? {
            runner::Outcome::Applied(()) => {}
            runner::Outcome::Refused(reason) => return Err(ApiError::Conflict(reason)),
            runner::Outcome::Duplicate(_) => return Err(ApiError::Conflict("this run is already stopped".into())),
        }
        require_run(&store, &run)
    })
    .await?;
    Ok(Json(json!({ "run_id": run_id, "status": status })))
}

#[derive(serde::Deserialize)]
struct StopBody {
    #[serde(default)]
    note: Option<String>,
}

/// `POST /runs/:id/resume`: the same path as `ilvuno run --resume`.
pub async fn resume<P>(State(state): State<AppState<P>>, Path(run_id): Path<String>) -> ApiResult<(StatusCode, Json<Value>)>
where
    P: Provider + Send + Sync + 'static,
{
    let home = state.home.clone();
    let (plan, status) = blocking({
        let run_id = run_id.clone();
        move || {
            let store = open_store(&home)?;
            let status = require_run(&store, &run_id)?;
            let plan = goal::load_snapshot(store.conn(), &run_id)?;
            Ok((plan, status))
        }
    })
    .await?;
    let claim = state.claim_drive(&format!("run {run_id}"))?;
    spawn_drive(&state, claim, plan, run_id.clone());
    Ok((StatusCode::ACCEPTED, Json(json!({ "run_id": run_id, "status": status }))))
}

/// The goal as the user wrote it, next to the run. The run's own snapshot is what the engine reads.
fn write_goal_file(home: &Home, run_id: &str, raw: &str) -> ApiResult<()> {
    let dir = home.run_dir(run_id);
    std::fs::create_dir_all(&dir).map_err(|e| ApiError::Internal(format!("create {}: {e}", dir.display())))?;
    let path = dir.join("goal.json");
    std::fs::write(&path, raw).map_err(|e| ApiError::Internal(format!("write {}: {e}", path.display())))?;
    Ok(())
}
