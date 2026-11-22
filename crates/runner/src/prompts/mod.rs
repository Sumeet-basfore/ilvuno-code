//! Versioned prompt texts (docs/provider-contract.md). The worker preamble is injected on every turn;
//! the journal records `WORKER_PREAMBLE_VERSION` on each dispatch start.

pub const WORKER_PREAMBLE_VERSION: &str = "worker-preamble/1";

/// Verbatim from docs/provider-contract.md, with `<placeholders>` filled by [`render_worker_prompt`].
/// `<3 sentences>` and `<paths>` stay literal: the worker fills them in its report command.
const WORKER_PREAMBLE: &str = "You are a worker in Ilvuno, a multi-agent runner. Your coordinator is thread <id>.
Your task id is <task_id>. Your round is <round_index>.
Your branch is <branch>. Work only inside your worktree: <path>.

Board: read `<run>/board.md` before you start and before you report.
Message policy for this run: <round|anytime|never>.

Rules:
- Announce your intent on the board before you touch a file.
- Write only inside your worktree. Never switch branches. Never merge.
- When you need a decision you cannot make: write a `question` to the board and stop your turn.
- When you finish: report exactly once with `ilvuno report --task <task_id> --status ok|failed
  --summary \"<3 sentences>\" --files \"<paths>\"`.
- Do not spawn other agents. Do not poll for work. You will be woken.

=== TASK ===
<task text>";

pub struct WorkerPrompt<'a> {
    pub coordinator: &'a str,
    pub task_id: &'a str,
    pub round_index: u32,
    pub branch: &'a str,
    pub worktree: &'a str,
    pub run_board: &'a str,
    pub message_policy: &'a str,
    pub task: &'a str,
}

pub fn render_worker_prompt(p: &WorkerPrompt<'_>) -> String {
    WORKER_PREAMBLE
        .replace("<id>", p.coordinator)
        .replace("<task_id>", p.task_id)
        .replace("<round_index>", &p.round_index.to_string())
        .replace("<branch>", p.branch)
        .replace("<path>", p.worktree)
        .replace("<run>", p.run_board)
        .replace("<round|anytime|never>", p.message_policy)
        .replace("<task text>", p.task)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preamble_keeps_the_report_contract_and_fills_placeholders() {
        let text = render_worker_prompt(&WorkerPrompt {
            coordinator: "runner",
            task_id: "t-1",
            round_index: 0,
            branch: "none",
            worktree: "/runs/r/threads/x",
            run_board: "/runs/r",
            message_policy: "round",
            task: "DO THE THING",
        });
        assert!(text.contains("Your task id is t-1. Your round is 0."));
        assert!(text.contains("ilvuno report --task t-1 --status ok|failed"));
        assert!(text.contains("--summary \"<3 sentences>\" --files \"<paths>\""));
        assert!(text.contains("Work only inside your worktree: /runs/r/threads/x."));
        assert!(text.contains("Message policy for this run: round."));
        assert!(text.ends_with("=== TASK ===\nDO THE THING"));
        assert!(!text.contains("<task_id>") && !text.contains("<task text>"));
    }
}
