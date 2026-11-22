//! Round/run state and the five invariants (docs/data-model.md). Pure functions: no I/O, no SQL.
//! Callers pass in the facts; these return `Refusal` with a human-readable reason.

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal(pub String);

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Planned,
    Working,
    /// Verdict pass; waiting at the human gate.
    Reviewed,
    Accepted,
    Merged,
    Blocked,
    BudgetExhausted,
    NoProgress,
    HumanPaused,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Working => "working",
            Self::Reviewed => "reviewed",
            Self::Accepted => "accepted",
            Self::Merged => "merged",
            Self::Blocked => "blocked",
            Self::BudgetExhausted => "budget_exhausted",
            Self::NoProgress => "no_progress",
            Self::HumanPaused => "human_paused",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "planned" => Self::Planned,
            "working" => Self::Working,
            "reviewed" => Self::Reviewed,
            "accepted" => Self::Accepted,
            "merged" => Self::Merged,
            "blocked" => Self::Blocked,
            "budget_exhausted" => Self::BudgetExhausted,
            "no_progress" => Self::NoProgress,
            "human_paused" => Self::HumanPaused,
            _ => return None,
        })
    }

    /// Terminal runs are not driven any further by the loop. `Reviewed` is waiting on a human, not done.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Accepted | Self::Merged | Self::Blocked | Self::BudgetExhausted | Self::NoProgress)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    PassWithNotes,
    Block,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::PassWithNotes => "pass_with_notes",
            Self::Block => "block",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "pass" => Self::Pass,
            "pass_with_notes" => Self::PassWithNotes,
            "block" => Self::Block,
            _ => return None,
        })
    }

    pub fn is_passing(self) -> bool {
        matches!(self, Self::Pass | Self::PassWithNotes)
    }
}

/// Invariant 1 and the round budget: no round starts before the previous one has a verdict, and
/// `max_rounds` is a number in a column, not advice in a prompt.
pub fn check_round_start(
    previous_verdict: Option<Option<Verdict>>,
    rounds_used: u32,
    max_rounds: u32,
    tokens_used: u64,
    budget_tokens: u64,
) -> Result<(), Refusal> {
    if let Some(None) = previous_verdict {
        return Err(Refusal("previous round has no verdict".into()));
    }
    if rounds_used >= max_rounds {
        return Err(Refusal(format!(
            "round budget exhausted: {rounds_used} of {max_rounds} rounds used (max_rounds = {max_rounds})"
        )));
    }
    if tokens_used >= budget_tokens {
        return Err(Refusal(format!("token budget exhausted: {tokens_used} of {budget_tokens} tokens used")));
    }
    Ok(())
}

/// Invariant 2: no dispatch without a ledger `intent` naming its scope.
pub fn check_dispatch(has_intent_for_scope: bool, scope: &[String]) -> Result<(), Refusal> {
    if scope.is_empty() {
        return Err(Refusal("dispatch has no scope".into()));
    }
    if !has_intent_for_scope {
        return Err(Refusal("no ledger intent names this dispatch's scope".into()));
    }
    Ok(())
}

/// Invariant 3: no merge unless the round passed and the run was accepted by a human.
pub fn check_merge(round_verdict: Option<Verdict>, run: RunStatus) -> Result<(), Refusal> {
    match round_verdict {
        Some(v) if v.is_passing() => {}
        _ => return Err(Refusal("merge refused: round has no passing verdict".into())),
    }
    if run != RunStatus::Accepted {
        return Err(Refusal(format!("merge refused: run status is {}, not accepted", run.as_str())));
    }
    Ok(())
}

/// Invariant 4: a thread may only write inside its own directory. Lexical, so it works for files that
/// do not exist yet and cannot be fooled by `..` segments.
pub fn check_write_path(thread_dir: &Path, target: &Path) -> Result<(), Refusal> {
    let root = normalize(thread_dir);
    let joined = if target.is_absolute() { target.to_path_buf() } else { thread_dir.join(target) };
    let resolved = normalize(&joined);
    if resolved.starts_with(&root) && resolved != root {
        Ok(())
    } else {
        Err(Refusal(format!("write outside the thread's worktree refused: {}", target.display())))
    }
}

