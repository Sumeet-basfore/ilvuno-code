//! `ilvuno report`: the one path a worker uses to finish a dispatch. Both the CLI subcommand and tests
//! call this, so the behaviour under test is the behaviour the agent gets.

use crate::commands::{report_dispatch, ReportStatus};
use crate::home::Home;
use crate::ledger;
use crate::store::{Outcome, Store};
use anyhow::{anyhow, Result};
use rusqlite::params;

pub fn run_report(
    home: &Home,
    task: &str,
    status: ReportStatus,
    summary: &str,
    files: &[String],
) -> Result<Outcome<()>> {
    let mut store = Store::open(&home.db_path())?;
    // Identical retries carry the same key and are deduplicated; a different report for the same task
    // is refused because the dispatch is no longer open.
    let request_id = format!("report:{task}:{status:?}:{summary}:{}", files.join(","));
    let outcome = store.transact("ilvuno.report", Some(&request_id), |tx, pending| {
        report_dispatch(tx, pending, task, status, summary, files)
    })?;
    if let Outcome::Applied(()) = outcome {
        let run_id: String = store.conn().query_row(
            "SELECT r.run_id FROM dispatch d JOIN rounds r ON r.id = d.round_id WHERE d.id = ?1",
            params![task],
            |r| r.get(0),
        )?;
        ledger::render_board(store.conn(), &run_id, &home.board_path(&run_id))
            .map_err(|e| anyhow!("board render: {e}"))?;
    }
    Ok(outcome)
}
