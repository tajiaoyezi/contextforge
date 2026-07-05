//! task-52.1 (Phase 52, ADR-053): workspace membership storage — 3-role RBAC foundation.
//!
//! Builds on Phase 51 workspace ownership (ADR-052 owner_id): this module adds a
//! `workspace_members` table that binds a user to a workspace with one of three
//! fixed roles (`admin` / `member` / `viewer`). Closes
//! `[SPEC-DEFER:phase-future.rbac-roles-permissions]` for the storage layer.
//!
//! Scope: membership storage only (CRUD + role CHECK). Proto `MembershipService`
//! (task-52.2), Go `roleMiddleware` (task-52.3), and auto-admin on workspace
//! create (task-52.4) are out of scope here. Permission evaluation (admin-gate)
//! is policy in ADR-053 D3, enforced at the Go middleware layer — this store
//! only persists and reads roles.
//!
//! Pattern: mirrors `core/src/identity/store.rs` (SqliteUserStore) — a separate
//! `membership.db` SQLite file with `Mutex<Connection>` (ADR-016 D1 single-owner-
//! per-DB). No FK to workspaces/users (cross-DB); join is app-level.

pub mod store;

pub use store::{Member, MembershipStoreError, Role, SqliteMembershipStore};
