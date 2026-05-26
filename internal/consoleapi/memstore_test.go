package consoleapi

import (
	"errors"
	"fmt"
	"testing"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

// task-15.1 (Phase 15 P0 #1) — MemStore chunk/trace cache fallback.
//
// In CONSOLE_API_FALLBACK_INMEM=1 mode, MemStore.Search emits a stub
// SearchResult; before task-15.1, drill-down requests for the corresponding
// SourceChunk / RetrievalTrace returned 503 because the stub was not
// persisted. The cache now captures stub outputs of Search() so subsequent
// GET /v1/source-chunks/{id} and GET /v1/search/{query_id}/trace surface
// 200 with the cached payload.

// TestMemStore_ChunkCacheHit_AfterSearch — AC1: Search → GetSourceChunk hits cache.
func TestMemStore_ChunkCacheHit_AfterSearch(t *testing.T) {
	s := NewMemStore()
	res, _, err := s.Search(contractv1.SearchRequest{
		Query:           "configuration",
		WorkspaceID:     "ws-1",
		RetrievalMethod: "bm25",
	})
	if err != nil {
		t.Fatalf("Search returned error: %v", err)
	}
	if res.ChunkID == "" {
		t.Fatalf("expected non-empty chunk id from stub; got empty")
	}
	chunk, err := s.GetSourceChunk(res.ChunkID)
	if err != nil {
		t.Fatalf("expected cached chunk hit; got err=%v", err)
	}
	if chunk.ChunkID != res.ChunkID {
		t.Errorf("chunk id mismatch: cached=%q want=%q", chunk.ChunkID, res.ChunkID)
	}
	if chunk.SourceFilePath != res.SourceFilePath {
		t.Errorf("source_file_path mismatch: cached=%q want=%q", chunk.SourceFilePath, res.SourceFilePath)
	}
	if chunk.WorkspaceID != res.WorkspaceID {
		t.Errorf("workspace_id mismatch: cached=%q want=%q", chunk.WorkspaceID, res.WorkspaceID)
	}
	if chunk.RedactionStatus != "none" {
		t.Errorf("expected redaction_status=none for fallback; got %q", chunk.RedactionStatus)
	}
}

// TestMemStore_TraceCacheHit_AfterSearch — AC1: Search → GetSearchTrace hits cache.
// Trace key alignment: MemStore.Search now sets trace.TraceID = res.QueryID so
// callers can resolve traces via the query_id returned by /v1/search.
func TestMemStore_TraceCacheHit_AfterSearch(t *testing.T) {
	s := NewMemStore()
	res, traceFromSearch, err := s.Search(contractv1.SearchRequest{
		Query:           "configuration",
		WorkspaceID:     "ws-1",
		RetrievalMethod: "bm25",
	})
	if err != nil {
		t.Fatalf("Search returned error: %v", err)
	}
	if traceFromSearch.TraceID != res.QueryID {
		t.Fatalf("trace.TraceID must align with res.QueryID; got trace=%q query=%q",
			traceFromSearch.TraceID, res.QueryID)
	}
	got, err := s.GetSearchTrace(res.QueryID)
	if err != nil {
		t.Fatalf("expected cached trace hit; got err=%v", err)
	}
	if got.TraceID != res.QueryID {
		t.Errorf("trace_id mismatch: cached=%q want=%q", got.TraceID, res.QueryID)
	}
	if got.Query != "configuration" {
		t.Errorf("query mismatch: cached=%q want=%q", got.Query, "configuration")
	}
}

// TestMemStore_CacheMiss_Returns503 — AC3: unknown chunk_id with no prior Search
// stays on the v0.7 503 path (deep defense / ADR-016 D4).
func TestMemStore_CacheMiss_Returns503(t *testing.T) {
	s := NewMemStore()
	// No Search() called → cache empty.
	_, err := s.GetSourceChunk("never-searched-chunk")
	if !errors.Is(err, ErrDataPlaneUnavailable) {
		t.Errorf("expected ErrDataPlaneUnavailable for cache miss; got %v", err)
	}
	_, err = s.GetSearchTrace("never-searched-trace")
	if !errors.Is(err, ErrDataPlaneUnavailable) {
		t.Errorf("expected ErrDataPlaneUnavailable for trace cache miss; got %v", err)
	}
}

// TestMemStore_GetChunksStats_Stub — task-15.3 AC5: MemStore fallback returns
// {total=0, today_delta=0} without error when SearchBackend is unwired.
func TestMemStore_GetChunksStats_Stub(t *testing.T) {
	s := NewMemStore()
	stats, err := s.GetChunksStats("")
	if err != nil {
		t.Fatalf("expected nil err for fallback stub; got %v", err)
	}
	if stats.Total != 0 || stats.TodayDelta != 0 {
		t.Errorf("expected {0,0}; got total=%d today_delta=%d", stats.Total, stats.TodayDelta)
	}
	// workspace_id should be a passive filter — same fallback shape.
	stats2, err := s.GetChunksStats("ws-abc")
	if err != nil {
		t.Fatalf("expected nil err for filtered fallback; got %v", err)
	}
	if stats2 != stats {
		t.Errorf("filtered fallback differs from default: %+v vs %+v", stats2, stats)
	}
}

// TestMemStore_CacheEviction_FIFO — AC2: 257th Search evicts oldest entry.
func TestMemStore_CacheEviction_FIFO(t *testing.T) {
	s := NewMemStore()
	cap := s.cacheCapacity
	if cap != memStoreCacheDefaultCapacity {
		t.Fatalf("unexpected default cache capacity: got %d want %d", cap, memStoreCacheDefaultCapacity)
	}
	// Synthesize cap+1 cache writes via the unlocked helper to avoid the stub
	// Search collision (all stub responses share chunk_id="chunk-1"); FIFO
	// eviction is a property of the helper, not Search.
	first := "ck-0000"
	s.mu.Lock()
	for i := 0; i < cap+1; i++ {
		id := fmt.Sprintf("ck-%04d", i)
		s.cacheChunkUnlocked(id, contractv1.SourceChunk{ChunkID: id})
	}
	s.mu.Unlock()
	// First entry should now be evicted.
	if _, ok := s.chunkCache[first]; ok {
		t.Errorf("FIFO eviction broken: oldest chunk %q still cached after %d writes", first, cap+1)
	}
	// Last entry should remain.
	last := fmt.Sprintf("ck-%04d", cap)
	if _, ok := s.chunkCache[last]; !ok {
		t.Errorf("expected newest chunk %q to be cached", last)
	}
	// Total entries should equal capacity.
	if got := len(s.chunkCache); got != cap {
		t.Errorf("expected cache size = cap=%d; got %d", cap, got)
	}
	// chunkCacheOrder length must equal cap (drift signal).
	if got := len(s.chunkCacheOrder); got != cap {
		t.Errorf("expected chunkCacheOrder length = cap=%d; got %d", cap, got)
	}
}
