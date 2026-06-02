// Package contractv1 mirrors the ContextForge-Console Core Integration
// Contract v1 type set as defined in:
//
//	H:/devlopment/code/ContextForge-Console/console-api/internal/coreadapter/contractv1/contractv1.go
//
// Single source of truth: ContextForge-Console PRD §Technical Approach
// "Contract v1 must-have 字段" + Console contractv1.go file above. Any field
// drift between this package and Console contractv1.go is a cross-repo
// contract drift (ADR-014 D4 + ADR-015 D1) — the resolution path is a
// cross-repo amendment PR driven by Console (or by user-coordinated
// joint PR), not a unilateral change in this file.
//
// Design constraints (mirror Console contractv1.go header):
//   - ADR-001 / D1: coreadapter is the only Console↔Core boundary; this
//     package only carries versioned contract types and must not depend on
//     any internal ContextForge business package (stdlib encoding/json +
//     time only).
//   - ADR-008 / D8 (Console): contract is explicitly anchored to v1;
//     breaking changes must go through a parallel contractv2 package
//     (adapter-layer absorbed), v1 is not force-upgraded.
//   - R7 / PRD §Field Tiering: each must-have field strictly matches
//     Console PRD §Technical Approach "Contract v1 must-have 字段" single
//     source of truth; nullable fields use *T to express "not applicable"
//     rather than a silent zero value.
//
// Refs: ADR-015 §D1 / ADR-014 §D2 §D3 / phase-10-console-contract-v1.md §6 AC1 /
// task-10.1-contractv1-types.md §6 AC1-5
package contractv1

import (
	"encoding/json"
	"time"
)

// ContractVersion is the explicit version anchor (ADR-015 §D1 / Console
// contractv1.go ContractVersion). Breaking changes go to contractv2.
const ContractVersion = "v1"

// FieldAvailability records which Contract v1 must-have fields a given
// adapter actually provided. Missing must-have fields are explicitly listed
// so the caller can degrade gracefully (R7), instead of consuming a silent
// zero value (PRD §Field Tiering).
type FieldAvailability struct {
	// Object is the contract object name, e.g. "Workspace".
	Object string `json:"object"`
	// Missing lists the must-have fields the Core did not provide
	// (snake_case field names).
	Missing []string `json:"missing_must_have_fields"`
}

// Complete reports whether all must-have fields are present for this object.
func (fa FieldAvailability) Complete() bool { return len(fa.Missing) == 0 }

// IsMissing reports whether the named must-have field is missing.
func (fa FieldAvailability) IsMissing(field string) bool {
	for _, m := range fa.Missing {
		if m == field {
			return true
		}
	}
	return false
}

// ---- Contract v1 must-have objects (fields strictly aligned to Console
//      PRD §Technical Approach "Contract v1 must-have 字段"; config_snapshot
//      and case_results use json.RawMessage / typed slices and do not bind
//      Core internal schemas, ADR-015 D1). ----

// WorkspaceCreate is the CreateWorkspace input payload.
type WorkspaceCreate struct {
	Name      string   `json:"name"`
	RootPath  string   `json:"root_path"`
	Allowlist []string `json:"allowlist,omitempty"`
	Denylist  []string `json:"denylist,omitempty"`
}

// Workspace — PRD must-have: workspace_id/name/root_path/status/
// config_snapshot/created_at/updated_at.
type Workspace struct {
	WorkspaceID    string            `json:"workspace_id"`
	Name           string            `json:"name"`
	RootPath       string            `json:"root_path"`
	Status         string            `json:"status"`
	ConfigSnapshot json.RawMessage   `json:"config_snapshot"`
	CreatedAt      time.Time         `json:"created_at"`
	UpdatedAt      time.Time         `json:"updated_at"`
	Availability   FieldAvailability `json:"field_availability"`
}

// IndexJob — PRD must-have: job_id/workspace_id/trigger_source/status/stage/
// processed_files/total_files/failed_files/skipped_files/error_message/
// started_at/finished_at/last_heartbeat_at. Nullable status-conditional
// fields use *T (R7: not-started / not-finished / no-heartbeat / no-error
// must not become a silent zero value).
type IndexJob struct {
	JobID           string            `json:"job_id"`
	WorkspaceID     string            `json:"workspace_id"`
	TriggerSource   string            `json:"trigger_source"`
	Status          string            `json:"status"`
	Stage           string            `json:"stage"`
	ProcessedFiles  int               `json:"processed_files"`
	TotalFiles      int               `json:"total_files"`
	FailedFiles     int               `json:"failed_files"`
	SkippedFiles    int               `json:"skipped_files"`
	ErrorMessage    *string           `json:"error_message"`
	StartedAt       *time.Time        `json:"started_at"`
	FinishedAt      *time.Time        `json:"finished_at"`
	LastHeartbeatAt *time.Time        `json:"last_heartbeat_at"`
	Availability    FieldAvailability `json:"field_availability"`
}

