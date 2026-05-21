// Package hermes implements the Hermes MEMORY.md / USER.md importer.
// task-3.2 scope.
//
// RED checkpoint: types per §5.3 contract; method bodies are deliberate stubs
// (Detect always (0, false); Import returns nil, nil) so the 4 RED tests below
// compile + fail with descriptive assertions. GREEN commit replaces stubs with
// real impl.
//
// Detect (v0.1 §2A 决策): filename MEMORY.md / USER.md (大小写不敏感) → 0.9, true.
// AC4 fallback (v0.1 §2A 决策): TrimSpace(content)=="" → task-3.1
// NewFileFallbackImporter + warning. Otherwise build hermes-specific record.
//
// Canonical record key fields (BINDING):
//   source_provider="hermes" / agent_scope=["hermes"] / provenance.importer="hermes-memory"
//   source_type="memory" / language="markdown" / redaction_status="pending"
package hermes

import (
	"github.com/tajiaoyezi/contextforge/internal/importer"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// New creates a Hermes-aware importer.
// Caller (CLI / daemon / tests): importer.Register(hermes.New()) or invoke directly.
func New() importer.Importer { return &hermesImporter{} }

// hermesImporter implements importer.Importer for Hermes MEMORY.md / USER.md.
type hermesImporter struct{}

// Name returns "hermes-memory" (matches PRD §Canonical Record provenance.importer example).
func (h *hermesImporter) Name() string { return "hermes-memory" }

// Detect: RED stub — always (0, false). GREEN: filename MEMORY.md/USER.md (case-insensitive) → 0.9, true.
func (h *hermesImporter) Detect(path string) (float64, bool) {
	return 0, false
}

// Import: RED stub — returns nil, nil. GREEN: read file → recognized = buildHermesRecord /
// unrecognized (empty content) = task-3.1 NewFileFallbackImporter + warning.
func (h *hermesImporter) Import(path, collectionID string) ([]*contextforgev1.ContextRecord, error) {
	return nil, nil
}
