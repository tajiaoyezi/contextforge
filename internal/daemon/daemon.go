// Package daemon supervises the contextforge-core (Rust data-plane) child
// process and health-checks it over local gRPC (ContextService.Health).
// task-1.4 (Phase 1 foundation). Contract: task-1.4 §5.3.
//
// Scope: launch core (AC2), basic auto-restart + health check on crash (AC3).
// Out of scope (task-1.4 §3): production process supervision hardening (Phase
// 8), Unix-domain-socket transport (task-1.3 deferred), TLS/auth/token.
package daemon

import (
	"context"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"sync"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// restartBackoff is the basic delay before respawning a crashed core (AC3 is
// the basic version; production backoff/jitter is Phase 8, out of scope).
const restartBackoff = 200 * time.Millisecond

// Options configures the supervised contextforge-core child.
type Options struct {
	// CoreBinPath is the contextforge-core binary path. Empty → exec.LookPath
	// "contextforge-core", falling back to the conventional cargo target path.
	CoreBinPath string
	// ListenAddr is the safe local address passed to core; empty → an
	// OS-assigned 127.0.0.1 port. Never 0.0.0.0/:: (aligns task-1.3
	// resolve_listen_addr; rejected early for defense in depth).
	ListenAddr string
	// AutoRestart enables the basic crash auto-restart supervisor (AC3).
	AutoRestart bool
}

// Daemon owns the contextforge-core child process lifecycle.
type Daemon struct {
	opts   Options
	mu     sync.Mutex
	cmd    *exec.Cmd
	conn   *grpc.ClientConn // reused gRPC client conn (lazy; closed by Stop)
	closed bool
	starts int // launches after the first (= auto-restart count)
	doneCh chan struct{}
}

// Start launches the contextforge-core child (AC2). The supervisor goroutine
// is the sole owner of cmd.Wait(); with opts.AutoRestart it respawns the child
// after an unexpected exit (AC3).
func Start(ctx context.Context, opts Options) (*Daemon, error) {
	bin, err := resolveCoreBin(opts.CoreBinPath)
	if err != nil {
		return nil, err
	}
	opts.CoreBinPath = bin

	if opts.ListenAddr == "" {
		a, ferr := freeLoopbackAddr()
		if ferr != nil {
			return nil, fmt.Errorf("daemon: reserve loopback port: %w", ferr)
		}
		opts.ListenAddr = a
	}
	if err := ensureLoopback(opts.ListenAddr); err != nil {
		return nil, err
	}

	d := &Daemon{opts: opts, doneCh: make(chan struct{})}
	cmd, err := d.launch()
	if err != nil {
		return nil, err
	}
	d.cmd = cmd
	go d.supervise()
	return d, nil
}

// HealthCheck probes core via local gRPC ContextService.Health and returns the
// status string (expected "SERVING") (AC2). Loopback plaintext is allowed by
// the v0.1 local-service security baseline (TLS/auth is Phase 6).
func (d *Daemon) HealthCheck(ctx context.Context) (string, error) {
	conn, err := d.clientConn()
	if err != nil {
		return "", err
	}
	resp, err := contextforgev1.NewContextServiceClient(conn).
		Health(ctx, &contextforgev1.HealthRequest{})
	if err != nil {
		return "", err
	}
	return resp.GetStatus(), nil
}

// clientConn lazily creates and reuses a single gRPC client conn to core.
// gRPC transparently reconnects to the same target across core auto-restarts,
// so one conn is correct and avoids a new conn per health poll.
func (d *Daemon) clientConn() (*grpc.ClientConn, error) {
	d.mu.Lock()
	defer d.mu.Unlock()
	if d.closed {
		return nil, fmt.Errorf("daemon: stopped")
	}
	if d.conn != nil {
		return d.conn, nil
	}
	conn, err := grpc.NewClient(d.opts.ListenAddr,
		grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, fmt.Errorf("daemon: gRPC client %s: %w", d.opts.ListenAddr, err)
	}
	d.conn = conn
	return conn, nil
}

// Restarts returns the cumulative auto-restart count (AC3).
func (d *Daemon) Restarts() int {
	d.mu.Lock()
	defer d.mu.Unlock()
	return d.starts
}

// Stop terminates the core child and stops the supervisor (idempotent).
func (d *Daemon) Stop() error {
	d.mu.Lock()
	if d.closed {
		d.mu.Unlock()
		return nil
	}
	d.closed = true
	cmd := d.cmd
	conn := d.conn
	d.mu.Unlock()

	if conn != nil {
		_ = conn.Close()
	}
	if cmd != nil && cmd.Process != nil {
		_ = cmd.Process.Kill()
	}
	<-d.doneCh // supervisor reaps the child (cmd.Wait) and exits
	return nil
}

// currentPID returns the live core child PID, or -1. Unexported test-support
// accessor for AC3 (kill the child to assert auto-restart); not part of the
// public §5.3 contract.
func (d *Daemon) currentPID() int {
	d.mu.Lock()
	defer d.mu.Unlock()
	if d.cmd != nil && d.cmd.Process != nil {
		return d.cmd.Process.Pid
	}
	return -1
}

// supervise is the sole owner of cmd.Wait(): it reaps the child and, when
// AutoRestart is set and the exit was not requested via Stop, respawns it.
func (d *Daemon) supervise() {
	defer close(d.doneCh)
	for {
		d.mu.Lock()
		cmd := d.cmd
		d.mu.Unlock()

		_ = cmd.Wait() // blocks until core exits (crash or Stop-kill)

		d.mu.Lock()
		if d.closed || !d.opts.AutoRestart {
			d.mu.Unlock()
			return
		}
		d.mu.Unlock()

		time.Sleep(restartBackoff)

		next, err := d.launch()
		if err != nil {
			return // give up restarting on launch failure (basic version)
		}
		d.mu.Lock()
		if d.closed { // Stop raced in during relaunch
			d.mu.Unlock()
			_ = next.Process.Kill()
			_ = next.Wait()
			return
		}
		d.cmd = next
		d.starts++
		d.mu.Unlock()
	}
}

func (d *Daemon) launch() (*exec.Cmd, error) {
	cmd := exec.Command(d.opts.CoreBinPath, d.opts.ListenAddr)
	cmd.Stdout = os.Stderr
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		return nil, fmt.Errorf("daemon: launch core %q: %w", d.opts.CoreBinPath, err)
	}
	return cmd, nil
}

