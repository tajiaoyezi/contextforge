package consoleapi

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

// fakeMembershipClient is an in-memory MembershipClient for task-52.3 tests
// (no gRPC dependency). Membership rows are keyed by workspaceID + userID.
type fakeMembershipClient struct {
	// members[workspaceID][userID] = role
	members      map[string]map[string]string
	addCalls     int
	removeCalls  int
	listCalls    int
	roleCalls    int
	getMyRoleErr error // optional: inject an error for the requireAdmin fail-open branch
}

func newFakeMembershipClient() *fakeMembershipClient {
	return &fakeMembershipClient{members: map[string]map[string]string{}}
}

// seed sets a (workspaceID, userID) → role mapping (test helper).
func (f *fakeMembershipClient) seed(workspaceID, userID, role string) {
	ws, ok := f.members[workspaceID]
	if !ok {
		ws = map[string]string{}
		f.members[workspaceID] = ws
	}
	ws[userID] = role
}

func (f *fakeMembershipClient) AddMember(workspaceID, userID, role string) error {
	f.addCalls++
	f.seed(workspaceID, userID, role)
	return nil
}

func (f *fakeMembershipClient) RemoveMember(workspaceID, userID string) error {
	f.removeCalls++
	if ws, ok := f.members[workspaceID]; ok {
		delete(ws, userID)
	}
	return nil
}

func (f *fakeMembershipClient) ListMembers(workspaceID string) ([]Member, error) {
	f.listCalls++
	out := []Member{}
	if ws, ok := f.members[workspaceID]; ok {
		for uid, role := range ws {
			out = append(out, Member{
				WorkspaceID:   workspaceID,
				UserID:        uid,
				Role:          role,
				CreatedAtUnix: 1,
			})
		}
	}
	return out, nil
}

func (f *fakeMembershipClient) GetMyRole(workspaceID, userID string) (string, error) {
	f.roleCalls++
	if f.getMyRoleErr != nil {
		return "", f.getMyRoleErr
	}
	if ws, ok := f.members[workspaceID]; ok {
		return ws[userID], nil // "" when not a member (matches the interface contract)
	}
	return "", nil
}

// newRBACTestRouter wires the bearer middleware (via NewRouter) with a shared
// legacy token + a fake UserClient that resolves per-user tokens + a fake
// MembershipClient that backs the admin-gate. Mirrors the workspace_owner_test.go
// newOwnerTestRouter wiring.
func newRBACTestRouter(
	ws *ownerCapturingWorkspace,
	users *fakeUserClient,
	membership *fakeMembershipClient,
	authToken string,
) http.Handler {
	deps := Deps{
		Workspace:  ws,
		User:       users,
		Membership: membership,
		AuthToken:  authToken,
	}
	return NewRouter(deps)
}

// =====================================================================
// TEST-52.3.1 / AC1: admin can add a member (requireAdmin → true for admin);
// non-admin verified user → 403; the AddMember backend is NOT called when blocked.
// =====================================================================

