// Package daemon (sub-file index.go) — task-9.3 Daemon.Index wrapper.
//
// Contract: task-9.3 §5.3. Forwards an IndexRequest to contextforge-core's
// ContextService.Index server-streaming RPC via the lazily-initialised gRPC
// client (daemon.go:clientConn). Consumes the IndexProgress stream and
// invokes the caller-provided onProgress callback per message (including
// the final done=true message). Returns the first transport-level error
// (or nil on clean completion).
//
// task-9.2 / ADR-013 §Decision #2: indexer-internal errors arrive in-band
// via the final IndexProgress.error field — caller is responsible for
// inspecting it (Daemon.Index does NOT promote them to error returns).
//
// The injected `cli.IndexBackend` (internal/cli/index.go) callable is set
// from cmd/contextforge/main.go to a closure that spawns a transient core
// daemon, waits Health=SERVING, calls this method, then defers Stop().
package daemon

import (
	"context"
	"io"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// Index forwards an IndexRequest to contextforge-core's ContextService.Index
// server-stream RPC and invokes onProgress per received IndexProgress message
// (in receive order). Returns the first transport-level error encountered,
// or nil on clean stream completion (final done=true message received).
//
// Caller MUST inspect the final IndexProgress.Error field via onProgress —
// indexer-internal failures arrive in-band per task-9.2 §5.3 contract.
// Context cancellation terminates the stream consumption goroutine; the
// server-side spawn_blocking indexer task continues to flush its current
// file before exiting (mpsc send failure is logged via final message;
// SQLite + Tantivy are file-grained atomic so no partial-write corruption).
func (d *Daemon) Index(
	ctx context.Context,
	req *contextforgev1.IndexRequest,
	onProgress func(*contextforgev1.IndexProgress),
) error {
	conn, err := d.clientConn()
	if err != nil {
		return err
	}
	stream, err := contextforgev1.NewContextServiceClient(conn).Index(ctx, req)
	if err != nil {
		return err
	}
	for {
		msg, recvErr := stream.Recv()
		if recvErr == io.EOF {
			return nil
		}
		if recvErr != nil {
			return recvErr
		}
		if onProgress != nil {
			onProgress(msg)
		}
		// Server is expected to close stream after sending done=true; the
		// EOF branch above triggers next iteration. We do not early-return
		// on done=true here in case the server emits trailing metadata.
	}
}
