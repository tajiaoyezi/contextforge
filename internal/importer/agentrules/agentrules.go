// Package agentrules implements the agent-rules importer for project instruction
// files (AGENTS.md, CLAUDE.md) and TBD Cursor/Zed rules (via direct use).
// task-3.4 scope. Registers itself via init() for registry integration.
package agentrules

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/importer"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/protobuf/types/known/timestamppb"
)

// agentRulesImporter implements Importer for AGENTS.md / CLAUDE.md / rule Markdown.
// Detect only auto-matches stable names (AGENTS/CLAUDE) per AC4 (TBD paths fall to
// FileFallbackImporter + warning). Direct construction supports any rule file with
// agent_rule marking (AC2).
type agentRulesImporter struct{}

// NewAgentRulesImporter creates the agent-rules importer instance.
func NewAgentRulesImporter() importer.Importer {
	return &agentRulesImporter{}
}

func (a *agentRulesImporter) Name() string { return "agent-rules" }

func (a *agentRulesImporter) Detect(path string) (float64, bool) {
	info, err := os.Stat(path)
	if err != nil || info.IsDir() {
		return 0, false
	}
	base := strings.ToLower(filepath.Base(path))
	if base == "agents.md" || base == "claude.md" {
		return 0.9, true
	}
	// Cursor/Zed and other TBD paths: do not auto-match (AC4). Direct Import still
	// available for "import agent-rules <path>" to mark as agent_rule.
	return 0, false
}

func (a *agentRulesImporter) Import(path string, collectionID string) ([]*contextforgev1.ContextRecord, error) {
	abs, err := filepath.Abs(path)
	if err != nil {
		abs = path
	}
	data, err := os.ReadFile(abs)
	if err != nil {
		return nil, err
	}
	content := string(data)
	lineCount := int64(strings.Count(content, "\n"))
	if len(data) > 0 && data[len(data)-1] != '\n' {
		lineCount++
	}
	if lineCount == 0 && len(data) > 0 {
		lineCount = 1
	}

	provider := inferProvider(path)
	rec := buildAgentRuleRecord(recordInput{
		path:         abs,
		collectionID: collectionID,
		content:      content,
		lineCount:    lineCount,
		sourceType:   "agent_rule",
		provider:     provider,
		importerName: "agent-rules",
	})
	return []*contextforgev1.ContextRecord{rec}, nil
}

// inferProvider maps filename to source_provider (claude-code for stable names,
// cursor/zed/local for direct TBD use per AC2).
func inferProvider(path string) string {
	base := strings.ToLower(filepath.Base(path))
	switch {
	case strings.Contains(base, "agents") || base == "agents.md":
		return "claude-code"
	case strings.Contains(base, "claude"):
		return "claude-code"
	case strings.Contains(base, "cursor"):
		return "cursor"
	case strings.Contains(base, "zed"):
		return "zed"
	default:
		return "local"
	}
}

// recordInput local mirror (buildRecord unexported in parent).
type recordInput struct {
	path         string
	collectionID string
	content      string
	lineCount    int64
	sourceType   string
	provider     string
	importerName string
}

// buildAgentRuleRecord duplicates 3.1 buildRecord exactly (to produce identical
// canonical shape + redaction_status=pending, without editing core in parallel).
func buildAgentRuleRecord(in recordInput) *contextforgev1.ContextRecord {
	now := timestamppb.New(time.Now().UTC())
	return &contextforgev1.ContextRecord{
		Id:              makeID(in),
		SchemaVersion:   "0.1",
		CollectionId:    in.collectionID,
		SourceType:      in.sourceType,
		SourceProvider:  in.provider,
		SourceUri:       sourceURI(in.path),
		AgentScope:      []string{},
		Title:           filepath.Base(in.path),
		Content:         in.content,
		ContentHash:     contentHash(in.content),
		RedactionStatus: "pending",
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

func makeID(in recordInput) string {
	h := sha256.New()
	fmt.Fprint(h, in.path, ":", in.content)
	return "ctx_" + hex.EncodeToString(h.Sum(nil))[:16]
}

func sourceURI(abs string) string {
	if strings.HasPrefix(abs, "/") {
		return "file://" + abs
	}
	return abs
}

func contentHash(content string) string {
	sum := sha256.Sum256([]byte(content))
	return hex.EncodeToString(sum[:])
}

func detectLanguage(path string) string {
	switch strings.ToLower(filepath.Ext(path)) {
	case ".md":
		return "markdown"
	default:
		return "text"
	}
}

// init registers on package load (blank import side-effect).
func init() {
	importer.Register(NewAgentRulesImporter())
}
