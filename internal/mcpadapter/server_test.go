package mcpadapter

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

type fakeSearcher struct {
	requests []*contextforgev1.SearchRequest
	resp     *contextforgev1.SearchResponse
	err      error
}

func (f *fakeSearcher) Search(_ context.Context, req *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error) {
	f.requests = append(f.requests, req)
	if f.err != nil {
		return nil, f.err
	}
	if f.resp != nil {
		return f.resp, nil
	}
	return fakeMCPSearchResponse("chk_deadbeef_0"), nil
}

func fakeMCPSearchResponse(chunkID string) *contextforgev1.SearchResponse {
	return &contextforgev1.SearchResponse{
		Results: []*contextforgev1.RetrievalResult{
			{
				ChunkId:         chunkID,
				ContextId:       "ctx-1",
				SourceType:      "file",
				FilePath:        "docs/example.md",
				LineStart:       7,
				LineEnd:         11,
				Score:           0.91,
				RetrievalMethod: "bm25",
				Reason:          "bm25 hit on marker",
				AgentScope:      []string{"claude"},
				RedactionStatus: "applied",
				Provenance: []*contextforgev1.Provenance{
					{Importer: "scanner", OriginalPath: "docs/example.md"},
				},
			},
		},
	}
}

func structuredMap(t *testing.T, v any) map[string]any {
	t.Helper()
	b, err := json.Marshal(v)
	if err != nil {
		t.Fatalf("marshal structured content: %v", err)
	}
	var out map[string]any
	if err := json.Unmarshal(b, &out); err != nil {
		t.Fatalf("unmarshal structured content: %v", err)
	}
	return out
}

// TEST-7.1.1 / SCEN-7.1.1 / AC1:
// context_search returns the same explainable RetrievalResult field set as REST /v1/search.
func TestTask71_AC1_ContextSearchReturnsRESTSearchFields(t *testing.T) {
	searcher := &fakeSearcher{resp: fakeMCPSearchResponse("chk_feedcafe_0")}
	s := &Server{
		Searcher:  searcher,
		DataDir:   t.TempDir(),
		Allowlist: []AllowlistEntry{{Name: "claude-desktop"}},
	}

	result, err := s.handleCallTool(context.Background(), "context_search", map[string]any{
		"query":       "marker",
		"collections": []any{"default"},
		"top_k":       float64(5),
	})
	if err != nil {
		t.Fatalf("TEST-7.1.1: context_search: %v", err)
	}
	if result.IsError {
		t.Fatalf("TEST-7.1.1: context_search returned isError=true")
	}
	if len(searcher.requests) != 1 {
		t.Fatalf("TEST-7.1.1: search calls=%d want 1", len(searcher.requests))
	}
	req := searcher.requests[0]
	if req.GetQuery() != "marker" || req.GetTopK() != 5 || req.GetExplain() {
		t.Fatalf("TEST-7.1.1: request mismatch: %+v", req)
	}

	body := structuredMap(t, result.StructuredContent)
	rows, ok := body["results"].([]any)
	if !ok || len(rows) != 1 {
		t.Fatalf("TEST-7.1.1: structuredContent.results=%#v want one result", body["results"])
	}
	row, ok := rows[0].(map[string]any)
	if !ok {
		t.Fatalf("TEST-7.1.1: result row type=%T", rows[0])
	}
	for _, key := range []string{
		"chunk_id", "context_id", "source_type", "file_path",
		"line_start", "line_end", "score", "retrieval_method",
		"reason", "agent_scope", "redaction_status", "provenance",
	} {
		if _, ok := row[key]; !ok {
			t.Fatalf("TEST-7.1.1: missing REST parity field %q in %#v", key, row)
		}
	}
}

// TEST-7.1.2 / SCEN-7.1.2 / AC2:
// All four MCP tools are real: search/read/explain call the search backend and
// collections scans the data directory.
func TestTask71_AC2_FourToolsAreReal(t *testing.T) {
	dataDir := t.TempDir()
	for _, id := range []string{"default", "team"} {
		if err := os.MkdirAll(filepath.Join(dataDir, "collections", id), 0o755); err != nil {
			t.Fatalf("mkdir collection: %v", err)
		}
	}
	searcher := &fakeSearcher{resp: fakeMCPSearchResponse("chk_deadbeef_0")}
	s := &Server{
		Searcher:  searcher,
		DataDir:   dataDir,
		Allowlist: []AllowlistEntry{{Name: "claude-desktop"}},
	}

	tools, err := s.handleListTools(context.Background())
	if err != nil {
		t.Fatalf("TEST-7.1.2: tools/list: %v", err)
	}
	seen := map[string]bool{}
	for _, tool := range tools {
		seen[tool.Name] = true
		if tool.InputSchema["type"] != "object" {
			t.Fatalf("TEST-7.1.2: tool %s missing object inputSchema", tool.Name)
		}
	}
	for _, name := range []string{"context_search", "context_read", "context_explain", "context_collections"} {
		if !seen[name] {
			t.Fatalf("TEST-7.1.2: missing tool %s in %#v", name, tools)
		}
	}

	readResult, err := s.handleCallTool(context.Background(), "context_read", map[string]any{
		"chunk_id":   "chk_deadbeef_0",
		"collection": "default",
	})
	if err != nil {
		t.Fatalf("TEST-7.1.2: context_read: %v", err)
	}
	if readResult.IsError || len(searcher.requests) == 0 {
		t.Fatalf("TEST-7.1.2: context_read should call backend and return success")
	}
	lastReq := searcher.requests[len(searcher.requests)-1]
	if lastReq.GetQuery() != "chk_deadbeef_0" || lastReq.GetTopK() != 1 {
		t.Fatalf("TEST-7.1.2: read request mismatch: %+v", lastReq)
	}

	explainResult, err := s.handleCallTool(context.Background(), "context_explain", map[string]any{
		"query":       "marker",
		"collections": []any{"default"},
	})
	if err != nil {
		t.Fatalf("TEST-7.1.2: context_explain: %v", err)
	}
	lastReq = searcher.requests[len(searcher.requests)-1]
	if !lastReq.GetExplain() {
		t.Fatalf("TEST-7.1.2: context_explain must set explain=true")
	}
	explainBody := structuredMap(t, explainResult.StructuredContent)
	if _, ok := explainBody["retrieval_trace"]; !ok {
		t.Fatalf("TEST-7.1.2: context_explain missing retrieval_trace in %#v", explainBody)
	}

	collectionsResult, err := s.handleCallTool(context.Background(), "context_collections", map[string]any{})
	if err != nil {
		t.Fatalf("TEST-7.1.2: context_collections: %v", err)
	}
	collectionsBody := structuredMap(t, collectionsResult.StructuredContent)
	collections, ok := collectionsBody["collections"].([]any)
	if !ok || len(collections) != 2 {
		t.Fatalf("TEST-7.1.2: collections=%#v want two entries", collectionsBody["collections"])
	}
}
