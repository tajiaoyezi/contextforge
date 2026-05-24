-- ContextForge Core migration 0011 (task-10.3 / ADR-015 §D3)
-- index_jobs table: Console Contract v1 IndexJob resource — async lifecycle
-- (queued → running → succeeded|failed|cancelled) + heartbeat + co-operative
-- cancel via cancel_requested flag.

CREATE TABLE IF NOT EXISTS index_jobs (
    job_id                    TEXT PRIMARY KEY NOT NULL,
    workspace_id              TEXT NOT NULL,
    trigger_source            TEXT NOT NULL,                    -- cli | rest | mcp | console-web
    status                    TEXT NOT NULL,                    -- queued | running | succeeded | failed | cancelled
    stage                     TEXT NOT NULL DEFAULT '',         -- parse | chunk | embed | index | done
    processed_files           INTEGER NOT NULL DEFAULT 0,
    total_files               INTEGER NOT NULL DEFAULT 0,
    failed_files              INTEGER NOT NULL DEFAULT 0,
    skipped_files             INTEGER NOT NULL DEFAULT 0,
    error_message             TEXT,
    started_at_unix           INTEGER,                          -- nullable; Unix epoch seconds
    finished_at_unix          INTEGER,                          -- nullable; Unix epoch seconds
    last_heartbeat_at_unix    INTEGER,                          -- nullable; Unix epoch seconds
    cancel_requested          INTEGER NOT NULL DEFAULT 0,       -- 0 = false, 1 = true (co-operative cancel flag)
    FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id)
);

CREATE INDEX IF NOT EXISTS idx_index_jobs_workspace_id ON index_jobs (workspace_id);
CREATE INDEX IF NOT EXISTS idx_index_jobs_status       ON index_jobs (status);
