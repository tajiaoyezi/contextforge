// Package contractv1 tests: validates type mirror parity, JSON roundtrip, and
// FieldAvailability helpers per task-10.1 §6 AC1-5.
package contractv1

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"
)

// TestJSONRoundtrip verifies each Contract v1 type marshals and unmarshals
// without losing data. AC2 — covers 17 types; covers nullable-field cases
// (pointer fields must serialize null when nil and deserialize as nil).
func TestJSONRoundtrip(t *testing.T) {
	now := time.Date(2026, 5, 24, 12, 0, 0, 0, time.UTC)
	finished := now.Add(time.Minute)
	errMsg := "boom"
	expanded := "expanded query"
	rankAfter := 3
	traceID := "trace-1"
	jobID := "job-1"

	cases := []struct {
		name string
		v    any
	}{
		{"WorkspaceCreate", WorkspaceCreate{Name: "a", RootPath: "/r", Allowlist: []string{"*.md"}, Denylist: []string{".env"}}},
		{"Workspace", Workspace{WorkspaceID: "w1", Name: "n", RootPath: "/r", Status: "ready", ConfigSnapshot: json.RawMessage(`{"k":1}`), CreatedAt: now, UpdatedAt: now, Availability: FieldAvailability{Object: "Workspace"}}},
		{"IndexJob_nullable_nil", IndexJob{JobID: "j1", WorkspaceID: "w1", TriggerSource: "rest", Status: "queued", Stage: "parse", ProcessedFiles: 0, TotalFiles: 100, FailedFiles: 0, SkippedFiles: 0, ErrorMessage: nil, StartedAt: nil, FinishedAt: nil, LastHeartbeatAt: nil, Availability: FieldAvailability{Object: "IndexJob"}}},
		{"IndexJob_nullable_set", IndexJob{JobID: "j2", WorkspaceID: "w1", TriggerSource: "cli", Status: "failed", Stage: "index", ErrorMessage: &errMsg, StartedAt: &now, FinishedAt: &finished, LastHeartbeatAt: &finished, Availability: FieldAvailability{Object: "IndexJob"}}},
		{"SearchRequest", SearchRequest{Query: "q", WorkspaceID: "w1", AgentScope: "session", RetrievalMethod: "hybrid", TopK: 10, ConfigSnapshot: json.RawMessage(`{}`), Availability: FieldAvailability{Object: "SearchRequest"}}},
		{"SearchResult_no_rerank", SearchResult{ResultID: "r1", QueryID: "q1", WorkspaceID: "w1", SourceFilePath: "/a/b", SourceFileType: "md", ChunkID: "c1", ChunkTextPreview: "...", LineStart: 1, LineEnd: 10, Score: 0.9, RankBeforeRerank: 1, RankAfterRerank: nil, RetrievalMethod: "bm25", Reason: "match", Citation: Citation{CitationID: "ct1", SourceFilePath: "/a/b", ChunkID: "c1", LineStart: 1, LineEnd: 10, Confidence: 0.9, Availability: FieldAvailability{Object: "Citation"}}, Availability: FieldAvailability{Object: "SearchResult"}}},
		{"SearchResult_with_rerank", SearchResult{ResultID: "r2", QueryID: "q1", WorkspaceID: "w1", SourceFilePath: "/a/c", SourceFileType: "go", ChunkID: "c2", LineStart: 5, LineEnd: 15, Score: 0.85, RankBeforeRerank: 2, RankAfterRerank: &rankAfter, RetrievalMethod: "hybrid", Reason: "rerank up", Citation: Citation{CitationID: "ct2"}, Availability: FieldAvailability{Object: "SearchResult"}}},
		{"RetrievalTrace_no_expand", RetrievalTrace{TraceID: "t1", Query: "q", ExpandedQuery: nil, CandidateGenerationSteps: []string{"bm25"}, LexicalCandidatesCount: 5, VectorCandidatesCount: 0, RerankSteps: nil, ScopeFilterResult: "ok", FinalContextCount: 5, Availability: FieldAvailability{Object: "RetrievalTrace"}}},
		{"RetrievalTrace_with_expand", RetrievalTrace{TraceID: "t2", Query: "q", ExpandedQuery: &expanded, FinalContextCount: 3, Availability: FieldAvailability{Object: "RetrievalTrace"}}},
		{"SourceChunk", SourceChunk{ChunkID: "c1", WorkspaceID: "w1", SourceFilePath: "/a/b", LineStart: 1, LineEnd: 10, ChunkTextPreview: "...", ChunkOffsetStart: 0, ChunkOffsetEnd: 100, RedactionStatus: "clean", Availability: FieldAvailability{Object: "SourceChunk"}}},
		{"Citation", Citation{CitationID: "ct1", SourceFilePath: "/a/b", ChunkID: "c1", LineStart: 1, LineEnd: 10, Confidence: 0.95, Availability: FieldAvailability{Object: "Citation"}}},
		{"MemoryItem", MemoryItem{MemoryID: "m1", AgentScope: "session", ContentPreview: "p", SourceType: "hermes", SourceRef: "MEMORY.md", CreatedAt: now, UpdatedAt: now, HitCount: 3, Status: "active", Availability: FieldAvailability{Object: "MemoryItem"}}},
		{"MemoryOperation", MemoryOperation{ID: "op1", MemoryID: "m1", OpType: "pin", Actor: "user", CreatedAt: now, SchemaVersion: "v1", Metadata: map[string]any{"src": "ui"}, Availability: FieldAvailability{Object: "MemoryOperation"}}},
		{"EvalRun_running", EvalRun{EvalRunID: "e1", WorkspaceID: "w1", Status: "running", ConfigSnapshot: json.RawMessage(`{}`), StartedAt: now, FinishedAt: nil, Metrics: map[string]float64{"top5": 0.8}, CaseResults: []CaseResult{{CaseID: "c1", Query: "q", ExpectedChunks: []string{"x"}, ActualChunks: []string{"x"}, Score: 1.0, Passed: true}}, SchemaVersion: "v1", Availability: FieldAvailability{Object: "EvalRun"}}},
		{"EvalRun_finished", EvalRun{EvalRunID: "e2", WorkspaceID: "w1", Status: "succeeded", ConfigSnapshot: json.RawMessage(`{"n":1}`), StartedAt: now, FinishedAt: &finished, Metrics: map[string]float64{"top5": 0.9}, CaseResults: nil, SchemaVersion: "v1", Availability: FieldAvailability{Object: "EvalRun"}}},
		{"EvalRunCreate", EvalRunCreate{WorkspaceID: "w1", ConfigSnapshot: map[string]any{"k": "v"}, DatasetRef: "golden-30"}},
		{"CaseResult", CaseResult{CaseID: "c1", Query: "q", ExpectedChunks: []string{"a"}, ActualChunks: []string{"a"}, Score: 1.0, Passed: true}},
		{"ObservabilityEvent_nullable_nil", ObservabilityEvent{EventID: "ev1", EventType: "error", Severity: "warn", Source: "indexer", Message: "x", Timestamp: now, TraceID: nil, JobID: nil, Availability: FieldAvailability{Object: "ObservabilityEvent"}}},
		{"ObservabilityEvent_nullable_set", ObservabilityEvent{EventID: "ev2", EventType: "info", Severity: "info", Source: "search", Message: "x", Timestamp: now, TraceID: &traceID, JobID: &jobID, Availability: FieldAvailability{Object: "ObservabilityEvent"}}},
		{"AgentScope", AgentScope{AgentID: "claude", Scope: "session", Namespace: "default"}},
		{"CoreHealth", CoreHealth{Status: "healthy", ContractVersion: "v1", LastConnectedAt: &now, ErrorReason: nil, MissingMustHaveFields: nil}},
	}
	for _, tc := range cases {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			data, err := json.Marshal(tc.v)
			if err != nil {
				t.Fatalf("marshal: %v", err)
			}
			ptr := reflect.New(reflect.TypeOf(tc.v))
			if err := json.Unmarshal(data, ptr.Interface()); err != nil {
				t.Fatalf("unmarshal: %v", err)
			}
			got := ptr.Elem().Interface()
			if !reflect.DeepEqual(tc.v, got) {
				t.Errorf("roundtrip mismatch:\n  in:  %#v\n  out: %#v\n  json: %s", tc.v, got, data)
			}
		})
	}
}

