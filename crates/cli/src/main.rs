//! `ilvuno` — headless runner CLI.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use cli::ops;
use runner::commands::ReportStatus;
use runner::driver::RunEnd;
use runner::goal::{self, GoalPlan};
use runner::note::run_note;
use runner::report::run_report;
use runner::run_status::{self, GateDecision};
use runner::store::Outcome;
use runner::tasks::run_task;
use runner::{replay, Home, Store};
use std::path::{Path, PathBuf};

/// Set by the runner on the orchestrator's session: the task that is calling `ilvuno task`.
const CALLER_TASK_ENV: &str = "ILVUNO_TASK";

#[derive(Parser)]
#[command(name = "ilvuno", version, about = "Headless multi-agent round runner")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create or migrate the database under ILVUNO_HOME.
    InitDb,
    /// Start a run from a goal file, or continue one from the journal with --resume.
    Run {
        #[arg(long, conflicts_with = "resume", required_unless_present = "resume")]
        goal: Option<PathBuf>,
        #[arg(long)]
        resume: Option<String>,
    },
    /// Add a task for a thread. It joins the open round, or opens the next one. Scope is what the task may touch.
    Task {
        #[arg(long)]
        thread: String,
        #[arg(long)]
        prompt: String,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        scope: Vec<String>,
    },
    /// Show a run: rounds, checks, tasks, artifacts.
    Status { run: String },
    /// Decide at the human gate.
    Gate {
        run: String,
        #[command(subcommand)]
        action: GateAction,
    },
    /// Park a run: no further rounds or tasks start until it is run again.
    Freeze { run: String },
    /// Print the journal transitions and check them against the projection.
    Replay { run: String },
    /// Write one run as plain files: run.json, board.md, journal.jsonl, artifacts/.
    Export {
        run: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Agent-only: report the outcome of a task. Agents call this; humans rarely do.
    Report {
        #[arg(long)]
        task: String,
        #[arg(long, value_parser = ["ok", "failed"])]
        status: String,
        #[arg(long)]
        summary: String,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        files: Vec<String>,
    },
    /// Agent-only: append a note (intent, finding, question, answer) to the run board.
    Note {
        #[arg(long)]
        task: String,
        #[arg(long, value_parser = ["intent", "finding", "question", "answer"])]
        kind: String,
        #[arg(long)]
        body: String,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        scope: Vec<String>,
    },
}

