package mcpadapter

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

func (s *Server) callContextSearch(ctx context.Context, args map[string]any) (any, error) {
	req, err := searchRequestFromArgs(args, false)
	if err != nil {
		return nil, err
	}
	return s.search(ctx, req)
}

func (s *Server) callContextRead(ctx context.Context, args map[string]any) (any, error) {
	chunkID, err := requiredStringArg(args, "chunk_id")
	if err != nil {
		return nil, err
	}
	collection := stringArg(args, "collection", "default")
	resp, err := s.search(ctx, &contextforgev1.SearchRequest{
		Query:       chunkID,
		Collections: []string{collection},
		TopK:        1,
	})
	if err != nil {
		return nil, err
	}
	if len(resp.GetResults()) == 0 {
		return nil, newRPCError(codeServerError, "chunk not found", map[string]any{"chunk_id": chunkID})
	}
	return resp.GetResults()[0], nil
}

func (s *Server) callContextExplain(ctx context.Context, args map[string]any) (any, error) {
	req, err := searchRequestFromArgs(args, true)
	if err != nil {
		return nil, err
	}
	resp, err := s.search(ctx, req)
	if err != nil {
		return nil, err
	}
	trace := make([]map[string]any, 0, len(resp.GetResults()))
	for _, r := range resp.GetResults() {
		trace = append(trace, map[string]any{
			"chunk_id":         r.GetChunkId(),
			"retrieval_method": r.GetRetrievalMethod(),
			"reason":           r.GetReason(),
			"provenance":       r.GetProvenance(),
		})
	}
	return map[string]any{
		"results":         resp.GetResults(),
		"retrieval_trace": trace,
	}, nil
}

func (s *Server) callContextCollections(_ context.Context, _ map[string]any) (any, error) {
	collDir := filepath.Join(s.DataDir, "collections")
	entries, err := os.ReadDir(collDir)
	if err != nil {
		if os.IsNotExist(err) {
			return map[string]any{"collections": []collectionInfo{}}, nil
		}
		return nil, fmt.Errorf("read collections: %w", err)
	}
	out := []collectionInfo{}
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}
		var lastIndexedAt string
		if info, _ := entry.Info(); info != nil {
			lastIndexedAt = info.ModTime().UTC().Format(time.RFC3339)
		}
		out = append(out, collectionInfo{
			ID:            entry.Name(),
			ChunkCount:    0,
			LastIndexedAt: lastIndexedAt,
		})
	}
	return map[string]any{"collections": out}, nil
}

type collectionInfo struct {
	ID            string `json:"id"`
	ChunkCount    int64  `json:"chunk_count"`
	LastIndexedAt string `json:"last_indexed_at"`
}

func (s *Server) search(ctx context.Context, req *contextforgev1.SearchRequest) (*contextforgev1.SearchResponse, error) {
	if s.Searcher == nil {
		return nil, newRPCError(codeInternalError, "search backend not wired", nil)
	}
	return s.Searcher.Search(ctx, req)
}

func searchRequestFromArgs(args map[string]any, forceExplain bool) (*contextforgev1.SearchRequest, error) {
	query, err := requiredStringArg(args, "query")
	if err != nil {
		return nil, err
	}
	topK := int32Arg(args, "top_k", 10)
	if topK <= 0 {
		topK = 10
	}
	explain := boolArg(args, "explain", false)
	if forceExplain {
		explain = true
	}
	collections := stringSliceArg(args, "collections")
	if len(collections) == 0 {
		if c := stringArg(args, "collection", ""); c != "" {
			collections = []string{c}
		} else {
			collections = []string{"default"}
		}
	}
	return &contextforgev1.SearchRequest{
		Query:       query,
		Collections: collections,
		AgentScope:  stringSliceArg(args, "agent_scope"),
		TopK:        topK,
		Filters: &contextforgev1.SearchFilters{
			SourceType: stringSliceArg(args, "source_type"),
			Language:   stringSliceArg(args, "language"),
		},
		Explain: explain,
	}, nil
}

func requiredStringArg(args map[string]any, key string) (string, error) {
	value := stringArg(args, key, "")
	if value == "" {
		return "", newRPCError(codeInvalidParams, "missing required argument: "+key, nil)
	}
	return value, nil
}

func stringArg(args map[string]any, key, fallback string) string {
	raw, ok := args[key]
	if !ok || raw == nil {
		return fallback
	}
	if s, ok := raw.(string); ok {
		if strings.TrimSpace(s) == "" {
			return fallback
		}
		return s
	}
	return fallback
}

func stringSliceArg(args map[string]any, key string) []string {
	raw, ok := args[key]
	if !ok || raw == nil {
		return nil
	}
	switch v := raw.(type) {
	case []string:
		return cleanStrings(v)
	case []any:
		out := make([]string, 0, len(v))
		for _, item := range v {
			if s, ok := item.(string); ok {
				out = append(out, s)
			}
		}
		return cleanStrings(out)
	case string:
		if strings.TrimSpace(v) == "" {
			return nil
		}
		return cleanStrings(strings.Split(v, ","))
	default:
		return nil
	}
}

func cleanStrings(in []string) []string {
	out := make([]string, 0, len(in))
	for _, s := range in {
		if trimmed := strings.TrimSpace(s); trimmed != "" {
			out = append(out, trimmed)
		}
	}
	return out
}

func int32Arg(args map[string]any, key string, fallback int32) int32 {
	raw, ok := args[key]
	if !ok || raw == nil {
		return fallback
	}
	switch v := raw.(type) {
	case int:
		return int32(v)
	case int32:
		return v
	case int64:
		return int32(v)
	case float64:
		return int32(v)
	default:
		return fallback
	}
}

func boolArg(args map[string]any, key string, fallback bool) bool {
	raw, ok := args[key]
	if !ok || raw == nil {
		return fallback
	}
	if b, ok := raw.(bool); ok {
		return b
	}
	return fallback
}
