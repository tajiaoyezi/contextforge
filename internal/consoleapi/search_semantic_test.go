package consoleapi

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

// capturingSearch is a SearchClient test double that records the last
// SearchRequest it received, so handler-level OR-merge can be asserted.
type capturingSearch struct {
	last contractv1.SearchRequest
}

func (c *capturingSearch) Search(req contractv1.SearchRequest) (contractv1.SearchResult, contractv1.RetrievalTrace, error) {
	c.last = req
	return contractv1.SearchResult{}, contractv1.RetrievalTrace{}, nil
}
func (c *capturingSearch) GetSourceChunk(string) (contractv1.SourceChunk, error) {
	return contractv1.SourceChunk{}, nil
}
func (c *capturingSearch) GetSearchTrace(string) (contractv1.RetrievalTrace, error) {
	return contractv1.RetrievalTrace{}, nil
}
func (c *capturingSearch) GetChunksStats(string) (contractv1.ChunksStats, error) {
	return contractv1.ChunksStats{}, nil
}
func (c *capturingSearch) ListQueries(int) ([]contractv1.QueryRecord, error) {
	return nil, nil
}

// TestTask201_HandleSearchSemanticORMerge — task-20.1 §6 AC2: handleSearch
// OR-merges the `?semantic=true` query param with the body `semantic` field
// (mirrors internal/daemon/rest.go); the resulting flag is forwarded to the
// downstream SearchClient. Default (neither set) → false.
func TestTask201_HandleSearchSemanticORMerge(t *testing.T) {
	cases := []struct {
		name string
		url  string
		body string
		want bool
	}{
		{"query param true", "/v1/search?semantic=true", `{"query":"q","workspace_id":"w"}`, true},
		{"body field true", "/v1/search", `{"query":"q","workspace_id":"w","semantic":true}`, true},
		{"query OR body", "/v1/search?semantic=true", `{"query":"q","workspace_id":"w","semantic":false}`, true},
		{"neither", "/v1/search", `{"query":"q","workspace_id":"w"}`, false},
		{"query param non-true", "/v1/search?semantic=1", `{"query":"q","workspace_id":"w"}`, false},
	}
	for _, tc := range cases {
		t.Run("TEST-20.1.2: "+tc.name, func(t *testing.T) {
			cap := &capturingSearch{}
			h := handleSearch(Deps{Search: cap})
			req := httptest.NewRequest("POST", tc.url, strings.NewReader(tc.body))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()
			h(w, req)
			if w.Code != http.StatusOK {
				t.Fatalf("status = %d, want 200 (body=%s)", w.Code, w.Body.String())
			}
			if cap.last.Semantic != tc.want {
				t.Errorf("forwarded Semantic = %v, want %v", cap.last.Semantic, tc.want)
			}
		})
	}
}
