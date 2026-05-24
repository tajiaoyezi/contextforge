-- task-14.1 (ADR-017 D1 Wave 4 / ADR-016 D5): eval_runs table.
-- 10 columns 1:1 mirror contractv1.EvalRun (config_snapshot / metrics /
-- case_results as JSON TEXT) + 3 indexes + CHECK on status (4 states).
CREATE TABLE IF NOT EXISTS eval_runs (
    eval_run_id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'succeeded', 'failed', 'cancelled')),
    config_snapshot_json TEXT NOT NULL DEFAULT '{}',
    started_at_unix INTEGER NOT NULL,
    finished_at_unix INTEGER,
    metrics_json TEXT NOT NULL DEFAULT '{}',
    case_results_json TEXT NOT NULL DEFAULT '[]',
    schema_version TEXT NOT NULL DEFAULT 'v1',
    dataset_ref TEXT,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_eval_runs_workspace ON eval_runs(workspace_id);
CREATE INDEX IF NOT EXISTS idx_eval_runs_status ON eval_runs(status);
CREATE INDEX IF NOT EXISTS idx_eval_runs_started_at ON eval_runs(started_at_unix);
