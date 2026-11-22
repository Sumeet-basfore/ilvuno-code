//! `ilvuno` — headless runner CLI.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use provider_claude::ClaudeProvider;
use runner::commands::{self, ReportStatus};
use runner::driver::{Driver, RunEnd};
use runner::goal::{self, GoalPlan};
use runner::report::run_report;
use runner::run_status::{self, GateDecision};
use runner::store::Outcome;
use runner::{replay, Home, Store};
use std::path::{Path, PathBuf};

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
    /// Show a run: rounds, verdicts, dispatches, artifacts.
    Status { run: String },
    /// Decide at the human gate.
    Gate {
        run: String,
        #[command(subcommand)]
        action: GateAction,
    },
    /// Park a run: no further rounds or dispatches start until it is run again.
    Freeze { run: String },
    /// Print the journal transitions and check them against the projection.
    Replay { run: String },
    /// Worker-only: report the outcome of a dispatch. Agents call this; humans rarely do.
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
}

#[derive(Subcommand)]
enum GateAction {
    /// Accept the round's output. The run becomes accepted.
    Accept,
    /// Send the run back with a note; the next round starts immediately.
    SendBack {
        #[arg(long)]
        note: String,
    },
}

fn home() -> Home {
    let root = std::env::var_os("ILVUNO_HOME").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share/ilvuno")
    });
    Home::new(root)
}

/// Directory of this binary, so workers can run `ilvuno report` by name.
fn own_bin_dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let home = home();
    match cli.command {
        Command::InitDb => {
            Store::open(&home.db_path())?;
            println!("database ready: {}", home.db_path().display());
        }
        Command::Run { goal, resume } => {
            let mut store = Store::open(&home.db_path())?;
            let (run_id, plan) = match (goal, resume) {
                (Some(path), None) => start_run(&mut store, &path)?,
                (None, Some(run_id)) => {
                    let plan = goal::load_snapshot(store.conn(), &run_id)?;
                    (run_id, plan)
                }
                _ => bail!("pass either --goal or --resume"),
            };
            let end = drive(&home, &mut store, &plan, &run_id).await?;
            print_end(&run_id, &end);
            if let RunEnd::Stopped { .. } = end {
                std::process::exit(2);
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
            let outcome = store.transact("gate.decide", None, |tx, p| run_status::gate_decide(tx, p, &run, &decision))?;
            if let Outcome::Refused(reason) = outcome {
                bail!("gate refused: {reason}");
            }
            match decision {
                GateDecision::Accept => println!("run {run} accepted by human"),
                GateDecision::SendBack { .. } => {
                    let plan = goal::load_snapshot(store.conn(), &run)?;
                    let end = drive(&home, &mut store, &plan, &run).await?;
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
                println!("round {} verdict={:?} dispatches={:?}", r.index, r.verdict, r.dispatches);
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

fn start_run(store: &mut Store, goal_path: &Path) -> Result<(String, GoalPlan)> {
    let plan = GoalPlan::load(goal_path).with_context(|| format!("goal {}", goal_path.display()))?;
    let raw = std::fs::read_to_string(goal_path)?;
    let base = goal_path.parent().unwrap_or_else(|| Path::new("."));
    let new_goal = plan.new_goal();
    let run_id = match store.transact("run.create", None, |tx, p| {
        let run_id = commands::create_run(tx, p, &new_goal)?;
        goal::snapshot(tx, &run_id, &raw, base)?;
        Ok(Outcome::Applied(run_id))
    })? {
        Outcome::Applied(id) => id,
        other => bail!("run not created: {other:?}"),
    };
    println!("run {run_id} created from {}", goal_path.display());
    Ok((run_id, plan))
}

async fn drive(home: &Home, store: &mut Store, plan: &GoalPlan, run_id: &str) -> Result<RunEnd> {
    let provider = ClaudeProvider::default();
    let bin_dir = own_bin_dir();
    let driver = Driver {
        provider: &provider,
        home,
        ilvuno_bin_dir: bin_dir.as_deref(),
        max_usd: plan.usd_per_call(),
    };
    driver.drive(store, plan, run_id).await
}

fn print_end(run_id: &str, end: &RunEnd) {
    match end {
        RunEnd::AtGate => println!("run {run_id} reached the human gate: `ilvuno gate {run_id} accept` or `send-back --note ...`"),
        RunEnd::Accepted => println!("run {run_id} is accepted"),
        RunEnd::Paused => println!("run {run_id} is paused"),
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
            for skipped in v["skipped"].as_array().into_iter().flatten() {
                println!("  [skip] {}: {}", skipped["name"].as_str().unwrap_or("?"), skipped["reason"].as_str().unwrap_or(""));
            }
        }
        let mut d = conn.prepare("SELECT slot, status, COALESCE(result_summary, '') FROM dispatch WHERE round_id = ?1 ORDER BY rowid")?;
        for item in d.query_map(rusqlite::params![round_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))? {
            let (slot, status, summary) = item?;
            println!("  dispatch {slot}: {status} {}", summary.chars().take(120).collect::<String>());
        }
    }
    let mut a = conn.prepare("SELECT path, bytes, sha256 FROM artifacts WHERE run_id = ?1 ORDER BY created_at")?;
    for item in a.query_map(rusqlite::params![run], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?)))? {
        let (path, bytes, sha) = item?;
        println!("artifact {path} ({bytes} bytes, sha256 {sha})");
    }
    Ok(())
}
