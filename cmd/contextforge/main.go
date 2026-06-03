// Command contextforge is the Go control-plane binary (task-1.4 Phase 1 +
// task-6.1 / task-6.2 Phase 6). It delegates to the internal/cli stdlib
// subcommand dispatcher and injects the production backends:
//   - SearchBackend (task-6.1): per-invocation daemon spawn for `contextforge search`
//   - ServeBackend  (task-6.2): long-running daemon + REST server for `contextforge serve`
//
// internal/cli deliberately does NOT import internal/daemon — that would
// resurrect the test-time import cycle with daemon_test.go (which imports
// cli for the task-1.4 end-to-end smoke). All daemon-coupled work lives
// here in package main.
package main

import (
	"context"
	"fmt"
	"io"
	"net"
	"os"
	"os/signal"
	"runtime"
	"strconv"
	"syscall"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/cli"
	"github.com/tajiaoyezi/contextforge/internal/config"
	"github.com/tajiaoyezi/contextforge/internal/daemon"
	"github.com/tajiaoyezi/contextforge/internal/exporter"
	"github.com/tajiaoyezi/contextforge/internal/mcpadapter"
	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// daemonHealthDeadline caps the post-spawn wait for the core gRPC server
// to report SERVING. 15s matches the task-1.4 daemon test polling budget
// (cold start + tonic listener bind can take a few seconds on Windows).
const daemonHealthDeadline = 15 * time.Second

func main() {
	cli.SetSearchBackend(searchViaDaemon)
	cli.SetServeBackend(doServe)
	cli.SetMCPBackend(doMCP)
	cli.SetIndexBackend(indexViaDaemon)
	exporter.SetSearchBackend(searchViaDaemonWithDataDir)
	exporter.SetChunkLoader(listChunksViaDaemonWithDataDir)
	os.Exit(cli.Execute(os.Args[1:], os.Stdout, os.Stderr))
}

// indexViaDaemon is the production `cli.IndexBackend` (task-9.3): per-invocation
// spawn of contextforge-core (same pattern as searchViaDaemon §2A 决策 B), wait
// for Health=SERVING, then consume the gRPC Index stream via Daemon.Index. The
// caller-provided onProgress callback is invoked per IndexProgress message
// (CLI renders to stdout). Returns the first gRPC transport error or nil on
// clean stream completion; indexer-internal errors arrive in-band via the
// final IndexProgress.Error field (caller inspects).
func indexViaDaemon(
	ctx context.Context,
	req *contextforgev1.IndexRequest,
	onProgress func(*contextforgev1.IndexProgress),
) error {
	d, err := daemon.Start(ctx, daemon.Options{AutoRestart: false})
	if err != nil {
		return fmt.Errorf("start core daemon: %w", err)
	}
	defer d.Stop()
	if err := waitDaemonHealthy(ctx, d); err != nil {
		return err
	}
	return d.Index(ctx, req, onProgress)
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
	if err := waitDaemonHealthy(ctx, d); err != nil {
		return nil, err
	}
	return d.Search(ctx, req)
}

// doServe is the production `cli.ServeBackend` (task-6.2): start a
// long-running daemon (AutoRestart=true), bind the REST listener (Unix
// socket or loopback TCP per ServeOpts), wait for the gRPC core to
// report SERVING, then enter ServeREST until SIGINT/SIGTERM triggers a
// graceful shutdown.
func doServe(_ context.Context, opts *cli.ServeOpts, stdout, stderr io.Writer) error {
	// ctx scope: signal handler cancels on SIGINT/SIGTERM → graceful shutdown
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	listener, addrStr, err := resolveListener(opts.Addr, opts.Unix, stderr)
	if err != nil {
		return err
	}
	defer listener.Close()

	// task-34.2: bridge config.toml [vector] → CONTEXTFORGE_VECTOR_BACKEND/_DIM so the spawned
	// core daemon's resolve_vector_backend picks them up (env-wins; no [vector] → unchanged).
	restoreVec := setVectorEnv(opts.DataDir)
	defer restoreVec()

	d, err := daemon.Start(ctx, daemon.Options{AutoRestart: true})
	if err != nil {
		return fmt.Errorf("start core daemon: %w", err)
	}
	defer d.Stop()

	if err := waitDaemonHealthy(ctx, d); err != nil {
		return err
	}

	fmt.Fprintf(stdout, "contextforge serve: listening on %s\n", addrStr)
	fmt.Fprintf(stdout, "  token file: %s\n", opts.TokenPath)
	fmt.Fprintln(stdout, "  Authorization: Bearer <token-from-file>")

	return d.ServeREST(ctx, listener, opts.Token, opts.DataDir)
}

// doMCP is the production `cli.MCPBackend` (task-7.1): load the client
// allowlist, start a long-running core daemon, wait for gRPC Health, then serve
// MCP stdio JSON-RPC until stdin EOF or signal cancellation.
func doMCP(ctx context.Context, opts cli.MCPOpts, stdin io.Reader, stdout, _ io.Writer) error {
	allowlist, err := mcpadapter.LoadAllowlist(opts.Allowlist)
	if err != nil {
		return fmt.Errorf("load MCP allowlist %q: %w", opts.Allowlist, err)
	}
	restoreEnv, err := setDataDirEnv(opts.DataDir)
	if err != nil {
		return err
	}
	defer restoreEnv()
	// task-34.2: bridge config.toml [vector] → core env (env-wins; no [vector] → unchanged).
	restoreVec := setVectorEnv(opts.DataDir)
	defer restoreVec()

	d, err := daemon.Start(ctx, daemon.Options{AutoRestart: true})
	if err != nil {
		return fmt.Errorf("start core daemon: %w", err)
	}
	defer d.Stop()
	if err := waitDaemonHealthy(ctx, d); err != nil {
		return err
	}

	server := &mcpadapter.Server{
		Searcher:  d,
		DataDir:   opts.DataDir,
		Allowlist: allowlist,
	}
	return server.Serve(ctx, stdin, stdout)
}

// resolveListener creates the listener per ServeOpts. Unix socket is
// preferred when --unix is given; Windows falls back to loopback TCP with
// a stderr warning (§3 In Scope: Windows Unix-domain not supported v0.1).
// When neither --addr nor --unix is given, picks a free loopback port.
func resolveListener(addr, unixPath string, stderr io.Writer) (net.Listener, string, error) {
	if unixPath != "" {
		if runtime.GOOS == "windows" {
			fmt.Fprintln(stderr,
				"contextforge serve: --unix not supported on Windows; "+
					"falling back to loopback TCP (auto-selected port)")
			unixPath = ""
		} else {
			ln, err := net.Listen("unix", unixPath)
			if err != nil {
				return nil, "", fmt.Errorf("unix listen %q: %w", unixPath, err)
			}
			if err := os.Chmod(unixPath, 0o600); err != nil {
				ln.Close()
				return nil, "", fmt.Errorf("chmod unix socket %q: %w", unixPath, err)
			}
			return ln, "unix://" + unixPath, nil
		}
	}

	bind := addr
	if bind == "" {
		bind = "127.0.0.1:0" // auto-select free loopback port
	}
	ln, err := net.Listen("tcp", bind)
	if err != nil {
		return nil, "", fmt.Errorf("tcp listen %q: %w", bind, err)
	}
	return ln, "http://" + ln.Addr().String(), nil
}

// waitDaemonHealthy polls daemon.HealthCheck until it returns "SERVING"
// or daemonHealthDeadline elapses. Shared between SearchBackend (one-shot)
// and ServeBackend (long-running startup gate).
func waitDaemonHealthy(ctx context.Context, d *daemon.Daemon) error {
	deadline := time.Now().Add(daemonHealthDeadline)
	var lastErr error
	for time.Now().Before(deadline) {
		hctx, hcancel := context.WithTimeout(ctx, 2*time.Second)
		s, herr := d.HealthCheck(hctx)
		hcancel()
		if herr == nil && s == "SERVING" {
			return nil
		}
		lastErr = herr
		time.Sleep(200 * time.Millisecond)
	}
	return fmt.Errorf("core did not reach SERVING within %s: %w",
		daemonHealthDeadline, lastErr)
}

// searchViaDaemonWithDataDir is the production exporter.SearchBackend. The
// daemon API is intentionally unchanged; contextforge-core already accepts
// CONTEXTFORGE_DATA_DIR, and exec.Command inherits the parent environment.
func searchViaDaemonWithDataDir(
	ctx context.Context,
	dataDir string,
	req *contextforgev1.SearchRequest,
) (*contextforgev1.SearchResponse, error) {
	restoreEnv, err := setDataDirEnv(dataDir)
	if err != nil {
		return nil, err
	}
	defer restoreEnv()
	return searchViaDaemon(ctx, req)
}

// listChunksViaDaemonWithDataDir is the production `exporter.ChunkLoader` (task-31.3): spawn a
// transient core daemon under the export's data dir, wait for SERVING, call daemon.ListAllChunks,
// and return a chunk_id → full-content map so the exporter fills real content + ContentHash.
func listChunksViaDaemonWithDataDir(
	ctx context.Context,
	dataDir string,
	collection string,
) (map[string]string, error) {
	restoreEnv, err := setDataDirEnv(dataDir)
	if err != nil {
		return nil, err
	}
	defer restoreEnv()
	d, err := daemon.Start(ctx, daemon.Options{AutoRestart: false})
	if err != nil {
		return nil, fmt.Errorf("start core daemon: %w", err)
	}
	defer d.Stop()
	if err := waitDaemonHealthy(ctx, d); err != nil {
		return nil, err
	}
	resp, err := d.ListAllChunks(ctx, &contextforgev1.ListAllChunksRequest{CollectionId: collection})
	if err != nil {
		return nil, err
	}
	out := make(map[string]string, len(resp.GetChunks()))
	for _, c := range resp.GetChunks() {
		out[c.GetChunkId()] = c.GetContent()
	}
	return out, nil
}

func setDataDirEnv(dataDir string) (func(), error) {
	old, hadOld := os.LookupEnv("CONTEXTFORGE_DATA_DIR")
	if dataDir != "" {
		if err := os.Setenv("CONTEXTFORGE_DATA_DIR", dataDir); err != nil {
			return nil, err
		}
	}
	return func() {
		if hadOld {
			_ = os.Setenv("CONTEXTFORGE_DATA_DIR", old)
		} else {
			_ = os.Unsetenv("CONTEXTFORGE_DATA_DIR")
		}
	}, nil
}

// setVectorEnv (task-34.2 / ADR-039 D2) best-effort loads config.toml from dataDir and bridges its
// [vector] section to the spawned core daemon via CONTEXTFORGE_VECTOR_BACKEND / CONTEXTFORGE_VECTOR_DIM
// (the core's resolve_vector_backend reads these), mirroring setDataDirEnv / CONTEXTFORGE_DATA_DIR.
// ENV WINS: a variable already present in the environment is left untouched (explicit env overrides
// the config file). A missing/malformed config or an empty [vector] exports nothing, so the core's
// existing env path is unchanged (no [vector] → BruteForce byte-equivalent, ADR-004). Config-load
// errors are non-fatal — the [vector] file source is opt-in. Returns a restore closure.
func setVectorEnv(dataDir string) func() {
	var restores []func()
	restore := func() {
		for _, r := range restores {
			r()
		}
	}
	if dataDir == "" {
		return restore
	}
	cfg, err := config.Load(dataDir)
	if err != nil {
		return restore // best-effort: missing/malformed config → env-only path unchanged
	}
	setIfAbsent := func(key, val string) {
		if val == "" {
			return
		}
		if _, had := os.LookupEnv(key); had {
			return // env wins: an explicit env var overrides the config file
		}
		if os.Setenv(key, val) == nil {
			restores = append(restores, func() { _ = os.Unsetenv(key) })
		}
	}
	setIfAbsent("CONTEXTFORGE_VECTOR_BACKEND", cfg.Vector.Backend)
	if cfg.Vector.Dim != 0 {
		setIfAbsent("CONTEXTFORGE_VECTOR_DIM", strconv.Itoa(cfg.Vector.Dim))
	}
	return restore
}
