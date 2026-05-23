// Command contextforge is the Go control-plane binary (task-1.4 Phase 1 +
// task-6.1 Phase 6). It delegates to the internal/cli stdlib subcommand
// dispatcher, wiring the production gRPC `Search` backend (per-invocation
// daemon spawn — §2A 决策 B) so internal/cli stays independent of
// internal/daemon (avoids a test-time import cycle with daemon_test.go).
package main

import (
	"context"
	"fmt"
	"os"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/cli"
	"github.com/tajiaoyezi/contextforge/internal/daemon"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// daemonHealthDeadline caps the post-spawn wait for the core gRPC server
// to report SERVING. 15s matches the task-1.4 daemon test polling budget
// (cold start + tonic listener bind can take a few seconds on Windows).
const daemonHealthDeadline = 15 * time.Second

func main() {
	cli.SetSearchBackend(searchViaDaemon)
	os.Exit(cli.Execute(os.Args[1:], os.Stdout, os.Stderr))
}

// searchViaDaemon is the production `cli.SearchBackend`: spawn a transient
// contextforge-core (§2A 决策 B per-invocation), wait until Health reports
// SERVING, call `daemon.Search`, then `defer d.Stop()` to clean up.
func searchViaDaemon(
	ctx context.Context,
	req *contextforgev1.SearchRequest,
) (*contextforgev1.SearchResponse, error) {
	d, err := daemon.Start(ctx, daemon.Options{AutoRestart: false})
	if err != nil {
		return nil, fmt.Errorf("start core daemon: %w", err)
	}
	defer d.Stop()

	deadline := time.Now().Add(daemonHealthDeadline)
	var lastErr error
	for time.Now().Before(deadline) {
		hctx, hcancel := context.WithTimeout(ctx, 2*time.Second)
		status, herr := d.HealthCheck(hctx)
		hcancel()
		if herr == nil && status == "SERVING" {
			lastErr = nil
			break
		}
		lastErr = herr
		time.Sleep(200 * time.Millisecond)
	}
	if lastErr != nil {
		return nil, fmt.Errorf("core did not reach SERVING within %s: %w",
			daemonHealthDeadline, lastErr)
	}

	return d.Search(ctx, req)
}