// SearchRequest — PRD must-have: query/workspace_id/agent_scope/
// retrieval_method/top_k/config_snapshot.
type SearchRequest struct {
	Query           string          `json:"query"`
	WorkspaceID     string          `json:"workspace_id"`
	AgentScope      string          `json:"agent_scope"`
	RetrievalMethod string          `json:"retrieval_method"`
	TopK            int             `json:"top_k"`
	ConfigSnapshot  json.RawMessage `json:"config_snapshot"`
	// Semantic — task-20.1 (Phase 20): add-only opt-in semantic-search flag.
	// OR-merged from the `?semantic=true` query param or this body field by
	// handleSearch, then forwarded to gRPC SearchRequest.Semantic. Default false
	// → BM25 (backward-compatible, ADR-015 add-only).
	Semantic     bool              `json:"semantic"`
	Availability FieldAvailability `json:"field_availability"`
}

// SearchResult — PRD must-have: result_id/query_id/workspace_id/
// source_file_path/source_file_type/chunk_id/chunk_text_preview/line_start/
// line_end/score/rank_before_rerank/rank_after_rerank/retrieval_method/
// reason/citation. rank_after_rerank uses *int (R7: not reranked is not
// rank 0).
type SearchResult struct {
	ResultID         string            `json:"result_id"`
	QueryID          string            `json:"query_id"`
	WorkspaceID      string            `json:"workspace_id"`
	SourceFilePath   string            `json:"source_file_path"`
	SourceFileType   string            `json:"source_file_type"`
	ChunkID          string            `json:"chunk_id"`
	ChunkTextPreview string            `json:"chunk_text_preview"`
	LineStart        int               `json:"line_start"`
	LineEnd          int               `json:"line_end"`
	Score            float64           `json:"score"`
	RankBeforeRerank int               `json:"rank_before_rerank"`
	RankAfterRerank  *int              `json:"rank_after_rerank"`
	RetrievalMethod  string            `json:"retrieval_method"`
	Reason           string            `json:"reason"`
	Citation         Citation          `json:"citation"`
	Availability     FieldAvailability `json:"field_availability"`
}

// RetrievalTrace — PRD must-have: trace_id/query/expanded_query/
// candidate_generation_steps/lexical_candidates_count/
// vector_candidates_count/rerank_steps/scope_filter_result/
// final_context_count. expanded_query uses *string (R7: no query expansion
// is not an empty string).
type RetrievalTrace struct {
	TraceID                  string            `json:"trace_id"`
	Query                    string            `json:"query"`
	ExpandedQuery            *string           `json:"expanded_query"`
	CandidateGenerationSteps []string          `json:"candidate_generation_steps"`
	LexicalCandidatesCount   int               `json:"lexical_candidates_count"`
	VectorCandidatesCount    int               `json:"vector_candidates_count"`
	RerankSteps              []string          `json:"rerank_steps"`
	ScopeFilterResult        string            `json:"scope_filter_result"`
	FinalContextCount        int               `json:"final_context_count"`
	Availability             FieldAvailability `json:"field_availability"`
}

// SourceChunk — PRD must-have: chunk_id/workspace_id/source_file_path/
// line_start/line_end/chunk_text_preview/chunk_offset_start/
// chunk_offset_end/redaction_status.
type SourceChunk struct {
	ChunkID          string            `json:"chunk_id"`
	WorkspaceID      string            `json:"workspace_id"`
	SourceFilePath   string            `json:"source_file_path"`
	LineStart        int               `json:"line_start"`
	LineEnd          int               `json:"line_end"`
	ChunkTextPreview string            `json:"chunk_text_preview"`
	ChunkOffsetStart int               `json:"chunk_offset_start"`
	ChunkOffsetEnd   int               `json:"chunk_offset_end"`
	RedactionStatus  string            `json:"redaction_status"`
	Availability     FieldAvailability `json:"field_availability"`
}

