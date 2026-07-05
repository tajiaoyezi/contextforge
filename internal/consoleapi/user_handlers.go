package consoleapi

import (
	"encoding/json"
	"errors"
	"net/http"
)

// verifiedActor resolves the effective actor for a memory pin/unpin op (task-50.3 / ADR-051).
// When the bearer middleware injected a verified user id (per-user token path), that id
// OVERRIDES the caller-declared X-Actor header value — closing
// [SPEC-DEFER:phase-future.memory-actor-authenticated-identity]. When no verified identity
// is present (trusted-network / legacy shared token), the declared X-Actor is used as-is
// (byte-equivalent v1.x).
func verifiedActor(r *http.Request, declared string) string {
	if v, ok := r.Context().Value(verifiedUserIDKey{}).(string); ok && v != "" {
		return v
	}
	return declared
}

// task-50.3 (Phase 50 / ADR-051): per-user identity registration endpoints.
//
//	POST /v1/users        — register a new user (id + name + token), returns the User
//	GET  /v1/users        — list all users (admin; returns 200 even when empty)
//
// These sit OUTSIDE the v1.0 22-endpoint Console Contract (which is frozen — ADR-015).
// They are add-only new routes; existing routes/contract are untouched.
// Authorization: trusted-network (empty token) OR any valid bearer (user token OR the
// legacy shared token). Admin-role gating is deferred ([SPEC-DEFER:phase-future.rbac-roles-permissions]).
//
// task-52.3 (Phase 52 / ADR-053): admin-gate applied via requireAdminAnyWorkspace. User management
// has NO workspace context in the REST path (/v1/users is global), so the gate cannot resolve a
// specific workspace role → fail-open with a documented TODO (rbac.go). Trusted-network / legacy
// shared token → admin (byte-equiv). The TODO closes once a global-admin role model lands.

type createUserBody struct {
	ID    string `json:"id"`
	Name  string `json:"name"`
	Token string `json:"token"`
}

func handleCreateUser(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.User == nil {
			writeError(w, http.StatusServiceUnavailable, "DATA_PLANE_UNAVAILABLE",
				"user service not wired (inmem-fallback / degraded mode)")
			return
		}
		// task-52.3 admin-gate: fail-open (no workspace context; see rbac.go TODO).
		if !requireAdminAnyWorkspace(deps, r) {
			writeError(w, http.StatusForbidden, "FORBIDDEN",
				"admin role required to create users (Phase 52 RBAC)")
			return
		}
		var body createUserBody
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "invalid JSON body: "+err.Error())
			return
		}
		if body.ID == "" || body.Token == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "id and token are required")
			return
		}
		user, err := deps.User.Create(body.ID, body.Name, body.Token)
		if err != nil {
			if errors.Is(err, ErrConflict) {
				writeError(w, http.StatusConflict, "CONFLICT", "user with this id or token already exists")
				return
			}
			writeError(w, http.StatusInternalServerError, "INTERNAL", err.Error())
			return
		}
		writeJSON(w, http.StatusCreated, user)
	}
}

func handleListUsers(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.User == nil {
			writeError(w, http.StatusServiceUnavailable, "DATA_PLANE_UNAVAILABLE",
				"user service not wired (inmem-fallback / degraded mode)")
			return
		}
		// task-52.3 admin-gate: fail-open (no workspace context; see rbac.go TODO).
		if !requireAdminAnyWorkspace(deps, r) {
			writeError(w, http.StatusForbidden, "FORBIDDEN",
				"admin role required to list users (Phase 52 RBAC)")
			return
		}
		users, err := deps.User.List()
		if err != nil {
			writeError(w, http.StatusInternalServerError, "INTERNAL", err.Error())
			return
		}
		// empty list → 200 with [] (not null), matching the v1.x list-endpoint convention.
		if users == nil {
			users = []User{}
		}
		writeJSON(w, http.StatusOK, users)
	}
}