#[derive(Subcommand)]
enum GateAction {
    /// Accept the round's output. The run becomes accepted.
    Accept,
    /// Send the run back with a note. The orchestrator, if there is one, plans the next round.
    SendBack {
        #[arg(long)]
        note: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let home = ops::home_from_env();
    match cli.command {
        Command::InitDb => {
            Store::open(&home.db_path())?;
            println!("database ready: {}", home.db_path().display());
        }
        Command::Run { goal, resume } => {
            let mut store = Store::open(&home.db_path())?;
            let (run_id, plan) = match (goal, resume) {
                (Some(path), None) => start_run(&mut store, &home, &path)?,
                (None, Some(run_id)) => {
                    let plan = goal::load_snapshot(store.conn(), &run_id)?;
                    (run_id, plan)
                }
                _ => bail!("pass either --goal or --resume"),
            };
            let end = ops::drive(&home, &mut store, &plan, &run_id).await?;
            print_end(&run_id, &end);
            if let RunEnd::Stopped { .. } = end {
                std::process::exit(2);
            }
        }
        Command::Task { thread, prompt, scope } => {
            let caller = std::env::var(CALLER_TASK_ENV).ok().filter(|s| !s.trim().is_empty());
            let scope: Vec<String> = scope.into_iter().filter(|s| !s.trim().is_empty()).collect();
            match run_task(&home, caller.as_deref(), &thread, &prompt, &scope)? {
                Outcome::Applied(task) => {
                    println!("task {} queued on {} in round {}", task.task_id, task.thread, task.round_index)
                }
                Outcome::Refused(reason) => {
                    eprintln!("task refused: {reason}");
                    std::process::exit(1);
                }
                Outcome::Duplicate(_) => {}
            }
        }
        Command::Status { run } => {
            let store = Store::open(&home.db_path())?;
            print_status(&store, &run)?;
        }
        Command::Gate { run, action } => {
            let mut store = Store::open(&home.db_path())?;
            let decision = match action {
                GateAction::Accept => GateDecision::Accept,
                GateAction::SendBack { note } => GateDecision::SendBack { note },
            };
            if let Outcome::Refused(reason) = run_status::decide_gate_and_render(&home, &mut store, &run, &decision)? {
                bail!("gate refused: {reason}");
            }
            match decision {
                GateDecision::Accept => println!("run {run} accepted by human"),
                GateDecision::SendBack { .. } => {
                    let plan = goal::load_snapshot(store.conn(), &run)?;
                    let end = ops::drive(&home, &mut store, &plan, &run).await?;
                    print_end(&run, &end);
                }
            }
        }
        Command::Freeze { run } => {
            let mut store = Store::open(&home.db_path())?;
            match store.transact("freeze", None, |tx, p| run_status::freeze(tx, p, &run))? {
                Outcome::Applied(()) => println!("run {run} paused"),
                Outcome::Refused(reason) => bail!("freeze refused: {reason}"),
                Outcome::Duplicate(_) => {}
            }
        }
        Command::Replay { run } => {
            let store = Store::open(&home.db_path())?;
            let replayed = replay::replay(store.conn(), &run)?;
            for line in &replayed.transitions {
                println!("{line}");
            }
            println!("--- rounds ---");
            for r in &replayed.rounds {
                println!("round {} verdict={:?} tasks={:?}", r.index, r.verdict, r.dispatches);
            }
            let problems = replay::compare_with_projection(store.conn(), &run, &replayed)?;
            if problems.is_empty() {
                println!("projection: MATCH ({} rounds, {} refusals)", replayed.rounds.len(), replayed.refusals.len());
            } else {
                for p in &problems {
                    println!("projection MISMATCH: {p}");
                }
                std::process::exit(1);
            }
        }
        Command::Note { task, kind, body, scope } => {
            let scope: Vec<String> = scope.into_iter().filter(|s| !s.trim().is_empty()).collect();
            match run_note(&home, &task, &kind, &body, &scope)? {
                Outcome::Applied(id) => println!("noted {kind} #{id} for {task}"),
                Outcome::Duplicate(_) => println!("already noted {task}"),
                Outcome::Refused(reason) => {
                    eprintln!("note refused: {reason}");
                    std::process::exit(1);
                }
            }
        }
        Command::Export { run, out } => {
            let store = Store::open(&home.db_path())?;
            let dir = runner::export::export(&store, &home, &run, &out)?;
            println!("exported {run} to {}", dir.display());
        }
        Command::Report { task, status, summary, files } => {
            let files: Vec<String> = files.into_iter().filter(|f| !f.trim().is_empty()).collect();
            let status = if status == "ok" { ReportStatus::Ok } else { ReportStatus::Failed };
            match run_report(&home, &task, status, &summary, &files)? {
                Outcome::Applied(()) => println!("reported {task}"),
                Outcome::Duplicate(_) => println!("already reported {task}"),
                Outcome::Refused(reason) => {
                    eprintln!("report refused: {reason}");
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}

fn start_run(store: &mut Store, home: &Home, goal_path: &Path) -> Result<(String, GoalPlan)> {
    // The goal's folder is where its relative paths resolve, so it is read from the file's location.
    let probe = GoalPlan::load(goal_path).with_context(|| format!("goal {}", goal_path.display()))?;
    let raw = std::fs::read_to_string(goal_path)?;
    let base = probe.base().to_path_buf();
    let (run_id, plan) = ops::create_run(store, home, &raw, |_| base.clone())?;
    println!("run {run_id} created from {}", goal_path.display());
    Ok((run_id, plan))
}

fn print_end(run_id: &str, end: &RunEnd) {
    match end {
        RunEnd::AtGate => println!("run {run_id} reached the human gate: `ilvuno gate {run_id} accept` or `send-back --note ...`"),
        RunEnd::Accepted => println!("run {run_id} is accepted"),
        RunEnd::Paused => println!("run {run_id} is paused"),
        RunEnd::Idle => println!(
            "run {run_id} is waiting for tasks. Add one with `ilvuno task --thread <name> --prompt ... --scope ...`, \
             then `ilvuno run --resume {run_id}`"
        ),
        RunEnd::Stopped { status, reason } => println!("run {run_id} stopped: {} ({reason})", status.as_str()),
    }
}

fn print_status(store: &Store, run: &str) -> Result<()> {
    let conn = store.conn();
    let (status, rounds_used, tokens): (String, i64, i64) = conn.query_row(
        "SELECT status, rounds_used, tokens_used FROM runs WHERE id = ?1",
        rusqlite::params![run],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    println!("run {run}: {status} (rounds used {rounds_used}, tokens {tokens})");
    let mut rounds = conn.prepare("SELECT id, \"index\", verdict, readiness FROM rounds WHERE run_id = ?1 ORDER BY \"index\"")?;
    for row in rounds.query_map(rusqlite::params![run], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, Option<String>>(3)?))
    })? {
        let (round_id, index, verdict, readiness) = row?;
        println!("round {index}: verdict {}", verdict.as_deref().unwrap_or("(open)"));
        if let Some(text) = readiness {
            let v: serde_json::Value = serde_json::from_str(&text)?;
            for check in v["checks"].as_array().into_iter().flatten() {
                let mark = if check["passed"] == serde_json::Value::Bool(true) { "pass" } else { "FAIL" };
                println!("  [{mark}] {}: {}", check["name"].as_str().unwrap_or("?"), check["detail"].as_str().unwrap_or(""));
            }
        }
        let mut d = conn.prepare(
            "SELECT t.name, d.status, COALESCE(d.result_summary, ''), d.created_by FROM dispatch d
             JOIN threads t ON t.id = d.thread_id WHERE d.round_id = ?1 ORDER BY d.rowid",
        )?;
        for item in d.query_map(rusqlite::params![round_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
        })? {
            let (thread, status, summary, by) = item?;
            println!("  task on {thread} ({by}): {status} {}", summary.chars().take(120).collect::<String>());
        }
    }
    let mut a = conn.prepare("SELECT path, bytes, sha256 FROM artifacts WHERE run_id = ?1 ORDER BY created_at")?;
    for item in a.query_map(rusqlite::params![run], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?)))? {
        let (path, bytes, sha) = item?;
        println!("artifact {path} ({bytes} bytes, sha256 {sha})");
    }
    Ok(())
}
