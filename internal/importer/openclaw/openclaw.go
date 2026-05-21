// Package openclaw imports OpenClaw workspaces through ContextForge's generic
// importer fallback path. OpenClaw schema-aware parsing is intentionally out of
// scope for v0.1 while PRD O3 remains TBD.
package openclaw

import (
	"fmt"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/tajiaoyezi/contextforge/internal/importer"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/protobuf/types/known/timestamppb"
)

const Provider = "openclaw"

type openClawImporter struct {
	agentName string
}

// NewImporter creates an OpenClaw workspace importer.
func NewImporter(agentName string) importer.Importer {
	if strings.TrimSpace(agentName) == "" {
		agentName = Provider
	}
	return &openClawImporter{agentName: agentName}
}

// CollectionID derives the default collection id from agent name and workspace name.
func CollectionID(workspacePath string, agentName string) string {
	if strings.TrimSpace(agentName) == "" {
		agentName = Provider
	}
	workspace := filepath.Base(filepath.Clean(workspacePath))
	if workspace == "." || workspace == string(filepath.Separator) || workspace == "" {
		workspace = "workspace"
	}
	return fmt.Sprintf("%s/%s", agentName, workspace)
}

// ImportWorkspace imports an OpenClaw workspace with the default importer.
func ImportWorkspace(path string, collectionID string, agentName string) ([]*contextforgev1.ContextRecord, error) {
	return NewImporter(agentName).Import(path, collectionID)
}

func (o *openClawImporter) Name() string { return "openclaw-workspace" }

func (o *openClawImporter) Detect(path string) (float64, bool) {
	info, err := os.Stat(path)
	if err != nil {
		return 0, false
	}
	if !info.IsDir() {
		_, ok := classify(path)
		return 0.6, ok
	}
	if strings.Contains(strings.ToLower(filepath.Base(path)), Provider) {
		return 0.8, true
	}
	if _, err := os.Stat(filepath.Join(path, ".openclaw")); err == nil {
		return 0.9, true
	}
	return 0, false
}

func (o *openClawImporter) Import(path string, collectionID string) ([]*contextforgev1.ContextRecord, error) {
	if collectionID == "" {
		collectionID = CollectionID(path, o.agentName)
	}
	files, err := collectImportableFiles(path)
	if err != nil {
		return nil, err
	}

	fallback := importer.NewFileFallbackImporter()
	workspaceName := filepath.Base(filepath.Clean(path))
	records := make([]*contextforgev1.ContextRecord, 0, len(files))
	for _, file := range files {
		sourceType, ok := classify(file)
		if !ok {
			continue
		}
		if sourceType == "memory" {
			log.Printf("[warning] OpenClaw memory schema for %q is TBD; using generic fallback importer", file)
		}
		recs, err := fallback.Import(file, collectionID)
		if err != nil {
			return nil, err
		}
		for _, rec := range recs {
			o.applyOpenClawFields(rec, file, collectionID, workspaceName, sourceType)
			records = append(records, rec)
		}
	}
	return records, nil
}

func (o *openClawImporter) applyOpenClawFields(rec *contextforgev1.ContextRecord, file string, collectionID string, workspaceName string, sourceType string) {
	rec.CollectionId = collectionID
	rec.SourceType = sourceType
	rec.SourceProvider = Provider
	rec.AgentScope = []string{o.agentName, "workspace:" + workspaceName}
	rec.Tags = []string{sourceType, Provider}

	info, err := os.Stat(file)
	var modified *timestamppb.Timestamp
	if err == nil {
		modified = timestamppb.New(info.ModTime().UTC())
	}
	if len(rec.Provenance) == 0 {
		rec.Provenance = []*contextforgev1.Provenance{{}}
	}
	rec.Provenance[0].Importer = o.Name()
	rec.Provenance[0].OriginalPath = file
	rec.Provenance[0].SourceModifiedAt = modified
}

func collectImportableFiles(path string) ([]string, error) {
	info, err := os.Stat(path)
	if err != nil {
		return nil, err
	}
	if !info.IsDir() {
		if _, ok := classify(path); ok {
			return []string{path}, nil
		}
		return nil, nil
	}

	var files []string
	err = filepath.WalkDir(path, func(current string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			return nil
		}
		if _, ok := classify(current); ok {
			files = append(files, current)
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	sort.Strings(files)
	return files, nil
}

func classify(path string) (string, bool) {
	ext := strings.ToLower(filepath.Ext(path))
	if isMemoryLike(path) {
		return "memory", true
	}
	if isLogLike(path) {
		return "log", true
	}
	switch ext {
	case ".json", ".yaml", ".yml", ".toml":
		return "config", true
	case ".md", ".txt":
		return "file", true
	default:
		return "", false
	}
}

func isMemoryLike(path string) bool {
	for _, part := range splitPath(path) {
		if part == "memory" || part == "memories" {
			return true
		}
	}
	base := strings.ToLower(filepath.Base(path))
	return strings.Contains(base, "memory")
}

func isLogLike(path string) bool {
	ext := strings.ToLower(filepath.Ext(path))
	if ext == ".log" || ext == ".jsonl" {
		return true
	}
	for _, part := range splitPath(path) {
		if part == "log" || part == "logs" {
			return true
		}
	}
	return false
}

func splitPath(path string) []string {
	clean := filepath.Clean(path)
	parts := strings.FieldsFunc(clean, func(r rune) bool {
		return r == filepath.Separator || r == '/'
	})
	for i, part := range parts {
		parts[i] = strings.ToLower(part)
	}
	return parts
}
