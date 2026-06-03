-- task-31.3 (ADR-036 D3): add-only per-case eval results subtable.
--
-- Per-case results were stored only as a serialized JSON blob in eval_runs.case_results_json,
-- which cannot be SQL-filtered/aggregated by case (e.g. "this case's score across runs", "passed
-- ratio"). This add-only subtable makes per-case rows queryable. update_case_results double-writes
-- (keeps case_results_json for the existing row_to_run read path); old runs whose subtable rows are
-- empty still read fully via the JSON blob (backward compatible). FK declared (CASCADE) to document
-- the relationship; no existing column/table is altered or dropped.
CREATE TABLE IF NOT EXISTS eval_case_results (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    eval_run_id          TEXT    NOT NULL,
    case_id              TEXT    NOT NULL,
    query                TEXT    NOT NULL,
    expected_chunks_json TEXT    NOT NULL,
    actual_chunks_json   TEXT    NOT NULL,
    score                REAL    NOT NULL,
    passed               INTEGER NOT NULL,
    FOREIGN KEY (eval_run_id) REFERENCES eval_runs(eval_run_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_eval_case_results_run ON eval_case_results(eval_run_id);
CREATE INDEX IF NOT EXISTS idx_eval_case_results_case ON eval_case_results(case_id);
