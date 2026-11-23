//! The user's machine checks, run when a round closes. A check is a file test or a command. The runner runs
//! it and records the result; it never judges the content (PRD §4: the machine decides, the human gate
//! decides the rest).

use crate::goal::Check;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// A command check that runs longer than this is killed and fails.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
const DETAIL_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// Where checks look. File checks read `artifacts`. Commands start in `goal_dir` and get `ILVUNO_ARTIFACTS`.
pub struct Locations<'a> {
    pub artifacts: &'a Path,
    pub goal_dir: &'a Path,
}

pub fn evaluate(checks: &[Check], at: &Locations<'_>) -> Vec<CheckResult> {
    checks
        .iter()
        .map(|check| {
            let (passed, detail) = match check {
                Check::FileExists { path } => file_exists(&at.artifacts.join(path)),
                Check::TextAbsent { path, needles } => text_absent(&at.artifacts.join(path), needles),
                Check::CommandExitZero { command } => command_exit_zero(command, at),
            };
            CheckResult { name: check.label(), passed, detail }
        })
        .collect()
}

fn file_exists(file: &Path) -> (bool, String) {
    if file.is_file() {
        (true, "found".into())
    } else {
        (false, format!("missing: {}", file.display()))
    }
}

fn text_absent(file: &Path, needles: &[String]) -> (bool, String) {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) => return (false, format!("cannot read {}: {e}", file.display())),
    };
    let found: Vec<&str> = needles.iter().filter(|n| text.contains(n.as_str())).map(String::as_str).collect();
    if found.is_empty() {
        (true, "none of the listed text is present".into())
    } else {
        (false, format!("found: {}", found.join(", ")))
    }
}

fn command_exit_zero(command: &str, at: &Locations<'_>) -> (bool, String) {
    let spawned = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(at.goal_dir)
        .env("ILVUNO_ARTIFACTS", at.artifacts)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => return (false, format!("cannot start sh: {e}")),
    };
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    match wait_with_timeout(&mut child) {
        None => (false, format!("timed out after {} seconds", COMMAND_TIMEOUT.as_secs())),
        Some(status) if status.success() => (true, "exit 0".into()),
        Some(status) => {
            let code = status.code().map_or_else(|| "by signal".to_string(), |c| c.to_string());
            let output = last_line(&stderr.join().unwrap_or_default(), &stdout.join().unwrap_or_default());
            (false, format!("exit {code}: {output}"))
        }
    }
}

/// Wait for the child, killing it at the timeout. `None` means it was killed.
fn wait_with_timeout(child: &mut Child) -> Option<ExitStatus> {
    let deadline = Instant::now() + COMMAND_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// Read a pipe on its own thread, so a chatty command cannot block on a full pipe.
fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> JoinHandle<String> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

/// The last non-empty line of stderr, or of stdout when stderr is silent.
fn last_line(stderr: &str, stdout: &str) -> String {
    let line = [stderr, stdout]
        .iter()
        .find_map(|text| text.lines().rev().find(|l| !l.trim().is_empty()))
        .unwrap_or("no output");
    line.trim().chars().take(DETAIL_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at<'a>(artifacts: &'a Path, goal_dir: &'a Path) -> Locations<'a> {
        Locations { artifacts, goal_dir }
    }

    fn run_one(check: Check, dir: &Path) -> CheckResult {
        evaluate(std::slice::from_ref(&check), &at(dir, dir)).remove(0)
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ilvuno-checks-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn file_exists_passes_only_for_a_file_in_the_artifacts_folder() {
        let dir = scratch("exists");
        assert!(!run_one(Check::FileExists { path: "brief.html".into() }, &dir).passed);
        std::fs::write(dir.join("brief.html"), "<p>ok</p>").expect("write");
        let result = run_one(Check::FileExists { path: "brief.html".into() }, &dir);
        assert!(result.passed, "{result:?}");
        assert_eq!(result.name, "file_exists brief.html");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_absent_fails_and_names_each_needle_found() {
        let dir = scratch("absent");
        std::fs::write(dir.join("brief.html"), "a TODO and a TBD").expect("write");
        let check = Check::TextAbsent {
            path: "brief.html".into(),
            needles: vec!["TODO".into(), "TBD".into(), "[citation needed]".into()],
        };
        let result = run_one(check, &dir);
        assert!(!result.passed);
        assert_eq!(result.detail, "found: TODO, TBD");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_absent_fails_when_the_file_is_missing() {
        let dir = scratch("absent-missing");
        let result = run_one(Check::TextAbsent { path: "brief.html".into(), needles: vec!["TODO".into()] }, &dir);
        assert!(!result.passed);
        assert!(result.detail.starts_with("cannot read"), "{}", result.detail);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn command_exit_zero_follows_the_exit_status() {
        let dir = scratch("command");
        assert!(run_one(Check::CommandExitZero { command: "true".into() }, &dir).passed);
        let failed = run_one(Check::CommandExitZero { command: "echo broken >&2; exit 3".into() }, &dir);
        assert!(!failed.passed);
        assert_eq!(failed.detail, "exit 3: broken");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn command_runs_in_the_goal_folder_with_the_artifacts_path_in_its_environment() {
        let dir = scratch("env");
        let check = Check::CommandExitZero { command: "test \"$ILVUNO_ARTIFACTS\" = \"$PWD/artifacts\"".into() };
        let artifacts = dir.join("artifacts");
        std::fs::create_dir_all(&artifacts).expect("artifacts");
        let result = evaluate(std::slice::from_ref(&check), &at(&artifacts, &dir)).remove(0);
        assert!(result.passed, "{result:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn results_keep_the_order_of_the_checks() {
        let dir = scratch("order");
        let checks = vec![Check::CommandExitZero { command: "false".into() }, Check::CommandExitZero { command: "true".into() }];
        let results = evaluate(&checks, &at(&dir, &dir));
        assert_eq!(results.iter().map(|r| r.passed).collect::<Vec<_>>(), vec![false, true]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
