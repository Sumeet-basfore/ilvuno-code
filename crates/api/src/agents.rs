//! `GET /agents`: which agent CLIs are on this machine, and what version each prints.
//!
//! Ilvuno does not sandbox agents; `safety` describes the CLI's own permissions.

use crate::blocking::blocking;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde::Serialize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long one `--version` call may take before its answer is dropped.
pub const VERSION_TIMEOUT: Duration = Duration::from_secs(3);

/// How long an answer is reused, so opening Settings does not spawn six processes again.
const CACHE_FOR: Duration = Duration::from_secs(30);

/// The agents the window offers. The order is the order the window shows them in.
const KNOWN: [(&str, &str); 6] = [
    ("claude", "Claude Code"),
    ("codex", "Codex"),
    ("pi", "Pi"),
    ("opencode", "opencode"),
    ("agy", "Antigravity"),
    ("cursor-agent", "Cursor Agent"),
];

#[derive(Clone, Serialize)]
pub struct Agent {
    pub id: &'static str,
    pub name: &'static str,
    pub path: Option<String>,
    pub found: bool,
    pub version: Option<String>,
    pub safety: &'static str,
}

/// The CLI's own permission behaviour, not a sandbox: Ilvuno adds no permission of its own.
fn safety(id: &str) -> &'static str {
    match id {
        "claude" => "asks_before_each_tool",
        "codex" | "pi" | "opencode" | "agy" | "cursor-agent" => "works_in_folder_asks_outside",
        _ => "unknown",
    }
}

/// Look up every known agent on `path_env` and ask the found ones for their version.
pub fn detect(path_env: Option<&OsStr>, timeout: Duration) -> Vec<Agent> {
    let dirs: Vec<PathBuf> = path_env.map(std::env::split_paths).into_iter().flatten().collect();
    KNOWN
        .iter()
        .map(|(id, name)| {
            let found = find_on_path(&dirs, id);
            let version = found.as_deref().and_then(|bin| version_of(bin, timeout));
            Agent {
                id,
                name,
                found: found.is_some(),
                path: found.map(|p| p.to_string_lossy().into_owned()),
                version,
                safety: safety(id),
            }
        })
        .collect()
}

fn find_on_path(dirs: &[PathBuf], bin: &str) -> Option<PathBuf> {
    dirs.iter().map(|dir| dir.join(bin)).find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Ask the binary for its version, with pipes so the answer can be read.
fn launch(bin: &Path) -> Option<std::process::Child> {
    let run = || {
        Command::new(bin)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    };
    match run() {
        // A file is briefly busy while it is being written: an installer upgrading the CLI, or — in a
        // threaded program — the copy of this process's own write handle that a just-forked child holds.
        // Both clear in milliseconds, so one retry is enough. (Rust's own tests hit this.)
        Err(err) if err.kind() == std::io::ErrorKind::ExecutableFileBusy => {
            std::thread::sleep(Duration::from_millis(50));
            run().ok()
        }
        other => other.ok(),
    }
}

/// The first non-empty line of `<bin> --version`, or `None` if it says nothing in time.
fn version_of(bin: &Path, timeout: Duration) -> Option<String> {
    let mut child = launch(bin)?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            // A CLI that does not answer in time says nothing about its version; we do not guess one.
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            // shortcut: poll at 10 ms; a `--version` that writes more than a pipe buffer would be killed.
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => return None,
        }
    }
    let output = child.wait_with_output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    first_line(&stdout).or_else(|| first_line(&String::from_utf8_lossy(&output.stderr)))
}

fn first_line(text: &str) -> Option<String> {
    text.lines().map(str::trim).find(|line| !line.is_empty()).map(str::to_string)
}

/// The last answer, with the time it was taken. `GET /agents` reads it; a stale one is thrown away.
#[derive(Default)]
pub struct AgentCache {
    cached: Mutex<Option<(Instant, Vec<Agent>)>>,
}

impl AgentCache {
    fn get(&self) -> Option<Vec<Agent>> {
        let cached = self.cached.lock().unwrap_or_else(|e| e.into_inner());
        cached.as_ref().filter(|(at, _)| at.elapsed() < CACHE_FOR).map(|(_, agents)| agents.clone())
    }

    /// Store an answer. Public so a test can answer without touching the real `PATH`.
    pub fn put(&self, agents: Vec<Agent>) {
        *self.cached.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), agents));
    }
}

pub async fn list<P>(State(state): State<AppState<P>>) -> ApiResult<Json<Vec<Agent>>>
where
    P: Send + Sync + 'static,
{
    if let Some(agents) = state.agents.get() {
        return Ok(Json(agents));
    }
    // Six processes, up to three seconds each, so they run off the async threads.
    let agents = blocking(|| Ok(detect(std::env::var_os("PATH").as_deref(), VERSION_TIMEOUT))).await?;
    state.agents.put(agents.clone());
    Ok(Json(agents))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ilvuno-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// A shell script that runs `body` when given `--version`.
    fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("script");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        path
    }

    fn one<'a>(agents: &'a [Agent], id: &str) -> &'a Agent {
        agents.iter().find(|a| a.id == id).expect("agent in list")
    }

    #[test]
    fn every_known_agent_is_reported_and_a_found_one_shows_its_version() {
        let dir = temp_dir("agents-found");
        script(&dir, "claude", "echo 2.1.329");
        let path = std::env::join_paths([&dir]).expect("join path");
        let agents = detect(Some(&path), VERSION_TIMEOUT);

        assert_eq!(agents.len(), KNOWN.len());
        let claude = one(&agents, "claude");
        assert!(claude.found);
        assert_eq!(claude.name, "Claude Code");
        assert_eq!(claude.path.as_deref().map(Path::new), Some(dir.join("claude").as_path()));
        assert_eq!(claude.version.as_deref(), Some("2.1.329"));
        assert_eq!(claude.safety, "asks_before_each_tool");

        // A known name that is not on this path: reported, not invented.
        let codex = one(&agents, "codex");
        assert!(!codex.found);
        assert_eq!(codex.path, None);
        assert_eq!(codex.version, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_version_line_is_the_first_non_empty_one() {
        let dir = temp_dir("agents-lines");
        script(&dir, "pi", "printf '\\n\\n0.4.2 (build 7)\\nsecond line\\n'");
        let path = std::env::join_paths([&dir]).expect("join path");
        let agents = detect(Some(&path), VERSION_TIMEOUT);
        assert!(one(&agents, "pi").found);
        assert_eq!(one(&agents, "pi").version.as_deref(), Some("0.4.2 (build 7)"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_version_call_that_hangs_yields_no_version_but_the_agent_is_still_found() {
        let dir = temp_dir("agents-hang");
        script(&dir, "codex", "sleep 30");
        let path = std::env::join_paths([&dir]).expect("join path");
        let started = Instant::now();
        let agents = detect(Some(&path), Duration::from_millis(200));
        assert!(started.elapsed() < Duration::from_secs(5), "the timeout did not stop the call");

        let codex = one(&agents, "codex");
        assert!(codex.found);
        assert_eq!(codex.version, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_file_without_the_execute_bit_is_not_found() {
        let dir = temp_dir("agents-noexec");
        std::fs::write(dir.join("agy"), "not a program").expect("file");
        let path = std::env::join_paths([&dir]).expect("join path");
        let agents = detect(Some(&path), VERSION_TIMEOUT);
        assert!(!one(&agents, "agy").found);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_unknown_agent_is_unknown_not_a_guess() {
        assert_eq!(safety("nobody"), "unknown");
        assert_eq!(safety("cursor-agent"), "works_in_folder_asks_outside");
    }
}
