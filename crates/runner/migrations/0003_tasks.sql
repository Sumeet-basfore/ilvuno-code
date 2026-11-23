-- Tasks replace the research plan. A thread is an agent plus a prompt; a task is a prompt, a thread and a
-- scope. The dispatch table is the task row (docs/data-model.md).

-- A thread has a name (how a task names it) and a standing prompt.
ALTER TABLE threads ADD COLUMN name TEXT;
ALTER TABLE threads ADD COLUMN prompt TEXT NOT NULL DEFAULT '';
CREATE UNIQUE INDEX threads_run_name ON threads (run_id, name);

-- Who wrote a task: the user, the orchestrator thread, or the runner (a failed round wakes the orchestrator).
ALTER TABLE dispatch ADD COLUMN created_by TEXT NOT NULL DEFAULT 'user';

-- No plan slots any more: a task names its thread directly.
ALTER TABLE dispatch DROP COLUMN slot;

-- No research template, no deliverable shape, no slot assignments. `done` and `checks` live in done_spec.
ALTER TABLE goals DROP COLUMN template;
ALTER TABLE goals DROP COLUMN deliverable;
ALTER TABLE goals DROP COLUMN assignments;
