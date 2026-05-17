package importer

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"path/filepath"
	"strings"
	"time"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/protobuf/types/known/timestamppb"
)

// recordInput captures the raw data needed to build a canonical ContextRecord.
type recordInput struct {
	path         string
	collectionID string
	content      string
	lineCount    int64
	sourceType   string // e.g. "file", "memory", "log", "agent_rule"
	provider     string // e.g. "local", "openclaw", "hermes"
	importerName string // e.g. "fallback", "hermes-memory"
}

// buildRecord maps recordInput to a ContextRecord following the frozen v0.1
// canonical schema (AC4). All core fields are populated; unrecognised future
// fields will land in metadata.extra.
func buildRecord(in recordInput) *contextforgev1.ContextRecord {
	now := timestamppb.New(time.Now().UTC())
	return &contextforgev1.ContextRecord{
		Id:              makeID(in),
		SchemaVersion:   "0.1",
		CollectionId:    in.collectionID,
		SourceType:      in.sourceType,
		SourceProvider:  in.provider,
		SourceUri:       sourceURI(in.path),
		AgentScope:      []string{}, // populated by specialised importers
		Title:           filepath.Base(in.path),
		Content:         in.content,
		ContentHash:     contentHash(in.content),
		RedactionStatus: "none",
		Language:        detectLanguage(in.path),
		FilePath:        in.path,
		LineStart:       1,
		LineEnd:         in.lineCount,
		Tags:            []string{in.sourceType},
		Provenance: []*contextforgev1.Provenance{{
			Importer:     in.importerName,
			OriginalPath: in.path,
			ImportedAt:   now,
		}},
		SecurityLabels: []string{"local_only"},
		CreatedAt:      now,
		UpdatedAt:      now,
		ExpiresAt:      nil,
		Version:        1,
	}
}

// makeID builds a deterministic ID from path + content hash (sha256) to avoid
// 32-bit collisions at 100k-chunk scale (task-5.1 dedup safety).
func makeID(in recordInput) string {
	h := sha256.New()
	fmt.Fprint(h, in.path, ":", in.content)
	return "ctx_" + hex.EncodeToString(h.Sum(nil))[:16]
}

// sourceURI turns an absolute path into a file:// URI.
func sourceURI(abs string) string {
	if strings.HasPrefix(abs, "/") {
		return "file://" + abs
	}
	return abs
}

// contentHash returns a sha256 hex checksum of content. 256-bit strength avoids
// birthday collisions at 100k-chunk scale (PRD §Constraints 性能 / task-5.1).
func contentHash(content string) string {
	sum := sha256.Sum256([]byte(content))
	return hex.EncodeToString(sum[:])
}

// detectLanguage maps common extensions to the Language enum/string used in
// the canonical record.
func detectLanguage(path string) string {
	switch strings.ToLower(filepath.Ext(path)) {
	case ".md":
		return "markdown"
	case ".go":
		return "go"
	case ".rs":
		return "rust"
	case ".py":
		return "python"
	case ".ts", ".tsx":
		return "typescript"
	case ".js", ".jsx":
		return "javascript"
	case ".json":
		return "json"
	case ".yaml", ".yml":
		return "yaml"
	case ".toml":
		return "toml"
	case ".log":
		return "log"
	case ".txt":
		return "text"
	default:
		return "text"
	}
}
