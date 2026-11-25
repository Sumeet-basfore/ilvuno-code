//! Goal files: the user's statement of a run. Validated at load (a system boundary), then stored as the
//! `goals` and `threads` rows and a snapshot, so `--resume` never reads a file that changed since.

use crate::commands::{NewGoal, NewThread};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

/// The most any single agent call may spend (AGENTS.md non-negotiable).
pub const MAX_USD_PER_CALL: f64 = 0.20;

/// Driver names this build can run.
pub const DRIVERS: &[&str] = &["claude", "acp"];

/// The reserved thread name of the planning thread. It is the only thread that may create tasks from a
/// task. The runner gives it the orchestrator policy; nothing else about it is special.
pub const ORCHESTRATOR: &str = "orchestrator";

pub const MESSAGE_POLICIES: &[&str] = &["anytime", "round", "never"];

const MAX_NAME_LEN: usize = 40;
const TITLE_LEN: usize = 80;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalFile {
    /// Optional project folder for relative paths. The desktop app sends it over HTTP, where there is
    /// no goal file on disk to take a parent from.
    #[serde(default)]
    pub base: Option<String>,
    pub goal: String,
    #[serde(default)]
    pub done: Option<String>,
    #[serde(default)]
    pub checks: Vec<Check>,
    pub providers: BTreeMap<String, ProviderSpec>,
    pub threads: Vec<ThreadSpec>,
    #[serde(default = "default_policy")]
    pub message_policy: String,
    pub budget: Budget,
}

/// A machine check. An unknown `type` is refused when the goal loads, never skipped.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Check {
    FileExists { path: String },
    CommandExitZero { command: String },
    TextAbsent { path: String, needles: Vec<String> },
}

