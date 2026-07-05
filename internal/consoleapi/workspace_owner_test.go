package consoleapi

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

// ownerCapturingWorkspace is a fake WorkspaceClient that records the owner_id
// passed to each owner-scoped method and reports which path (owned vs legacy)
// the handler took. It also serves as the byte-equivalence witness: the trusted-
// network / legacy-shared-token paths must NOT touch the owner-scoped methods.
type ownerCapturingWorkspace struct {
	// records of owner-scoped calls.
	createOwnerCalls []string // ownerID per CreateOwned
	listOwnerCalls   []string // ownerID per ListOwned
	getOwnerCalls    []string // ownerID per GetIfOwned
	// records of legacy (non-owner) calls.
	createCalls int
	listCalls   int
	getCalls    int
	updateCalls int
	// backing store for the body of each method (workspace the create returns,
	// list the list methods return). Kept tiny — this fake asserts dispatch +
	// owner wiring, not store semantics (those live in router_test.go).
	created contractv1.Workspace
	listed  []contractv1.Workspace
	got     *contractv1.Workspace
}

func (f *ownerCapturingWorkspace) Create(req contractv1.WorkspaceCreate) (contractv1.Workspace, error) {
	f.createCalls++
	return f.created, nil
}

func (f *ownerCapturingWorkspace) CreateOwned(req contractv1.WorkspaceCreate, ownerID string) (contractv1.Workspace, error) {
	f.createOwnerCalls = append(f.createOwnerCalls, ownerID)
	out := f.created
	out.OwnerID = ownerID
	return out, nil
}

func (f *ownerCapturingWorkspace) List() ([]contractv1.Workspace, error) {
	f.listCalls++
	return f.listed, nil
}

func (f *ownerCapturingWorkspace) ListOwned(ownerID string) ([]contractv1.Workspace, error) {
	f.listOwnerCalls = append(f.listOwnerCalls, ownerID)
	return f.listed, nil
}

func (f *ownerCapturingWorkspace) Get(id string) (*contractv1.Workspace, error) {
	f.getCalls++
	return f.got, nil
}

func (f *ownerCapturingWorkspace) GetIfOwned(workspaceID, ownerID string) (*contractv1.Workspace, error) {
	f.getOwnerCalls = append(f.getOwnerCalls, ownerID)
	return f.got, nil
}

func (f *ownerCapturingWorkspace) Update(id string, allowlist, denylist []string) (contractv1.Workspace, error) {
	f.updateCalls++
	return f.created, nil
}

// newOwnerTestRouter wires the bearer middleware (via NewRouter) with a shared
// legacy token + a fake UserClient that resolves the per-user token, mirroring
// the user_identity_test.go wiring exactly.
func newOwnerTestRouter(ws *ownerCapturingWorkspace, users *fakeUserClient, authToken string) http.Handler {
	deps := Deps{
		Workspace: ws,
		AuthToken: authToken,
		User:      users,
	}
	return NewRouter(deps)
}

// TEST-51.3.1 / AC1: POST workspace with a per-user token → CreateOwned is
// called with the verified userID, and the response carries owner_id = userID.
func TestTask513_1_PerUserTokenCreateUsesVerifiedOwner(t *testing.T) {
	ws := &ownerCapturingWorkspace{
		created: contractv1.Workspace{WorkspaceID: "ws-new", Name: "demo", RootPath: "/tmp/demo", Status: "ready"},
	}
	users := newFakeUserClient()
	users.users["alice-secret"] = User{ID: "user-alice", Name: "Alice", Token: "alice-secret"}
	router := newOwnerTestRouter(ws, users, "shared-legacy-token")

	body := `{"name":"demo","root_path":"/tmp/demo"}`
	req := httptest.NewRequest("POST", "/v1/workspaces", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer alice-secret")
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	if len(ws.createOwnerCalls) != 1 {
		t.Fatalf("expected 1 CreateOwned call; got %d (legacy Create calls=%d)",
			len(ws.createOwnerCalls), ws.createCalls)
	}
	if ws.createOwnerCalls[0] != "user-alice" {
		t.Errorf("expected owner_id %q; got %q", "user-alice", ws.createOwnerCalls[0])
	}
	if ws.createCalls != 0 {
		t.Errorf("trusted/legacy Create must NOT be called with a per-user token; got %d calls", ws.createCalls)
	}
	// Response must carry the verified owner_id.
	var resp contractv1.Workspace
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal response: %v", err)
	}
	if resp.OwnerID != "user-alice" {
		t.Errorf("expected response owner_id %q; got %q", "user-alice", resp.OwnerID)
	}
}

