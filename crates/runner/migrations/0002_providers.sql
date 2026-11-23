-- Per-run provider choice (docs/data-model.md, goals.providers and goals.assignments).
-- Existing goal rows get an empty object: they ran on the v0 default before this migration.
ALTER TABLE goals ADD COLUMN providers TEXT NOT NULL DEFAULT '{}';
ALTER TABLE goals ADD COLUMN assignments TEXT NOT NULL DEFAULT '{}';
