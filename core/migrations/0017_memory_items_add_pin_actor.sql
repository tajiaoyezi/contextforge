-- task-27.1 (Phase 27 / ADR-032 D1): add-only pin-actor + pinned-at-timestamp
-- columns on memory_items (推进 ADR-022 §Trade-offs `pin_actor` + `memory-pinned-at-timestamp`).
-- ALTER ADD COLUMN with DEFAULT backfills existing rows without rewrite (same
-- pattern as is_pinned DEFAULT 0). Applied via a guarded check in
-- SqliteMemoryStore::open (ALTER ADD COLUMN is not IF-NOT-EXISTS-able, so the
-- store skips this when the column already exists — idempotent across boots).
ALTER TABLE memory_items ADD COLUMN pinned_by TEXT NOT NULL DEFAULT '';
ALTER TABLE memory_items ADD COLUMN pinned_at_unix INTEGER NOT NULL DEFAULT 0;