func TestTask523_1_AdminCanAddMember_NonAdmin403(t *testing.T) {
	ws := &ownerCapturingWorkspace{
		created: contractv1.Workspace{WorkspaceID: "ws-1", Name: "demo", Status: "ready"},
		got:     &(contractv1.Workspace{WorkspaceID: "ws-1", Name: "demo", Status: "ready"}),
	}
	users := newFakeUserClient()
	users.users["admin-secret"] = User{ID: "user-admin", Name: "Admin", Token: "admin-secret"}
	users.users["member-secret"] = User{ID: "user-mem", Name: "Member", Token: "member-secret"}
	membership := newFakeMembershipClient()
	// seed the admin role on ws-1 for user-admin; user-mem is a plain "member".
	membership.seed("ws-1", "user-admin", "admin")
	membership.seed("ws-1", "user-mem", "member")
	router := newRBACTestRouter(ws, users, membership, "shared-legacy-token")

	// (a) admin can add a member → 201 + AddMember called once.
	body := `{"user_id":"user-new","role":"viewer"}`
	req := httptest.NewRequest("POST", "/v1/workspaces/ws-1/members", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer admin-secret")
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusCreated {
		t.Fatalf("admin add member: expected 201; got %d body=%s", w.Code, w.Body.String())
	}
	if membership.addCalls != 1 {
		t.Errorf("admin add member: expected 1 AddMember call; got %d", membership.addCalls)
	}
	var added Member
	if err := json.Unmarshal(w.Body.Bytes(), &added); err != nil {
		t.Fatalf("unmarshal added member: %v", err)
	}
	if added.UserID != "user-new" || added.Role != "viewer" {
		t.Errorf("added member shape: want user-new/viewer; got %+v", added)
	}

	// (b) non-admin (member role) → 403; AddMember NOT called for this request.
	membership.addCalls = 0 // reset counter after (a)
	badReq := httptest.NewRequest("POST", "/v1/workspaces/ws-1/members",
		strings.NewReader(`{"user_id":"user-other","role":"member"}`))
	badReq.Header.Set("Authorization", "Bearer member-secret")
	badReq.Header.Set("Content-Type", "application/json")
	badW := httptest.NewRecorder()
	router.ServeHTTP(badW, badReq)
	if badW.Code != http.StatusForbidden {
		t.Fatalf("non-admin add member: expected 403; got %d body=%s", badW.Code, badW.Body.String())
	}
	if membership.addCalls != 0 {
		t.Errorf("non-admin add member: AddMember must NOT be called when gate blocks; got %d calls", membership.addCalls)
	}
	if !strings.Contains(badW.Body.String(), `"code":"FORBIDDEN"`) {
		t.Errorf("non-admin add member: expected FORBIDDEN code; got body=%s", badW.Body.String())
	}

	// (c) invalid role → 400 (admin caller, gate passes, validation rejects).
	membership.addCalls = 0
	invReq := httptest.NewRequest("POST", "/v1/workspaces/ws-1/members",
		strings.NewReader(`{"user_id":"user-x","role":"superuser"}`))
	invReq.Header.Set("Authorization", "Bearer admin-secret")
	invReq.Header.Set("Content-Type", "application/json")
	invW := httptest.NewRecorder()
	router.ServeHTTP(invW, invReq)
	if invW.Code != http.StatusBadRequest {
		t.Fatalf("invalid role: expected 400; got %d body=%s", invW.Code, invW.Body.String())
	}
}

// =====================================================================
// TEST-52.3.2 / AC2: PATCH /v1/workspaces/{id}/config — admin ok (200); member
// role → 403 (admin-gate blocks before Update). Mirrors the workspace config
// destructive-op gating (workspace_id is in the path → properly gated).
// =====================================================================

func TestTask523_2_PatchWorkspaceConfig_AdminOk_Member403(t *testing.T) {
	ws := &ownerCapturingWorkspace{
		created: contractv1.Workspace{WorkspaceID: "ws-1", Name: "demo", Status: "ready"},
	}
	users := newFakeUserClient()
	users.users["admin-secret"] = User{ID: "user-admin", Name: "Admin", Token: "admin-secret"}
	users.users["member-secret"] = User{ID: "user-mem", Name: "Member", Token: "member-secret"}
	membership := newFakeMembershipClient()
	membership.seed("ws-1", "user-admin", "admin")
	membership.seed("ws-1", "user-mem", "member")
	router := newRBACTestRouter(ws, users, membership, "shared-legacy-token")

	body := `{"allowlist":["*.go"],"denylist":["*.tmp"]}`

	// (a) admin → 200 + Update called once.
	req := httptest.NewRequest("PATCH", "/v1/workspaces/ws-1/config?confirm=true", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer admin-secret")
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("admin patch config: expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	if ws.updateCalls != 1 {
		t.Errorf("admin patch config: expected 1 Update call; got %d", ws.updateCalls)
	}

	// (b) member → 403; Update NOT called for this request.
	ws.updateCalls = 0
	badReq := httptest.NewRequest("PATCH", "/v1/workspaces/ws-1/config?confirm=true", strings.NewReader(body))
	badReq.Header.Set("Authorization", "Bearer member-secret")
	badReq.Header.Set("Content-Type", "application/json")
	badW := httptest.NewRecorder()
	router.ServeHTTP(badW, badReq)
	if badW.Code != http.StatusForbidden {
		t.Fatalf("member patch config: expected 403; got %d body=%s", badW.Code, badW.Body.String())
	}
	if ws.updateCalls != 0 {
		t.Errorf("member patch config: Update must NOT be called when gate blocks; got %d calls", ws.updateCalls)
	}
}

// =====================================================================
// TEST-52.3.3 / AC3: trusted-network (empty token) → admin (byte-equivalent).
// No verified identity is injected → requireAdmin short-circuits to true for
// BOTH the membership add + the workspace config patch (all admin).
// =====================================================================

func TestTask523_3_TrustedNetworkByteEquivalent(t *testing.T) {
	ws := &ownerCapturingWorkspace{
		created: contractv1.Workspace{WorkspaceID: "ws-1", Name: "demo", Status: "ready"},
	}
	membership := newFakeMembershipClient()
	// trusted-network: empty AuthToken → bearerAuthMiddleware is a no-op → no
	// verified identity injected → requireAdmin treats the caller as admin.
	router := newRBACTestRouter(ws, newFakeUserClient(), membership, "")

	// (a) add member with NO Authorization header → 201 (admin byte-equiv).
	body := `{"user_id":"user-new","role":"member"}`
	req := httptest.NewRequest("POST", "/v1/workspaces/ws-1/members", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusCreated {
		t.Fatalf("trusted-network add member: expected 201; got %d body=%s", w.Code, w.Body.String())
	}
	if membership.addCalls != 1 {
		t.Errorf("trusted-network add member: expected 1 AddMember call; got %d", membership.addCalls)
	}
	// requireAdmin must NOT have consulted GetMyRole (verified identity empty → short-circuit).
	if membership.roleCalls != 0 {
		t.Errorf("trusted-network: GetMyRole must NOT be called (byte-equiv); got %d calls", membership.roleCalls)
	}

	// (b) patch workspace config with NO Authorization header → 200 (admin byte-equiv).
	patchReq := httptest.NewRequest("PATCH", "/v1/workspaces/ws-1/config?confirm=true",
		strings.NewReader(`{"allowlist":[],"denylist":[]}`))
	patchReq.Header.Set("Content-Type", "application/json")
	patchW := httptest.NewRecorder()
	router.ServeHTTP(patchW, patchReq)
	if patchW.Code != http.StatusOK {
		t.Fatalf("trusted-network patch config: expected 200; got %d body=%s", patchW.Code, patchW.Body.String())
	}
	if ws.updateCalls != 1 {
		t.Errorf("trusted-network patch config: expected 1 Update call; got %d", ws.updateCalls)
	}
}

// =====================================================================
// Extra coverage: GET list members (read-only, no admin-gate) + DELETE remove
// member (admin-only) + GetMyRole error fail-open.
// =====================================================================

// TEST-52.3.4 (extra): GET members is read-only — any caller (member role) may list.
func TestTask523_4_ListMembers_ReadOnly(t *testing.T) {
	ws := &ownerCapturingWorkspace{
		got: &(contractv1.Workspace{WorkspaceID: "ws-1", Name: "demo", Status: "ready"}),
	}
	users := newFakeUserClient()
	users.users["member-secret"] = User{ID: "user-mem", Name: "Member", Token: "member-secret"}
	membership := newFakeMembershipClient()
	membership.seed("ws-1", "user-admin", "admin")
	membership.seed("ws-1", "user-mem", "member")
	router := newRBACTestRouter(ws, users, membership, "shared-legacy-token")

	// a member (non-admin) listing members → 200 (GET is not admin-gated).
	req := httptest.NewRequest("GET", "/v1/workspaces/ws-1/members", nil)
	req.Header.Set("Authorization", "Bearer member-secret")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("list members: expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	var list []Member
	if err := json.Unmarshal(w.Body.Bytes(), &list); err != nil {
		t.Fatalf("unmarshal members: %v", err)
	}
	if len(list) != 2 {
		t.Errorf("expected 2 members; got %d", len(list))
	}
}

// TEST-52.3.5 (extra): DELETE remove member — admin ok (204); member → 403.
func TestTask523_5_RemoveMember_AdminOk_Member403(t *testing.T) {
	ws := &ownerCapturingWorkspace{
		got: &(contractv1.Workspace{WorkspaceID: "ws-1", Name: "demo", Status: "ready"}),
	}
	users := newFakeUserClient()
	users.users["admin-secret"] = User{ID: "user-admin", Name: "Admin", Token: "admin-secret"}
	users.users["member-secret"] = User{ID: "user-mem", Name: "Member", Token: "member-secret"}
	membership := newFakeMembershipClient()
	membership.seed("ws-1", "user-admin", "admin")
	membership.seed("ws-1", "user-mem", "member")
	membership.seed("ws-1", "user-target", "viewer") // to be removed
	router := newRBACTestRouter(ws, users, membership, "shared-legacy-token")

	// (a) admin removes a member → 204.
	req := httptest.NewRequest("DELETE", "/v1/workspaces/ws-1/members/user-target", nil)
	req.Header.Set("Authorization", "Bearer admin-secret")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNoContent {
		t.Fatalf("admin remove member: expected 204; got %d body=%s", w.Code, w.Body.String())
	}
	if membership.removeCalls != 1 {
		t.Errorf("admin remove member: expected 1 RemoveMember call; got %d", membership.removeCalls)
	}

	// (b) member removes a member → 403; RemoveMember NOT called.
	membership.removeCalls = 0
	badReq := httptest.NewRequest("DELETE", "/v1/workspaces/ws-1/members/user-other", nil)
	badReq.Header.Set("Authorization", "Bearer member-secret")
	badW := httptest.NewRecorder()
	router.ServeHTTP(badW, badReq)
	if badW.Code != http.StatusForbidden {
		t.Fatalf("member remove member: expected 403; got %d body=%s", badW.Code, badW.Body.String())
	}
	if membership.removeCalls != 0 {
		t.Errorf("member remove member: RemoveMember must NOT be called when gate blocks; got %d calls", membership.removeCalls)
	}
}

// TEST-52.3.6 (extra): GetMyRole returning an error → requireAdmin fail-opens (admin).
// Documents the infra-issue fail-open branch (don't block on data-plane errors).
func TestTask523_6_GetMyRoleError_FailOpen(t *testing.T) {
	ws := &ownerCapturingWorkspace{
		created: contractv1.Workspace{WorkspaceID: "ws-1", Name: "demo", Status: "ready"},
	}
	users := newFakeUserClient()
	users.users["alice-secret"] = User{ID: "user-alice", Name: "Alice", Token: "alice-secret"}
	membership := newFakeMembershipClient()
	membership.getMyRoleErr = ErrDataPlaneUnavailable // simulate data-plane error
	router := newRBACTestRouter(ws, users, membership, "shared-legacy-token")

	// even a non-seeded (would-be non-admin) user is allowed when GetMyRole errors (fail-open).
	req := httptest.NewRequest("PATCH", "/v1/workspaces/ws-1/config?confirm=true",
		strings.NewReader(`{"allowlist":[],"denylist":[]}`))
	req.Header.Set("Authorization", "Bearer alice-secret")
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("fail-open patch config: expected 200 (GetMyRole error → fail-open); got %d body=%s", w.Code, w.Body.String())
	}
}
