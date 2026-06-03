package exporter

import (
	"context"
	"crypto/sha256"
	"fmt"
	"path/filepath"
	"strings"
	"sync"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/protobuf/types/known/timestamppb"
)

const pseudoFullScanTopK int32 = 1_000_000

// SearchBackend is the production bridge to daemon.Search. It is injectable so
// internal/cli can import exporter without creating a cli -> exporter ->
// daemon -> cli test import cycle.
type SearchBackend func(
	ctx context.Context,
	dataDir string,
	req *contextforgev1.SearchRequest,
) (*contextforgev1.SearchResponse, error)

var searchBackendState struct {
	sync.Mutex
	backend SearchBackend
}

// ChunkLoader is the task-31.3 bridge to daemon.ListAllChunks: returns chunk_id → full content for a
// collection, so the exporter can fill real ContextRecord.Content + a real ContentHash (vs the prior
// content=""). Injectable for the same import-cycle reason as SearchBackend; nil → content stays "".
type ChunkLoader func(ctx context.Context, dataDir, collection string) (map[string]string, error)

var chunkLoaderState struct {
	sync.Mutex
	loader ChunkLoader
}

// SetChunkLoader wires the daemon.ListAllChunks-backed full-text loader (task-31.3) and returns a
// restore function for tests.
func SetChunkLoader(l ChunkLoader) func() {
	chunkLoaderState.Lock()
	prev := chunkLoaderState.loader
	chunkLoaderState.loader = l
	chunkLoaderState.Unlock()
	return func() {
		chunkLoaderState.Lock()
		chunkLoaderState.loader = prev
		chunkLoaderState.Unlock()
	}
}

func currentChunkLoader() ChunkLoader {
	chunkLoaderState.Lock()
	defer chunkLoaderState.Unlock()
	return chunkLoaderState.loader
}

// SetSearchBackend wires the daemon.Search-backed loader and returns a restore
// function for tests.
func SetSearchBackend(b SearchBackend) func() {
	searchBackendState.Lock()
	prev := searchBackendState.backend
	searchBackendState.backend = b
	searchBackendState.Unlock()
	return func() {
		searchBackendState.Lock()
		searchBackendState.backend = prev
		searchBackendState.Unlock()
	}
}

func currentSearchBackend() SearchBackend {
	searchBackendState.Lock()
	defer searchBackendState.Unlock()
	return searchBackendState.backend
}

// loadRecords performs the task-6.3 pseudo full-scan path:
// daemon.Search(query="*", top_k=large) -> minimal ContextRecord mapping.
func loadRecords(ctx context.Context, dataDir, collection string) ([]*contextforgev1.ContextRecord, error) {
	backend := currentSearchBackend()
	if backend == nil {
		return nil, fmt.Errorf("exporter: search backend not wired")
	}
	if collection == "" {
		collection = "default"
	}
	resp, err := backend(ctx, dataDir, &contextforgev1.SearchRequest{
		Query:       "*",
		Collections: []string{collection},
		TopK:        pseudoFullScanTopK,
		Explain:     true,
	})
	if err != nil {
		return nil, err
	}

	// task-31.3: fetch real chunk full text (SearchResponse carries none) so records get real
	// content + a real ContentHash. Best-effort: a nil loader (e.g. unit tests not wiring it) or a
	// load error leaves content="" (backward-compatible with the prior behavior).
	var chunkText map[string]string
	if loader := currentChunkLoader(); loader != nil {
		if m, lerr := loader(ctx, dataDir, collection); lerr == nil {
			chunkText = m
		}
	}

	now := timestamppb.Now()
	out := make([]*contextforgev1.ContextRecord, 0, len(resp.GetResults()))
	for _, r := range resp.GetResults() {
		if r == nil {
			continue
		}
		id := r.GetContextId()
		if id == "" {
			id = r.GetChunkId()
		}
		title := strings.TrimSuffix(filepath.Base(r.GetFilePath()), filepath.Ext(r.GetFilePath()))
		if title == "." || title == string(filepath.Separator) || title == "" {
			title = id
		}
		content := chunkText[r.GetChunkId()]
		rec := &contextforgev1.ContextRecord{
			Id:              id,
			SchemaVersion:   "0.1",
			CollectionId:    collection,
			SourceType:      r.GetSourceType(),
			SourceProvider:  "contextforge-search",
			SourceUri:       fileURI(r.GetFilePath()),
			AgentScope:      r.GetAgentScope(),
			Title:           title,
			Content:         content,
			ContentHash:     contentHash(content),
			RedactionStatus: r.GetRedactionStatus(),
			Language:        languageFromPath(r.GetFilePath()),
			FilePath:        r.GetFilePath(),
			LineStart:       r.GetLineStart(),
			LineEnd:         r.GetLineEnd(),
			Provenance:      r.GetProvenance(),
			CreatedAt:       now,
			UpdatedAt:       now,
			Version:         1,
		}
		out = append(out, rec)
	}
	return out, nil
}

func contentHash(content string) string {
	sum := sha256.Sum256([]byte(content))
	return fmt.Sprintf("sha256:%x", sum)
}

func fileURI(path string) string {
	if path == "" {
		return ""
	}
	if strings.Contains(path, "://") {
		return path
	}
	return "file://" + filepath.ToSlash(path)
}

func languageFromPath(path string) string {
	switch strings.ToLower(filepath.Ext(path)) {
	case ".go":
		return "go"
	case ".rs":
		return "rust"
	case ".md", ".markdown":
		return "markdown"
	case ".py":
		return "python"
	case ".ts", ".tsx":
		return "typescript"
	case ".js", ".jsx":
		return "javascript"
	case ".json", ".jsonl":
		return "json"
	case ".yaml", ".yml":
		return "yaml"
	case ".toml":
		return "toml"
	default:
		return strings.TrimPrefix(strings.ToLower(filepath.Ext(path)), ".")
	}
}
