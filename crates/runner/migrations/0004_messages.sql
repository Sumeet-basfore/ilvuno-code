-- What a thread said, once per task. The streamed deltas are not stored; the final text is enough
-- for a thread view, and the provider keeps the original session.
ALTER TABLE dispatch ADD COLUMN result_text TEXT;
