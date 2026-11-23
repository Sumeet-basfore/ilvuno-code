//! `ilvuno note`: the worker's way onto the board. Until this existed the preamble told workers to
//! "announce your intent on the board" while the worker policy permitted exactly one shell command,
//! so every worker burned turns discovering it could not. Both the CLI subcommand and the tests call
//! this, so the behaviour under test is the behaviour the agent gets.

use crate::commands::note;
use crate::home::Home;
use crate::ledger;
use crate::store::{Outcome, Store};
use anyhow::{anyhow, Result};
use rusqlite::params;

pub fn run_note(
    home: &Home,
    task: &str,
    kind: &str,
    body: &str,
    scope: &[String],
) -> Result<Outcome<i64>> {
    let mut store = Store::open(&home.db_path())?;
    // Same note twice is one row: agents retry after a timeout, and a duplicate board entry is noise.
    let request_id = format!("note:{task}:{kind}:{body}:{}", scope.join(","));
    let outcome = store.transact("ilvuno.note", Some(&request_id), |tx, pending| {
        note(tx, pending, task, kind, body, scope)
    })?;
    if let Outcome::Applied(_) = outcome {
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
