//! Versioned prompt texts (docs/provider-contract.md). Each is injected on every turn, not once per session,
//! because agents lose them. The journal records the version on each task start. The texts below are the
//! same text the contract document shows; a test fails if they diverge.

pub const WORKER_PREAMBLE_VERSION: &str = "worker-preamble/4";
pub const ORCHESTRATOR_PREAMBLE_VERSION: &str = "orchestrator-preamble/1";

/// Placeholders in `<angle brackets>` are filled by the renderers. `<3 sentences>`, `<paths>` and
/// `ok|failed` stay literal: the agent fills them in its report command.
pub const WORKER_PREAMBLE: &str = r#"You are a worker in Ilvuno, a multi-agent runner. Your thread is <thread>. Your coordinator is <coordinator>.
Your task id is <task_id>. Your round is <round_index>.
Work only inside your folder: <path>. You may read the run folder <run>.

Board: read `<run>/board.md` before you start and before you report.
Message policy for this run: <round|anytime|never>.

Rules:
- Before you touch a file, say so: `<bin> note --task <task_id> --kind intent --body "what you are about to do"`.
- Write only inside your folder. Never switch branches. Never merge.
- When you need a decision you cannot make, ask and stop your turn:
  `<bin> note --task <task_id> --kind question --body "the question"`.
- When you finish: report exactly once with `<bin> report --task <task_id> --status ok|failed
  --summary "<3 sentences>" --files "<paths>"`.
- Do not spawn other agents. Do not poll for work. You will be woken.

=== TASK ===
<task text>"#;

pub const ORCHESTRATOR_PREAMBLE: &str = r#"You are the orchestrator in Ilvuno, a multi-agent runner. Your thread is <thread>.
Your task id is <task_id>. Your round is <round_index>.
Work only inside your folder: <path>. You may read the run folder <run>.

Board: read `<run>/board.md` before you plan and before you report.
Workers on this run: <workers>. The goal folder is <goal folder>.

You plan and delegate. You do not write the deliverable yourself.
Rules:
- Give a worker work with: `<bin> task --thread <worker> --prompt "what to do" --scope "a,b"`.
  Every task needs a prompt and a scope. A task without a scope is refused.
- The scope lists the only files and sources that task may touch. Give a source as an absolute path.
- If two workers would touch the same file, change the plan. Never let them race.
- Ask the human when you need a decision: `<bin> note --task <task_id> --kind question --body "the question"`.
- When your plan is made, report once: `<bin> report --task <task_id> --status ok --summary "<3 sentences>"`.
- You cannot open the gate. The checks decide, and the human decides.
- When a worker reports a failure, do not mark it done. Ask, or escalate to the human.
- Do not spawn other agents. Do not poll for work. You will be woken.

=== TASK ===
<task text>"#;

pub struct WorkerPrompt<'a> {
    pub thread: &'a str,
    pub coordinator: &'a str,
    pub task_id: &'a str,
    pub round_index: u32,
    pub folder: &'a str,
    pub run: &'a str,
    pub message_policy: &'a str,
    pub bin: &'a str,
    pub task: &'a str,
}

pub struct OrchestratorPrompt<'a> {
    pub thread: &'a str,
    pub task_id: &'a str,
    pub round_index: u32,
    pub folder: &'a str,
    pub run: &'a str,
    pub goal_folder: &'a str,
    pub workers: &'a str,
    pub bin: &'a str,
    pub task: &'a str,
}

pub fn render_worker(p: &WorkerPrompt<'_>) -> String {
    WORKER_PREAMBLE
        .replace("<thread>", p.thread)
        .replace("<coordinator>", p.coordinator)
        .replace("<task_id>", p.task_id)
        .replace("<round_index>", &p.round_index.to_string())
        .replace("<path>", p.folder)
        .replace("<run>", p.run)
        .replace("<round|anytime|never>", p.message_policy)
        .replace("<bin>", p.bin)
        .replace("<task text>", p.task)
}

