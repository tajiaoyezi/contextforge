// Package agentrules implements the agent-rules importer for project instruction
// files (AGENTS.md, CLAUDE.md) and TBD Cursor/Zed rules (via direct use).
// task-3.4 scope. Registers itself via init() for registry integration.
package agentrules

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/tajiaoyezi/contextforge/internal/importer"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
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
	// RED skeleton: intentionally not implemented to produce failing (red) tests.
	// GREEN will replace with full read + canonical buildRecord (redaction=pending).
	_ = path
	_ = collectionID
	return nil, &redSkeletonError{msg: "RED: agent-rules Import not implemented yet (SCEN-3.4.*)"}
}

type redSkeletonError struct{ msg string }

func (e *redSkeletonError) Error() string { return e.msg }

// init registers the importer so blank-import activates it in registry.
func init() {
	importer.Register(NewAgentRulesImporter())
}