// Citation — PRD must-have: citation_id/source_file_path/chunk_id/
// line_start/line_end/confidence.
type Citation struct {
	CitationID     string            `json:"citation_id"`
	SourceFilePath string            `json:"source_file_path"`
	ChunkID        string            `json:"chunk_id"`
	LineStart      int               `json:"line_start"`
	LineEnd        int               `json:"line_end"`
	Confidence     float64           `json:"confidence"`
	Availability   FieldAvailability `json:"field_availability"`
}

// MemoryItem — PRD must-have: memory_id/agent_scope/content_preview/
// source_type/source_ref/created_at/updated_at/hit_count/status.
// task-17.1 / ADR-022 D1: is_pinned add-only field — pin state snapshot
// surfaced for Console UI list sort + icon rendering. Cross-repo aligned
// with ContextForge-Console master @ 415ee30 (PR #101); v0.9 and earlier
// daemon responses lacking the key unmarshal to bool zero value (false),
// preserving forward / backward compatibility (ADR-022 D4).
type MemoryItem struct {
	MemoryID       string    `json:"memory_id"`
	AgentScope     string    `json:"agent_scope"`
	ContentPreview string    `json:"content_preview"`
	SourceType     string    `json:"source_type"`
	SourceRef      string    `json:"source_ref"`
	CreatedAt      time.Time `json:"created_at"`
	UpdatedAt      time.Time `json:"updated_at"`
	HitCount       int       `json:"hit_count"`
	Status         string    `json:"status"`
	IsPinned       bool      `json:"is_pinned"`
	// task-27.1 / ADR-032 D1: add-only pin-actor + pinned-at-timestamp. v0.19
	// and earlier daemon responses lacking the keys unmarshal to zero values
	// ("" / 0), preserving forward / backward compatibility.
	PinnedBy     string            `json:"pinned_by"`
	PinnedAtUnix int64             `json:"pinned_at_unix"`
	Availability FieldAvailability `json:"field_availability"`
}

// MemoryOperation — schema_version field aligns with the Console D3 schema
// evolution convention (Console PRD §Implementation Phases Phase 5 AC5
// captures schema_version).
type MemoryOperation struct {
	ID            string            `json:"id"`
	MemoryID      string            `json:"memory_id"`
	OpType        string            `json:"op_type"` // pin / deprecate / soft_delete
	Actor         string            `json:"actor"`
	CreatedAt     time.Time         `json:"created_at"`
	SchemaVersion string            `json:"schema_version"`
	Metadata      map[string]any    `json:"metadata,omitempty"`
	Availability  FieldAvailability `json:"field_availability"`
}

// EvalRun — PRD must-have: eval_run_id/workspace_id/status/config_snapshot/
// started_at/finished_at/metrics/case_results/schema_version. finished_at
// uses *time.Time (R7: running is not zero time); case_results is a typed
// []CaseResult (Console task-6.1 v5 explicit typed; mirrors front-end Zod
// schema).
// Status: running / succeeded / failed / cancelled (Console task-6.1 v5).
type EvalRun struct {
	EvalRunID      string             `json:"eval_run_id"`
	WorkspaceID    string             `json:"workspace_id"`
	Status         string             `json:"status"` // running / succeeded / failed / cancelled
	ConfigSnapshot json.RawMessage    `json:"config_snapshot"`
	StartedAt      time.Time          `json:"started_at"`
	FinishedAt     *time.Time         `json:"finished_at"`
	Metrics        map[string]float64 `json:"metrics"`
	CaseResults    []CaseResult       `json:"case_results"`
	SchemaVersion  string             `json:"schema_version"`
	Availability   FieldAvailability  `json:"field_availability"`
}

// EvalRunCreate — TriggerEvalRun input payload.
type EvalRunCreate struct {
	WorkspaceID    string         `json:"workspace_id"`
	ConfigSnapshot map[string]any `json:"config_snapshot"`
	DatasetRef     string         `json:"dataset_ref,omitempty"`
}

// CaseResult — EvalRun.case_results element (Console task-6.1 v5 typed).
type CaseResult struct {
	CaseID         string   `json:"case_id"`
	Query          string   `json:"query"`
	ExpectedChunks []string `json:"expected_chunks"`
	ActualChunks   []string `json:"actual_chunks"`
	Score          float64  `json:"score"`
	Passed         bool     `json:"passed"`
}

