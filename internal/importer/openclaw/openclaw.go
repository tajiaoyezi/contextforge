// Package openclaw imports OpenClaw workspaces through ContextForge's generic
// importer fallback path. OpenClaw schema-aware parsing is intentionally out of
// scope for v0.1 while PRD O3 remains TBD.
package openclaw

import (
	"github.com/tajiaoyezi/contextforge/internal/importer"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

const Provider = "openclaw"

type openClawImporter struct {
	agentName string
}

// NewImporter creates an OpenClaw workspace importer.
func NewImporter(agentName string) importer.Importer {
	return &openClawImporter{agentName: agentName}
}

// CollectionID derives the default collection id from agent name and workspace name.
func CollectionID(workspacePath string, agentName string) string {
	return ""
}

// ImportWorkspace imports an OpenClaw workspace with the default importer.
func ImportWorkspace(path string, collectionID string, agentName string) ([]*contextforgev1.ContextRecord, error) {
	return NewImporter(agentName).Import(path, collectionID)
}

func (o *openClawImporter) Name() string { return "openclaw-workspace" }

func (o *openClawImporter) Detect(path string) (float64, bool) {
	return 0, false
}

func (o *openClawImporter) Import(path string, collectionID string) ([]*contextforgev1.ContextRecord, error) {
	return []*contextforgev1.ContextRecord{}, nil
}
