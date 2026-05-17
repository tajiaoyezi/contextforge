// Package daemon supervises the contextforge-core (Rust data-plane) child
// process and health-checks it over local gRPC (ContextService.Health).
// task-1.4 (Phase 1 foundation). Contract: task-1.4 §5.3.
//
// Scope: launch core (AC2), basic auto-restart + health check on crash (AC3).
// Out of scope (task-1.4 §3): production process supervision hardening (Phase
// 8), Unix-domain-socket transport (task-1.3 deferred), TLS/auth/token.
package daemon

import "context"

// Options configures the supervised contextforge-core child.
type Options struct {
	// CoreBinPath is the contextforge-core binary path (default: exec.LookPath
	// "contextforge-core", falling back to the conventional target path).
	CoreBinPath string
	// ListenAddr is the safe local address passed to core; default
	// "127.0.0.1:<port>", never 0.0.0.0 (aligns task-1.3 resolve_listen_addr).
	ListenAddr string
	// AutoRestart enables the basic crash auto-restart supervisor (AC3).
	AutoRestart bool
}

// Daemon owns the contextforge-core child process lifecycle.
type Daemon struct {
	opts     Options
	restarts int
}

// task-1.4 RED skeleton (§2.5.1 compilable bridge): unimplemented bodies panic
// so the AC tests fail on behaviour, not on a compile error. GREEN replaces.

// Start launches the contextforge-core child (AC2); when opts.AutoRestart it
// also starts the restart-supervisor goroutine (AC3).
func Start(ctx context.Context, opts Options) (*Daemon, error) {
	panic("unimplemented: daemon.Start (task-1.4 RED skeleton)")
}

// HealthCheck probes core via local gRPC ContextService.Health and returns the
// status string (expected "SERVING") (AC2).
func (d *Daemon) HealthCheck(ctx context.Context) (string, error) {
	panic("unimplemented: daemon.HealthCheck (task-1.4 RED skeleton)")
}

// Restarts returns the cumulative auto-restart count (AC3).
func (d *Daemon) Restarts() int {
	panic("unimplemented: daemon.Restarts (task-1.4 RED skeleton)")
}

// Stop terminates the core child and stops the supervisor (idempotent).
func (d *Daemon) Stop() error {
	panic("unimplemented: daemon.Stop (task-1.4 RED skeleton)")
}

// currentPID returns the live core child PID. Unexported test-support accessor
// for AC3 (killing the child to assert auto-restart); not part of the public
// §5.3 contract.
func (d *Daemon) currentPID() int {
	panic("unimplemented: daemon.currentPID (task-1.4 RED skeleton)")
}
