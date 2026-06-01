-- task-26.1 (Phase 26 / ADR-031 D1): FTS5 shadow table for content search over
-- search_traces. The readable trace text (RetrievalTrace.query) lives inside the
-- base64-encoded prost blob in search_traces.trace_json, so a pure-SQL trigger
-- cannot extract it. The shadow table therefore stores query_id (UNINDEXED join
-- key) + query_text (full-text indexed) and is synced explicitly Rust-side on
-- put / prune (SqliteTracePersist), with a one-time boot backfill that decodes
-- existing trace_json rows (see search_persist.rs::open).
--
-- FTS5 is bundled with rusqlite (features=["bundled"]) — 0 new dependency, 0
-- network (ADR-004 / ADR-008). IF NOT EXISTS keeps the migration idempotent on
-- daemon boot (承 0015 pattern); old search_traces.db (0015-only schema) gets
-- the FTS table created here, then backfilled by open().
CREATE VIRTUAL TABLE IF NOT EXISTS search_traces_fts USING fts5(
    query_id UNINDEXED,
    query_text
);
