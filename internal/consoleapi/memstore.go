package consoleapi

import (
	"encoding/json"
	"fmt"
	"os"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/contractv1"
)

// MemStore is a single in-memory backing struct that implements all four
// client interfaces (WorkspaceClient / JobClient / SearchClient /
// EventsClient). v0.3 trade-off (§10 #1): cross-process Rust ↔ Go SQLite
// sharing is deferred to task-future.cross-process-sqlite-sharing; v0.3 ships
// REST contract conformance only.
//
// MemStore is safe for concurrent use.
type MemStore struct {
	mu         sync.Mutex
	workspaces map[string]contractv1.Workspace
	jobs       map[string]contractv1.IndexJob
	jobOrder   []string                        // insertion order
	events     []contractv1.ObservabilityEvent // append-only ring (capped at 1000)
	// Optional injected Search backend (production wires to retriever / Rust
	// CoreService::search). Tests provide a fake.
	SearchBackend SearchClient
	// task-15.1 (Phase 15 P0 #1): fallback search-result cache. MemStore.Search
	// emits a stub SearchResult; without persisting it, subsequent
	// GetSourceChunk / GetSearchTrace hit a 503 path that breaks Console UI
	// drill-down flow under CONSOLE_API_FALLBACK_INMEM=1. The two caches
	// preserve the most recent search outputs with access-order LRU eviction at
	// cacheCapacity (read hits + overwrites move-to-front; the eviction victim is
	// the least-recently-used key). Cache miss falls through to the v0.7
	// ErrDataPlaneUnavailable path (deep-defense unchanged).
	chunkCache      map[string]contractv1.SourceChunk
	chunkCacheOrder []string
	traceCache      map[string]contractv1.RetrievalTrace
	traceCacheOrder []string
	cacheCapacity   int
	// monotonic id seed for jobs.
	jobSeq uint64
}

// memStoreCacheDefaultCapacity caps both chunk and trace caches in the fallback MemStore. 256 is
// sufficient for single-user Console UI demo flow; operators can override via the
// CONTEXTFORGE_CONSOLEAPI_CACHE_CAP env var (task-31.2).
const memStoreCacheDefaultCapacity = 256

// resolveCacheCapacity reads CONTEXTFORGE_CONSOLEAPI_CACHE_CAP (a positive int) and falls back to
// memStoreCacheDefaultCapacity when the var is unset, empty, non-numeric, or <= 0 (task-31.2).
func resolveCacheCapacity() int {
	if v := os.Getenv("CONTEXTFORGE_CONSOLEAPI_CACHE_CAP"); v != "" {
		if n, err := strconv.Atoi(v); err == nil && n > 0 {
			return n
		}
	}
	return memStoreCacheDefaultCapacity
}

func NewMemStore() *MemStore {
	return &MemStore{
		workspaces:    map[string]contractv1.Workspace{},
		jobs:          map[string]contractv1.IndexJob{},
		chunkCache:    map[string]contractv1.SourceChunk{},
		traceCache:    map[string]contractv1.RetrievalTrace{},
		cacheCapacity: resolveCacheCapacity(),
	}
}

// moveToMRU removes key from order (if present) and appends it to the back,
// marking it most-recently-used. Front of the slice is therefore the
// least-recently-used key (the eviction victim). O(n) over a slice capped at
// cacheCapacity (default 256). Returns the updated slice; caller reassigns.
func moveToMRU(order []string, key string) []string {
	for i, k := range order {
		if k == key {
			order = append(order[:i], order[i+1:]...)
			break
		}
	}
	return append(order, key)
}

// cacheChunkUnlocked records sc under chunkID with access-order LRU eviction at
// cacheCapacity (an existing-key overwrite counts as a use → move-to-front).
// Caller must hold s.mu.
func (s *MemStore) cacheChunkUnlocked(chunkID string, sc contractv1.SourceChunk) {
	if chunkID == "" {
		return
	}
	if _, exists := s.chunkCache[chunkID]; exists {
		s.chunkCache[chunkID] = sc
		s.chunkCacheOrder = moveToMRU(s.chunkCacheOrder, chunkID)
		return
	}
	s.chunkCache[chunkID] = sc
	s.chunkCacheOrder = append(s.chunkCacheOrder, chunkID)
	if len(s.chunkCacheOrder) > s.cacheCapacity {
		evict := s.chunkCacheOrder[0]
		s.chunkCacheOrder = s.chunkCacheOrder[1:]
		delete(s.chunkCache, evict)
	}
}

