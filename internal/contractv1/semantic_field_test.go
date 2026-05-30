package contractv1

import (
	"encoding/json"
	"testing"
)

// TestTask201_SearchRequestSemanticRoundtrip — task-20.1 §6 AC1: the add-only
// Semantic flag survives JSON marshal/unmarshal, and a request that omits it
// (legacy Console client) decodes to false (backward-compatible, ADR-015 add-only).
func TestTask201_SearchRequestSemanticRoundtrip(t *testing.T) {
	t.Run("TEST-20.1.1: present+true round-trips", func(t *testing.T) {
		in := SearchRequest{Query: "q", WorkspaceID: "w", Semantic: true}
		b, err := json.Marshal(in)
		if err != nil {
			t.Fatalf("marshal: %v", err)
		}
		var out SearchRequest
		if err := json.Unmarshal(b, &out); err != nil {
			t.Fatalf("unmarshal: %v", err)
		}
		if !out.Semantic {
			t.Errorf("Semantic should round-trip true; got %+v (json=%s)", out, b)
		}
	})

	t.Run("TEST-20.1.1: absent defaults false (legacy client)", func(t *testing.T) {
		var legacy SearchRequest
		if err := json.Unmarshal([]byte(`{"query":"q","workspace_id":"w"}`), &legacy); err != nil {
			t.Fatalf("unmarshal legacy: %v", err)
		}
		if legacy.Semantic {
			t.Errorf("absent semantic must default to false; got true")
		}
	})
}
