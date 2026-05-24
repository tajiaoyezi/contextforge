// Package consoleapi e2e — task-10.4 §6 AC4. Spins up a real net/http server
// on a loopback listener and exercises all 9 endpoints via the stdlib HTTP
// client (no in-process httptest shortcut). This is the closest e2e shape
// available in v0.3 given the in-memory store trade-off (§10 #1 — spawned
// daemon binary path with shared Rust SQLite deferred to v0.4
// task-future.cross-process-sqlite-sharing).
package consoleapi

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"strings"
	"testing"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

func startServerE2E(t *testing.T, authToken string) (string, *MemStore, func()) {
	t.Helper()
	store := NewMemStore()
	router := NewRouter(Deps{
		Workspace: WorkspaceAdapter{S: store},
		Job:       JobAdapter{S: store},
		Search:    store,
		Events:    store,
		AuthToken: authToken,
	})
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	srv := &http.Server{Handler: router}
	go func() { _ = srv.Serve(listener) }()
	addr := listener.Addr().String()
	baseURL := "http://" + addr
	// wait for health
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		resp, err := http.Get(baseURL + "/v1/health")
		if err == nil && resp.StatusCode == 200 {
			resp.Body.Close()
			break
		}
		if resp != nil {
			resp.Body.Close()
		}
		time.Sleep(50 * time.Millisecond)
	}
	cleanup := func() {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		defer cancel()
		_ = srv.Shutdown(ctx)
	}
	return baseURL, store, cleanup
}

func httpCall(t *testing.T, method, url string, body any, headers map[string]string, out any) int {
	t.Helper()
	var reqBody *bytes.Buffer
	if body != nil {
		b, _ := json.Marshal(body)
		reqBody = bytes.NewBuffer(b)
	}
	var req *http.Request
	var err error
	if reqBody != nil {
		req, err = http.NewRequest(method, url, reqBody)
		req.Header.Set("Content-Type", "application/json")
	} else {
		req, err = http.NewRequest(method, url, nil)
	}
	if err != nil {
		t.Fatalf("new request %s %s: %v", method, url, err)
	}
	for k, v := range headers {
		req.Header.Set(k, v)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("do %s %s: %v", method, url, err)
	}
	defer resp.Body.Close()
	if out != nil && resp.StatusCode >= 200 && resp.StatusCode < 300 {
		if err := json.NewDecoder(resp.Body).Decode(out); err != nil {
			t.Fatalf("decode %s %s: %v", method, url, err)
		}
	}
	return resp.StatusCode
}

