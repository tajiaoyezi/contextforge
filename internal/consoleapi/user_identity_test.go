package consoleapi

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// fakeUserClient is an in-memory UserClient for task-50.3 tests (no gRPC dependency).
type fakeUserClient struct {
	users map[string]User // token → User
}

func newFakeUserClient() *fakeUserClient {
	return &fakeUserClient{users: map[string]User{}}
}

func (f *fakeUserClient) Create(id, name, token string) (User, error) {
	if _, exists := f.users[token]; exists {
		return User{}, ErrConflict
	}
	for _, u := range f.users {
		if u.ID == id {
			return User{}, ErrConflict
		}
	}
	u := User{ID: id, Name: name, Token: token, CreatedAtUnix: 1}
	f.users[token] = u
	return u, nil
}

func (f *fakeUserClient) GetByToken(token string) (User, error) {
	if u, ok := f.users[token]; ok {
		return u, nil
	}
	return User{}, nil // zero-value + nil err = not found (matches interface contract)
}

func (f *fakeUserClient) List() ([]User, error) {
	out := make([]User, 0, len(f.users))
	for _, u := range f.users {
		out = append(out, u)
	}
	return out, nil
}

// TEST-50.3.1 / AC1: register a user → use that user's token to call pin → actor = verified userID
// (the caller-declared X-Actor is overridden by the verified identity).
func TestTask503_AC1_RegisterThenPinActorIsVerified(t *testing.T) {
	store := NewMemStore()
	memMem := NewMemMemoryStore()
	memMem.SeedFixtures()
	cap := &actorCapturingMemory{MemMemoryStore: memMem}
	users := newFakeUserClient()
	deps := Deps{
		Workspace: WorkspaceAdapter{S: store},
		Job:       JobAdapter{S: store},
		Search:    store,
		Events:    store,
		Memory:    cap,
		User:      users,
		AuthToken: "shared-legacy-token", // enable bearer checking
	}
	router := NewRouter(deps)

	// Register a user via POST /v1/users (using the shared token — trusted to register).
	regReq := httptest.NewRequest("POST", "/v1/users", strings.NewReader(`{"id":"user-alice","name":"Alice","token":"alice-secret"}`))
	regReq.Header.Set("Authorization", "Bearer shared-legacy-token")
	regReq.Header.Set("Content-Type", "application/json")
	regW := httptest.NewRecorder()
	router.ServeHTTP(regW, regReq)
	if regW.Code != http.StatusCreated {
		t.Fatalf("register: expected 201; got %d body=%s", regW.Code, regW.Body.String())
	}

	// Pin a memory item using the per-user token + a LYING X-Actor header.
	// The verified identity (user-alice) must OVERRIDE the declared X-Actor ("mallory").
	pinReq := httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/pin", nil)
	pinReq.Header.Set("Authorization", "Bearer alice-secret")
	pinReq.Header.Set("X-Actor", "mallory") // caller tries to impersonate
	pinW := httptest.NewRecorder()
	router.ServeHTTP(pinW, pinReq)
	if pinW.Code != http.StatusNoContent {
		t.Fatalf("pin: expected 204; got %d body=%s", pinW.Code, pinW.Body.String())
	}
	if cap.lastActor != "user-alice" {
		t.Errorf("expected verified actor %q (override of X-Actor mallory); got %q", "user-alice", cap.lastActor)
	}
}

// TEST-50.3.2 / AC2: trusted-network mode (empty AuthToken) → byte-equivalent (X-Actor used as-is,
// no verified identity injected). No User resolution happens.
func TestTask503_AC2_TrustedNetworkByteEquivalent(t *testing.T) {
	store := NewMemStore()
	memMem := NewMemMemoryStore()
	memMem.SeedFixtures()
	cap := &actorCapturingMemory{MemMemoryStore: memMem}
	deps := Deps{
		Workspace: WorkspaceAdapter{S: store},
		Job:       JobAdapter{S: store},
		Search:    store,
		Events:    store,
		Memory:    cap,
		User:      newFakeUserClient(),
		AuthToken: "", // trusted-network mode
	}
	router := NewRouter(deps)

	req := httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/pin", nil)
	req.Header.Set("X-Actor", "declared-bob")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNoContent {
		t.Fatalf("expected 204; got %d body=%s", w.Code, w.Body.String())
	}
	if cap.lastActor != "declared-bob" {
		t.Errorf("trusted-network: expected declared actor %q (byte-equiv); got %q", "declared-bob", cap.lastActor)
	}
}

// TEST-50.3.3 / AC3: legacy shared token still works → X-Actor stays caller-declared (backward compat).
func TestTask503_AC3_LegacySharedTokenBackwardCompat(t *testing.T) {
	store := NewMemStore()
	memMem := NewMemMemoryStore()
	memMem.SeedFixtures()
	cap := &actorCapturingMemory{MemMemoryStore: memMem}
	users := newFakeUserClient()
	users.users["alice-secret"] = User{ID: "user-alice", Name: "Alice", Token: "alice-secret"}
	deps := Deps{
		Workspace: WorkspaceAdapter{S: store},
		Job:       JobAdapter{S: store},
		Search:    store,
		Events:    store,
		Memory:    cap,
		User:      users,
		AuthToken: "shared-legacy-token",
	}
	router := NewRouter(deps)

	// Legacy shared token → X-Actor used as-is (no verified override).
	req := httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/pin", nil)
	req.Header.Set("Authorization", "Bearer shared-legacy-token")
	req.Header.Set("X-Actor", "legacy-declared")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNoContent {
		t.Fatalf("expected 204; got %d body=%s", w.Code, w.Body.String())
	}
	if cap.lastActor != "legacy-declared" {
		t.Errorf("legacy shared token: expected declared actor %q (backward compat); got %q", "legacy-declared", cap.lastActor)
	}

	// Invalid token (neither shared nor per-user) → 401.
	badReq := httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/pin", nil)
	badReq.Header.Set("Authorization", "Bearer wrong-token")
	badW := httptest.NewRecorder()
	router.ServeHTTP(badW, badReq)
	if badW.Code != http.StatusUnauthorized {
		t.Errorf("invalid token: expected 401; got %d", badW.Code)
	}
}

// (bodyReader helpers removed — tests use strings.NewReader directly.)
