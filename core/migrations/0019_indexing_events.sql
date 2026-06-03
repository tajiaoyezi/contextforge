-- task-33.3 (Phase 33 / ADR-038 D3): indexing_events table — a persistent
-- replay source for indexing.* lifecycle events. Phase 11/26 emit
-- indexing.progress / .cancelled / .error to the in-memory EventBus only
-- (best-effort broadcast; lost on restart), and the audit_log replay source
-- (0010) cannot carry these because AuditLogEntry has no job_id / processed /
-- total columns. A dedicated table mirrors the indexing lifecycle 1:1.
--
-- Schema is internal-only; not exposed via contractv1 / proto (ADR-015 D1).
-- id is the deterministic ordering key (INTEGER PRIMARY KEY = rowid alias,
-- monotonic) so replay rebuilds events in id ASC order with a stable
-- evt-idx-{id} dedup id (mirrors the 0010 audit_log evt-audit-{id} pattern).
CREATE TABLE IF NOT EXISTS indexing_events (
    id         INTEGER PRIMARY KEY,
    job_id     TEXT NOT NULL,
    stage      TEXT NOT NULL,                 -- 'indexing' | 'cancelled' | 'error'
    processed  INTEGER NOT NULL DEFAULT 0,
    total      INTEGER NOT NULL DEFAULT 0,
    message    TEXT NOT NULL DEFAULT '',
    ts_unix    INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_indexing_events_job_id ON indexing_events (job_id);
