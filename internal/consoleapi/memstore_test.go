package consoleapi

import (
	"errors"
	"testing"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

// TestMemStoreCacheCap — task-31.2 AC2: fallback cache cap is env-configurable
// (CONTEXTFORGE_CONSOLEAPI_CACHE_CAP), falling back to the 256 default when unset/invalid.
func TestMemStoreCacheCap(t *testing.T) {
	if memStoreCacheDefaultCapacity != 256 {
		t.Fatalf("default cap drifted: want 256, got %d", memStoreCacheDefaultCapacity)
	}
	// env override → cap 2; FIFO eviction keeps the cache at 2.
	t.Setenv("CONTEXTFORGE_CONSOLEAPI_CACHE_CAP", "2")
	if got := resolveCacheCapacity(); got != 2 {
		t.Fatalf("env cap: want 2, got %d", got)
	}
	s := NewMemStore()
	s.mu.Lock()
	s.cacheChunkUnlocked("c1", contractv1.SourceChunk{})
	s.cacheChunkUnlocked("c2", contractv1.SourceChunk{})
	s.cacheChunkUnlocked("c3", contractv1.SourceChunk{})
	n := len(s.chunkCache)
	s.mu.Unlock()
	if n != 2 {
		t.Errorf("cap=2: chunk cache should hold 2 (oldest evicted), got %d", n)
	}
	// invalid value → fallback to default 256.
	t.Setenv("CONTEXTFORGE_CONSOLEAPI_CACHE_CAP", "not-a-number")
	if got := resolveCacheCapacity(); got != 256 {
		t.Fatalf("invalid env cap should fall back to 256, got %d", got)
	}
}

// task-17.1 / ADR-022 (Phase 17): MemMemoryStore fallback wires IsPinned into
// Pin / Get / List + SeedFixtures preset of mem-fixture-1 to IsPinned: true.

// TestMemMemoryStore_Pin_TogglesIsPinned — task-17.1 AC2.
func TestMemMemoryStore_Pin_TogglesIsPinned(t *testing.T) {
	s := NewMemMemoryStore()
	s.SeedFixtures()

	item, err := s.Get("mem-fixture-1")
	if err != nil || item == nil {
		t.Fatalf("Get mem-fixture-1: err=%v item=%v", err, item)
	}
	if !item.IsPinned {
		t.Errorf("fixture-1 preset expected IsPinned=true (ADR-022 D3 fixture-1); got false")
	}

	if err := s.Pin("mem-fixture-1", false); err != nil {
		t.Fatalf("Pin(false): %v", err)
	}
	item, _ = s.Get("mem-fixture-1")
	if item.IsPinned {
		t.Errorf("after Pin(false) expected IsPinned=false; got true")
	}

	if err := s.Pin("mem-fixture-1", true); err != nil {
		t.Fatalf("Pin(true): %v", err)
	}
	item, _ = s.Get("mem-fixture-1")
	if !item.IsPinned {
		t.Errorf("after Pin(true) expected IsPinned=true; got false")
	}
}

// TestMemMemoryStore_List_ReturnsIsPinned — task-17.1 AC3.
func TestMemMemoryStore_List_ReturnsIsPinned(t *testing.T) {
	s := NewMemMemoryStore()
	s.SeedFixtures()

	items, err := s.List(MemoryListFilter{})
	if err != nil {
		t.Fatalf("List: %v", err)
	}

	var sawPinned, sawUnpinned bool
	for _, item := range items {
		switch item.MemoryID {
		case "mem-fixture-1":
			if item.IsPinned {
				sawPinned = true
			}
		case "mem-fixture-2", "mem-fixture-3", "mem-fixture-5":
			if !item.IsPinned {
				sawUnpinned = true
			}
		}
	}
	if !sawPinned {
		t.Errorf("expected at least one pinned fixture (mem-fixture-1) in list; items=%+v", items)
	}
	if !sawUnpinned {
		t.Errorf("expected at least one unpinned fixture in list; items=%+v", items)
	}
}

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

// assertNoCacheOrderDup fails if the eviction-order slice contains a duplicate
// key (a move-to-front bug that forgot to remove the prior position desyncs the
// order slice from the map and corrupts eviction — task-33.2 R1).
func assertNoCacheOrderDup(t *testing.T, order []string) {
	t.Helper()
	seen := map[string]bool{}
	for _, k := range order {
		if seen[k] {
			t.Errorf("duplicate key %q in cache order slice %v", k, order)
		}
		seen[k] = true
	}
}

// TestMemStore_CacheEviction_LRU — TEST-33.2.1 (task-33.2 B1): the chunk cache is
// access-order LRU, not FIFO. Filling cap then *reading* the oldest key
// (GetSourceChunk) makes it most-recently-used, so the next insert evicts the
// least-recently-used key — not the first-inserted one (which FIFO would evict).
func TestMemStore_CacheEviction_LRU(t *testing.T) {
	s := NewMemStore()
	if s.cacheCapacity != memStoreCacheDefaultCapacity {
		t.Fatalf("unexpected default cache capacity: got %d want %d", s.cacheCapacity, memStoreCacheDefaultCapacity)
	}
	s.cacheCapacity = 3 // small cap for deterministic eviction
	s.mu.Lock()
	for _, id := range []string{"a", "b", "c"} {
		s.cacheChunkUnlocked(id, contractv1.SourceChunk{ChunkID: id})
	}
	s.mu.Unlock()
	// Read "a" via the public path → "a" becomes most-recently-used.
	// (FIFO would not reorder, leaving "a" first in the eviction order.)
	if _, err := s.GetSourceChunk("a"); err != nil {
		t.Fatalf("expected cached hit for 'a'; got %v", err)
	}
	// Insert "d" → over cap → evict least-recently-used = "b" (NOT "a").
	s.mu.Lock()
	s.cacheChunkUnlocked("d", contractv1.SourceChunk{ChunkID: "d"})
	s.mu.Unlock()
	if _, ok := s.chunkCache["a"]; !ok {
		t.Errorf("LRU broken: recently-read 'a' was evicted")
	}
	if _, ok := s.chunkCache["b"]; ok {
		t.Errorf("LRU broken: least-recently-used 'b' should have been evicted")
	}
	// Drift signals: cache size and order length both equal cap; no dup in order.
	if got := len(s.chunkCache); got != 3 {
		t.Errorf("expected cache size = cap=3; got %d", got)
	}
	if got := len(s.chunkCacheOrder); got != 3 {
		t.Errorf("expected chunkCacheOrder length = cap=3; got %d", got)
	}
	assertNoCacheOrderDup(t, s.chunkCacheOrder)
}

// TestMemStore_CacheEviction_LRU_Trace — TEST-33.2.2 (task-33.2 B1): the trace
// cache is access-order LRU. Both an existing-key overwrite (cacheTraceUnlocked)
// and a read hit (GetSearchTrace) move the key to most-recently-used; eviction
// drops the least-recently-used key.
func TestMemStore_CacheEviction_LRU_Trace(t *testing.T) {
	s := NewMemStore()
	s.cacheCapacity = 3
	s.mu.Lock()
	for _, id := range []string{"a", "b", "c"} {
		s.cacheTraceUnlocked(id, contractv1.RetrievalTrace{TraceID: id})
	}
	// Overwrite existing key "a" → move-to-front (FIFO would leave it first in
	// the eviction order).
	s.cacheTraceUnlocked("a", contractv1.RetrievalTrace{TraceID: "a", Query: "updated"})
	s.mu.Unlock()
	// Insert "d" → evict least-recently-used = "b" ("a" survives the overwrite).
	s.mu.Lock()
	s.cacheTraceUnlocked("d", contractv1.RetrievalTrace{TraceID: "d"})
	s.mu.Unlock()
	if _, ok := s.traceCache["a"]; !ok {
		t.Errorf("LRU broken: overwritten 'a' was evicted")
	}
	if _, ok := s.traceCache["b"]; ok {
		t.Errorf("LRU broken: least-recently-used 'b' should have been evicted")
	}
	// Read hit also moves to front: "c" is now LRU after the "d" insert; reading
	// it makes "a" the least-recently-used, so the next insert evicts "a".
	if _, err := s.GetSearchTrace("c"); err != nil {
		t.Fatalf("expected cached hit for 'c'; got %v", err)
	}
	s.mu.Lock()
	s.cacheTraceUnlocked("e", contractv1.RetrievalTrace{TraceID: "e"})
	s.mu.Unlock()
	if _, ok := s.traceCache["c"]; !ok {
		t.Errorf("LRU broken: recently-read 'c' was evicted")
	}
	if _, ok := s.traceCache["a"]; ok {
		t.Errorf("LRU broken: least-recently-used 'a' should have been evicted after 'c' was read")
	}
	if got := len(s.traceCacheOrder); got != 3 {
		t.Errorf("expected traceCacheOrder length = cap=3; got %d", got)
	}
	assertNoCacheOrderDup(t, s.traceCacheOrder)
}

// TestMemMemoryStore_EventParity — task-31.1 AC1: memory write ops emit memory.* events into the
// wired fallback ring (parity with workspace/job + the Rust data plane), with Rust-aligned
// event_type names; the five write ops keep their return/error contract.
func TestMemMemoryStore_EventParity(t *testing.T) {
	store := NewMemStore()
	mem := NewMemMemoryStore()
	mem.SetEventSink(store.EmitEvent)
	mem.SeedFixtures()

	// 5 successful write ops on a seeded item → 5 events with the expected event_type names.
	if err := mem.Pin("mem-fixture-2", true); err != nil {
		t.Fatalf("Pin: %v", err)
	}
	if err := mem.Deprecate("mem-fixture-2"); err != nil {
		t.Fatalf("Deprecate: %v", err)
	}
	if err := mem.Unpin("mem-fixture-2"); err != nil {
		t.Fatalf("Unpin: %v", err)
	}
	if err := mem.SoftDelete("mem-fixture-2"); err != nil {
		t.Fatalf("SoftDelete: %v", err)
	}
	if err := mem.HardDelete("mem-fixture-2"); err != nil {
		t.Fatalf("HardDelete: %v", err)
	}

	evts, err := store.Recent(100, 0)
	if err != nil {
		t.Fatalf("Recent: %v", err)
	}
	got := map[string]int{}
	for _, e := range evts {
		got[e.EventType]++
	}
	// Pin + Unpin both map to memory.pin (Rust MemoryPin | MemoryUnpin → memory.pin).
	for et, want := range map[string]int{
		"memory.pin":         2, // Pin + Unpin
		"memory.deprecate":   1,
		"memory.soft_delete": 1,
		"memory.hard_delete": 1,
	} {
		if got[et] != want {
			t.Errorf("event_type %q: got %d, want %d (all events: %v)", et, got[et], want, got)
		}
	}

	// Error path (missing item) emits nothing + returns ErrNotFound.
	before := len(evts)
	if err := mem.Pin("does-not-exist", true); !errors.Is(err, ErrNotFound) {
		t.Fatalf("Pin(missing) expected ErrNotFound, got %v", err)
	}
	after, _ := store.Recent(100, 0)
	if len(after) != before {
		t.Errorf("error path must not emit an event: ring grew %d → %d", before, len(after))
	}

	// No sink wired → no panic, ops still succeed (observation != authority).
	noSink := NewMemMemoryStore()
	noSink.SeedFixtures()
	if err := noSink.Pin("mem-fixture-1", false); err != nil {
		t.Fatalf("Pin without sink should still succeed: %v", err)
	}
}
