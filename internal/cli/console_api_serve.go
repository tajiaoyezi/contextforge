// task-10.6 (Phase 10): `contextforge console-api-serve` — starts the
// Console Contract v1 REST surface (internal/consoleapi) on a loopback HTTP
// port. Used by scripts/console_smoke.sh + deploy/console-stack.yml docker
// service for Console UI integration.
//
// task-11.2 (Phase 11, ADR-016 §D3/§D4): default backend = gRPC client
// against contextforge-core data plane (127.0.0.1:50551). The v0.3 in-memory
// MemStore (task-10.4 §10 trade-off #1) is preserved as an env-gated
// fallback (CONSOLE_API_FALLBACK_INMEM=1) — when gRPC is unreachable AND
// fallback-inmem is unset, /v1/health returns degraded + 503 (all business
// endpoints also return ErrDataPlaneUnavailable → HTTP 503).

package cli

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/consoleapi"
	"github.com/tajiaoyezi/contextforge/internal/consoleapi/grpcclient"
)

// envBoolTrue maps "1" / "true" / "yes" / "on" (case-insensitive) → true.
func envBoolTrue(v string) bool {
	switch v {
	case "1", "true", "TRUE", "True", "yes", "YES", "Yes", "on", "ON", "On":
		return true
	}
	return false
}

func runConsoleAPIServe(args []string, stdout, stderr io.Writer) int {
	fs := flag.NewFlagSet("console-api-serve", flag.ContinueOnError)
	fs.SetOutput(stderr)
	addr := fs.String("addr", "127.0.0.1:48181", "bind address (default 127.0.0.1:48181; docker compose 0.0.0.0:48181)")
	grpcAddr := fs.String("grpc-addr", "127.0.0.1:50551", "contextforge-core gRPC data plane address (task-11.2; ADR-016 §D2)")
	fallbackInmem := fs.Bool("fallback-inmem", envBoolTrue(os.Getenv("CONSOLE_API_FALLBACK_INMEM")), "Enable in-memory MemStore fallback when gRPC unreachable (env CONSOLE_API_FALLBACK_INMEM=1; ADR-016 §D4)")
	authToken := fs.String("auth-token", os.Getenv("CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN"), "Bearer auth token; empty = trusted-network mode (env CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN takes precedence)")
	if err := fs.Parse(args); err != nil {
		return 2
	}

	deps, backend, closer := buildDeps(*grpcAddr, *fallbackInmem, *authToken, stdout, stderr)
	if closer != nil {
		defer closer()
	}
	deps.AuthToken = *authToken
	deps.BackendKind = backend
	router := consoleapi.NewRouter(deps)

	listener, err := net.Listen("tcp", *addr)
	if err != nil {
		fmt.Fprintf(stderr, "contextforge console-api-serve: listen %s: %v\n", *addr, err)
		return 1
	}
	defer listener.Close()
	resolved := listener.Addr().String()
	authMode := "trusted-network"
	if *authToken != "" {
		authMode = "bearer-token"
	}
	fmt.Fprintf(stdout, "contextforge console-api-serve: listening on http://%s (auth=%s backend=%s)\n", resolved, authMode, backend)
	fmt.Fprintf(stdout, "  Console Contract v1: 20 routes (22 endpoint conformance; v0.7 ADR-017 Accepted)\n")
	fmt.Fprintf(stdout, "  surfaces: health / workspace(4) / index-job(4) / search(3) / memory(5) / eval(2) / events(1)\n")

	srv := &http.Server{Handler: router, ReadHeaderTimeout: 10 * time.Second}
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	serveErr := make(chan error, 1)
	go func() { serveErr <- srv.Serve(listener) }()

	select {
	case <-ctx.Done():
		fmt.Fprintln(stdout, "contextforge console-api-serve: shutdown signal received")
	case err := <-serveErr:
		if !errors.Is(err, http.ErrServerClosed) {
			fmt.Fprintf(stderr, "contextforge console-api-serve: server error: %v\n", err)
			return 1
		}
	}
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	_ = srv.Shutdown(shutdownCtx)
	return 0
}

// buildDeps constructs the consoleapi.Deps based on flags + env.
//
// Priority (ADR-016 §D3/§D4):
//  1. fallbackInmem == true → MemStore, log warning, /v1/health → degraded
//  2. else → grpcclient.New(grpcAddr) + Ping; on success, gRPC-backed Deps
//  3. else → degraded Deps wrapper (all RPCs return ErrDataPlaneUnavailable);
//     /v1/health returns 503 + missing=["data_plane"]
//
// Returns (deps, backend-name, cleanup-fn-or-nil).
func buildDeps(grpcAddr string, fallbackInmem bool, _ string, stdout, stderr io.Writer) (consoleapi.Deps, string, func()) {
	if fallbackInmem {
		store := consoleapi.NewMemStore()
		memMem := consoleapi.NewMemMemoryStore()
		// task-31.1: memory ops emit memory.* events into the shared fallback ring (parity with
		// workspace/job fallback + the Rust data plane), visible via GET /v1/observability/events.
		memMem.SetEventSink(store.EmitEvent)
		memMem.SeedFixtures()
		memEval := consoleapi.NewMemEvalStore()
		fmt.Fprintln(stderr, "WARN console-api: using in-memory fallback store (CONSOLE_API_FALLBACK_INMEM=1; data plane bypassed; ADR-016 §D4)")
		return consoleapi.Deps{
			Workspace: consoleapi.WorkspaceAdapter{S: store},
			Job:       consoleapi.JobAdapter{S: store},
			Search:    store,
			Events:    store,
			Memory:    memMem,
			Eval:      memEval,
		}, "inmem-fallback", nil
	}

	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cli, err := grpcclient.New(ctx, grpcAddr)
	if err != nil {
		fmt.Fprintf(stderr, "WARN console-api: gRPC dial %s failed (%v); /v1/health → degraded + 503 (ADR-016 §D4: set CONSOLE_API_FALLBACK_INMEM=1 OR start contextforge-core daemon)\n", grpcAddr, err)
		return degradedDeps(), "degraded", nil
	}
	pingCtx, pingCancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer pingCancel()
	if pingErr := cli.Ping(pingCtx); pingErr != nil {
		fmt.Fprintf(stderr, "WARN console-api: gRPC %s Ping failed (%v); /v1/health → degraded + 503\n", grpcAddr, pingErr)
		_ = cli.Close()
		return degradedDeps(), "degraded", nil
	}
	fmt.Fprintf(stdout, "console-api: gRPC backend connected at %s (ADR-016 §D2 cross-process bridge)\n", grpcAddr)
	return consoleapi.Deps{
		Workspace:    cli.Workspace(),
		Job:          cli.Job(),
		Search:       cli.Search(),
		Events:       cli.Events(),
		EventsStream: cli.EventsStream(), // task-26.2 (ADR-031 D3): SSE push
		Memory:       cli.Memory(),
		Eval:         cli.Eval(),
		Health:       cli.Health(), // task-15.6 (Phase 15 P2 #7)
		User:         cli.User(),   // task-50.3 (Phase 50 / ADR-051): per-user identity
	}, "grpc", func() { _ = cli.Close() }
}

// degradedDeps returns Deps whose 4 clients all return ErrDataPlaneUnavailable
// → REST handlers translate to 503 + missing=["data_plane"].
func degradedDeps() consoleapi.Deps {
	return consoleapi.Deps{
		Workspace: degradedWorkspace{},
		Job:       degradedJob{},
		Search:    degradedSearch{},
		Events:    degradedEvents{},
		Memory:    degradedMemory{},
		Eval:      degradedEval{},
	}
}