// cacheTraceUnlocked records trace under traceKey (set to QueryID by the
// MemStore.Search stub so GetSearchTrace can lookup by query_id). Access-order
// LRU eviction at cacheCapacity (an existing-key overwrite counts as a use →
// move-to-front). Caller must hold s.mu.
func (s *MemStore) cacheTraceUnlocked(traceKey string, t contractv1.RetrievalTrace) {
	if traceKey == "" {
		return
	}
	if _, exists := s.traceCache[traceKey]; exists {
		s.traceCache[traceKey] = t
		s.traceCacheOrder = moveToMRU(s.traceCacheOrder, traceKey)
		return
	}
	s.traceCache[traceKey] = t
	s.traceCacheOrder = append(s.traceCacheOrder, traceKey)
	if len(s.traceCacheOrder) > s.cacheCapacity {
		evict := s.traceCacheOrder[0]
		s.traceCacheOrder = s.traceCacheOrder[1:]
		delete(s.traceCache, evict)
	}
}

// emitEvent records an ObservabilityEvent (capped at 1000 most-recent).
func (s *MemStore) emitEvent(eventType, severity, source, message string, jobID *string) {
	s.events = append(s.events, contractv1.ObservabilityEvent{
		EventID:      fmt.Sprintf("evt-%d", time.Now().UnixNano()),
		EventType:    eventType,
		Severity:     severity,
		Source:       source,
		Message:      message,
		Timestamp:    time.Now().UTC(),
		JobID:        jobID,
		Availability: contractv1.FieldAvailability{Object: "ObservabilityEvent"},
	})
	if len(s.events) > 1000 {
		s.events = s.events[len(s.events)-1000:]
	}
}

// EmitEvent is a thread-safe observability-event sink for sibling fallback stores
// (task-31.1: MemMemoryStore memory ops emit memory.* events here for parity with the
// workspace/job paths + the Rust data plane). Best-effort; takes its own lock (the
// unexported emitEvent assumes the caller already holds s.mu).
func (s *MemStore) EmitEvent(eventType, severity, source, message string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.emitEvent(eventType, severity, source, message, nil)
}

// ---- WorkspaceClient + JobClient adapters (split to avoid method collision) ----

// WorkspaceAdapter wraps MemStore for WorkspaceClient interface.
type WorkspaceAdapter struct{ S *MemStore }

func (a WorkspaceAdapter) Create(req contractv1.WorkspaceCreate) (contractv1.Workspace, error) {
	return a.S.CreateWorkspace(req)
}
func (a WorkspaceAdapter) List() ([]contractv1.Workspace, error) { return a.S.ListWorkspaces() }
func (a WorkspaceAdapter) Get(id string) (*contractv1.Workspace, error) {
	return a.S.GetWorkspace(id)
}
func (a WorkspaceAdapter) Update(id string, allowlist, denylist []string) (contractv1.Workspace, error) {
	return a.S.UpdateWorkspaceConfig(id, allowlist, denylist)
}

// task-51.3 (Phase 51 / ADR-052 D3): owner-scoped methods on the in-memory
// fallback store. The MemStore fallback does NOT track owner_id (single-user
// local-first demo per ADR-016 §D4), so these delegate to the byte-equivalent
// non-owner methods — the real ownership enforcement lives in the Rust
// WorkspaceStore (task-51.1) reached via the grpcclient path. The fallback
// stays permissive (shows/creates all workspaces unowned) so the Console UI
// demo + conformance tests remain functional when the daemon is unreachable.
func (a WorkspaceAdapter) CreateOwned(req contractv1.WorkspaceCreate, _ string) (contractv1.Workspace, error) {
	return a.S.CreateWorkspace(req)
}
func (a WorkspaceAdapter) ListOwned(_ string) ([]contractv1.Workspace, error) {
	return a.S.ListWorkspaces()
}
func (a WorkspaceAdapter) GetIfOwned(id, _ string) (*contractv1.Workspace, error) {
	return a.S.GetWorkspace(id)
}

// JobAdapter wraps MemStore for JobClient interface.
type JobAdapter struct{ S *MemStore }

func (a JobAdapter) Enqueue(workspaceID, triggerSource string) (contractv1.IndexJob, error) {
	return a.S.EnqueueJob(workspaceID, triggerSource)
}
func (a JobAdapter) Get(jobID string) (*contractv1.IndexJob, error) { return a.S.GetJob(jobID) }
func (a JobAdapter) Cancel(jobID string) error                      { return a.S.CancelJob(jobID) }
func (a JobAdapter) ListActive() ([]contractv1.IndexJob, error)     { return a.S.ListActiveJobs() }

// ---- MemStore raw methods ----

