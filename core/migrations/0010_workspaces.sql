-- ContextForge Core migration 0010 (task-10.2 / ADR-015 §D2)
-- workspaces table: Console Contract v1 Workspace resource persistence.
-- workspace_id = collection_id (1:1 mapping, v0.3 simplification).
-- See: core/src/workspace/mod.rs (SqliteWorkspaceStore loads this via include_str!).

CREATE TABLE IF NOT EXISTS workspaces (
    workspace_id     TEXT PRIMARY KEY NOT NULL, -- same value as collection_id
    name             TEXT NOT NULL,
    root_path        TEXT NOT NULL,
    status           TEXT NOT NULL,             -- ready | updating | deleted
    config_snapshot  TEXT NOT NULL,             -- JSON serialized opaque blob
    allowlist        TEXT,                      -- JSON array of strings (nullable)
    denylist         TEXT,                      -- JSON array of strings (nullable)
    created_at_unix  INTEGER NOT NULL,          -- Unix epoch seconds (Rust 侧)
    updated_at_unix  INTEGER NOT NULL           -- Unix epoch seconds (Rust 侧)
    -- Go REST handler (task-10.4) 序列化时 time.Unix(sec,0).UTC() → RFC3339 string
    -- 喂 Console JSON wire；trade-off: 避新增 chrono dep（task-10.2 §10 #1）。
);

CREATE INDEX IF NOT EXISTS idx_workspaces_status         ON workspaces (status);
CREATE INDEX IF NOT EXISTS idx_workspaces_created_at     ON workspaces (created_at_unix);
