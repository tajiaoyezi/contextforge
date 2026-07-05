-- task-50.1 (Phase 50 / ADR-051 D2): per-user identity table for verified actor propagation.
-- Closes [SPEC-DEFER:phase-future.memory-actor-authenticated-identity] (actor declared → verified).
--
-- Schema mirrors the local-first SQLite pattern (ADR-004/016 D1: Rust is sole SQLite owner).
-- Token stored plaintext (local-first compromise; hash storage is
-- [SPEC-DEFER:phase-future.token-hash-storage] Phase 51+ — needs salt + HMAC evaluation).
--
-- Column semantics:
--   id              — stable user identifier (caller-generated UUID / slug; surfaced as the verified actor)
--   name            — human-readable label (not unique; display only)
--   token           — bearer token presented at the REST layer; UNIQUE (one token → one user)
--   created_at_unix — insertion time (seconds since epoch; 0 if unset)
CREATE TABLE IF NOT EXISTS users (
    id              TEXT    PRIMARY KEY NOT NULL,
    name            TEXT    NOT NULL DEFAULT '',
    token           TEXT    NOT NULL UNIQUE,
    created_at_unix INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_users_token ON users(token);