func (s *MemStore) CreateWorkspace(req contractv1.WorkspaceCreate) (contractv1.Workspace, error) {
	if strings.TrimSpace(req.Name) == "" {
		return contractv1.Workspace{}, fmt.Errorf("%w: name must not be empty", ErrInvalidRequest)
	}
	if strings.TrimSpace(req.RootPath) == "" {
		return contractv1.Workspace{}, fmt.Errorf("%w: root_path must not be empty", ErrInvalidRequest)
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	id := workspaceIDFromName(req.Name, len(s.workspaces))
	if _, exists := s.workspaces[id]; exists {
		return contractv1.Workspace{}, fmt.Errorf("%w: workspace_id already exists: %s", ErrInvalidRequest, id)
	}
	allow := req.Allowlist
	deny := req.Denylist
	if allow == nil {
		allow = []string{}
	}
	if deny == nil {
		deny = []string{}
	}
	cfg, _ := json.Marshal(map[string]any{
		"allowlist": allow,
		"denylist":  deny,
	})
	now := time.Now().UTC()
	ws := contractv1.Workspace{
		WorkspaceID:    id,
		Name:           req.Name,
		RootPath:       req.RootPath,
		Status:         "ready",
		ConfigSnapshot: cfg,
		CreatedAt:      now,
		UpdatedAt:      now,
		Availability:   contractv1.FieldAvailability{Object: "Workspace"},
	}
	s.workspaces[id] = ws
	source := "consoleapi"
	s.emitEvent("workspace.created", "info", source, "workspace created: "+id, nil)
	return ws, nil
}

func (s *MemStore) ListWorkspaces() ([]contractv1.Workspace, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	out := make([]contractv1.Workspace, 0, len(s.workspaces))
	for _, ws := range s.workspaces {
		out = append(out, ws)
	}
	sort.Slice(out, func(i, j int) bool {
		return out[i].CreatedAt.Before(out[j].CreatedAt)
	})
	return out, nil
}

func (s *MemStore) GetWorkspace(id string) (*contractv1.Workspace, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	ws, ok := s.workspaces[id]
	if !ok {
		return nil, nil
	}
	return &ws, nil
}

// UpdateWorkspaceConfig overwrites allowlist + denylist and bumps UpdatedAt.
// task-12.1 (ADR-017 D1 Wave 1) fallback mode pair to gRPC WorkspaceService.UpdateConfig.
func (s *MemStore) UpdateWorkspaceConfig(id string, allowlist, denylist []string) (contractv1.Workspace, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	ws, ok := s.workspaces[id]
	if !ok {
		return contractv1.Workspace{}, fmt.Errorf("%w: workspace %s", ErrNotFound, id)
	}
	if allowlist == nil {
		allowlist = []string{}
	}
	if denylist == nil {
		denylist = []string{}
	}
	cfg, _ := json.Marshal(map[string]any{
		"allowlist": allowlist,
		"denylist":  denylist,
	})
	ws.ConfigSnapshot = cfg
	ws.UpdatedAt = time.Now().UTC()
	s.workspaces[id] = ws
	s.emitEvent("workspace.updated", "info", "consoleapi", "workspace config updated: "+id, nil)
	return ws, nil
}

// ---- Job raw methods ----

func (s *MemStore) EnqueueJob(workspaceID, triggerSource string) (contractv1.IndexJob, error) {
	if strings.TrimSpace(workspaceID) == "" {
		return contractv1.IndexJob{}, fmt.Errorf("%w: workspace_id is required", ErrInvalidRequest)
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if _, ok := s.workspaces[workspaceID]; !ok {
		return contractv1.IndexJob{}, fmt.Errorf("%w: workspace not found: %s", ErrNotFound, workspaceID)
	}
	s.jobSeq++
	jobID := fmt.Sprintf("job-%d", s.jobSeq)
	job := contractv1.IndexJob{
		JobID:         jobID,
		WorkspaceID:   workspaceID,
		TriggerSource: triggerSource,
		Status:        "queued",
		Stage:         "",
		Availability:  contractv1.FieldAvailability{Object: "IndexJob"},
	}
	s.jobs[jobID] = job
	s.jobOrder = append(s.jobOrder, jobID)
	jid := jobID
	s.emitEvent("indexjob.enqueued", "info", "consoleapi", "index job enqueued: "+jobID, &jid)
	return job, nil
}

func (s *MemStore) GetJob(id string) (*contractv1.IndexJob, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	job, ok := s.jobs[id]
	if !ok {
		return nil, nil
	}
	return &job, nil
}

// ListActiveJobs returns jobs in status queued or running (insertion order).
// task-12.1 (ADR-017 D1 Wave 1) fallback pair to gRPC JobService.List status=active.
func (s *MemStore) ListActiveJobs() ([]contractv1.IndexJob, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	out := make([]contractv1.IndexJob, 0, len(s.jobOrder))
	for _, id := range s.jobOrder {
		j, ok := s.jobs[id]
		if !ok {
			continue
		}
		if j.Status == "queued" || j.Status == "running" {
			out = append(out, j)
		}
	}
	return out, nil
}

func (s *MemStore) CancelJob(jobID string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	job, ok := s.jobs[jobID]
	if !ok {
		return fmt.Errorf("%w: %s", ErrNotFound, jobID)
	}
	switch job.Status {
	case "succeeded", "failed", "cancelled":
		return fmt.Errorf("%w: %s", ErrJobTerminal, job.Status)
	}
	job.Status = "cancelled"
	now := time.Now().UTC()
	job.FinishedAt = &now
	s.jobs[jobID] = job
	jid := jobID
	s.emitEvent("indexjob.cancelled", "info", "consoleapi", "index job cancelled: "+jobID, &jid)
	return nil
}

// ---- SearchClient (delegates to injected backend; provides stub for tests) ----

// GetSourceChunk — task-12.2 + task-15.1 fallback path. MemStore has no real
// index, but task-15.1 caches stub SearchResults emitted by Search() so a
// drill-down GET after POST /v1/search returns 200 rather than 503. Cache miss
// still returns ErrDataPlaneUnavailable for deep defense (ADR-016 D4).
func (s *MemStore) GetSourceChunk(chunkID string) (contractv1.SourceChunk, error) {
	s.mu.Lock()
	if sc, ok := s.chunkCache[chunkID]; ok {
		s.chunkCacheOrder = moveToMRU(s.chunkCacheOrder, chunkID) // access-order LRU: a read is a use
		s.mu.Unlock()
		return sc, nil
	}
	s.mu.Unlock()
	if s.SearchBackend != nil {
		return s.SearchBackend.GetSourceChunk(chunkID)
	}
	return contractv1.SourceChunk{}, ErrDataPlaneUnavailable
}

// GetSearchTrace — task-12.3 + task-15.1 fallback path. Looks up the trace
// cache keyed by query_id (Search() aligns trace.TraceID with res.QueryID so
// callers can resolve traces via the query_id from the search response).
func (s *MemStore) GetSearchTrace(queryID string) (contractv1.RetrievalTrace, error) {
	s.mu.Lock()
	if t, ok := s.traceCache[queryID]; ok {
		s.traceCacheOrder = moveToMRU(s.traceCacheOrder, queryID) // access-order LRU: a read is a use
		s.mu.Unlock()
		return t, nil
	}
	s.mu.Unlock()
	if s.SearchBackend != nil {
		return s.SearchBackend.GetSearchTrace(queryID)
	}
	return contractv1.RetrievalTrace{}, ErrDataPlaneUnavailable
}

// GetChunksStats — task-15.3 fallback path. MemStore has no real index;
// returns zero stats so Console UI renders "no data" rather than 503.
// [SPEC-OWNER:task-15.3]
func (s *MemStore) GetChunksStats(workspaceID string) (contractv1.ChunksStats, error) {
	if s.SearchBackend != nil {
		return s.SearchBackend.GetChunksStats(workspaceID)
	}
	return contractv1.ChunksStats{Total: 0, TodayDelta: 0}, nil
}

// ListQueries — task-15.5 fallback path. Returns QueryRecord entries derived
// from the in-memory traceCache (populated by Search). traceCache keys are
// query_ids; values include trace.Query. fallback ts_unix is set when Search()
// writes the trace; if absent we return 0 [SPEC-OWNER:task-15.5].
func (s *MemStore) ListQueries(limit int) ([]contractv1.QueryRecord, error) {
	if s.SearchBackend != nil {
		return s.SearchBackend.ListQueries(limit)
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	out := make([]contractv1.QueryRecord, 0, len(s.traceCache))
	for queryID, trace := range s.traceCache {
		out = append(out, contractv1.QueryRecord{
			QueryID: queryID,
			Query:   trace.Query,
			TsUnix:  0, // fallback does not stamp ts; [SPEC-OWNER:task-15.5]
		})
	}
	// Stable secondary order by QueryID so tests are deterministic.
	sort.Slice(out, func(i, j int) bool { return out[i].QueryID > out[j].QueryID })
	if limit <= 0 {
		limit = 20
	}
	if limit > 100 {
		limit = 100
	}
	if len(out) > limit {
		out = out[:limit]
	}
	return out, nil
}

func (s *MemStore) Search(req contractv1.SearchRequest) (contractv1.SearchResult, contractv1.RetrievalTrace, error) {
	if s.SearchBackend != nil {
		return s.SearchBackend.Search(req)
	}
	// fallback minimal response for v0.3 contract conformance (no real search).
	res := contractv1.SearchResult{
		ResultID:         "result-stub-1",
		QueryID:          "query-1",
		WorkspaceID:      req.WorkspaceID,
		SourceFilePath:   "examples/quickstart/sample-project/docs/config.md",
		SourceFileType:   "md",
		ChunkID:          "chunk-1",
		ChunkTextPreview: "configuration sample preview",
		LineStart:        1,
		LineEnd:          10,
		Score:            0.5,
		RankBeforeRerank: 1,
		RankAfterRerank:  nil,
		RetrievalMethod:  req.RetrievalMethod,
		Reason:           "v0.3 in-memory stub — wire real retriever in v0.4",
		Citation: contractv1.Citation{
			CitationID:     "cit-1",
			SourceFilePath: "examples/quickstart/sample-project/docs/config.md",
			ChunkID:        "chunk-1",
			LineStart:      1,
			LineEnd:        10,
			Confidence:     0.5,
			Availability:   contractv1.FieldAvailability{Object: "Citation"},
		},
		Availability: contractv1.FieldAvailability{Object: "SearchResult"},
	}
	expanded := ""
	// task-15.1: align TraceID with QueryID so GetSearchTrace keyed by query_id
	// hits the cache (Console UI flow: POST /v1/search → response.result.query_id
	// → GET /v1/search/{query_id}/trace).
	trace := contractv1.RetrievalTrace{
		TraceID:                  res.QueryID,
		Query:                    req.Query,
		ExpandedQuery:            &expanded,
		CandidateGenerationSteps: []string{"bm25"},
		LexicalCandidatesCount:   1,
		VectorCandidatesCount:    0,
		RerankSteps:              []string{},
		ScopeFilterResult:        "ok",
		FinalContextCount:        1,
		Availability:             contractv1.FieldAvailability{Object: "RetrievalTrace"},
	}
	// task-15.1: cache the stub so drill-down GETs return 200 instead of 503.
	chunk := buildSourceChunkFromResult(res)
	s.mu.Lock()
	s.cacheChunkUnlocked(res.ChunkID, chunk)
	s.cacheTraceUnlocked(res.QueryID, trace)
	s.mu.Unlock()
	return res, trace, nil
}

// buildSourceChunkFromResult maps a SearchResult to a SourceChunk for cache
// population. The fallback has no real chunk text, so ChunkTextPreview is
// reused as the chunk_text_preview field; offset fields are 0; redaction status
// reports "none" (fallback skips secret scanning by design).
func buildSourceChunkFromResult(res contractv1.SearchResult) contractv1.SourceChunk {
	return contractv1.SourceChunk{
		ChunkID:          res.ChunkID,
		WorkspaceID:      res.WorkspaceID,
		SourceFilePath:   res.SourceFilePath,
		LineStart:        res.LineStart,
		LineEnd:          res.LineEnd,
		ChunkTextPreview: res.ChunkTextPreview,
		ChunkOffsetStart: 0,
		ChunkOffsetEnd:   0,
		RedactionStatus:  "none",
		Availability:     contractv1.FieldAvailability{Object: "SourceChunk"},
	}
}

// ---- EventsClient ----

// Recent — MemStore fallback for GET /v1/observability/events.
//
// task-16.2 (Phase 16 P4 #11): fallback impl. There is no real event source
// in MemStore (no broadcast channel), so we can't truly block-on-event. Two
// behaviors:
//   - Ring buffer non-empty → return slice immediately (wait ignored).
//   - Ring buffer empty + wait > 0 → sleep `min(wait, 1s)` then return `[]`.
//     Capping at 1s avoids HTTP handler holding goroutine for a long time on
//     a fallback path that has no chance of new events arriving; capping
//     above 0 avoids Console UI poll-storm if it sets `?wait=30s` and the
//     ring is empty (the alternative — immediate `[]` return — would have
//     Console UI immediately re-request, burning CPU).
func (s *MemStore) Recent(limit int, wait time.Duration) ([]contractv1.ObservabilityEvent, error) {
	s.mu.Lock()
	have := len(s.events)
	s.mu.Unlock()

	if have == 0 && wait > 0 {
		sleepFor := wait
		if sleepFor > time.Second {
			sleepFor = time.Second
		}
		time.Sleep(sleepFor)
	}

	s.mu.Lock()
	defer s.mu.Unlock()
	if limit <= 0 || limit > len(s.events) {
		limit = len(s.events)
	}
	if limit == 0 {
		return []contractv1.ObservabilityEvent{}, nil
	}
	out := make([]contractv1.ObservabilityEvent, limit)
	copy(out, s.events[len(s.events)-limit:])
	return out, nil
}

// =====================================================================
// task-13.2 (ADR-017 D1 Wave 3) — MemMemoryStore fallback impl.
// =====================================================================

// MemMemoryStore implements MemoryClient for the env-gated MemStore fallback
// (CONSOLE_API_FALLBACK_INMEM=1). All 5 methods succeed in-memory (no audit
// writes; data lost on restart) so Console UI demo / conformance test remain
// functional when the Rust daemon is unreachable (ADR-016 §D4 degraded-but-
// functional fallback semantics).
type MemMemoryStore struct {
	mu    sync.Mutex
	items map[string]contractv1.MemoryItem
	// task-31.1: optional best-effort observability sink. When wired (SetEventSink), memory write
	// ops emit memory.* events for parity with the workspace/job fallback paths + the Rust data
	// plane (core/src/data_plane/memory.rs). nil → no-op (observation != authority, ADR-021).
	emit func(eventType, severity, source, message string)
}

func NewMemMemoryStore() *MemMemoryStore {
	return &MemMemoryStore{items: map[string]contractv1.MemoryItem{}}
}

// SetEventSink wires the fallback observability ring (task-31.1). Typically MemStore.EmitEvent,
// so memory ops appear in GET /v1/observability/events under CONSOLE_API_FALLBACK_INMEM=1.
func (s *MemMemoryStore) SetEventSink(emit func(eventType, severity, source, message string)) {
	s.emit = emit
}

// emitMemoryEvent is a best-effort observability emit (no-op when no sink wired). event_type names
// mirror the Rust audit_op_to_event_type mapping (memory.pin / memory.deprecate / memory.soft_delete
// / memory.hard_delete; Unpin shares memory.pin with op=unpin in the message).
func (s *MemMemoryStore) emitMemoryEvent(eventType, message string) {
	if s.emit != nil {
		s.emit(eventType, "info", "consoleapi", message)
	}
}

// SeedFixtures populates 5 hard-coded memory items for fallback demo mode.
// task-13.2 §3 in scope; trade-off accepted (smoke test + Console UI demo).
// task-17.1 / ADR-022 D3 §"MemMemoryStore SeedFixtures 默认 false"：fixture-1
// preset to IsPinned: true so Console UI fallback mode renders at least one
// pinned row when verifying the new is_pinned field.
func (s *MemMemoryStore) SeedFixtures() {
	s.mu.Lock()
	defer s.mu.Unlock()
	now := time.Now().UTC()
	seeds := []contractv1.MemoryItem{
		{MemoryID: "mem-fixture-1", AgentScope: "agent-default:session", ContentPreview: "first fixture item",
			SourceType: "fixture", SourceRef: "memstore:1", CreatedAt: now, UpdatedAt: now,
			HitCount: 0, Status: "active", IsPinned: true,
			Availability: contractv1.FieldAvailability{Object: "MemoryItem"}},
		{MemoryID: "mem-fixture-2", AgentScope: "agent-default:project", ContentPreview: "second fixture item",
			SourceType: "fixture", SourceRef: "memstore:2", CreatedAt: now, UpdatedAt: now,
			HitCount: 0, Status: "active",
			Availability: contractv1.FieldAvailability{Object: "MemoryItem"}},
		{MemoryID: "mem-fixture-3", AgentScope: "agent-default:global", ContentPreview: "third fixture item",
			SourceType: "fixture", SourceRef: "memstore:3", CreatedAt: now, UpdatedAt: now,
			HitCount: 0, Status: "active",
			Availability: contractv1.FieldAvailability{Object: "MemoryItem"}},
		{MemoryID: "mem-fixture-4", AgentScope: "agent-test:session", ContentPreview: "fourth fixture (deprecated)",
			SourceType: "fixture", SourceRef: "memstore:4", CreatedAt: now, UpdatedAt: now,
			HitCount: 0, Status: "deprecated",
			Availability: contractv1.FieldAvailability{Object: "MemoryItem"}},
		{MemoryID: "mem-fixture-5", AgentScope: "agent-test:project", ContentPreview: "fifth fixture",
			SourceType: "fixture", SourceRef: "memstore:5", CreatedAt: now, UpdatedAt: now,
			HitCount: 0, Status: "active",
			Availability: contractv1.FieldAvailability{Object: "MemoryItem"}},
	}
	for _, item := range seeds {
		s.items[item.MemoryID] = item
	}
}

func (s *MemMemoryStore) List(filter MemoryListFilter) ([]contractv1.MemoryItem, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	out := make([]contractv1.MemoryItem, 0, len(s.items))
	for _, item := range s.items {
		if !filter.IncludeSoftDeleted && item.Status == "soft_deleted" {
			continue
		}
		if filter.Scope != "" && item.AgentScope != filter.Scope {
			continue
		}
		if filter.AgentID != "" && !strings.HasPrefix(item.AgentScope, filter.AgentID) {
			continue
		}
		if filter.Namespace != "" && !strings.HasSuffix(item.AgentScope, filter.Namespace) {
			continue
		}
		out = append(out, item)
	}
	sort.Slice(out, func(i, j int) bool { return out[i].MemoryID < out[j].MemoryID })
	return out, nil
}

func (s *MemMemoryStore) Get(id string) (*contractv1.MemoryItem, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if item, ok := s.items[id]; ok {
		return &item, nil
	}
	return nil, nil
}

// task-40.1: actor carries the calling actor through (X-Actor header → here). The in-memory
// fallback store does not persist a pinned_by column (the real first-class actor field lives on
// the SqliteMemoryStore via the gRPC data plane, task-27.1 / ADR-032 D1), so actor is accepted
// for interface parity and is a no-op here; the real propagation path is grpcclient.Pin.
func (s *MemMemoryStore) Pin(id string, pin bool, actor string) error {
	_ = actor
	s.mu.Lock()
	defer s.mu.Unlock()
	item, ok := s.items[id]
	if !ok {
		return fmt.Errorf("%w: memory %s", ErrNotFound, id)
	}
	// task-17.1 / ADR-022 D1: pin state now first-class on MemoryItem.IsPinned;
	// Pin(id, true/false) toggles the snapshot in-place + bumps UpdatedAt.
	item.IsPinned = pin
	item.UpdatedAt = time.Now().UTC()
	s.items[id] = item
	s.emitMemoryEvent("memory.pin", fmt.Sprintf("memory %s pin=%t", id, pin))
	return nil
}

func (s *MemMemoryStore) Deprecate(id string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	item, ok := s.items[id]
	if !ok {
		return fmt.Errorf("%w: memory %s", ErrNotFound, id)
	}
	item.Status = "deprecated"
	item.UpdatedAt = time.Now().UTC()
	s.items[id] = item
	s.emitMemoryEvent("memory.deprecate", "memory deprecated: "+id)
	return nil
}

func (s *MemMemoryStore) SoftDelete(id string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	item, ok := s.items[id]
	if !ok {
		return fmt.Errorf("%w: memory %s", ErrNotFound, id)
	}
	item.Status = "soft_deleted"
	item.UpdatedAt = time.Now().UTC()
	s.items[id] = item
	s.emitMemoryEvent("memory.soft_delete", "memory soft-deleted: "+id)
	return nil
}

// task-27.2 (ADR-032 D2): explicit Unpin (idempotent; clears the pin snapshot).
// task-44.1 (ADR-049 D3): signature aligned with the gRPC client (actor param);
// the in-memory fallback has no audit/event path so the actor is accepted but unused.
func (s *MemMemoryStore) Unpin(id string, _actor string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	item, ok := s.items[id]
	if !ok {
		return fmt.Errorf("%w: memory %s", ErrNotFound, id)
	}
	item.IsPinned = false
	item.PinnedBy = ""
	item.PinnedAtUnix = 0
	item.UpdatedAt = time.Now().UTC()
	s.items[id] = item
	// op=unpin shares the memory.pin event_type (Rust MemoryPin | MemoryUnpin → memory.pin).
	s.emitMemoryEvent("memory.pin", "memory "+id+" op=unpin")
	return nil
}

// task-27.2 (ADR-032 D2): hard-delete physically removes the item (vs
// soft-delete's status flip) — Get afterwards returns nil.
func (s *MemMemoryStore) HardDelete(id string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if _, ok := s.items[id]; !ok {
		return fmt.Errorf("%w: memory %s", ErrNotFound, id)
	}
	delete(s.items, id)
	s.emitMemoryEvent("memory.hard_delete", "memory hard-deleted: "+id)
	return nil
}

// =====================================================================
// task-14.2 (ADR-017 D1 Wave 4) — MemEvalStore fallback impl.
// In-memory only; status auto-advances to "succeeded" after 2s with mock
// metrics (deps demo / fallback degraded-but-functional per ADR-016 §D4).
// =====================================================================

type MemEvalStore struct {
	mu   sync.Mutex
	runs map[string]contractv1.EvalRun
	seq  uint64
}

func NewMemEvalStore() *MemEvalStore {
	return &MemEvalStore{runs: map[string]contractv1.EvalRun{}}
}

func (s *MemEvalStore) Create(req contractv1.EvalRunCreate) (contractv1.EvalRun, error) {
	s.mu.Lock()
	s.seq++
	id := fmt.Sprintf("eval-mem-%d", s.seq)
	now := time.Now().UTC()
	cfg, _ := json.Marshal(req.ConfigSnapshot)
	run := contractv1.EvalRun{
		EvalRunID:      id,
		WorkspaceID:    req.WorkspaceID,
		Status:         "running",
		ConfigSnapshot: cfg,
		StartedAt:      now,
		FinishedAt:     nil,
		Metrics:        map[string]float64{},
		CaseResults:    []contractv1.CaseResult{},
		SchemaVersion:  "v1",
		Availability:   contractv1.FieldAvailability{Object: "EvalRun"},
	}
	s.runs[id] = run
	s.mu.Unlock()
	// Auto-advance to succeeded after 2s with mock metrics.
	go func() {
		time.Sleep(2 * time.Second)
		s.mu.Lock()
		defer s.mu.Unlock()
		if existing, ok := s.runs[id]; ok && existing.Status == "running" {
			done := time.Now().UTC()
			existing.Status = "succeeded"
			existing.FinishedAt = &done
			existing.Metrics = map[string]float64{
				"recall@5":    0.7,
				"recall@10":   0.85,
				"precision@5": 0.6,
			}
			existing.CaseResults = []contractv1.CaseResult{{
				CaseID: "mem-c-1", Query: "demo", ExpectedChunks: []string{"chk-1"},
				ActualChunks: []string{"chk-1"}, Score: 1.0, Passed: true,
			}}
			s.runs[id] = existing
		}
	}()
	return run, nil
}

func (s *MemEvalStore) Get(id string) (*contractv1.EvalRun, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if run, ok := s.runs[id]; ok {
		return &run, nil
	}
	return nil, nil
}

func (s *MemEvalStore) UpdateProgress(id, status string, metrics map[string]float64,
	caseResults []contractv1.CaseResult, errorMessage string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	run, ok := s.runs[id]
	if !ok {
		return fmt.Errorf("%w: eval %s", ErrNotFound, id)
	}
	run.Status = status
	if metrics != nil {
		run.Metrics = metrics
	}
	if caseResults != nil {
		run.CaseResults = caseResults
	}
	if status == "succeeded" || status == "failed" || status == "cancelled" {
		done := time.Now().UTC()
		run.FinishedAt = &done
	}
	s.runs[id] = run
	return nil
}

// List returns eval runs ordered by started_at DESC, optionally filtered by
// workspace_id / status. Limit ≤ 0 defaults to 50; > 200 clamped. task-15.4.
func (s *MemEvalStore) List(filter contractv1.ListEvalRunsFilter) ([]contractv1.EvalRun, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	out := make([]contractv1.EvalRun, 0, len(s.runs))
	for _, run := range s.runs {
		if filter.WorkspaceID != "" && run.WorkspaceID != filter.WorkspaceID {
			continue
		}
		if filter.Status != "" && run.Status != filter.Status {
			continue
		}
		out = append(out, run)
	}
	sort.Slice(out, func(i, j int) bool {
		return out[i].StartedAt.After(out[j].StartedAt)
	})
	limit := int(filter.Limit)
	if limit <= 0 {
		limit = 50
	}
	if limit > 200 {
		limit = 200
	}
	if len(out) > limit {
		out = out[:limit]
	}
	return out, nil
}

// workspaceIDFromName derives a deterministic kebab-case-ish id from name.
// Trade-off: v0.3 simple slug; v0.4 may move to UUID + persistence.
func workspaceIDFromName(name string, salt int) string {
	id := strings.ToLower(strings.TrimSpace(name))
	id = strings.Map(func(r rune) rune {
		switch {
		case r >= 'a' && r <= 'z', r >= '0' && r <= '9', r == '-', r == '_':
			return r
		case r == ' ':
			return '-'
		}
		return -1
	}, id)
	if id == "" {
		id = fmt.Sprintf("ws-%d", salt+1)
	}
	if len(id) > 48 {
		id = id[:48]
	}
	return id
}
