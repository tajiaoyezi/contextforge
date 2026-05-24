// Package conformance — task-10.5 §6 AC1-5.
//
// Reverse-uses the Console (ContextForge-Console) HTTPAdapter expectation
// oracle (set by Console's fakehttpserver + http_adapter.go) to verify that
// the ContextForge Go REST surface (task-10.4 internal/consoleapi) returns
// JSON that Console HTTPAdapter can deserialize without missing must-have
// fields.
//
// Strategy (ADR-015 §D5):
//   - Spin up an in-process ContextForge Go REST server on a loopback
//     listener (cheapest faithful e2e — same code path as the production
//     `internal/daemon` REST wiring would take when env-configured to mount
//     internal/consoleapi).
//   - Drive 9 endpoint flow using an embedded minimal Console-style HTTP
//     client (avoids cross-repo Go module import of the live Console
//     adapter; v0.4 follow-up may vendor or `go.mod replace` the adapter).
//   - Decode every response into contractv1.* types (the Go mirror of
//     Console's contractv1.go).
//   - Assert FieldAvailability.Complete() == true for every returned
//     Contract v1 object.
//   - Verify Console error-mapping conventions: 404 → ErrNotFound /
//     409 → ErrConflict.
//   - Cross-repo gate: when `$CONSOLE_REPO` env not set → t.Skip (D5
//     historical-skip; CI defaults to SKIP unless explicitly opted in).
//
// Note: task-10.4 §10 trade-off #1 means v0.3 conformance covers REST wire
// shape but NOT cross-process Rust↔Go SQLite consistency. The latter is
// task-future.cross-process-sqlite-sharing.
//
// Refs: ADR-015 §D5 / phase-10 §6 AC5 / task-10.5 §6 AC1-5

package conformance_test