// TestNullablePointerJSONNull verifies pointer fields serialize null when
// nil and deserialize back as nil pointer (not zero value of the pointed
// type). AC2 — covers the *time.Time / *string / *int nullable contract.
func TestNullablePointerJSONNull(t *testing.T) {
	job := IndexJob{
		JobID: "j1", WorkspaceID: "w1", TriggerSource: "cli", Status: "queued",
		ErrorMessage: nil, StartedAt: nil, FinishedAt: nil, LastHeartbeatAt: nil,
	}
	data, err := json.Marshal(job)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	s := string(data)
	for _, key := range []string{
		`"error_message":null`,
		`"started_at":null`,
		`"finished_at":null`,
		`"last_heartbeat_at":null`,
	} {
		if !strings.Contains(s, key) {
			t.Errorf("expected %q in JSON; got: %s", key, s)
		}
	}
	var out IndexJob
	if err := json.Unmarshal(data, &out); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if out.ErrorMessage != nil || out.StartedAt != nil || out.FinishedAt != nil || out.LastHeartbeatAt != nil {
		t.Errorf("nullable fields should deserialize as nil pointers; got %+v", out)
	}
}

// TestFieldAvailability verifies Complete() / IsMissing() helpers. AC3.
func TestFieldAvailability(t *testing.T) {
	empty := FieldAvailability{Object: "Workspace"}
	if !empty.Complete() {
		t.Errorf("empty Missing should be Complete; got false")
	}
	if empty.IsMissing("config_snapshot") {
		t.Errorf("empty Missing should not match any field")
	}
	some := FieldAvailability{Object: "IndexJob", Missing: []string{"last_heartbeat_at", "error_message"}}
	if some.Complete() {
		t.Errorf("non-empty Missing should not be Complete")
	}
	if !some.IsMissing("last_heartbeat_at") {
		t.Errorf("IsMissing should match listed field")
	}
	if !some.IsMissing("error_message") {
		t.Errorf("IsMissing should match second listed field")
	}
	if some.IsMissing("workspace_id") {
		t.Errorf("IsMissing should not match unlisted field")
	}
}

