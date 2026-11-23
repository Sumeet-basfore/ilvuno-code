//! Goal files: the user's statement of a run. Validated at load (system boundary), then turned into the
//! `goals` row and the research plan.

use crate::commands::NewGoal;
use crate::research::SYNTH_SLOT;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The most any single agent call may spend (AGENTS.md non-negotiable).
pub const MAX_USD_PER_CALL: f64 = 0.20;

/// Driver names this build can run. `codex` (native app-server) is not built yet; use `acp` for Codex.
pub const DRIVERS: &[&str] = &["claude", "acp"];

/// Provider id used when a goal declares no providers at all (the v0 behaviour).
pub const DEFAULT_PROVIDER: &str = "claude-main";

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
    /// Provider instances the run may use, in file order. Empty means one Claude instance.
    #[serde(default, deserialize_with = "ordered_providers")]
    pub providers: Vec<(String, ProviderSpec)>,
    /// Slot → provider id. A slot not listed here uses the run default.
    #[serde(default)]
    pub assignments: BTreeMap<String, String>,
    /// Run default. Filled at load: the explicit value, else the first entry of `providers`.
    #[serde(default, rename = "default")]
    pub default_provider: Option<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
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

/// Reads the providers map keeping its order: the first entry is the run default.
fn ordered_providers<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<(String, ProviderSpec)>, D::Error> {
    struct Ordered;
    impl<'de> serde::de::Visitor<'de> for Ordered {
        type Value = Vec<(String, ProviderSpec)>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a map from provider id to { driver, command, model }")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(entry) = map.next_entry::<String, ProviderSpec>()? {
                out.push(entry);
            }
            Ok(out)
        }
    }
    de.deserialize_map(Ordered)
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
        let mut file: GoalFile = serde_json::from_str(raw).context("parse goal")?;
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
        resolve_providers(&mut file)?;
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
            providers: self.providers_json(),
            assignments: Value::Object(
                self.file.assignments.iter().map(|(slot, id)| (slot.clone(), Value::String(id.clone()))).collect(),
            ),
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

    /// Every provider instance the run may use, in file order.
    pub fn providers(&self) -> &[(String, ProviderSpec)] {
        &self.file.providers
    }

    pub fn provider_spec(&self, id: &str) -> Option<&ProviderSpec> {
        self.file.providers.iter().find(|(p, _)| p == id).map(|(_, spec)| spec)
    }

    /// The provider id a slot runs on: its assignment, or the run default. Never a fallback that is
    /// not declared in the goal; validation already checked every assignment.
    pub fn provider_for_slot(&self, slot: &str) -> &str {
        self.file
            .assignments
            .get(slot)
            .or(self.file.default_provider.as_ref())
            .map(String::as_str)
            .unwrap_or(DEFAULT_PROVIDER)
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

/// Fill the defaults, then refuse anything the run could not honour. Every refusal names the slot or
/// provider id involved, so the user can find the line to fix.
fn resolve_providers(file: &mut GoalFile) -> Result<()> {
    if file.providers.is_empty() {
        let spec = ProviderSpec { driver: "claude".into(), command: None, model: None, mode: None };
        file.providers.push((DEFAULT_PROVIDER.into(), spec));
    }
    let mut seen = BTreeSet::new();
    for (id, spec) in &file.providers {
        if !seen.insert(id.as_str()) {
            bail!("provider {id:?} is listed twice in providers");
        }
        if !DRIVERS.contains(&spec.driver.as_str()) {
            bail!("provider {id:?} uses driver {:?}; this build runs only: {}", spec.driver, DRIVERS.join(", "));
        }
        if spec.driver == "acp" && spec.command.as_deref().is_none_or(str::is_empty) {
            bail!("provider {id:?} uses driver acp and needs a command");
        }
    }
    let default = match file.default_provider.take() {
        Some(id) if file.providers.iter().any(|(p, _)| *p == id) => id,
        Some(id) => bail!("default provider {id:?} is not in providers"),
        None => file.providers[0].0.clone(),
    };
    file.default_provider = Some(default);
    let slots: BTreeSet<&str> = file.groups.iter().map(|g| g.slot.as_str()).chain([SYNTH_SLOT]).collect();
    for (slot, id) in &file.assignments {
        if !slots.contains(slot.as_str()) {
            bail!("assignment for slot {slot:?}: this template has no such slot");
        }
        if !file.providers.iter().any(|(p, _)| p == id) {
            bail!("slot {slot:?} is assigned provider {id:?}, which is not in providers");
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn research_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research")
    }

    /// A goal with the given `providers` / `assignments` / `default` fragments, on the real source.
    fn goal(extra: &str) -> String {
        format!(
            r#"{{
              "title": "t", "goal_text": "g", "template": "research", "source": "09-spike.md",
              "groups": [{{"slot": "researcher-a", "sections": [1, 2]}}, {{"slot": "researcher-b", "sections": [3, 4]}}],
              "required_sections": [1, 2, 3, 4], "checks": [],
              "budget": {{"rounds": 2, "tokens": 1000}}{extra}
            }}"#
        )
    }

    fn load(extra: &str) -> Result<GoalPlan> {
        GoalPlan::from_raw(&goal(extra), &research_dir())
    }

    fn err(extra: &str) -> String {
        format!("{:#}", load(extra).expect_err("goal must be refused"))
    }

    #[test]
    fn goal_without_providers_runs_on_one_claude_instance() {
        let plan = load("").expect("valid goal");
        assert_eq!(plan.providers().len(), 1);
        assert_eq!(plan.provider_for_slot("researcher-a"), DEFAULT_PROVIDER);
        assert_eq!(plan.provider_for_slot("synthesizer"), DEFAULT_PROVIDER);
        assert_eq!(plan.provider_spec(DEFAULT_PROVIDER).map(|s| s.driver.as_str()), Some("claude"));
    }

    #[test]
    fn each_slot_gets_its_assigned_provider() {
        let plan = load(
            r#", "providers": {"fake-a": {"driver": "claude"}, "fake-b": {"driver": "claude"}},
                 "assignments": {"researcher-a": "fake-a", "researcher-b": "fake-b", "synthesizer": "fake-b"}"#,
        )
        .expect("valid goal");
        assert_eq!(plan.provider_for_slot("researcher-a"), "fake-a");
        assert_eq!(plan.provider_for_slot("researcher-b"), "fake-b");
        assert_eq!(plan.provider_for_slot("synthesizer"), "fake-b");
    }

    #[test]
    fn same_provider_for_every_slot_is_allowed() {
        let plan = load(
            r#", "providers": {"only": {"driver": "claude"}},
                 "assignments": {"researcher-a": "only", "researcher-b": "only", "synthesizer": "only"}"#,
        )
        .expect("valid goal");
        assert!(["researcher-a", "researcher-b", "synthesizer"].iter().all(|s| plan.provider_for_slot(s) == "only"));
    }

    #[test]
    fn unassigned_slots_use_the_first_listed_provider() {
        let plan = load(
            r#", "providers": {"first": {"driver": "claude"}, "second": {"driver": "claude"}},
                 "assignments": {"synthesizer": "second"}"#,
        )
        .expect("valid goal");
        assert_eq!(plan.provider_for_slot("researcher-a"), "first");
        assert_eq!(plan.provider_for_slot("synthesizer"), "second");
    }

    #[test]
    fn explicit_default_overrides_the_first_listed_provider() {
        let plan = load(r#", "providers": {"first": {"driver": "claude"}, "second": {"driver": "claude"}}, "default": "second""#)
            .expect("valid goal");
        assert_eq!(plan.provider_for_slot("researcher-a"), "second");
    }

    #[test]
    fn unknown_provider_id_is_refused_naming_slot_and_id() {
        let message = err(r#", "providers": {"fake-a": {"driver": "claude"}}, "assignments": {"researcher-b": "fake-z"}"#);
        assert!(message.contains("\"researcher-b\"") && message.contains("\"fake-z\""), "{message}");
    }

    #[test]
    fn unknown_driver_is_refused_naming_the_provider() {
        let message = err(r#", "providers": {"codex-main": {"driver": "codex"}}"#);
        assert!(message.contains("\"codex-main\"") && message.contains("\"codex\""), "{message}");
    }

    #[test]
    fn acp_provider_without_command_is_refused() {
        let message = err(r#", "providers": {"codex-main": {"driver": "acp"}}"#);
        assert!(message.contains("\"codex-main\"") && message.contains("command"), "{message}");
    }

    #[test]
    fn assignment_to_a_slot_the_template_lacks_is_refused() {
        let message = err(r#", "providers": {"a": {"driver": "claude"}}, "assignments": {"researcher_b": "a"}"#);
        assert!(message.contains("researcher_b"), "{message}");
    }

    #[test]
    fn unknown_default_is_refused() {
        let message = err(r#", "providers": {"a": {"driver": "claude"}}, "default": "b""#);
        assert!(message.contains("\"b\""), "{message}");
    }

    #[test]
    fn duplicate_provider_id_is_refused() {
        let message = err(r#", "providers": {"a": {"driver": "claude"}, "a": {"driver": "claude"}}"#);
        assert!(message.contains("\"a\""), "{message}");
    }
}