pub fn render_orchestrator(p: &OrchestratorPrompt<'_>) -> String {
    ORCHESTRATOR_PREAMBLE
        .replace("<thread>", p.thread)
        .replace("<task_id>", p.task_id)
        .replace("<round_index>", &p.round_index.to_string())
        .replace("<path>", p.folder)
        .replace("<run>", p.run)
        .replace("<goal folder>", p.goal_folder)
        .replace("<workers>", p.workers)
        .replace("<bin>", p.bin)
        .replace("<task text>", p.task)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worker_text() -> String {
        render_worker(&WorkerPrompt {
            thread: "reader-1",
            coordinator: "the orchestrator thread orchestrator",
            task_id: "t-1",
            round_index: 0,
            folder: "/runs/r/threads/reader-1",
            run: "/runs/r",
            message_policy: "round",
            bin: "/opt/ilvuno",
            task: "DO THE THING",
        })
    }

    #[test]
    fn worker_preamble_keeps_the_report_contract_and_fills_placeholders() {
        let text = worker_text();
        assert!(text.contains("Your thread is reader-1. Your coordinator is the orchestrator thread orchestrator."));
        assert!(text.contains("Your task id is t-1. Your round is 0."));
        assert!(text.contains("/opt/ilvuno report --task t-1 --status ok|failed"));
        assert!(text.contains("--summary \"<3 sentences>\" --files \"<paths>\""));
        assert!(text.contains("Work only inside your folder: /runs/r/threads/reader-1."));
        assert!(text.contains("/opt/ilvuno note --task t-1 --kind intent"));
        assert!(text.contains("Message policy for this run: round."));
        assert!(text.ends_with("=== TASK ===\nDO THE THING"));
        assert!(!text.contains("<task_id>") && !text.contains("<task text>"));
    }

    #[test]
    fn task_text_is_filled_last_so_it_is_never_rewritten() {
        let text = render_worker(&WorkerPrompt {
            thread: "reader-1",
            coordinator: "the user",
            task_id: "t-1",
            round_index: 0,
            folder: "/f",
            run: "/r",
            message_policy: "round",
            bin: "ilvuno",
            task: "quote <thread> and <bin> literally",
        });
        assert!(text.ends_with("quote <thread> and <bin> literally"));
    }

    #[test]
    fn orchestrator_preamble_names_the_workers_and_the_task_command() {
        let text = render_orchestrator(&OrchestratorPrompt {
            thread: "orchestrator",
            task_id: "t-9",
            round_index: 2,
            folder: "/runs/r/threads/orchestrator",
            run: "/runs/r",
            goal_folder: "/home/me/goals",
            workers: "reader-1, reader-2",
            bin: "/opt/ilvuno",
            task: "Goal: x",
        });
        assert!(text.contains("Workers on this run: reader-1, reader-2. The goal folder is /home/me/goals."));
        assert!(text.contains("/opt/ilvuno task --thread <worker> --prompt"));
        assert!(text.contains("/opt/ilvuno report --task t-9 --status ok --summary"));
        assert!(text.contains("You cannot open the gate."));
        assert!(text.ends_with("=== TASK ===\nGoal: x"));
    }

    #[test]
    fn both_versions_are_distinct_and_named() {
        assert_eq!(WORKER_PREAMBLE_VERSION, "worker-preamble/4");
        assert_eq!(ORCHESTRATOR_PREAMBLE_VERSION, "orchestrator-preamble/1");
    }

    #[test]
    fn provider_contract_shows_both_prompt_texts_verbatim() {
        let doc = include_str!("../../../../docs/provider-contract.md");
        assert!(doc.contains(WORKER_PREAMBLE), "docs/provider-contract.md must show worker-preamble/4 verbatim");
        assert!(doc.contains(ORCHESTRATOR_PREAMBLE), "docs/provider-contract.md must show orchestrator-preamble/1 verbatim");
    }
}
