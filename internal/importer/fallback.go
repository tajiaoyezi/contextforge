package importer

import (
	"os"
	"path/filepath"
	"strings"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// fileFallbackImporter is the catch-all importer for any file that no
// schema-aware importer recognises. It treats the file as a raw text/binary
// source and maps it to a single ContextRecord (AC2).
type fileFallbackImporter struct{}

// NewFileFallbackImporter creates the generic file fallback importer.
func NewFileFallbackImporter() Importer {
	return &fileFallbackImporter{}
}

func (f *fileFallbackImporter) Name() string { return "fallback" }

func (f *fileFallbackImporter) Detect(path string) (float64, bool) {
	// Fallback accepts any regular file with low confidence so that it acts as
	// the safety net when no higher-confidence importer matches.
	info, err := os.Stat(path)
	if err != nil || info.IsDir() {
		return 0, false
	}
	return 0.1, true
}

func (f *fileFallbackImporter) Import(path string, collectionID string) ([]*contextforgev1.ContextRecord, error) {
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

	rec := buildRecord(recordInput{
		path:         abs,
		collectionID: collectionID,
		content:      content,
		lineCount:    lineCount,
		sourceType:   "file",
		provider:     "local",
		importerName: "fallback",
	})
	return []*contextforgev1.ContextRecord{rec}, nil
}
