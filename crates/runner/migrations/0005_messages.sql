-- A message is a task with no file scope: the composer's 'talk to this agent' turn.
ALTER TABLE dispatch ADD COLUMN kind TEXT NOT NULL DEFAULT 'task';
