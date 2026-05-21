// Package hermes implements the Hermes MEMORY.md / USER.md importer.
// task-3.2 scope.
//
// Detect (v0.1 §2A 决策): filename MEMORY.md / USER.md (大小写不敏感) → 0.9, true.
// AC4 fallback (v0.1 §2A 决策): TrimSpace(content)=="" → task-3.1
// NewFileFallbackImporter + warning. Otherwise build hermes-specific record.
//
// Canonical record key fields (BINDING):
//   source_provider="hermes" / agent_scope=["hermes"] / provenance.importer="hermes-memory"
//   source_type="memory" / language="markdown" / redaction_status="pending"
//   (task-3.1 §10 Waiver — 下游 scanner/indexer 脱敏)
package hermes

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/importer"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/protobuf/types/known/timestamppb"
)

// New creates a Hermes-aware importer.
// Caller (CLI / daemon / tests): importer.Register(hermes.New()) or invoke directly.
func New() importer.Importer { return &hermesImporter{} }

// hermesImporter implements importer.Importer for Hermes MEMORY.md / USER.md.
type hermesImporter struct{}

// Name returns "hermes-memory" (matches PRD §Canonical Record provenance.importer example).
func (h *hermesImporter) Name() string { return "hermes-memory" }

// Detect: filename MEMORY.md / USER.md (case-insensitive) → 0.9, true.
// Otherwise (0, false). Pure path check — does not read the file (v0.1 §2A 决策:
// filename-only detection; PRD §O3 实测后可加版本 marker).
func (h *hermesImporter) Detect(path string) (float64, bool) {
	base := strings.ToUpper(filepath.Base(path))
	if base == "MEMORY.MD" || base == "USER.MD" {
		return 0.9, true
	}
	return 0, false
}

// Import: read file → recognized = buildHermesRecord / unrecognized
// (TrimSpace empty) = task-3.1 NewFileFallbackImporter + warning (AC4).
func (h *hermesImporter) Import(path, collectionID string) ([]*contextforgev1.ContextRecord, error) {
	abs, err := filepath.Abs(path)
	if err != nil {
		abs = path
	}
	data, err := os.ReadFile(abs)
	if err != nil {
		return nil, err
	}
	content := string(data)

	if strings.TrimSpace(content) == "" {
		log.Printf("[warning] %s: hermes content is empty; falling back to generic file importer (AC4)", abs)
		return h.fallbackImport(abs, collectionID)
	}

	return h.buildHermesRecord(abs, content, collectionID), nil
}

// fallbackImport delegates to task-3.1 FileFallbackImporter (AC4 — 复用 3.1 框架).
func (h *hermesImporter) fallbackImport(path, collectionID string) ([]*contextforgev1.ContextRecord, error) {
	fb := importer.NewFileFallbackImporter()
	return fb.Import(path, collectionID)
}

// buildHermesRecord constructs a ContextRecord with hermes-specific provider /
// scope / provenance / source_type / language / redaction_status (BINDING).
func (h *hermesImporter) buildHermesRecord(path, content, collectionID string) []*contextforgev1.ContextRecord {
	now := timestamppb.New(time.Now().UTC())
	var sourceModified *timestamppb.Timestamp
	if info, err := os.Stat(path); err == nil {
		sourceModified = timestamppb.New(info.ModTime().UTC())
	}

	lineCount := int64(strings.Count(content, "\n"))
	if len(content) > 0 && !strings.HasSuffix(content, "\n") {
		lineCount++
	}
	if lineCount == 0 && len(content) > 0 {
		lineCount = 1
	}

	rec := &contextforgev1.ContextRecord{
		Id:              makeID(path, content),
		SchemaVersion:   "0.1",
		CollectionId:    collectionID,
		SourceType:      "memory",
		SourceProvider:  "hermes",
		SourceUri:       sourceURI(path),
		AgentScope:      []string{"hermes"},
		Title:           filepath.Base(path),
		Content:         content,
		ContentHash:     contentHash(content),
		RedactionStatus: "pending", // BINDING: task-3.1 §10 Waiver
		Language:        "markdown",
		FilePath:        path,
		LineStart:       1,
		LineEnd:         lineCount,
		Tags:            []string{"memory", "hermes"},
		Provenance: []*contextforgev1.Provenance{{
			Importer:         "hermes-memory",
			OriginalPath:     path,
			ImportedAt:       now,
			SourceModifiedAt: sourceModified,
		}},
		SecurityLabels: []string{"local_only"},
		CreatedAt:      now,
		UpdatedAt:      now,
		ExpiresAt:      nil,
		Version:        1,
	}
	return []*contextforgev1.ContextRecord{rec}
}

// makeID builds a deterministic ID from path + content hash (sha256). Prefixed
// with "ctx_hermes_" for module-level traceability while keeping the same 16-hex
// truncation convention as task-3.1.
func makeID(path, content string) string {
	h := sha256.New()
	fmt.Fprint(h, path, ":", content)
	return "ctx_hermes_" + hex.EncodeToString(h.Sum(nil))[:16]
}

// sourceURI turns an absolute path into a file:// URI (matches task-3.1 convention).
func sourceURI(abs string) string {
	if strings.HasPrefix(abs, "/") {
		return "file://" + abs
	}
	return abs
}

// contentHash returns sha256 hex (no algo-prefix, matches task-3.1 importer convention
// for cross-module Phase 5 memoryops alignment).
func contentHash(content string) string {
	sum := sha256.Sum256([]byte(content))
	return hex.EncodeToString(sum[:])
}
