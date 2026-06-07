package consoleapi

import (
	"net/http"
	"net/http/httptest"
	"testing"
)

// actorCapturingMemory embeds *MemMemoryStore (full MemoryClient impl) and records the actor
// passed to Pin, so we can assert handleMemoryPin forwards the X-Actor header (task-40.1).
type actorCapturingMemory struct {
	*MemMemoryStore
	lastActor string
}

func (m *actorCapturingMemory) Pin(id string, pin bool, actor string) error {
	m.lastActor = actor
	return m.MemMemoryStore.Pin(id, pin, actor)
}

// TEST-40.1.3 (task-40.1 / ADR-045 D1): handleMemoryPin reads the X-Actor header and forwards it
// through MemoryClient.Pin(id, pin, actor). Absent header → empty actor (server falls back to
// "console-api", byte-equivalent default). The lenient body contract (ADR-022 D2) is unchanged.
func TestTask401_HandleMemoryPin_ForwardsXActorHeader(t *testing.T) {
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
	}
	router := NewRouter(deps)

	// With X-Actor header → forwarded verbatim.
	req := httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/pin", nil)
	req.Header.Set("X-Actor", "alice")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	if w.Code != http.StatusNoContent {
		t.Fatalf("expected 204; got %d body=%s", w.Code, w.Body.String())
	}
	if cap.lastActor != "alice" {
		t.Errorf("expected forwarded actor %q; got %q", "alice", cap.lastActor)
	}

	// Without X-Actor header → empty actor (server-side fallback to "console-api").
	req2 := httptest.NewRequest("POST", "/v1/memory/mem-fixture-1/pin", nil)
	w2 := httptest.NewRecorder()
	router.ServeHTTP(w2, req2)
	if w2.Code != http.StatusNoContent {
		t.Fatalf("expected 204 (no header); got %d body=%s", w2.Code, w2.Body.String())
	}
	if cap.lastActor != "" {
		t.Errorf("expected empty actor without header; got %q", cap.lastActor)
	}
}
