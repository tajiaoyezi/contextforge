package contractv1

import (
	"encoding/json"
	"strings"
	"testing"
)

// TestTask392_SearchRequestHybridRoundtrip — task-39.2 §6 AC1: the add-only Hybrid flag
// survives JSON marshal/unmarshal, and a request that omits it (legacy Console client)
// decodes to false (backward-compatible, ADR-015 add-only; mirrors the Semantic precedent).
func TestTask392_SearchRequestHybridRoundtrip(t *testing.T) {
	t.Run("TEST-39.2.1: present+true round-trips", func(t *testing.T) {
		in := SearchRequest{Query: "q", WorkspaceID: "w", Hybrid: true}
		b, err := json.Marshal(in)
		if err != nil {
			t.Fatalf("marshal: %v", err)
		}
		var out SearchRequest
		if err := json.Unmarshal(b, &out); err != nil {
			t.Fatalf("unmarshal: %v", err)
		}
		if !out.Hybrid {
			t.Errorf("Hybrid should round-trip true; got %+v (json=%s)", out, b)
		}
	})

	t.Run("TEST-39.2.1: absent defaults false (legacy client)", func(t *testing.T) {
		var legacy SearchRequest
		if err := json.Unmarshal([]byte(`{"query":"q","workspace_id":"w"}`), &legacy); err != nil {
			t.Fatalf("unmarshal legacy: %v", err)
		}
		if legacy.Hybrid {
			t.Errorf("absent hybrid must default to false; got true")
		}
	})
}

// TestTask392_SearchResultHybridScoreRoundtrip — task-39.2 §6 AC1: the add-only HybridScore
// provenance field survives JSON marshal/unmarshal under the `hybrid_score` tag (parity with
// the console data-plane SearchResultItem.hybrid_score=17, carried end-to-end not inferred).
func TestTask392_SearchResultHybridScoreRoundtrip(t *testing.T) {
	in := SearchResult{ResultID: "r", HybridScore: 0.5}
	b, err := json.Marshal(in)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	if !strings.Contains(string(b), `"hybrid_score"`) {
		t.Errorf("expected hybrid_score json tag; got %s", b)
	}
	var out SearchResult
	if err := json.Unmarshal(b, &out); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if out.HybridScore != 0.5 {
		t.Errorf("HybridScore should round-trip 0.5; got %v", out.HybridScore)
	}
}
