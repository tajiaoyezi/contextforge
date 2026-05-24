// task-10.6 (Phase 10): `contextforge console-api-serve` — starts the
// Console Contract v1 REST surface (internal/consoleapi) on a loopback HTTP
// port. Used by scripts/console_smoke.sh + deploy/console-stack.yml docker
// service for Console UI integration. v0.3 in-memory MemStore (see
// task-10.4 §10 trade-off #1).

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
)

func runConsoleAPIServe(args []string, stdout, stderr io.Writer) int {
	fs := flag.NewFlagSet("console-api-serve", flag.ContinueOnError)
	fs.SetOutput(stderr)
	addr := fs.String("addr", "127.0.0.1:48181", "bind address (default 127.0.0.1:48181; docker compose 0.0.0.0:48181)")
	authToken := fs.String("auth-token", os.Getenv("CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN"), "Bearer auth token; empty = trusted-network mode (env CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN takes precedence)")
	if err := fs.Parse(args); err != nil {
		return 2
	}

	store := consoleapi.NewMemStore()
	router := consoleapi.NewRouter(consoleapi.Deps{
		Workspace: consoleapi.WorkspaceAdapter{S: store},
		Job:       consoleapi.JobAdapter{S: store},
		Search:    store,
		Events:    store,
		AuthToken: *authToken,
	})

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
	fmt.Fprintf(stdout, "contextforge console-api-serve: listening on http://%s (auth=%s)\n", resolved, authMode)
	fmt.Fprintf(stdout, "  endpoints: GET /v1/health, POST/GET/GET /v1/workspaces*, POST/GET/POST /v1/index-jobs*, POST /v1/search, GET /v1/observability/events\n")

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
