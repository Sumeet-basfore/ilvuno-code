-- Stage 2: a run's project may be a git repository. The columns name the checkout the run's threads
-- branch from. Null means folder mode, exactly as before this migration.
ALTER TABLE runs ADD COLUMN git_root TEXT;
ALTER TABLE runs ADD COLUMN git_base TEXT;
