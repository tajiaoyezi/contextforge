package consoleapi

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TEST-44.1.4 (task-44.1 / ADR-049 D3): the Go unpin-actor 透传链 is wired —
// handleMemoryUnpin reads the X-Actor header (mirroring handleMemoryPin) and
// grpcclient fills pb.UnpinMemoryRequest.Actor. Static source grep (mirrors the
// smoke_syntax_test.go convention) since the actor's real landing point is the
// Rust audit/event source (covered by TEST-44.1.1/.2/.3), not the Go side.
func TestTask441_UnpinActorPropagationWired(t *testing.T) {
	for _, tgt := range []struct {
		path string
		must []string
	}{
		{"internal/consoleapi/handlers.go", []string{`handleMemoryUnpin`, `r.Header.Get("X-Actor")`, `deps.Memory.Unpin(id, actor)`}},
		{"internal/consoleapi/grpcclient/grpcclient.go", []string{`func (m *memoryClient) Unpin(id string, actor string) error`, `pb.UnpinMemoryRequest{MemoryId: id, Actor: actor}`}},
		{"internal/consoleapi/types.go", []string{`Unpin(memoryID string, actor string) error`}},
	} {
		body, err := os.ReadFile(filepath.Join("..", "..", tgt.path))
		if err != nil {
			t.Fatalf("read %s: %v", tgt.path, err)
		}
		s := string(body)
		for _, m := range tgt.must {
			if !strings.Contains(s, m) {
				t.Fatalf("%s missing %q (unpin actor 透传链 not wired)", tgt.path, m)
			}
		}
	}
}