// ObservabilityEvent — PRD must-have: event_id/event_type/severity/source/
// message/timestamp/trace_id/job_id. trace_id and job_id use *string
// (R7: event not associated with trace/job is not empty string).
type ObservabilityEvent struct {
	EventID      string            `json:"event_id"`
	EventType    string            `json:"event_type"`
	Severity     string            `json:"severity"`
	Source       string            `json:"source"`
	Message      string            `json:"message"`
	Timestamp    time.Time         `json:"timestamp"`
	TraceID      *string           `json:"trace_id"`
	JobID        *string           `json:"job_id"`
	Availability FieldAvailability `json:"field_availability"`
}

// AgentScope is the ListMemory scope filter input (Console task-1.2 §3 +2
// objects).
type AgentScope struct {
	AgentID   string `json:"agent_id"`
	Scope     string `json:"scope"`     // e.g. "session" / "project" / "global"
	Namespace string `json:"namespace"` // optional grouping key
}

// CoreHealth aggregates Core connection health and contract diagnostics
// (Console task-1.2 §3 +2 objects). last_connected_at / error_reason use *T
// (R7: never-connected / no-error is not zero value).
type CoreHealth struct {
	Status string `json:"status"` // "healthy" / "degraded" / "unreachable"
	// ContractVersion exposes the contract version on the health/diagnostics
	// surface (Console AC5).
	ContractVersion string     `json:"contract_version"`
	LastConnectedAt *time.Time `json:"last_connected_at"`
	ErrorReason     *string    `json:"error_reason"`
	// MissingMustHaveFields aggregates each object's missing must-have fields
	// so the diagnostics page can show "Core did not provide required fields"
	// (Console AC4 / R7 / PRD §Field Tiering).
	MissingMustHaveFields []FieldAvailability `json:"missing_must_have_fields"`
	// task-15.6 (Phase 15 P2 #7 / ADR-020): 5-link component breakdown.
	// Only populated when the REST handler is invoked with ?detailed=true;
	// omitted (omitempty) for the default binary health response so the
	// existing v0.7 client contract is unchanged. Keys: db / index / embed /
	// retriever / eval.
	Components map[string]ComponentHealth `json:"components,omitempty"`
	// Total wall-clock cost of the detailed probe sweep. Reported alongside
	// `Components` when present.
	TotalLatencyMs *int64 `json:"total_latency_ms,omitempty"`
}

// task-15.6 / ADR-020 D2: per-component health record.
type ComponentHealth struct {
	Name        string  `json:"name"`
	Status      string  `json:"status"` // "healthy" / "degraded" / "unreachable"
	LatencyMs   *int64  `json:"latency_ms,omitempty"`
	ErrorReason *string `json:"error_reason,omitempty"`
}

// HasMissingMustHaveFields reports whether any Core-not-provided must-have
// field exists.
func (h CoreHealth) HasMissingMustHaveFields() bool {
	return len(h.MissingMustHaveFields) > 0
}

// task-15.5 (Phase 15 P1 #5): query history record. Returned by GET /v1/queries
// as a JSON array. add-only — does not amend RetrievalTrace shape (workspace_id
// and ts_unix are out-of-band metadata kept by the Rust TraceStore wrapper).
type QueryRecord struct {
	QueryID     string `json:"query_id"`
	Query       string `json:"query"`
	TsUnix      int64  `json:"ts_unix"`
	WorkspaceID string `json:"workspace_id,omitempty"`
}

// task-15.4 (Phase 15 P1 #4): filter for GET /v1/eval-runs list endpoint.
// All fields optional; empty string = no constraint. Limit ≤ 0 falls back
// to server-side default 50; > 200 is clamped server-side.
type ListEvalRunsFilter struct {
	WorkspaceID string
	Status      string
	Limit       int32
}

// task-15.3 (Phase 15 P1 #3): Dashboard "已索引块" stats response from
// GET /v1/stats/chunks. add-only — does not touch any other contract type.
//
// `Total` is the cross-workspace live-doc count (Tantivy `num_docs` excluding
// tombstones); `TodayDelta` counts chunks indexed since UTC today_start
// (lexicographic compare on chunks.indexed_at; v0.8 fallback returns 0 when
// SQLite probe fails per [SPEC-OWNER:task-15.3]).
type ChunksStats struct {
	Total      int64 `json:"total"`
	TodayDelta int64 `json:"today_delta"`
}
