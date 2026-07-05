-- task-51.1 (Phase 51 / ADR-052): add owner_id column to workspaces for per-user ownership.
-- Guarded ALTER TABLE (SQLite lacks ADD COLUMN IF NOT EXISTS pre-3.35; the store checks
-- PRAGMA table_info first and only runs this when owner_id is absent — same pattern as
-- 0017_memory_items_add_pin_actor.sql).
-- NULL owner_id = "unowned" (trusted-network + any verified user can see; transitional
-- for existing data backfill). New workspaces created by a verified user set owner_id = userID.
ALTER TABLE workspaces ADD COLUMN owner_id TEXT;
