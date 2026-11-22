//! Permission policy: the runner decides, the adapter only translates (docs/provider-contract.md).
//! Deny by default. A worker may read its sources and its own directory, write only its own
//! directory, and run exactly one shell command: its own `ilvuno report`.

use crate::state::check_write_path;
use provider_core::Decision;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// True when the command has no live shell syntax: nothing that chains, substitutes or redirects.
/// Single-quoted text is literal. Inside double quotes only `$` and backtick are live.
pub fn is_shell_inert(command: &str) -> bool {
    let (mut single, mut double) = (false, false);
    let mut escaped = false;
    for c in command.chars() {
        if escaped {
            return false;
        }
        match c {
            '\\' if !single => escaped = true,
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            '\n' if !single => return false,
            '`' if !single => return false,
            '$' if !single => return false,
            ';' | '&' | '|' | '<' | '>' | '(' | ')' if !single && !double => return false,
            _ => {}
        }
    }
    !escaped && !single && !double
}

#[derive(Debug, Clone)]
pub struct WorkerPolicy {
    pub thread_dir: PathBuf,
    pub read_sources: Vec<PathBuf>,
    pub task_id: String,
    pub ilvuno_bin: String,
}

impl WorkerPolicy {
    pub fn decide(&self, tool: &str, input: &Value) -> Decision {
        match self.check(tool, input) {
            Ok(()) => Decision::Allow { updated_input: input.clone() },
            Err(reason) => Decision::Deny { message: reason },
        }
    }

    fn check(&self, tool: &str, input: &Value) -> Result<(), String> {
        match tool {
            "Read" => {
                let path = file_path(input)?;
                let allowed = self.read_sources.iter().any(|s| s == Path::new(path))
                    || check_write_path(&self.thread_dir, Path::new(path)).is_ok();
                if allowed { Ok(()) } else { Err(format!("read of {path} is outside this worker's scope")) }
            }
            "Write" | "Edit" | "MultiEdit" => {
                let path = file_path(input)?;
                check_write_path(&self.thread_dir, Path::new(path)).map_err(|r| r.0)
            }
            "Bash" => self.check_bash(input),
            other => Err(format!("tool {other} is not permitted for a worker in this run")),
        }
    }

    fn check_bash(&self, input: &Value) -> Result<(), String> {
        let command = input.get("command").and_then(Value::as_str).unwrap_or_default();
        if !is_shell_inert(command) {
            return Err("shell control syntax is not permitted".into());
        }
        let prefix = format!("{} report --task {} ", self.ilvuno_bin, self.task_id);
        if command.starts_with(&prefix) {
            Ok(())
        } else {
            Err("only the worker report command is permitted".into())
        }
    }
}

fn file_path(input: &Value) -> Result<&str, String> {
    input
        .get("file_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing file_path".to_string())
}

/// Summary of a decision for the journal (`permission.decided`).
pub fn decision_payload(tool: &str, decision: &Decision) -> Value {
    match decision {
        Decision::Allow { .. } => json!({ "tool": tool, "behavior": "allow" }),
        Decision::Deny { message } => json!({ "tool": tool, "behavior": "deny", "message": message }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> WorkerPolicy {
        WorkerPolicy {
            thread_dir: PathBuf::from("/runs/r1/threads/researcher-a"),
            read_sources: vec![PathBuf::from("/repo/research/09-spike.md")],
            task_id: "task-7".into(),
            ilvuno_bin: "ilvuno".into(),
        }
    }

    fn denied(d: &Decision) -> bool {
        matches!(d, Decision::Deny { .. })
    }

    #[test]
    fn writes_only_inside_own_directory() {
        let p = policy();
        assert!(!denied(&p.decide("Write", &json!({"file_path": "/runs/r1/threads/researcher-a/findings.md"}))));
        assert!(denied(&p.decide("Write", &json!({"file_path": "/runs/r1/threads/other/findings.md"}))));
        assert!(denied(&p.decide("Edit", &json!({"file_path": "/home/sumeet/.bashrc"}))));
    }

    #[test]
    fn reads_sources_and_own_directory_only() {
        let p = policy();
        assert!(!denied(&p.decide("Read", &json!({"file_path": "/repo/research/09-spike.md"}))));
        assert!(!denied(&p.decide("Read", &json!({"file_path": "/runs/r1/threads/researcher-a/x"}))));
        assert!(denied(&p.decide("Read", &json!({"file_path": "/home/sumeet/.aws/credentials"}))));
    }

    #[test]
    fn bash_allows_only_its_own_report_command() {
        let p = policy();
        let ok = json!({"command": "ilvuno report --task task-7 --status ok --summary \"done\" --files findings.md"});
        assert!(!denied(&p.decide("Bash", &ok)));
        let other_task = json!({"command": "ilvuno report --task task-8 --status ok"});
        assert!(denied(&p.decide("Bash", &other_task)));
        let chained = json!({"command": "ilvuno report --task task-7 --status ok; rm -rf ~"});
        assert!(denied(&p.decide("Bash", &chained)));
        let subst = json!({"command": "ilvuno report --task task-7 --summary $(cat ~/.ssh/id_rsa)"});
        assert!(denied(&p.decide("Bash", &subst)));
        assert!(denied(&p.decide("Bash", &json!({"command": "git push origin main"}))));
    }

    #[test]
    fn quoted_summary_text_is_literal_but_live_syntax_is_refused() {
        assert!(is_shell_inert(r#"ilvuno report --task t --summary "Parts (a) and (b): done; mostly ok" --files findings.md"#));
        assert!(is_shell_inert("ilvuno report --task t --summary 'a; b | c'"));
        assert!(!is_shell_inert(r#"ilvuno report --task t --summary "x" ; rm -rf ~"#));
        assert!(!is_shell_inert(r#"ilvuno report --task t --summary "$(whoami)""#));
        assert!(!is_shell_inert(r#"ilvuno report --task t --summary `id`"#));
        assert!(!is_shell_inert("ilvuno report --task t --summary x > /etc/passwd"));
        assert!(!is_shell_inert(r#"ilvuno report --task t --summary "unclosed"#));
    }

    #[test]
    fn unlisted_tools_are_denied_by_default() {
        let p = policy();
        assert!(denied(&p.decide("Agent", &json!({"prompt": "spawn"}))));
        assert!(denied(&p.decide("WebFetch", &json!({"url": "https://example.com"}))));
    }
}
