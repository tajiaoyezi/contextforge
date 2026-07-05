-- task-52.1 (Phase 52 / ADR-053 D2): workspace membership table for 3-role RBAC.
-- Each row binds a user to a workspace with one of 3 fixed roles (admin/member/viewer).
-- workspace owner (Phase 51 owner_id) auto-gets admin membership at create time (task-52.4).
-- PK(workspace_id, user_id) prevents duplicate memberships. CHECK constrains role to the
-- 3-value enum. No FK to workspaces/users (cross-DB: membership.db is separate from
-- workspaces.db + users.db per ADR-016 D1 single-owner-per-DB) — app-level join.
CREATE TABLE IF NOT EXISTS workspace_members (
    workspace_id    TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    role            TEXT NOT NULL CHECK(role IN ('admin','member','viewer')),
    created_at_unix INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (workspace_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_workspace_members_user ON workspace_members(user_id);
