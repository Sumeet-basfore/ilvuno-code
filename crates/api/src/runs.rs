//! Run routes: list, detail, board, start, resume.

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
use serde_json::{json, Value};

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
