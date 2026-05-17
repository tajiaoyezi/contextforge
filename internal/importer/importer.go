// Package importer implements the Agent importer framework: Importer abstraction,
// registry, and canonical-record mapping. task-3.1 scope.
package importer

import (
	"sync"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// Importer is the abstraction for importing an external source into canonical
// ContextRecords. Each concrete importer (hermes, openclaw, agent-rules, fallback)
// implements this interface.
type Importer interface {
	// Name returns the importer identifier, e.g. "fallback", "hermes-memory".
	Name() string

	// Detect reports whether this importer recognises the given path and how
	// confident it is (0.0–1.0). The registry uses the highest-confidence match.
	Detect(path string) (confidence float64, ok bool)

	// Import reads the source at path and maps it to one or more ContextRecords.
	// collectionID is the target collection.
	Import(path string, collectionID string) ([]*contextforgev1.ContextRecord, error)
}

var (
	registry   []Importer
	registryMu sync.RWMutex
)

// Register adds an Importer to the global registry. It is safe for concurrent use.
func Register(importer Importer) {
	registryMu.Lock()
	defer registryMu.Unlock()
	registry = append(registry, importer)
}

// Resolve selects the best Importer for path. If no registered importer reports
// ok, FileFallbackImporter is returned so that import never hard-fails (AC2/AC3).
func Resolve(path string) (Importer, error) {
	registryMu.RLock()
	imps := make([]Importer, len(registry))
	copy(imps, registry)
	registryMu.RUnlock()

	var best Importer
	var bestConf float64
	for _, imp := range imps {
		conf, ok := imp.Detect(path)
		if ok && conf > bestConf {
			bestConf = conf
			best = imp
		}
	}
	if best == nil {
		return NewFileFallbackImporter(), nil
	}
	return best, nil
}
