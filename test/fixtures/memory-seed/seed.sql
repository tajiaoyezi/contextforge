-- task-13.2 (ADR-017 D1 Wave 3) memory_items smoke fixture.
-- 5 rows covering: active + deprecated + soft_deleted statuses,
-- different agent_scope shapes for filter test coverage.
--
-- Apply via: sqlite3 <data_dir>/memory.db < test/fixtures/memory-seed/seed.sql
INSERT OR REPLACE INTO memory_items (
    memory_id, agent_scope, content_preview, source_type, source_ref,
    created_at_unix, updated_at_unix, hit_count, status, is_pinned
) VALUES
    ('mem-seed-1', 'agent-default:session', 'first seeded item — active', 'fixture', 'seed:1', 1700000000, 1700000000, 0, 'active', 0),
    ('mem-seed-2', 'agent-default:project', 'second seeded item — active', 'fixture', 'seed:2', 1700000010, 1700000010, 0, 'active', 0),
    ('mem-seed-3', 'agent-default:global', 'third seeded item — active', 'fixture', 'seed:3', 1700000020, 1700000020, 0, 'active', 0),
    ('mem-seed-4', 'agent-test:session', 'fourth seeded item — deprecated', 'fixture', 'seed:4', 1700000030, 1700000030, 0, 'deprecated', 0),
    ('mem-seed-5', 'agent-test:project', 'fifth seeded item — active', 'fixture', 'seed:5', 1700000040, 1700000040, 0, 'active', 0);
