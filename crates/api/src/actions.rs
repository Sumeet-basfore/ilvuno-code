//! Run actions: add a task, decide at the gate. Each calls the function the CLI calls.

use crate::blocking::{blocking, open_store, require_run};
use crate::drive::spawn_drive;
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, DriveClaim};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use provider_core::Provider;
use runner::goal;
use runner::run_status::{self, GateDecision};
use runner::store::Outcome;
use runner::tasks::{self, CreatedBy, NewTask};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct TaskBody {
    pub thread: String,
    pub prompt: String,
    #[serde(default)]
    pub scope: Vec<String>,
    /// "task" claims files (default). "message" is a chat turn: no scope, no board claim.
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Deserialize)]
pub struct GateBody {
    pub decision: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// `POST /runs/:id/tasks`: the same write as `ilvuno task` run without an orchestrator caller.
pub async fn create_task<P>(
    State(state): State<AppState<P>>,
    Path(run_id): Path<String>,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<Value>)>
where
    P: Send + Sync + 'static,
{
    let body: TaskBody = parse_body(&body)?;
    let home = state.home.clone();
    let outcome = blocking(move || {
        let mut store = open_store(&home)?;
        require_run(&store, &run_id)?;
        let task = NewTask {
            run_id: &run_id,
            thread: &body.thread,
            prompt: &body.prompt,
            scope: &body.scope,
            created_by: CreatedBy::User,
            author_thread: None,
            kind: body.kind.as_deref().unwrap_or("task"),
        };
        Ok(tasks::submit_task(&home, &mut store, &task)?)
    })
    .await?;
    match outcome {
        Outcome::Applied(task) => Ok((StatusCode::CREATED, Json(json!({ "task_id": task.task_id })))),
        Outcome::Refused(reason) => Err(ApiError::Conflict(reason)),
        Outcome::Duplicate(_) => Err(ApiError::Internal("task write reported a duplicate without a request id".into())),
    }
}

/// `POST /runs/:id/gate`: accept, or send back with a note. A send-back drives the run again in the background.
pub async fn gate<P>(State(state): State<AppState<P>>, Path(run_id): Path<String>, body: Bytes) -> ApiResult<Json<Value>>
where
    P: Provider + Send + Sync + 'static,
{
    let decision = gate_decision(parse_body(&body)?)?;
    // A send-back starts driving, so it needs the slot first. An accept only records a decision.
    let claim: Option<DriveClaim> = match decision {
        GateDecision::SendBack { .. } => Some(state.claim_drive(&format!("run {run_id}"))?),
        GateDecision::Accept => None,
    };
    let send_back = matches!(decision, GateDecision::SendBack { .. });
    let home = state.home.clone();
    let (status, plan) = blocking({
        let run_id = run_id.clone();
        move || {
            let mut store = open_store(&home)?;
            require_run(&store, &run_id)?;
            match run_status::decide_gate_and_render(&home, &mut store, &run_id, &decision)? {
                Outcome::Applied(()) => {}
                Outcome::Refused(reason) => return Err(ApiError::Conflict(reason)),
                Outcome::Duplicate(_) => return Err(ApiError::Internal("gate reported a duplicate without a request id".into())),
            }
            let status = require_run(&store, &run_id)?;
            let plan = if send_back { Some(goal::load_snapshot(store.conn(), &run_id)?) } else { None };
            Ok((status, plan))
        }
    })
    .await?;
    if let (Some(claim), Some(plan)) = (claim, plan) {
        spawn_drive(&state, claim, plan, run_id.clone());
    }
    Ok(Json(json!({ "run_id": run_id, "status": status })))
}

/// Parse a route body. A malformed body is the client's mistake, so it is a 400 with the parser's reason.
pub fn parse_body<T: DeserializeOwned>(body: &[u8]) -> ApiResult<T> {
    serde_json::from_slice(body).map_err(|e| ApiError::BadRequest(format!("invalid request body: {e}")))
}

fn gate_decision(body: GateBody) -> ApiResult<GateDecision> {
    match body.decision.as_str() {
        "accept" => Ok(GateDecision::Accept),
        "send_back" => match body.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
            Some(note) => Ok(GateDecision::SendBack { note }),
            None => Err(ApiError::BadRequest("a send_back needs a non-empty note".into())),
        },
        _ => Err(ApiError::BadRequest("decision must be \"accept\" or \"send_back\"".into())),
    }
}
