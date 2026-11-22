-- Schema from docs/data-model.md. Two additions, both required by the docs' own rules:
--   receipts: one row per command request_id (invariant 5: same request_id twice => one receipt).
--   dispatch.slot: the plan slot a dispatch fills (e.g. "researcher-a"), so replay can match plans.

CREATE TABLE goals (
    id TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL,
    title TEXT NOT NULL,
    goal_text TEXT NOT NULL,
    template TEXT NOT NULL,
    deliverable TEXT NOT NULL,
    done_spec TEXT NOT NULL,
    message_policy TEXT NOT NULL DEFAULT 'round',
    budget_rounds INTEGER NOT NULL DEFAULT 3,
    budget_tokens INTEGER NOT NULL DEFAULT 150000,
    budget_usd REAL
);

CREATE TABLE runs (
    id TEXT PRIMARY KEY,
    goal_id TEXT NOT NULL REFERENCES goals(id),
    status TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    rounds_used INTEGER NOT NULL DEFAULT 0,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    blocked_reason TEXT,
    accepted_by TEXT,
    merged_by TEXT
);

CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id),
    role TEXT NOT NULL,
    provider TEXT NOT NULL,
    native_session_id TEXT,
    branch TEXT,
    worktree_path TEXT,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_active_at INTEGER
);

CREATE TABLE rounds (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id),
    "index" INTEGER NOT NULL,
    plan TEXT NOT NULL,
    status TEXT NOT NULL,
    verdict TEXT,
    tokens_delta INTEGER NOT NULL DEFAULT 0,
    cost_delta REAL NOT NULL DEFAULT 0,
    readiness TEXT,
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    UNIQUE (run_id, "index")
);

CREATE TABLE dispatch (
    id TEXT PRIMARY KEY,
    round_id TEXT NOT NULL REFERENCES rounds(id),
    thread_id TEXT NOT NULL REFERENCES threads(id),
    slot TEXT NOT NULL,
    task TEXT NOT NULL,
    scope TEXT NOT NULL,
    status TEXT NOT NULL,
    result_summary TEXT,
    result_json TEXT,
    request_id TEXT UNIQUE,
    created_at INTEGER NOT NULL
);

CREATE TABLE ledger (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL REFERENCES runs(id),
    round_id TEXT REFERENCES rounds(id),
    thread_id TEXT,
    kind TEXT NOT NULL,
    scope TEXT NOT NULL,
    body TEXT NOT NULL,
    evidence TEXT,
    created_at INTEGER NOT NULL
);
CREATE INDEX ledger_run_id ON ledger (run_id, id);
CREATE INDEX ledger_kind ON ledger (kind);
CREATE VIRTUAL TABLE ledger_fts USING fts5 (body, content = 'ledger', content_rowid = 'id');
CREATE TRIGGER ledger_ai AFTER INSERT ON ledger BEGIN
    INSERT INTO ledger_fts (rowid, body) VALUES (new.id, new.body);
END;

CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id),
    round_id TEXT REFERENCES rounds(id),
    from_thread TEXT,
    to_thread TEXT,
    body TEXT NOT NULL,
    delivery TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    delivered_at INTEGER
);

CREATE TABLE artifacts (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id),
    round_id TEXT REFERENCES rounds(id),
    thread_id TEXT,
    kind TEXT NOT NULL,
    path TEXT NOT NULL,
    mime TEXT,
    bytes INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE journal (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    round_id TEXT,
    thread_id TEXT,
    at INTEGER NOT NULL,
    kind TEXT NOT NULL,
    payload TEXT NOT NULL
);
CREATE INDEX journal_run ON journal (run_id);

CREATE TABLE provider_instances (
    id TEXT PRIMARY KEY,
    driver TEXT NOT NULL,
    command TEXT,
    display_name TEXT NOT NULL,
    capabilities TEXT,
    last_seen_at INTEGER
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE receipts (
    request_id TEXT PRIMARY KEY,
    command TEXT NOT NULL,
    outcome TEXT NOT NULL,
    detail TEXT NOT NULL,
    at INTEGER NOT NULL
);
