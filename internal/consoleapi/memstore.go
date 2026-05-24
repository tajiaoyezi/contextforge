package consoleapi

import (
	"encoding/json"
	"fmt"
	"sort"
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
	jobOrder   []string                       // insertion order
	events     []contractv1.ObservabilityEvent // append-only ring (capped at 1000)
	// Optional injected Search backend (production wires to retriever / Rust
	// CoreService::search). Tests provide a fake.
	SearchBackend SearchClient
	// monotonic id seed for jobs.
	jobSeq uint64
}

func NewMemStore() *MemStore {
	return &MemStore{
		workspaces: map[string]contractv1.Workspace{},
		jobs:       map[string]contractv1.IndexJob{},
	}
}

// emitEvent records an ObservabilityEvent (capped at 1000 most-recent).
func (s *MemStore) emitEvent(eventType, severity, source, message string, jobID *string) {
	s.events = append(s.events, contractv1.ObservabilityEvent{
		EventID:   fmt.Sprintf("evt-%d", time.Now().UnixNano()),
		EventType: eventType,
		Severity:  severity,
		Source:    source,
		Message:   message,
		Timestamp: time.Now().UTC(),
		JobID:     jobID,
		Availability: contractv1.FieldAvailability{Object: "ObservabilityEvent"},
	})
	if len(s.events) > 1000 {
		s.events = s.events[len(s.events)-1000:]
	}
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

// GetSourceChunk — task-12.2 fallback path. MemStore has no real index → return
// ErrDataPlaneUnavailable so the REST layer surfaces 503 (deep defense; ADR-016 D4).
func (s *MemStore) GetSourceChunk(_ string) (contractv1.SourceChunk, error) {
	if s.SearchBackend != nil {
		return s.SearchBackend.GetSourceChunk("")
	}
	return contractv1.SourceChunk{}, ErrDataPlaneUnavailable
}

// GetSearchTrace — task-12.3 fallback path. Same rationale as GetSourceChunk.
func (s *MemStore) GetSearchTrace(_ string) (contractv1.RetrievalTrace, error) {
	if s.SearchBackend != nil {
		return s.SearchBackend.GetSearchTrace("")
	}
	return contractv1.RetrievalTrace{}, ErrDataPlaneUnavailable
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
	trace := contractv1.RetrievalTrace{
		TraceID:                  "trace-1",
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
	return res, trace, nil
}

// ---- EventsClient ----

func (s *MemStore) Recent(limit int) ([]contractv1.ObservabilityEvent, error) {
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
}

func NewMemMemoryStore() *MemMemoryStore {
	return &MemMemoryStore{items: map[string]contractv1.MemoryItem{}}
}

// SeedFixtures populates 5 hard-coded memory items for fallback demo mode.
// task-13.2 §3 in scope; trade-off accepted (smoke test + Console UI demo).
func (s *MemMemoryStore) SeedFixtures() {
	s.mu.Lock()
	defer s.mu.Unlock()
	now := time.Now().UTC()
	seeds := []contractv1.MemoryItem{
		{MemoryID: "mem-fixture-1", AgentScope: "agent-default:session", ContentPreview: "first fixture item",
			SourceType: "fixture", SourceRef: "memstore:1", CreatedAt: now, UpdatedAt: now,
			HitCount: 0, Status: "active",
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

func (s *MemMemoryStore) Pin(id string, _ bool) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	item, ok := s.items[id]
	if !ok {
		return fmt.Errorf("%w: memory %s", ErrNotFound, id)
	}
	// pin state not exposed in contractv1.MemoryItem; bump UpdatedAt to signal change
	item.UpdatedAt = time.Now().UTC()
	s.items[id] = item
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