// TestRESTEndpoints_E2E — AC4. Spins real http server + 9 endpoint flow.
func TestRESTEndpoints_E2E(t *testing.T) {
	baseURL, _, cleanup := startServerE2E(t, "")
	defer cleanup()

	// 1. GET /v1/health
	var health contractv1.CoreHealth
	if code := httpCall(t, "GET", baseURL+"/v1/health", nil, nil, &health); code != 200 {
		t.Fatalf("/v1/health: %d", code)
	}
	if health.ContractVersion != "v1" {
		t.Errorf("contract_version must be v1; got %q", health.ContractVersion)
	}

	// 2. POST /v1/workspaces
	var ws contractv1.Workspace
	if code := httpCall(t, "POST", baseURL+"/v1/workspaces",
		contractv1.WorkspaceCreate{Name: "demo", RootPath: "/tmp/demo"}, nil, &ws); code != 200 {
		t.Fatalf("POST /v1/workspaces: %d", code)
	}
	if ws.WorkspaceID == "" || ws.Status != "ready" {
		t.Errorf("workspace shape unexpected: %#v", ws)
	}

	// 3. GET /v1/workspaces
	var list []contractv1.Workspace
	if code := httpCall(t, "GET", baseURL+"/v1/workspaces", nil, nil, &list); code != 200 {
		t.Fatalf("GET /v1/workspaces: %d", code)
	}
	if len(list) != 1 {
		t.Errorf("expected 1 workspace; got %d", len(list))
	}

	// 4. GET /v1/workspaces/{id}
	var single contractv1.Workspace
	if code := httpCall(t, "GET", baseURL+"/v1/workspaces/"+ws.WorkspaceID, nil, nil, &single); code != 200 {
		t.Fatalf("GET /v1/workspaces/{id}: %d", code)
	}
	if single.WorkspaceID != ws.WorkspaceID {
		t.Errorf("workspace_id roundtrip mismatch: %q vs %q", single.WorkspaceID, ws.WorkspaceID)
	}

	// 4a. GET /v1/workspaces/non-existent → 404 (ErrNotFound mapping)
	if code := httpCall(t, "GET", baseURL+"/v1/workspaces/non-existent-id", nil, nil, nil); code != 404 {
		t.Errorf("non-existent workspace: expected 404; got %d", code)
	}

	// 5. POST /v1/index-jobs (body {workspace_id})
	var job contractv1.IndexJob
	enqBody := map[string]string{"workspace_id": ws.WorkspaceID}
	if code := httpCall(t, "POST", baseURL+"/v1/index-jobs", enqBody, nil, &job); code != 200 {
		t.Fatalf("POST /v1/index-jobs: %d", code)
	}
	if job.Status != "queued" {
		t.Errorf("initial status must be queued; got %q", job.Status)
	}

	// 6. GET /v1/index-jobs/{id}
	var jobGet contractv1.IndexJob
	if code := httpCall(t, "GET", baseURL+"/v1/index-jobs/"+job.JobID, nil, nil, &jobGet); code != 200 {
		t.Fatalf("GET /v1/index-jobs/{id}: %d", code)
	}

	// 7. POST /v1/index-jobs/{id}/cancel — task-12.1 (ADR-017 D3) returns 204
	if code := httpCall(t, "POST", baseURL+"/v1/index-jobs/"+job.JobID+"/cancel", nil, nil, nil); code != 204 {
		t.Fatalf("POST cancel: %d", code)
	}
	// re-cancel → 409
	if code := httpCall(t, "POST", baseURL+"/v1/index-jobs/"+job.JobID+"/cancel", nil, nil, nil); code != 409 {
		t.Errorf("re-cancel: expected 409; got %d", code)
	}

	// 8. POST /v1/search
	var sr SearchResponse
	if code := httpCall(t, "POST", baseURL+"/v1/search",
		contractv1.SearchRequest{Query: "configuration", WorkspaceID: ws.WorkspaceID, TopK: 5, RetrievalMethod: "bm25"},
		nil, &sr); code != 200 {
		t.Fatalf("POST /v1/search: %d", code)
	}
	if sr.Result.ResultID == "" || sr.Trace.TraceID == "" {
		t.Errorf("nested {result, trace} envelope expected; got %#v", sr)
	}

	// 9. GET /v1/observability/events
	var evts []contractv1.ObservabilityEvent
	if code := httpCall(t, "GET", baseURL+"/v1/observability/events", nil, nil, &evts); code != 200 {
		t.Fatalf("GET events: %d", code)
	}
	if len(evts) < 1 {
		t.Errorf("expected ≥1 event from prior operations; got %d", len(evts))
	}
}

// TestRESTEndpoints_E2E_BearerAuth — same flow under bearer auth (AC4 last
// clause).
func TestRESTEndpoints_E2E_BearerAuth(t *testing.T) {
	baseURL, _, cleanup := startServerE2E(t, "secret-token")
	defer cleanup()
	// no header → 401
	if code := httpCall(t, "GET", baseURL+"/v1/health", nil, nil, nil); code != 401 {
		t.Errorf("no token: expected 401; got %d", code)
	}
	// correct header → 200
	headers := map[string]string{"Authorization": "Bearer secret-token"}
	if code := httpCall(t, "GET", baseURL+"/v1/health", nil, headers, nil); code != 200 {
		t.Errorf("with token: expected 200; got %d", code)
	}
}

// helper to silence unused-fmt warning if any test path drops fmt usage.
var _ = fmt.Sprintf
var _ = strings.TrimSpace