// resolveCoreBin locates the contextforge-core binary.
func resolveCoreBin(p string) (string, error) {
	if p != "" {
		if _, err := os.Stat(p); err != nil {
			return "", fmt.Errorf("daemon: core binary %q: %w", p, err)
		}
		return p, nil
	}
	if lp, err := exec.LookPath("contextforge-core"); err == nil {
		return lp, nil
	}
	if root, err := repoRoot(); err == nil {
		cand := filepath.Join(root, "target", "debug", "contextforge-core")
		if _, err := os.Stat(cand); err == nil {
			return cand, nil
		}
	}
	return "", fmt.Errorf("daemon: contextforge-core not found (set Options.CoreBinPath)")
}

func repoRoot() (string, error) {
	d, err := os.Getwd()
	if err != nil {
		return "", err
	}
	for {
		if _, err := os.Stat(filepath.Join(d, "go.mod")); err == nil {
			return d, nil
		}
		parent := filepath.Dir(d)
		if parent == d {
			return "", fmt.Errorf("go.mod not found walking up from cwd")
		}
		d = parent
	}
}

func freeLoopbackAddr() (string, error) {
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return "", err
	}
	defer l.Close()
	return l.Addr().String(), nil
}

// ensureLoopback rejects a wildcard / 0.0.0.0 / :: bind early (defense in
// depth; task-1.3 resolve_listen_addr also rejects it in core).
func ensureLoopback(addr string) error {
	host, _, err := net.SplitHostPort(addr)
	if err != nil {
		return fmt.Errorf("daemon: invalid ListenAddr %q: %w", addr, err)
	}
	ip := net.ParseIP(host)
	if ip == nil {
		return fmt.Errorf("daemon: ListenAddr host %q is not an IP "+
			"(use 127.0.0.1 / ::1)", host)
	}
	if ip.IsUnspecified() {
		return fmt.Errorf("daemon: refusing wildcard ListenAddr %q "+
			"(0.0.0.0/:: forbidden; use 127.0.0.1 / ::1)", addr)
	}
	return nil
}
