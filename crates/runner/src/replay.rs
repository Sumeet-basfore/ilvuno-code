//! Replay: rebuild rounds, tasks and verdicts from the journal alone, then compare with the projection
//! tables. If they differ, the journal is not the source of truth and we say so.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundSummary {
    pub index: u32,
    pub round_id: String,
    pub verdict: Option<String>,
    /// (task id, thread name, status) in task order.
    pub dispatches: Vec<(String, String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replay {
    pub transitions: Vec<String>,
    pub rounds: Vec<RoundSummary>,
    pub final_status: Option<String>,
    pub refusals: Vec<String>,
}

pub fn replay(conn: &Connection, run_id: &str) -> Result<Replay> {
    let mut stmt = conn.prepare("SELECT seq, kind, round_id, payload FROM journal WHERE run_id = ?1 ORDER BY seq")?;
    let rows: Vec<(i64, String, Option<String>, String)> = stmt
        .query_map(params![run_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<rusqlite::Result<_>>()?;

    let mut out = Replay { transitions: Vec::new(), rounds: Vec::new(), final_status: None, refusals: Vec::new() };
    for (seq, kind, round_id, payload) in rows {
        let value: Value = serde_json::from_str(&payload)?;
        out.transitions.push(format!("#{seq} {kind} {}", compact(&value)));
        match kind.as_str() {
            "round.planned" => out.rounds.push(RoundSummary {
                index: value["round_index"].as_u64().unwrap_or(0) as u32,
                round_id: round_id.unwrap_or_default(),
                verdict: None,
                dispatches: Vec::new(),
            }),
            "dispatch.queued" => {
                if let Some(round) = round_mut(&mut out, round_id.as_deref()) {
                    let entry = (str_of(&value, "dispatch_id"), str_of(&value, "thread"), "queued".to_string());
                    round.dispatches.push(entry);
                }
            }
            "dispatch.requeued" | "dispatch.started" | "dispatch.reported" | "dispatch.failed" => {
                let status = match kind.as_str() {
                    "dispatch.started" => "running",
                    "dispatch.requeued" => "queued",
                    "dispatch.reported" => "reported",
                    _ => "failed",
                };
                let dispatch_id = str_of(&value, "dispatch_id");
                if let Some(round) = round_mut(&mut out, round_id.as_deref()) {
                    if let Some(entry) = round.dispatches.iter_mut().find(|(id, _, _)| *id == dispatch_id) {
                        entry.2 = status.into();
                    }
                }
            }
            "round.verdict" => {
                if let Some(round) = round_mut(&mut out, round_id.as_deref()) {
                    round.verdict = value["verdict"].as_str().map(str::to_string);
                }
            }
            "run.transition" => out.final_status = value["to"].as_str().map(str::to_string),
            "task.refused" | "dispatch.refused" | "merge.refused" => {
                out.refusals.push(value["reason"].as_str().unwrap_or_default().to_string())
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Mismatches between the replayed journal and the projection tables. Empty means they agree.
pub fn compare_with_projection(conn: &Connection, run_id: &str, replayed: &Replay) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    let mut stmt = conn.prepare("SELECT id, \"index\", verdict FROM rounds WHERE run_id = ?1 ORDER BY \"index\"")?;
    let projected: Vec<(String, i64, Option<String>)> = stmt
        .query_map(params![run_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if projected.len() != replayed.rounds.len() {
        problems.push(format!("rounds: projection {} vs journal {}", projected.len(), replayed.rounds.len()));
    }
    for (round_id, index, verdict) in &projected {
        let Some(from_journal) = replayed.rounds.iter().find(|r| &r.round_id == round_id) else {
            problems.push(format!("round {index} missing from journal"));
            continue;
        };
        if from_journal.verdict != *verdict {
            problems.push(format!("round {index} verdict: projection {verdict:?} vs journal {:?}", from_journal.verdict));
        }
        let mut dstmt = conn.prepare(
            "SELECT d.id, t.name, d.status FROM dispatch d JOIN threads t ON t.id = d.thread_id
             WHERE d.round_id = ?1 ORDER BY d.rowid",
        )?;
        let dispatches: Vec<(String, String, String)> = dstmt
            .query_map(params![round_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        if dispatches != from_journal.dispatches {
            problems.push(format!("round {index} tasks: projection {dispatches:?} vs journal {:?}", from_journal.dispatches));
        }
    }
    let status: String = conn.query_row("SELECT status FROM runs WHERE id = ?1", params![run_id], |r| r.get(0))?;
    if replayed.final_status.as_deref() != Some(status.as_str()) {
        problems.push(format!("run status: projection {status} vs journal {:?}", replayed.final_status));
    }
    Ok(problems)
}

fn round_mut<'a>(out: &'a mut Replay, round_id: Option<&str>) -> Option<&'a mut RoundSummary> {
    let id = round_id?;
    out.rounds.iter_mut().find(|r| r.round_id == id)
}

fn str_of(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().to_string()
}

fn compact(v: &Value) -> String {
    let text = v.to_string();
    if text.chars().count() > 160 {
        format!("{}…", text.chars().take(160).collect::<String>())
    } else {
        text
    }
}
