//! Permission policy: the runner decides, the adapter only translates (docs/provider-contract.md).
//! Deny by default. Every thread may read the run folder and the scope entries of its own task, and may
//! write only inside its own folder. Shell access is limited to `ilvuno note` and `ilvuno report` for its
//! own task; the orchestrator also gets `ilvuno task`.

use crate::state::{check_write_path, normalize};
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

/// Which permissions a thread gets. The orchestrator may also create tasks; nothing else differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyKind {
    Worker,
    Orchestrator,
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub kind: PolicyKind,
    /// The thread's own folder: the only place it may write.
    pub thread_dir: PathBuf,
    /// Paths it may read besides its own folder: the run folder and the task's absolute scope entries.
    pub read_roots: Vec<PathBuf>,
    pub task_id: String,
    /// Absolute path of the runner binary, the same prefix the agent's commands must start with.
    pub ilvuno_bin: String,
}

impl Policy {
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
                if self.may_read(Path::new(path)) {
                    Ok(())
                } else {
                    Err(format!("read of {path} is outside this thread's scope"))
                }
            }
            "Write" | "Edit" | "MultiEdit" => {
                let path = file_path(input)?;
                check_write_path(&self.thread_dir, Path::new(path)).map_err(|r| r.0)
            }
            "Bash" => self.check_bash(input),
            other => Err(format!("tool {other} is not permitted for a thread in this run")),
        }
    }

    fn may_read(&self, path: &Path) -> bool {
        let resolved = normalize(&if path.is_absolute() { path.to_path_buf() } else { self.thread_dir.join(path) });
        self.read_roots.iter().any(|root| resolved.starts_with(normalize(root)))
            || check_write_path(&self.thread_dir, path).is_ok()
    }

    /// The command prefixes this thread may run. Each includes the task id, so a thread cannot report or
    /// note for another task.
    fn allowed_prefixes(&self) -> Vec<String> {
        let bin = &self.ilvuno_bin;
        let task = &self.task_id;
        let mut prefixes = vec![format!("{bin} note --task {task} "), format!("{bin} report --task {task} ")];
        if self.kind == PolicyKind::Orchestrator {
            prefixes.push(format!("{bin} task --thread "));
        }
        prefixes
    }

    fn check_bash(&self, input: &Value) -> Result<(), String> {
        let command = input.get("command").and_then(Value::as_str).unwrap_or_default();
        if !is_shell_inert(command) {
            return Err("shell control syntax is not permitted".into());
        }
        if self.allowed_prefixes().iter().any(|prefix| command.starts_with(prefix)) {
            Ok(())
        } else {
            Err("only the ilvuno commands for this thread's task are permitted".into())
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

    fn worker() -> Policy {
        Policy {
            kind: PolicyKind::Worker,
            thread_dir: PathBuf::from("/runs/r1/threads/reader-1"),
            read_roots: vec![PathBuf::from("/runs/r1"), PathBuf::from("/repo/docs/security.md")],
            task_id: "task-7".into(),
            ilvuno_bin: "ilvuno".into(),
        }
    }

    fn orchestrator() -> Policy {
        Policy {
            kind: PolicyKind::Orchestrator,
            thread_dir: PathBuf::from("/runs/r1/threads/orchestrator"),
            ..worker()
        }
    }

    fn denied(d: &Decision) -> bool {
        matches!(d, Decision::Deny { .. })
    }

    #[test]
    fn writes_only_inside_own_folder() {
        let p = worker();
        assert!(!denied(&p.decide("Write", &json!({"file_path": "/runs/r1/threads/reader-1/findings.md"}))));
        assert!(denied(&p.decide("Write", &json!({"file_path": "/runs/r1/threads/reader-2/findings.md"}))));
        assert!(denied(&p.decide("Edit", &json!({"file_path": "/home/sumeet/.bashrc"}))));
    }

    #[test]
    fn reads_run_folder_scope_entries_and_own_folder_only() {
        let p = worker();
        assert!(!denied(&p.decide("Read", &json!({"file_path": "/repo/docs/security.md"}))));
        assert!(!denied(&p.decide("Read", &json!({"file_path": "/runs/r1/board.md"}))));
        assert!(!denied(&p.decide("Read", &json!({"file_path": "/runs/r1/threads/reader-2/findings.md"}))));
        assert!(!denied(&p.decide("Read", &json!({"file_path": "/runs/r1/threads/reader-1/x"}))));
        assert!(denied(&p.decide("Read", &json!({"file_path": "/home/sumeet/.aws/credentials"}))));
        assert!(denied(&p.decide("Read", &json!({"file_path": "/runs/r1/../../.ssh/id_rsa"}))));
    }

    #[test]
    fn bash_allows_its_own_note_command() {
        let p = worker();
        let ok = json!({"command": "ilvuno note --task task-7 --kind intent --body \"writing findings.md\""});
        assert!(!denied(&p.decide("Bash", &ok)));
        assert!(denied(&p.decide("Bash", &json!({"command": "ilvuno note --task task-8 --kind intent --body x"}))));
        assert!(denied(&p.decide("Bash", &json!({"command": "ilvuno note --task task-7 --kind intent --body x; rm -rf ~"}))));
        assert!(denied(&p.decide("Bash", &json!({"command": "ilvuno gate r1 accept"}))));
    }

    #[test]
    fn bash_allows_only_its_own_report_command() {
        let p = worker();
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
    fn a_worker_cannot_create_tasks() {
        let p = worker();
        assert!(denied(&p.decide("Bash", &json!({"command": "ilvuno task --thread reader-2 --prompt x --scope a.md"}))));
    }

    #[test]
    fn the_orchestrator_may_create_tasks_but_never_open_the_gate() {
        let p = orchestrator();
        let task = json!({"command": "ilvuno task --thread reader-1 --prompt \"read it\" --scope /repo/docs/security.md"});
        assert!(!denied(&p.decide("Bash", &task)));
        let note = json!({"command": "ilvuno note --task task-7 --kind question --body \"which source first?\""});
        assert!(!denied(&p.decide("Bash", &note)));
        let report = json!({"command": "ilvuno report --task task-7 --status ok --summary \"planned\""});
        assert!(!denied(&p.decide("Bash", &report)));
        assert!(denied(&p.decide("Bash", &json!({"command": "ilvuno gate run1 accept"}))));
        assert!(denied(&p.decide("Bash", &json!({"command": "ilvuno task --thread reader-1 --prompt x --scope a; rm -rf ~"}))));
    }

    #[test]
    fn the_orchestrator_writes_only_inside_its_own_folder() {
        let p = orchestrator();
        assert!(!denied(&p.decide("Write", &json!({"file_path": "/runs/r1/threads/orchestrator/plan.md"}))));
        assert!(denied(&p.decide("Write", &json!({"file_path": "/runs/r1/threads/reader-1/brief.html"}))));
    }

    #[test]
    fn shell_inert_check_keeps_quoted_text_and_refuses_live_syntax() {
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
        let p = worker();
        assert!(denied(&p.decide("Agent", &json!({"prompt": "spawn"}))));
        assert!(denied(&p.decide("WebFetch", &json!({"url": "https://example.com"}))));
    }
}
