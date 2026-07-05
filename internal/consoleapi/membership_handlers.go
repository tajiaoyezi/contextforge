package consoleapi

import (
	"net/http"
	"strings"
)

// task-52.3 (Phase 52 / ADR-053): membership management REST endpoints.
//
//	POST   /v1/workspaces/{id}/members          — admin-only: add a member to a workspace
//	GET    /v1/workspaces/{id}/members          — admin/member: list members
//	DELETE /v1/workspaces/{id}/members/{user_id} — admin-only: remove a member
//
// These sit OUTSIDE the v1.0 22-endpoint Console Contract (frozen — ADR-015);
// they are add-only new routes. The admin-gate (requireAdmin) is applied to the
// two destructive endpoints (POST + DELETE); GET is read-only (anyone with a
// valid token may list). Trusted-network / legacy shared token → admin
// (byte-equivalent; see rbac.go requireAdmin).

type addMemberBody struct {
	UserID string `json:"user_id"`
	Role   string `json:"role"` // "admin" | "member" | "viewer"; defaults to "member"
}

// handleAddMember — POST /v1/workspaces/{id}/members (admin-only).
// Body shape: {"user_id": "...", "role": "admin|member|viewer"}. role defaults to
// "member" when omitted. Returns 201 + the added Member on success.
func handleAddMember(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.Membership == nil {
			writeError(w, http.StatusServiceUnavailable, "SERVICE_UNAVAILABLE", ErrDataPlaneUnavailable.Error())
			return
		}
		workspaceID := trimID(r)
		if workspaceID == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing workspace id")
			return
		}
		// admin-gate: only workspace admins (or trusted-network/legacy) may add members.
		if !requireAdmin(deps, r, workspaceID) {
			writeError(w, http.StatusForbidden, "FORBIDDEN",
				"admin role required to manage workspace members (Phase 52 RBAC)")
			return
		}
		var body addMemberBody
		if !readJSONBody(w, r, &body) {
			return
		}
		if body.UserID == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "user_id is required")
			return
		}
		role := strings.TrimSpace(body.Role)
		if role == "" {
			role = "member" // default role when omitted
		}
		if role != "admin" && role != "member" && role != "viewer" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST",
				"role must be one of admin|member|viewer")
			return
		}
		if err := deps.Membership.AddMember(workspaceID, body.UserID, role); err != nil {
			mapStorageError(w, err)
			return
		}
		writeJSON(w, http.StatusCreated, Member{
			WorkspaceID: workspaceID,
			UserID:      body.UserID,
			Role:        role,
		})
	}
}

// handleListMembers — GET /v1/workspaces/{id}/members.
// Read-only: any caller with a valid token may list. Returns 200 + []Member.
func handleListMembers(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.Membership == nil {
			writeError(w, http.StatusServiceUnavailable, "SERVICE_UNAVAILABLE", ErrDataPlaneUnavailable.Error())
			return
		}
		workspaceID := trimID(r)
		if workspaceID == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing workspace id")
			return
		}
		members, err := deps.Membership.ListMembers(workspaceID)
		if err != nil {
			mapStorageError(w, err)
			return
		}
		if members == nil {
			members = []Member{}
		}
		writeJSON(w, http.StatusOK, members)
	}
}

// handleRemoveMember — DELETE /v1/workspaces/{id}/members/{user_id} (admin-only).
// Returns 204 No Content on success.
func handleRemoveMember(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.Membership == nil {
			writeError(w, http.StatusServiceUnavailable, "SERVICE_UNAVAILABLE", ErrDataPlaneUnavailable.Error())
			return
		}
		workspaceID := trimID(r)
		if workspaceID == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing workspace id")
			return
		}
		// admin-gate: only workspace admins (or trusted-network/legacy) may remove members.
		if !requireAdmin(deps, r, workspaceID) {
			writeError(w, http.StatusForbidden, "FORBIDDEN",
				"admin role required to manage workspace members (Phase 52 RBAC)")
			return
		}
		userID := strings.TrimSpace(r.PathValue("user_id"))
		if userID == "" {
			writeError(w, http.StatusBadRequest, "BAD_REQUEST", "missing user_id")
			return
		}
		if err := deps.Membership.RemoveMember(workspaceID, userID); err != nil {
			mapStorageError(w, err)
			return
		}
		w.WriteHeader(http.StatusNoContent)
	}
}