pub fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The loop's no-progress rule: the same non-empty set of failing checks two rounds running.
pub fn is_no_progress(previous_failing: Option<&[String]>, failing: &[String]) -> bool {
    match previous_failing {
        Some(prev) => !failing.is_empty() && prev == failing,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_round_while_previous_round_has_no_verdict() {
        let r = check_round_start(Some(None), 1, 3, 0, 150_000);
        assert_eq!(r, Err(Refusal("previous round has no verdict".into())));
    }

    #[test]
    fn allows_round_after_passing_verdict() {
        assert!(check_round_start(Some(Some(Verdict::Pass)), 1, 3, 0, 150_000).is_ok());
    }

    #[test]
    fn refuses_fourth_round_with_reason() {
        let err = check_round_start(Some(Some(Verdict::Block)), 3, 3, 0, 150_000).unwrap_err();
        assert!(err.0.contains("3 of 3"), "reason was: {err}");
        assert!(err.0.contains("max_rounds = 3"));
    }

    #[test]
    fn refuses_round_when_token_budget_spent() {
        let err = check_round_start(None, 0, 3, 150_000, 150_000).unwrap_err();
        assert!(err.0.starts_with("token budget exhausted"));
    }

    #[test]
    fn refuses_dispatch_without_scope() {
        assert!(check_dispatch(true, &[]).is_err());
    }

    #[test]
    fn refuses_dispatch_without_intent() {
        let r = check_dispatch(false, &["research/09-spike.md".into()]);
        assert_eq!(r, Err(Refusal("no ledger intent names this dispatch's scope".into())));
    }

    #[test]
    fn allows_dispatch_with_scope_and_intent() {
        assert!(check_dispatch(true, &["x".into()]).is_ok());
    }

    #[test]
    fn refuses_merge_unless_accepted() {
        assert!(check_merge(Some(Verdict::Pass), RunStatus::Reviewed).is_err());
        assert!(check_merge(Some(Verdict::Block), RunStatus::Accepted).is_err());
        assert!(check_merge(None, RunStatus::Accepted).is_err());
        assert!(check_merge(Some(Verdict::PassWithNotes), RunStatus::Accepted).is_ok());
    }

    #[test]
    fn refuses_write_outside_own_worktree() {
        let dir = Path::new("/runs/r1/threads/a");
        assert!(check_write_path(dir, Path::new("/runs/r1/threads/a/out.md")).is_ok());
        assert!(check_write_path(dir, Path::new("notes.md")).is_ok());
        assert!(check_write_path(dir, Path::new("/runs/r1/threads/b/out.md")).is_err());
        assert!(check_write_path(dir, Path::new("../b/out.md")).is_err());
        assert!(check_write_path(dir, Path::new("/etc/passwd")).is_err());
        assert!(check_write_path(dir, Path::new("/runs/r1/threads/a/../../../x")).is_err());
    }

    #[test]
    fn no_progress_needs_identical_nonempty_failures_twice() {
        let a = vec!["check-1".to_string()];
        assert!(is_no_progress(Some(&a), &a));
        assert!(!is_no_progress(None, &a));
        assert!(!is_no_progress(Some(&a), &[]));
        assert!(!is_no_progress(Some(&a), &["check-2".to_string()]));
    }

    #[test]
    fn run_status_round_trips_and_terminality() {
        for s in ["planned", "working", "reviewed", "accepted", "merged", "blocked", "human_paused"] {
            assert_eq!(RunStatus::parse(s).map(RunStatus::as_str), Some(s));
        }
        assert!(!RunStatus::Reviewed.is_terminal());
        assert!(RunStatus::Blocked.is_terminal());
    }
}