// TestContractVersionConstant verifies the ContractVersion anchor. AC1.
func TestContractVersionConstant(t *testing.T) {
	if ContractVersion != "v1" {
		t.Errorf("ContractVersion must be \"v1\"; got %q", ContractVersion)
	}
}

// TestCoreHealthHasMissing verifies CoreHealth.HasMissingMustHaveFields().
func TestCoreHealthHasMissing(t *testing.T) {
	h := CoreHealth{Status: "healthy", ContractVersion: "v1"}
	if h.HasMissingMustHaveFields() {
		t.Errorf("empty MissingMustHaveFields should report false")
	}
	h.MissingMustHaveFields = []FieldAvailability{{Object: "Workspace", Missing: []string{"config_snapshot"}}}
	if !h.HasMissingMustHaveFields() {
		t.Errorf("populated MissingMustHaveFields should report true")
	}
}

// TestContractMirrorParity verifies json-tag set per type matches Console
// contractv1.go. Skips when CONSOLE_REPO is not set. AC4.
//
// Strategy: parse Console contractv1.go for `type <Name> struct` blocks and
// their `json:"..."` tag lists; cross-check against our local types' tags
// using reflect. Only structural / tag mismatches fail (not internal field
// ordering — but our local file mirrors order for readability).
func TestContractMirrorParity(t *testing.T) {
	repo := os.Getenv("CONSOLE_REPO")
	if repo == "" {
		t.Skip("CONSOLE_REPO env not set; skipping cross-repo parity check (D5 historical-skip)")
	}
	consolePath := filepath.Join(repo, "console-api", "internal", "coreadapter", "contractv1", "contractv1.go")
	source, err := os.ReadFile(consolePath)
	if err != nil {
		t.Fatalf("read Console contractv1.go at %s: %v", consolePath, err)
	}
	consoleTags := extractStructTags(string(source))

	// Build expected map of local types via reflect.
	localTypes := []any{
		FieldAvailability{},
		WorkspaceCreate{}, Workspace{},
		IndexJob{},
		SearchRequest{}, SearchResult{}, RetrievalTrace{}, SourceChunk{}, Citation{},
		MemoryItem{}, MemoryOperation{},
		EvalRun{}, EvalRunCreate{}, CaseResult{},
		ObservabilityEvent{},
		AgentScope{}, CoreHealth{},
	}
	for _, v := range localTypes {
		typ := reflect.TypeOf(v)
		name := typ.Name()
		localTagSet := jsonTagSet(typ)
		consoleTagSet, ok := consoleTags[name]
		if !ok {
			t.Errorf("type %q not found in Console contractv1.go", name)
			continue
		}
		if !setEqual(localTagSet, consoleTagSet) {
			t.Errorf("type %q tag set mismatch:\n  local:   %v\n  console: %v", name, sortedKeys(localTagSet), sortedKeys(consoleTagSet))
		}
	}
}