import (
	"bytes"
	"encoding/json"
	"errors"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/consoleapi"
	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

// errNotFound / errConflict mirror the Console HTTPAdapter sentinel set
// (Console internal/coreadapter/http_adapter.go). The embedded client
// returns these so test assertions can use errors.Is.
var (
	errNotFound = errors.New("conformance: not found")
	errConflict = errors.New("conformance: conflict")
)

// minimalConsoleHTTPClient mirrors the shape of Console's HTTPAdapter
// (console-api/internal/coreadapter/http_adapter.go) for the 9 endpoints
// this test exercises. We embed instead of importing to avoid a cross-repo
// Go module dependency in v0.3 — see task-10.5 §10 trade-off note.
type minimalConsoleHTTPClient struct {
	baseURL string
	client  *http.Client
}

func newClient(baseURL string) *minimalConsoleHTTPClient {
	return &minimalConsoleHTTPClient{
		baseURL: baseURL,
		client:  &http.Client{Timeout: 10 * time.Second},
	}
}

func (c *minimalConsoleHTTPClient) call(method, path string, body any, out any) (int, error) {
	var reqBody *bytes.Buffer
	if body != nil {
		b, err := json.Marshal(body)
		if err != nil {
			return 0, err
		}
		reqBody = bytes.NewBuffer(b)
	}
	var req *http.Request
	var err error
	if reqBody != nil {
		req, err = http.NewRequest(method, c.baseURL+path, reqBody)
		req.Header.Set("Content-Type", "application/json")
	} else {
		req, err = http.NewRequest(method, c.baseURL+path, nil)
	}
	if err != nil {
		return 0, err
	}
	resp, err := c.client.Do(req)
	if err != nil {
		return 0, err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 400 {
		// Console sentinel mapping
		switch resp.StatusCode {
		case http.StatusNotFound:
			return resp.StatusCode, errNotFound
		case http.StatusConflict:
			return resp.StatusCode, errConflict
		}
		return resp.StatusCode, nil
	}
	if out != nil {
		if err := json.NewDecoder(resp.Body).Decode(out); err != nil {
			return resp.StatusCode, err
		}
	}
	return resp.StatusCode, nil
}

func startConformanceServer(t *testing.T) (string, func()) {
	t.Helper()
	store := consoleapi.NewMemStore()
	router := consoleapi.NewRouter(consoleapi.Deps{
		Workspace: consoleapi.WorkspaceAdapter{S: store},
		Job:       consoleapi.JobAdapter{S: store},
		Search:    store,
		Events:    store,
		AuthToken: "",
	})
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	srv := &http.Server{Handler: router}
	go func() { _ = srv.Serve(listener) }()
	baseURL := "http://" + listener.Addr().String()
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
	return baseURL, func() { _ = srv.Close() }
}

// TestConsoleContractV1Conformance — task-10.5 §6 AC1 / AC2 / AC4 / AC5.
// Runs the 9 endpoint flow with Console-style client + asserts every
// returned contractv1.* type has FieldAvailability.Complete() == true.
// Console sentinel errors (NotFound / Conflict) verified.
//
// Env contract: `$CONSOLE_REPO` must point at the Console clone root; when
// unset, the test SKIPs (AC3 — D5 historical-skip).
func TestConsoleContractV1Conformance(t *testing.T) {
	consoleRepo := os.Getenv("CONSOLE_REPO")
	if consoleRepo == "" {
		t.Skip("CONSOLE_REPO env required for cross-repo conformance test (AC3 D5 historical-skip)")
	}
	// sanity: contractv1.go single source of truth exists in the named
	// Console repo
	contractPath := filepath.Join(consoleRepo, "console-api", "internal", "coreadapter", "contractv1", "contractv1.go")
	if _, err := os.Stat(contractPath); err != nil {
		t.Fatalf("CONSOLE_REPO does not contain contractv1.go at %s: %v", contractPath, err)
	}

	baseURL, cleanup := startConformanceServer(t)
	defer cleanup()
	c := newClient(baseURL)

	// 1. GET /v1/health
	var health contractv1.CoreHealth
	if code, err := c.call("GET", "/v1/health", nil, &health); err != nil || code != 200 {
		t.Fatalf("/v1/health: code=%d err=%v", code, err)
	}
	if health.ContractVersion != "v1" {
		t.Errorf("contract_version must be \"v1\"; got %q", health.ContractVersion)
	}
	if health.HasMissingMustHaveFields() {
		t.Errorf("CoreHealth must report no missing must-have fields; got %v", health.MissingMustHaveFields)
	}

	// 2. POST /v1/workspaces
	var ws contractv1.Workspace
	if code, err := c.call("POST", "/v1/workspaces",
		contractv1.WorkspaceCreate{Name: "conformance-demo", RootPath: "/tmp/conformance-demo"}, &ws); err != nil || code != 200 {
		t.Fatalf("POST /v1/workspaces: code=%d err=%v", code, err)
	}
	assertComplete(t, "Workspace", ws.Availability)
	if ws.WorkspaceID == "" || ws.Status != "ready" {
		t.Errorf("workspace shape unexpected: %#v", ws)
	}

	// 3. GET /v1/workspaces (list)
	var list []contractv1.Workspace
	if code, err := c.call("GET", "/v1/workspaces", nil, &list); err != nil || code != 200 {
		t.Fatalf("GET /v1/workspaces: code=%d err=%v", code, err)
	}
	if len(list) != 1 {
		t.Errorf("list len: want 1; got %d", len(list))
	}
	assertComplete(t, "Workspace[0]", list[0].Availability)

	// 4. GET /v1/workspaces/{id}
	var single contractv1.Workspace
	if code, err := c.call("GET", "/v1/workspaces/"+ws.WorkspaceID, nil, &single); err != nil || code != 200 {
		t.Fatalf("GET /v1/workspaces/{id}: code=%d err=%v", code, err)
	}
	assertComplete(t, "Workspace.Get", single.Availability)

	// 4a. Console ErrNotFound mapping (AC5 first clause)
	code, err := c.call("GET", "/v1/workspaces/non-existent-id-xyz", nil, nil)
	if !errors.Is(err, errNotFound) || code != 404 {
		t.Errorf("non-existent workspace: want 404+errNotFound; got code=%d err=%v", code, err)
	}

	// 5. POST /v1/index-jobs (body {workspace_id})
	var job contractv1.IndexJob
	enqBody := map[string]string{"workspace_id": ws.WorkspaceID}
	if code, err := c.call("POST", "/v1/index-jobs", enqBody, &job); err != nil || code != 200 {
		t.Fatalf("POST /v1/index-jobs: code=%d err=%v", code, err)
	}
	assertComplete(t, "IndexJob", job.Availability)
	if job.Status != "queued" {
		t.Errorf("initial status: want queued; got %q", job.Status)
	}

	// 6. GET /v1/index-jobs/{id}
	var jobGet contractv1.IndexJob
	if code, err := c.call("GET", "/v1/index-jobs/"+job.JobID, nil, &jobGet); err != nil || code != 200 {
		t.Fatalf("GET /v1/index-jobs/{id}: code=%d err=%v", code, err)
	}
	assertComplete(t, "IndexJob.Get", jobGet.Availability)

	// 7. POST /v1/index-jobs/{id}/cancel (first cancel → 200; second → 409)
	if code, err := c.call("POST", "/v1/index-jobs/"+job.JobID+"/cancel", nil, nil); err != nil || code != 200 {
		t.Fatalf("first cancel: code=%d err=%v", code, err)
	}
	code2, err2 := c.call("POST", "/v1/index-jobs/"+job.JobID+"/cancel", nil, nil)
	if !errors.Is(err2, errConflict) || code2 != 409 {
		t.Errorf("second cancel: want 409+errConflict (Console mapping); got code=%d err=%v", code2, err2)
	}

	// 8. POST /v1/search (verify nested {result, trace})
	var sr struct {
		Result contractv1.SearchResult   `json:"result"`
		Trace  contractv1.RetrievalTrace `json:"trace"`
	}
	if code, err := c.call("POST", "/v1/search",
		contractv1.SearchRequest{
			Query: "configuration", WorkspaceID: ws.WorkspaceID,
			TopK: 5, RetrievalMethod: "bm25", AgentScope: "session",
		}, &sr); err != nil || code != 200 {
		t.Fatalf("POST /v1/search: code=%d err=%v", code, err)
	}
	assertComplete(t, "SearchResult", sr.Result.Availability)
	assertComplete(t, "RetrievalTrace", sr.Trace.Availability)
	assertComplete(t, "SearchResult.Citation", sr.Result.Citation.Availability)

	// 9. GET /v1/observability/events
	var evts []contractv1.ObservabilityEvent
	if code, err := c.call("GET", "/v1/observability/events", nil, &evts); err != nil || code != 200 {
		t.Fatalf("GET events: code=%d err=%v", code, err)
	}
	if len(evts) < 1 {
		t.Errorf("expected ≥1 event from prior operations; got %d", len(evts))
	}
	for i, evt := range evts {
		assertComplete(t, "ObservabilityEvent["+itoa(i)+"]", evt.Availability)
	}

	// Final sanity: contract version anchor verification via reading
	// Console source single source of truth.
	if !strings.Contains(readFile(t, contractPath), `ContractVersion = "v1"`) {
		t.Errorf("Console contractv1.go must anchor ContractVersion = \"v1\" — possible cross-repo drift")
	}
}

// assertComplete reports FieldAvailability.Complete() == true; helps the
// test produce a useful diff when must-have fields are missing.
func assertComplete(t *testing.T, label string, fa contractv1.FieldAvailability) {
	t.Helper()
	if !fa.Complete() {
		t.Errorf("%s FieldAvailability incomplete; missing=%v", label, fa.Missing)
	}
}

func readFile(t *testing.T, path string) string {
	t.Helper()
	b, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	return string(b)
}

// itoa is a tiny stdlib-free int → string helper to avoid pulling
// strconv into this test file (keeps the file dependency surface minimal).
func itoa(i int) string {
	if i == 0 {
		return "0"
	}
	neg := i < 0
	if neg {
		i = -i
	}
	var buf [20]byte
	pos := len(buf)
	for i > 0 {
		pos--
		buf[pos] = byte('0' + i%10)
		i /= 10
	}
	if neg {
		pos--
		buf[pos] = '-'
	}
	return string(buf[pos:])
}
