//! Goal files: the user's statement of a run. Validated at load (system boundary), then turned into the
//! `goals` row and the research plan.

use crate::commands::NewGoal;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The most any single agent call may spend (AGENTS.md non-negotiable).
pub const MAX_USD_PER_CALL: f64 = 0.20;

#[derive(Debug, Deserialize)]
pub struct GoalFile {
    pub title: String,
    pub goal_text: String,
    pub template: String,
    /// Source document, relative to the goal file's directory.
    pub source: String,
    #[serde(default)]
    pub deliverable: Value,
    pub groups: Vec<Group>,
    pub required_sections: Vec<u32>,
    pub checks: Vec<String>,
    #[serde(default = "default_policy")]
    pub message_policy: String,
    pub budget: Budget,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Group {
    pub slot: String,
    pub sections: Vec<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Budget {
    pub rounds: u32,
    pub tokens: u64,
    #[serde(default = "default_usd")]
    pub max_usd_per_call: f64,
}

fn default_policy() -> String {
    "round".to_string()
}

fn default_usd() -> f64 {
    MAX_USD_PER_CALL
}

/// A validated goal plus the source text the checker needs.
#[derive(Debug)]
pub struct GoalPlan {
    pub file: GoalFile,
    pub source_abs: PathBuf,
    /// File name used in `data-src` citations (e.g. `09-spike.md`).
    pub source_name: String,
    pub source_text: String,
}

impl GoalPlan {
    pub fn load(goal_path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(goal_path).with_context(|| format!("read {}", goal_path.display()))?;
        let base = goal_path.parent().unwrap_or_else(|| Path::new("."));
        Self::from_raw(&raw, base)
    }

    /// Validate goal JSON. `base` is the directory relative paths in the goal resolve against.
    pub fn from_raw(raw: &str, base: &Path) -> Result<Self> {
        let file: GoalFile = serde_json::from_str(raw).context("parse goal")?;
        if file.template != "research" {
            bail!("only the research template exists in v0 (got {:?})", file.template);
        }
        if file.budget.max_usd_per_call <= 0.0 || file.budget.max_usd_per_call > MAX_USD_PER_CALL {
            bail!("budget.max_usd_per_call must be in (0, {MAX_USD_PER_CALL}]");
        }
        if file.budget.rounds == 0 {
            bail!("budget.rounds must be at least 1");
        }
        let source_abs = base.join(&file.source);
        let source_abs = std::fs::canonicalize(&source_abs)
            .with_context(|| format!("source {} is not readable", source_abs.display()))?;
        let source_text = std::fs::read_to_string(&source_abs)
            .with_context(|| format!("source {} is not readable", source_abs.display()))?;
        let headings = source_sections(&source_text);
        if let Some(missing) = file.required_sections.iter().find(|n| !headings.contains(n)) {
            bail!("required section {missing} does not exist in {}", source_abs.display());
        }
        let covered: BTreeSet<u32> = file.groups.iter().flat_map(|g| g.sections.iter().copied()).collect();
        if let Some(uncovered) = file.required_sections.iter().find(|n| !covered.contains(n)) {
            bail!("required section {uncovered} is not assigned to any researcher group");
        }
        if file.groups.is_empty() {
            bail!("goal has no researcher groups");
        }
        let source_name = source_abs
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        Ok(Self { file, source_abs, source_name, source_text })
    }

    pub fn new_goal(&self) -> NewGoal {
        NewGoal {
            title: self.file.title.clone(),
            goal_text: self.file.goal_text.clone(),
            template: self.file.template.clone(),
            deliverable: self.file.deliverable.clone(),
            done_spec: self.done_spec(),
            budget_rounds: self.file.budget.rounds,
            budget_tokens: self.file.budget.tokens,
            budget_usd: Some(self.file.budget.max_usd_per_call),
        }
    }

    pub fn done_spec(&self) -> Value {
        json!({
            "type": "research",
            "checks": self.file.checks,
            "human_gate": "accept the report",
            "max_rounds": self.file.budget.rounds,
            "required_sections": self.file.required_sections,
        })
    }

    pub fn usd_per_call(&self) -> f64 {
        self.file.budget.max_usd_per_call
    }
}

/// Numbers from `## N. ...` headings in the source.
pub fn source_sections(text: &str) -> BTreeSet<u32> {
    text.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("## ")?;
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            let after = &rest[digits.len()..];
            if digits.is_empty() || !after.starts_with('.') {
                return None;
            }
            digits.parse().ok()
        })
        .collect()
}

const SNAPSHOT_PREFIX: &str = "goal_snapshot:";

/// Freeze the exact goal a run started with, so `--resume` never reads a file that changed since.
pub fn snapshot(tx: &rusqlite::Transaction<'_>, run_id: &str, raw: &str, base: &Path) -> Result<()> {
    let value = json!({ "raw": raw, "base": base.to_string_lossy() }).to_string();
    tx.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![format!("{SNAPSHOT_PREFIX}{run_id}"), value],
    )?;
    Ok(())
}

pub fn load_snapshot(conn: &rusqlite::Connection, run_id: &str) -> Result<GoalPlan> {
    let value: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![format!("{SNAPSHOT_PREFIX}{run_id}")],
            |r| r.get(0),
        )
        .with_context(|| format!("no goal snapshot for run {run_id}"))?;
    let v: Value = serde_json::from_str(&value)?;
    let raw = v["raw"].as_str().context("snapshot raw")?;
    let base = v["base"].as_str().context("snapshot base")?;
    GoalPlan::from_raw(raw, Path::new(base))
}