// TEST-51.3.2 / AC2: GET workspaces with a per-user token → ListOwned is called
// with the verified userID (own + unowned, server-side filter).
func TestTask513_2_PerUserTokenListUsesVerifiedOwner(t *testing.T) {
	ws := &ownerCapturingWorkspace{
		listed: []contractv1.Workspace{
			{WorkspaceID: "ws-a", OwnerID: "user-alice"},
			{WorkspaceID: "ws-shared"}, // unowned (legacy backfill)
		},
	}
	users := newFakeUserClient()
	users.users["alice-secret"] = User{ID: "user-alice", Name: "Alice", Token: "alice-secret"}
	router := newOwnerTestRouter(ws, users, "shared-legacy-token")

	req := httptest.NewRequest("GET", "/v1/workspaces", nil)
	req.Header.Set("Authorization", "Bearer alice-secret")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200; got %d body=%s", w.Code, w.Body.String())
	}
	if len(ws.listOwnerCalls) != 1 {
		t.Fatalf("expected 1 ListOwned call; got %d (legacy List calls=%d)",
			len(ws.listOwnerCalls), ws.listCalls)
	}
	if ws.listOwnerCalls[0] != "user-alice" {
		t.Errorf("expected owner_id %q; got %q", "user-alice", ws.listOwnerCalls[0])
	}
	if ws.listCalls != 0 {
		t.Errorf("legacy List must NOT be called with a per-user token; got %d calls", ws.listCalls)
	}
	var list []contractv1.Workspace
	if err := json.Unmarshal(w.Body.Bytes(), &list); err != nil {
		t.Fatalf("unmarshal response: %v", err)
	}
	if len(list) != 2 {
		t.Errorf("expected 2 workspaces returned; got %d", len(list))
	}
}

// TEST-51.3.3 / AC3: trusted-network (empty token) → Create/List byte-equivalent
// (legacy non-owner path, no owner filter, no verified identity injected).
func TestTask513_3_TrustedNetworkByteEquivalent(t *testing.T) {
	ws := &ownerCapturingWorkspace{
		created: contractv1.Workspace{WorkspaceID: "ws-new", Name: "demo", RootPath: "/tmp/demo", Status: "ready"},
		listed:  []contractv1.Workspace{{WorkspaceID: "ws-a"}, {WorkspaceID: "ws-b"}},
	}
	// AuthToken empty → bearerAuthMiddleware is a no-op (trusted-network), so no
	// verified identity is ever injected → handlers must take the legacy path.
	router := newOwnerTestRouter(ws, newFakeUserClient(), "")

	// Create.
	body := `{"name":"demo","root_path":"/tmp/demo"}`
	createReq := httptest.NewRequest("POST", "/v1/workspaces", strings.NewReader(body))
	createReq.Header.Set("Content-Type", "application/json")
	createW := httptest.NewRecorder()
	router.ServeHTTP(createW, createReq)
	if createW.Code != http.StatusOK {
		t.Fatalf("create: expected 200; got %d body=%s", createW.Code, createW.Body.String())
	}
	if ws.createCalls != 1 {
		t.Errorf("trusted-network create: expected 1 legacy Create call; got %d", ws.createCalls)
	}
	if len(ws.createOwnerCalls) != 0 {
		t.Errorf("trusted-network create: CreateOwned must NOT be called; got %d", len(ws.createOwnerCalls))
	}

	// List.
	listReq := httptest.NewRequest("GET", "/v1/workspaces", nil)
	listW := httptest.NewRecorder()
	router.ServeHTTP(listW, listReq)
	if listW.Code != http.StatusOK {
		t.Fatalf("list: expected 200; got %d body=%s", listW.Code, listW.Body.String())
	}
	if ws.listCalls != 1 {
		t.Errorf("trusted-network list: expected 1 legacy List call; got %d", ws.listCalls)
	}
	if len(ws.listOwnerCalls) != 0 {
		t.Errorf("trusted-network list: ListOwned must NOT be called; got %d", len(ws.listOwnerCalls))
	}
}

