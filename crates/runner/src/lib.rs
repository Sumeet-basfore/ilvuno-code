//! Ilvuno runner: journal, ledger, rounds, dispatch, invariants, SQLite (docs/data-model.md).

pub mod checks;
pub mod commands;
pub mod dispatch;
pub mod driver;
pub mod execution;
pub mod goal;
pub mod prompts;
pub mod replay;
pub mod report;
pub mod research;
pub mod second_opinion;
pub mod run_status;
pub mod home;
pub mod ledger;
pub mod note;
pub mod policy;
pub mod state;
pub mod store;

pub use home::Home;
pub use state::{RunStatus, Verdict};
pub use store::{Outcome, Store};