impl Check {
    /// The name used in the readiness report and in the no-progress rule.
    pub fn label(&self) -> String {
        match self {
            Check::FileExists { path } => format!("file_exists {path}"),
            Check::CommandExitZero { command } => format!("command_exit_zero {command}"),
            Check::TextAbsent { path, .. } => format!("text_absent {path}"),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ThreadSpec {
    pub name: String,
    /// Key of an entry in `providers`: the agent this thread runs on.
    pub agent: String,
    /// The thread's standing job. Sent with every task on this thread.
    #[serde(default)]
    pub prompt: String,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProviderSpec {
    pub driver: String,
    /// Binary (claude) or adapter command (acp). Required for acp.
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    /// Adapter's initial mode. For codex-acp this sets INITIAL_AGENT_MODE; `agent` (its default)
    /// auto-approves risky actions, so approvals only reach us in `workspace-write` or `read-only`.
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// A validated goal plus the folder it was written in. Relative command paths resolve from that folder.
#[derive(Debug)]
pub struct GoalPlan {
    pub file: GoalFile,
    pub base: PathBuf,
}

impl GoalPlan {
    pub fn load(goal_path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(goal_path).with_context(|| format!("read {}", goal_path.display()))?;
        let parent = goal_path.parent().unwrap_or_else(|| Path::new("."));
        // Absolute, so every path handed to an agent or a check is the same wherever ilvuno was started.
        let base = std::fs::canonicalize(parent).with_context(|| format!("goal folder {}", parent.display()))?;
        Self::from_raw(&raw, &base)
    }

    /// Validate goal JSON. `base` is the folder the goal file sits in; a `base` key in the JSON wins,
    /// because a goal posted over HTTP has no file to take a parent from.
    pub fn from_raw(raw: &str, base: &Path) -> Result<Self> {
        let file: GoalFile = serde_json::from_str(raw).context("parse goal")?;
        validate(&file)?;
        let base = match file.base.as_deref() {
            Some(b) => {
                // A relative base resolves beside the goal file, like every other path in a goal, not from
                // wherever the process was started. An absolute base stands on its own.
                let named = Path::new(b);
                let full = if named.is_absolute() { named.to_path_buf() } else { base.join(named) };
                let p = std::fs::canonicalize(&full).with_context(|| format!("goal base {b}"))?;
                if !p.is_dir() {
                    anyhow::bail!("goal base {} is not a folder", p.display());
                }
                p
            }
            None => base.to_path_buf(),
        };
        Ok(Self { file, base })
    }

    pub fn base(&self) -> &Path {
        &self.base
    }

    pub fn checks(&self) -> &[Check] {
        &self.file.checks
    }

    pub fn providers(&self) -> &BTreeMap<String, ProviderSpec> {
        &self.file.providers
    }

    pub fn provider_spec(&self, id: &str) -> Option<&ProviderSpec> {
        self.file.providers.get(id)
    }

    pub fn usd_per_call(&self) -> f64 {
        self.file.budget.max_usd_per_call
    }

    pub fn new_goal(&self) -> Result<NewGoal> {
        Ok(NewGoal {
            title: title_of(&self.file.goal),
            goal_text: self.file.goal.clone(),
            done: self.file.done.clone(),
            checks: serde_json::to_value(&self.file.checks)?,
            message_policy: self.file.message_policy.clone(),
            budget_rounds: self.file.budget.rounds,
            budget_tokens: self.file.budget.tokens,
            budget_usd: Some(self.file.budget.max_usd_per_call),
            providers: self.providers_json(),
            threads: self
                .file
                .threads
                .iter()
                .map(|t| NewThread { name: t.name.clone(), agent: t.agent.clone(), prompt: t.prompt.clone() })
                .collect(),
        })
    }

    fn providers_json(&self) -> Value {
        Value::Object(
            self.file
                .providers
                .iter()
                .map(|(id, s)| (id.clone(), json!({ "driver": s.driver, "command": s.command, "model": s.model, "mode": s.mode })))
                .collect(),
        )
    }
}

fn title_of(goal: &str) -> String {
    goal.lines().next().unwrap_or_default().trim().chars().take(TITLE_LEN).collect()
}

/// Fill nothing in; refuse anything the run could not honour. Every refusal names the field involved.
fn validate(file: &GoalFile) -> Result<()> {
    if file.goal.trim().is_empty() {
        bail!("goal is empty: say what you want");
    }
    if file.done.as_deref().is_some_and(|d| d.trim().is_empty()) {
        bail!("done is empty: write what finished looks like, or leave the field out");
    }
    let budget = &file.budget;
    if budget.rounds == 0 {
        bail!("budget.rounds must be at least 1");
    }
    if budget.tokens == 0 {
        bail!("budget.tokens must be at least 1");
    }
    if budget.max_usd_per_call <= 0.0 || budget.max_usd_per_call > MAX_USD_PER_CALL {
        bail!("budget.max_usd_per_call must be in (0, {MAX_USD_PER_CALL}]");
    }
    if !MESSAGE_POLICIES.contains(&file.message_policy.as_str()) {
        bail!("message_policy {:?}: use one of {}", file.message_policy, MESSAGE_POLICIES.join(", "));
    }
    for check in &file.checks {
        validate_check(check)?;
    }
    validate_providers(file)?;
    validate_threads(file)
}

fn validate_check(check: &Check) -> Result<()> {
    match check {
        Check::FileExists { path } => relative_path(path),
        Check::TextAbsent { path, needles } => {
            relative_path(path)?;
            if needles.is_empty() || needles.iter().any(|n| n.is_empty()) {
                bail!("text_absent {path}: needles must be a non-empty list of non-empty text");
            }
            Ok(())
        }
        Check::CommandExitZero { command } => {
            if command.trim().is_empty() {
                bail!("command_exit_zero: command is empty");
            }
            Ok(())
        }
    }
}

/// Check paths name files in the run's artifacts folder: relative, and never above it.
fn relative_path(path: &str) -> Result<()> {
    let p = Path::new(path);
    if path.trim().is_empty() || p.is_absolute() || p.components().any(|c| matches!(c, Component::ParentDir)) {
        bail!("check path {path:?} must be relative to the artifacts folder, with no ..");
    }
    Ok(())
}

fn validate_providers(file: &GoalFile) -> Result<()> {
    if file.providers.is_empty() {
        bail!("providers is empty: name at least one agent");
    }
    for (id, spec) in &file.providers {
        if !DRIVERS.contains(&spec.driver.as_str()) {
            bail!("provider {id:?} uses driver {:?}; this build runs only: {}", spec.driver, DRIVERS.join(", "));
        }
        if spec.driver == "acp" && spec.command.as_deref().is_none_or(str::is_empty) {
            bail!("provider {id:?} uses driver acp and needs a command");
        }
    }
    Ok(())
}

fn validate_threads(file: &GoalFile) -> Result<()> {
    if file.threads.is_empty() {
        bail!("threads is empty: add at least one thread");
    }
    let mut seen = BTreeSet::new();
    for thread in &file.threads {
        if !valid_name(&thread.name) {
            bail!("thread name {:?}: use 1 to {MAX_NAME_LEN} letters, digits, - or _, starting with a letter or digit", thread.name);
        }
        if !seen.insert(thread.name.as_str()) {
            bail!("thread {:?} is listed twice", thread.name);
        }
        if !file.providers.contains_key(&thread.agent) {
            bail!("thread {:?} uses agent {:?}, which is not in providers", thread.name, thread.agent);
        }
    }
    Ok(())
}

fn valid_name(name: &str) -> bool {
    let starts_ok = name.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    starts_ok
        && name.len() <= MAX_NAME_LEN
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A goal with the given fragments spliced into the valid base. `extra` must start with a comma.
    fn goal_with(threads: &str, extra: &str) -> String {
        format!(
            r#"{{
              "goal": "read two sources", "providers": {{"claude-main": {{"driver": "claude"}}}},
              "threads": [{threads}],
              "budget": {{"rounds": 2, "tokens": 1000}}{extra}
            }}"#
        )
    }

    const ONE_THREAD: &str = r#"{"name": "reader-1", "agent": "claude-main", "prompt": "read"}"#;

    fn load(raw: &str) -> Result<GoalPlan> {
        GoalPlan::from_raw(raw, Path::new("."))
    }

    #[test]
    fn a_relative_base_resolves_beside_the_goal_file() {
        let dir = std::env::temp_dir().join(format!("ilvuno-goal-base-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("repo")).expect("repo dir");
        let raw = goal_with(ONE_THREAD, r#", "base": "repo""#);
        let plan = GoalPlan::from_raw(&raw, &dir).expect("goal with a relative base");
        assert_eq!(plan.base(), std::fs::canonicalize(dir.join("repo")).expect("canonical repo").as_path());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn err(raw: &str) -> String {
        format!("{:#}", load(raw).expect_err("goal must be refused"))
    }

    #[test]
    fn valid_goal_loads_threads_checks_and_budget() {
        let plan = load(&goal_with(
            ONE_THREAD,
            r#", "done": "brief.html exists", "checks": [{"type": "file_exists", "path": "brief.html"}]"#,
        ))
        .expect("valid goal");
        assert_eq!(plan.checks(), &[Check::FileExists { path: "brief.html".into() }]);
        assert_eq!(plan.file.threads[0].name, "reader-1");
        assert_eq!(plan.provider_spec("claude-main").map(|s| s.driver.as_str()), Some("claude"));
        assert_eq!(plan.usd_per_call(), MAX_USD_PER_CALL);
    }

    #[test]
    fn done_and_checks_are_optional() {
        assert!(load(&goal_with(ONE_THREAD, "")).expect("valid").checks().is_empty());
    }

    #[test]
    fn unknown_check_type_is_refused_at_load() {
        let message = err(&goal_with(ONE_THREAD, r#", "checks": [{"type": "llm_judge", "question": "good?"}]"#));
        assert!(message.contains("llm_judge"), "{message}");
    }

    #[test]
    fn old_research_fields_are_refused_at_load() {
        let raw = goal_with(ONE_THREAD, "").replace("\"goal\":", "\"template\": \"research\", \"goal\":");
        let message = err(&raw);
        assert!(message.contains("template"), "{message}");
    }

    #[test]
    fn thread_naming_an_unknown_agent_is_refused() {
        let message = err(&goal_with(r#"{"name": "reader-1", "agent": "codex-main"}"#, ""));
        assert!(message.contains("\"reader-1\"") && message.contains("\"codex-main\""), "{message}");
    }

    #[test]
    fn duplicate_thread_name_is_refused() {
        let message = err(&goal_with(&format!("{ONE_THREAD}, {ONE_THREAD}"), ""));
        assert!(message.contains("\"reader-1\""), "{message}");
    }

    #[test]
    fn thread_name_that_is_not_a_plain_folder_name_is_refused() {
        let message = err(&goal_with(r#"{"name": "../escape", "agent": "claude-main"}"#, ""));
        assert!(message.contains("thread name"), "{message}");
    }

    #[test]
    fn absolute_or_parent_check_path_is_refused() {
        let message = err(&goal_with(ONE_THREAD, r#", "checks": [{"type": "file_exists", "path": "/etc/passwd"}]"#));
        assert!(message.contains("/etc/passwd"), "{message}");
        let message = err(&goal_with(ONE_THREAD, r#", "checks": [{"type": "file_exists", "path": "../x"}]"#));
        assert!(message.contains("../x"), "{message}");
    }

    #[test]
    fn empty_needles_are_refused() {
        let message = err(&goal_with(ONE_THREAD, r#", "checks": [{"type": "text_absent", "path": "a.html", "needles": []}]"#));
        assert!(message.contains("needles"), "{message}");
    }

    #[test]
    fn empty_done_is_refused() {
        let message = err(&goal_with(ONE_THREAD, r#", "done": "  ""#));
        assert!(message.contains("done is empty"), "{message}");
    }

    #[test]
    fn acp_provider_without_command_is_refused() {
        let raw = goal_with(r#"{"name": "reader-1", "agent": "codex-main"}"#, "")
            .replace(r#""claude-main": {"driver": "claude"}"#, r#""codex-main": {"driver": "acp"}"#);
        let message = err(&raw);
        assert!(message.contains("\"codex-main\"") && message.contains("command"), "{message}");
    }

    #[test]
    fn unknown_driver_is_refused_naming_the_provider() {
        let raw = goal_with(ONE_THREAD, "").replace(r#""driver": "claude""#, r#""driver": "codex""#);
        let message = err(&raw);
        assert!(message.contains("\"claude-main\"") && message.contains("\"codex\""), "{message}");
    }

    #[test]
    fn unknown_message_policy_is_refused() {
        let message = err(&goal_with(ONE_THREAD, r#", "message_policy": "sometimes""#));
        assert!(message.contains("sometimes"), "{message}");
    }

    #[test]
    fn usd_above_the_per_call_cap_is_refused() {
        let raw = goal_with(ONE_THREAD, "").replace("\"tokens\": 1000", "\"tokens\": 1000, \"max_usd_per_call\": 0.5");
        let message = err(&raw);
        assert!(message.contains("max_usd_per_call"), "{message}");
    }

    #[test]
    fn orchestrator_is_an_ordinary_named_thread_with_the_reserved_name() {
        let plan = load(&goal_with(
            &format!(r#"{{"name": "{ORCHESTRATOR}", "agent": "claude-main"}}, {ONE_THREAD}"#),
            "",
        ))
        .expect("valid");
        assert_eq!(plan.file.threads[0].name, ORCHESTRATOR);
    }
}
