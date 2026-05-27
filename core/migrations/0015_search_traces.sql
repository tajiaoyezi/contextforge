-- task-16.1 (Phase 16 P4 #10): search_traces table for TraceStore SQLite
-- persistence. 5 cols 1:1 mirror Rust TraceRecord wrapper (PbRetrievalTrace
-- prost-encoded bytes → base64 TEXT) + 1 index for ts_unix DESC ordering.
--
-- Schema is internal-only; not exposed via contractv1 (ADR-015 D1 add-only).
CREATE TABLE IF NOT EXISTS search_traces (
    query_id      TEXT PRIMARY KEY NOT NULL,
    trace_json    TEXT NOT NULL,
    workspace_id  TEXT NOT NULL,
    ts_unix       INTEGER NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_search_traces_ts_desc ON search_traces (ts_unix DESC);
