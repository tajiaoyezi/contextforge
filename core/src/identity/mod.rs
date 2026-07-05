//! task-50.1 (Phase 50 / ADR-051): per-user identity foundation.
//!
//! Verified identity storage for the actor-propagation seam. Closes
//! `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`: the `X-Actor`
//! HTTP header is caller-declared today; Phase 50 lets the Go REST layer resolve
//! a bearer token to a verified `User` (via the UserService gRPC RPC in task-50.2)
//! and override the declared actor with the authenticated id.
//!
//! Scope: identity storage + resolution only. RBAC / roles / permissions / workspace
//! ownership / Postgres / OAuth-OIDC are explicitly out of scope
//! (`[SPEC-DEFER:phase-future.rbac-roles-permissions]` etc., Phase 51-54+).

pub mod store;

pub use store::{SqliteUserStore, User, UserStoreError};