// jsonTagSet returns the set of json tag names on the struct's fields,
// stripped of options (omitempty etc.).
func jsonTagSet(t reflect.Type) map[string]struct{} {
	set := make(map[string]struct{})
	for i := 0; i < t.NumField(); i++ {
		tag := t.Field(i).Tag.Get("json")
		if tag == "" || tag == "-" {
			continue
		}
		name := tag
		if idx := strings.IndexByte(tag, ','); idx >= 0 {
			name = tag[:idx]
		}
		set[name] = struct{}{}
	}
	return set
}

// extractStructTags parses Go source text for `type X struct { ... }` blocks
// and returns map[StructName]map[jsonTag]struct{}. Heuristic-only parser
// (no go/parser to avoid build-time deps on the parsed file's imports).
func extractStructTags(src string) map[string]map[string]struct{} {
	result := make(map[string]map[string]struct{})
	lines := strings.Split(src, "\n")
	inStruct := ""
	for _, line := range lines {
		trim := strings.TrimSpace(line)
		if inStruct != "" {
			if trim == "}" {
				inStruct = ""
				continue
			}
			// look for `json:"..."`
			idx := strings.Index(line, `json:"`)
			if idx < 0 {
				continue
			}
			rest := line[idx+len(`json:"`):]
			end := strings.IndexByte(rest, '"')
			if end < 0 {
				continue
			}
			tag := rest[:end]
			if tag == "" || tag == "-" {
				continue
			}
			name := tag
			if cIdx := strings.IndexByte(tag, ','); cIdx >= 0 {
				name = tag[:cIdx]
			}
			result[inStruct][name] = struct{}{}
		} else {
			// match `type <Name> struct {` (possibly with trailing comment)
			if !strings.HasPrefix(trim, "type ") {
				continue
			}
			fields := strings.Fields(trim)
			if len(fields) < 3 || fields[2] != "struct" {
				continue
			}
			name := fields[1]
			inStruct = name
			result[name] = make(map[string]struct{})
		}
	}
	return result
}

func setEqual(a, b map[string]struct{}) bool {
	if len(a) != len(b) {
		return false
	}
	for k := range a {
		if _, ok := b[k]; !ok {
			return false
		}
	}
	return true
}

func sortedKeys(s map[string]struct{}) []string {
	keys := make([]string, 0, len(s))
	for k := range s {
		keys = append(keys, k)
	}
	// no need to import sort for test diff; trivial bubble for determinism
	for i := 0; i < len(keys); i++ {
		for j := i + 1; j < len(keys); j++ {
			if keys[j] < keys[i] {
				keys[i], keys[j] = keys[j], keys[i]
			}
		}
	}
	return keys
}
