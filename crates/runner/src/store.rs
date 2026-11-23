//! SQLite access. One command = one transaction: projection writes, journal rows and the receipt
//! commit together or not at all (docs/data-model.md, rule 1).

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;
use std::path::Path;

const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("../migrations/0001_init.sql")),
    ("0002_providers", include_str!("../migrations/0002_providers.sql")),
    ("0003_tasks", include_str!("../migrations/0003_tasks.sql")),
    ("0004_messages", include_str!("../migrations/0004_messages.sql")),
    ("0005_messages", include_str!("../migrations/0005_messages.sql")),
    ("0006_git", include_str!("../migrations/0006_git.sql")),
];

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Result of a command. `Refused` is a real outcome, recorded in the journal, not an error.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome<T> {
    Applied(T),
    Refused(String),
    /// The request_id was already seen; nothing was written. Carries the original outcome text.
    Duplicate(String),
}

impl<T> Outcome<T> {
    /// The refusal reason, if this outcome is a refusal.
    pub fn refusal(&self) -> Option<&str> {
        match self {
            Outcome::Refused(reason) => Some(reason),
            _ => None,
        }
    }
}

pub struct JournalRow {
    pub run_id: String,
    pub round_id: Option<String>,
    pub thread_id: Option<String>,
    pub kind: String,
    pub payload: Value,
}

/// Journal rows a command has produced; flushed inside the command's transaction.
#[derive(Default)]
pub struct Pending {
    rows: Vec<JournalRow>,
}

impl Pending {
    pub fn push(&mut self, run_id: &str, round_id: Option<&str>, thread_id: Option<&str>, kind: &str, payload: Value) {
        self.rows.push(JournalRow {
            run_id: run_id.to_string(),
            round_id: round_id.map(str::to_string),
            thread_id: thread_id.map(str::to_string),
            kind: kind.to_string(),
            payload,
        });
    }
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let conn = Connection::open(db_path).with_context(|| format!("open {}", db_path.display()))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")?;
        migrate(&conn)?;
        Ok(Self { conn })
    }

    /// Read access for projections and replay.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Run one command. `body` writes projections and pushes journal rows; it returns `Refused` (and
    /// should still push a refusal row) when an invariant says no. An `Err` rolls everything back.
    pub fn transact<T>(
        &mut self,
        command: &str,
        request_id: Option<&str>,
        body: impl FnOnce(&Transaction<'_>, &mut Pending) -> Result<Outcome<T>>,
    ) -> Result<Outcome<T>> {
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(rid) = request_id {
            let prior: Option<String> = tx
                .query_row("SELECT detail FROM receipts WHERE request_id = ?1", params![rid], |r| r.get(0))
                .optional()?;
            if let Some(detail) = prior {
                return Ok(Outcome::Duplicate(detail));
            }
        }
        let mut pending = Pending::default();
        let outcome = body(&tx, &mut pending)?;
        let (outcome_name, detail) = match &outcome {
            Outcome::Applied(_) => ("applied", "applied".to_string()),
            Outcome::Refused(reason) => ("refused", reason.clone()),
            Outcome::Duplicate(_) => unreachable!("body never returns Duplicate"),
        };
        for row in &pending.rows {
            tx.execute(
                "INSERT INTO journal (run_id, round_id, thread_id, at, kind, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![row.run_id, row.round_id, row.thread_id, now_ms(), row.kind, row.payload.to_string()],
            )?;
        }
        if let Some(rid) = request_id {
            tx.execute(
                "INSERT INTO receipts (request_id, command, outcome, detail, at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![rid, command, outcome_name, detail, now_ms()],
            )?;
        }
        tx.commit()?;
        Ok(outcome)
    }
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations (version TEXT PRIMARY KEY, applied_at INTEGER NOT NULL);")?;
    for (version, sql) in MIGRATIONS {
        let applied: bool = conn
            .query_row("SELECT EXISTS (SELECT 1 FROM schema_migrations WHERE version = ?1)", params![version], |r| r.get(0))?;
        if applied {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql).with_context(|| format!("migration {version}"))?;
        tx.execute("INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)", params![version, now_ms()])?;
        tx.commit()?;
    }
    Ok(())
}
