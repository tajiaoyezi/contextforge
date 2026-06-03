package exporter

import (
	"context"
	"testing"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// TestTask313_ExporterFillsRealContent — task-31.3 AC2: with the ChunkLoader wired (daemon
// ListAllChunks), exported records carry real content + a real ContentHash (not content="" /
// sha256-of-empty). A nil loader keeps the prior backward-compatible empty content.
func TestTask313_ExporterFillsRealContent(t *testing.T) {
	restoreSearch := SetSearchBackend(func(
		_ context.Context, _ string, _ *contextforgev1.SearchRequest,
	) (*contextforgev1.SearchResponse, error) {
		return &contextforgev1.SearchResponse{
			Results: []*contextforgev1.RetrievalResult{
				{ChunkId: "chunk-1", ContextId: "ctx-1", FilePath: "a/readme.md"},
			},
		}, nil
	})
	defer restoreSearch()

	restoreLoader := SetChunkLoader(func(
		_ context.Context, _ string, _ string,
	) (map[string]string, error) {
		return map[string]string{"chunk-1": "the real chunk body"}, nil
	})
	defer restoreLoader()

	recs, err := loadRecords(context.Background(), "/tmp/data", "default")
	if err != nil {
		t.Fatalf("loadRecords: %v", err)
	}
	if len(recs) != 1 {
		t.Fatalf("want 1 record, got %d", len(recs))
	}
	if recs[0].Content != "the real chunk body" {
		t.Errorf("content not filled from ChunkLoader: %q", recs[0].Content)
	}
	if want := contentHash("the real chunk body"); recs[0].ContentHash != want {
		t.Errorf("ContentHash = %q, want %q", recs[0].ContentHash, want)
	}
	if recs[0].ContentHash == contentHash("") {
		t.Errorf("ContentHash must not be sha256-of-empty")
	}

	// Backward compat: a nil loader leaves content empty (prior behavior).
	restoreNil := SetChunkLoader(nil)
	defer restoreNil()
	recs2, err := loadRecords(context.Background(), "/tmp/data", "default")
	if err != nil {
		t.Fatalf("loadRecords (nil loader): %v", err)
	}
	if recs2[0].Content != "" {
		t.Errorf("nil ChunkLoader should leave content empty, got %q", recs2[0].Content)
	}
}
