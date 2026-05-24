-- task-13.1 (ADR-017 D1 Wave 3 / ADR-016 D5): memory_items table.
-- 9 columns 1:1 mirror contractv1.MemoryItem + orthogonal is_pinned flag
-- (Console contract `status` field is 3-state active/deprecated/soft_deleted;
-- pin is orthogonal so a separate column keeps both axes independent).
CREATE TABLE IF NOT EXISTS memory_items (
    memory_id TEXT PRIMARY KEY NOT NULL,
    agent_scope TEXT NOT NULL DEFAULT '',
    content_preview TEXT NOT NULL DEFAULT '',
    source_type TEXT NOT NULL DEFAULT '',
    source_ref TEXT NOT NULL DEFAULT '',
    created_at_unix INTEGER NOT NULL,
    updated_at_unix INTEGER NOT NULL,
    hit_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'deprecated', 'soft_deleted')),
    is_pinned INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_memory_agent_scope ON memory_items(agent_scope);
CREATE INDEX IF NOT EXISTS idx_memory_status ON memory_items(status);
CREATE INDEX IF NOT EXISTS idx_memory_created_at ON memory_items(created_at_unix);