// TEST-51.3.4 (extra): legacy shared token → byte-equivalent (no verified
// identity injected; the bearer middleware matches the shared token via
// constant-time compare and skips per-user resolution). Mirrors AC3 backward
// compatibility for the shared-token path.
func TestTask513_4_LegacySharedTokenByteEquivalent(t *testing.T) {
	ws := &ownerCapturingWorkspace{
		created: contractv1.Workspace{WorkspaceID: "ws-new", Name: "demo", RootPath: "/tmp/demo", Status: "ready"},
		listed:  []contractv1.Workspace{{WorkspaceID: "ws-a"}},
	}
	users := newFakeUserClient()
	router := newOwnerTestRouter(ws, users, "shared-legacy-token")

	// Create with the shared token (NOT a per-user token).
	body := `{"name":"demo","root_path":"/tmp/demo"}`
	createReq := httptest.NewRequest("POST", "/v1/workspaces", strings.NewReader(body))
	createReq.Header.Set("Authorization", "Bearer shared-legacy-token")
	createReq.Header.Set("Content-Type", "application/json")
	createW := httptest.NewRecorder()
	router.ServeHTTP(createW, createReq)
	if createW.Code != http.StatusOK {
		t.Fatalf("create: expected 200; got %d body=%s", createW.Code, createW.Body.String())
	}
	if ws.createCalls != 1 {
		t.Errorf("legacy shared token create: expected 1 legacy Create call; got %d", ws.createCalls)
	}
	if len(ws.createOwnerCalls) != 0 {
		t.Errorf("legacy shared token create: CreateOwned must NOT be called; got %d", len(ws.createOwnerCalls))
	}

	// List with the shared token.
	listReq := httptest.NewRequest("GET", "/v1/workspaces", nil)
	listReq.Header.Set("Authorization", "Bearer shared-legacy-token")
	listW := httptest.NewRecorder()
	router.ServeHTTP(listW, listReq)
	if listW.Code != http.StatusOK {
		t.Fatalf("list: expected 200; got %d body=%s", listW.Code, listW.Body.String())
	}
	if ws.listCalls != 1 {
		t.Errorf("legacy shared token list: expected 1 legacy List call; got %d", ws.listCalls)
	}
	if len(ws.listOwnerCalls) != 0 {
		t.Errorf("legacy shared token list: ListOwned must NOT be called; got %d", len(ws.listOwnerCalls))
	}
}

// TEST-51.3.5 (extra): GET /v1/workspaces/{id} with a per-user token where the
// workspace is NOT owned by the verified user → 403 Forbidden (GetIfOwned
// returned nil). Without ownership enforcement in the in-memory fake this also
// documents the GetIfOwned dispatch wiring end-to-end.
func TestTask513_5_PerUserTokenGetNotOwnedReturns403(t *testing.T) {
	ws := &ownerCapturingWorkspace{got: nil} // not found OR not owned
	users := newFakeUserClient()
	users.users["alice-secret"] = User{ID: "user-alice", Name: "Alice", Token: "alice-secret"}
	router := newOwnerTestRouter(ws, users, "shared-legacy-token")

	req := httptest.NewRequest("GET", "/v1/workspaces/ws-bobs", nil)
	req.Header.Set("Authorization", "Bearer alice-secret")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusForbidden {
		t.Fatalf("expected 403; got %d body=%s", w.Code, w.Body.String())
	}
	if !strings.Contains(w.Body.String(), `"code":"FORBIDDEN"`) {
		t.Errorf("expected FORBIDDEN code; got body=%s", w.Body.String())
	}
	if len(ws.getOwnerCalls) != 1 || ws.getOwnerCalls[0] != "user-alice" {
		t.Errorf("expected 1 GetIfOwned call with owner user-alice; got %v", ws.getOwnerCalls)
	}
	if ws.getCalls != 0 {
		t.Errorf("legacy Get must NOT be called with a per-user token; got %d", ws.getCalls)
	}
}
