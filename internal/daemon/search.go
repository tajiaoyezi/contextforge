// Package daemon (sub-file search.go) — task-6.1 daemon.Search wrapper.
//
// Contract: task-6.1 §5.3. Forwards a SearchRequest to contextforge-core
// via the lazily-initialised gRPC client connection (daemon.go:clientConn).
// gRPC Status codes propagate as-is — the caller (internal/cli/search.go,
// task-6.2 REST handler, or future MCP handler) decides on retry / exit.
package daemon

import (
	"context"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// Search forwards a SearchRequest to contextforge-core's ContextService.Search
// via the reused gRPC client connection (clientConn lazy-initialises and
// daemon.Stop closes it).
//
// task-6.1 §5.3 caller: internal/cli/search.go (per-invocation spawn pattern;
// CLI calls Start → polls Health → calls Search → Stop). task-6.2 REST API
// and task-7.1 MCP wrapper also reuse this method.
func (d *Daemon) Search(
	ctx context.Context,
	req *contextforgev1.SearchRequest,
) (*contextforgev1.SearchResponse, error) {
	conn, err := d.clientConn()
	if err != nil {
		return nil, err
	}
	return contextforgev1.NewContextServiceClient(conn).Search(ctx, req)
}

// ListAllChunks forwards a ListAllChunksRequest to contextforge-core's
// ContextService.ListAllChunks (task-31.3: full-text chunk listing for the exporter, which
// SearchResponse cannot supply). Reuses the same client connection as Search.
func (d *Daemon) ListAllChunks(
	ctx context.Context,
	req *contextforgev1.ListAllChunksRequest,
) (*contextforgev1.ListAllChunksResponse, error) {
	conn, err := d.clientConn()
	if err != nil {
		return nil, err
	}
	return contextforgev1.NewContextServiceClient(conn).ListAllChunks(ctx, req)
}
