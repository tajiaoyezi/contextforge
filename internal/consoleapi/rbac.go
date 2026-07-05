package consoleapi

import "net/http"

// task-52.3 (Phase 52 / ADR-053): role-based admin-gate for destructive + workspace
// management endpoints. The gate is applied selectively to endpoints that HAVE a
// workspace_id in their path (PATCH /v1/workspaces/{id}/config, POST/DELETE
// /v1/workspaces/{id}/members). Memory destructive ops + user management do NOT
// carry a workspace_id in the REST path → fail-open with a documented TODO.
//
// Behavior contract (§4.1 of task-52.3):
//   - trusted-network (empty token) → admin (no verified identity → byte-equiv)
//   - legacy shared token            → admin (no verified identity → byte-equiv)
//   - per-user token, role=="admin" → allowed
//   - per-user token, role!="admin" → blocked (caller returns 403)
//
// requireAdmin checks whether the verified user (resolved by bearerAuthMiddleware
// from a per-user token) has the "admin" role on the given workspace.
//
// Returns true when access is allowed (admin role OR trusted-network/legacy token
// OR membership service unavailable / no workspace context → fail-open). Returns
// false ONLY when the user is verified AND definitively NOT an admin → the caller
// must return 403 Forbidden. The fail-open branches keep byte-equivalence with
// v1.x for trusted-network / legacy paths and avoid blocking ops on infra issues.
func requireAdmin(deps Deps, r *http.Request, workspaceID string) bool {
	verifiedUser, _ := r.Context().Value(verifiedUserIDKey{}).(string)
	if verifiedUser == "" {
		// trusted-network (empty token) OR legacy shared token — neither injects a
		// verified identity (bearerAuthMiddleware skips per-user resolution). Both
		// are treated as admin (byte-equivalent with v1.x).
		return true
	}
	// No membership service wired (inmem-fallback / degraded) OR no workspace
	// context available → fail-open (cannot check → allow, documented in §3 of task).
	if deps.Membership == nil || workspaceID == "" {
		return true
	}
	role, err := deps.Membership.GetMyRole(workspaceID, verifiedUser)
	if err != nil {
		// Fail-open on errors (don't block on infra issues — GetMyRole not_found is
		// already mapped to "" + nil by the client wrapper, so this is a real error).
		return true
	}
	return role == "admin"
}

// requireAdminAnyWorkspace gates endpoints WITHOUT a workspace_id in their REST
// path (memory destructive ops / user management). Per the pragmatic scope in
// task-52.3 §3, these fail-open (allow) for now because the workspace context is
// not available in the request — a documented TODO. The helper still short-circuits
// to admin for trusted-network / legacy token (byte-equivalence preserved) and is
// kept here so a future task can tighten it without touching every call site.
//
// TODO(task-future.rbac-memory-user-workspace-context): tighten the memory
// destructive + user-management admin-gate once the workspace context is threaded
// through these paths (e.g. memory items carry workspace_id, or a global admin
// role). Until then this is fail-open for verified non-admin users.
func requireAdminAnyWorkspace(deps Deps, r *http.Request) bool {
	verifiedUser, _ := r.Context().Value(verifiedUserIDKey{}).(string)
	if verifiedUser == "" {
		// trusted-network / legacy shared token → admin (byte-equiv).
		return true
	}
	// Pragmatic fail-open: no workspace context available in the REST path → cannot
	// check role. Documented TODO above. (deps is accepted for symmetry with
	// requireAdmin + a future tightening; intentionally not used right now.)
	_ = deps
	return true
}
