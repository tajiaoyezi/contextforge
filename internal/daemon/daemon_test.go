package daemon

import (
	"context"
	"fmt"
	"io"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/cli"
)

// coreBin is the contextforge-core binary built once by TestMain (task-1.4
// §5.2: cargo build -p contextforge-core, drives the real Go↔Rust gRPC path).
var coreBin string

func TestMain(m *testing.M) {
	root, err := repoRoot()
	if err != nil {
		fmt.Fprintln(os.Stderr, "task-1.4 daemon test: locate repo root:", err)
		os.Exit(1)
	}
	build := exec.Command("cargo", "build", "-p", "contextforge-core")
	build.Dir = root
	build.Stdout = os.Stderr
	build.Stderr = os.Stderr
	if err := build.Run(); err != nil {
		fmt.Fprintln(os.Stderr, "task-1.4 daemon test: cargo build -p contextforge-core:", err)
		os.Exit(1)
	}
	coreBin = filepath.Join(root, "target", "debug", coreBinName())
	if _, err := os.Stat(coreBin); err != nil {
		fmt.Fprintln(os.Stderr, "task-1.4 daemon test: core binary missing:", err)
		os.Exit(1)
	}
	os.Exit(m.Run())
}

// repoRoot is the package-internal walk-up helper defined in daemon.go (same
// package); reused here by TestMain to locate the cargo workspace root.

func freeAddr(t *testing.T) string {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("reserve free port: %v", err)
	}
	defer l.Close()
	return l.Addr().String()
}

func pollHealth(t *testing.T, d *Daemon, within time.Duration) string {
	t.Helper()
	deadline := time.Now().Add(within)
	var last string
	for time.Now().Before(deadline) {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		s, err := d.HealthCheck(ctx)
		cancel()
		if err == nil && s == "SERVING" {
			return s
		}
		last = s
		time.Sleep(200 * time.Millisecond)
	}
	return last
}

// TEST-1.4.2
// SCEN-1.4.2 / AC2: daemon 拉起 contextforge-core，经 local gRPC Health 返回 SERVING。
func TestTask14_AC2_StartCoreAndHealthSERVING(t *testing.T) {
	addr := freeAddr(t)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	d, err := Start(ctx, Options{CoreBinPath: coreBin, ListenAddr: addr})
	if err != nil {
		t.Fatalf("Start: %v", err)
	}
	defer d.Stop()

	if s := pollHealth(t, d, 15*time.Second); s != "SERVING" {
		t.Fatalf("HealthCheck = %q, want SERVING", s)
	}
}

// TEST-1.4.3
// SCEN-1.4.3 / AC3: core 子进程被杀后 AutoRestart 使 Restarts()>=1 且 health 恢复 SERVING。
func TestTask14_AC3_AutoRestartAfterCrash(t *testing.T) {
	addr := freeAddr(t)
	ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
	defer cancel()

	d, err := Start(ctx, Options{CoreBinPath: coreBin, ListenAddr: addr, AutoRestart: true})
	if err != nil {
		t.Fatalf("Start: %v", err)
	}
	defer d.Stop()

	if s := pollHealth(t, d, 15*time.Second); s != "SERVING" {
		t.Fatalf("pre-kill HealthCheck = %q, want SERVING", s)
	}

	pid := d.currentPID()
	if pid <= 0 {
		t.Fatalf("currentPID = %d, want > 0", pid)
	}
	// Cross-platform kill: os.FindProcess+Kill works on Unix (SIGKILL) and
	// Windows (TerminateProcess). syscall.Kill is Unix-only. The supervisor
	// may reap+restart between currentPID() and Kill(); a stale PID is benign
	// — the process is already gone, which is exactly the crash we want, and
	// the auto-restart assertions below still validate AC3.
	if p, err := os.FindProcess(pid); err == nil {
		_ = p.Kill()
	}

	deadline := time.Now().Add(30 * time.Second)
	for time.Now().Before(deadline) {
		if d.Restarts() >= 1 {
			break
		}
		time.Sleep(200 * time.Millisecond)
	}
	if d.Restarts() < 1 {
		t.Fatalf("Restarts() = %d after crash, want >= 1", d.Restarts())
	}
	if s := pollHealth(t, d, 20*time.Second); s != "SERVING" {
		t.Fatalf("post-restart HealthCheck = %q, want SERVING", s)
	}
}

// TEST-1.4.5
// SCEN-1.4.5 / AC5: 端到端 init → core 拉起 → gRPC health SERVING（phase-1 §6 落点）。
func TestTask14_AC5_EndToEndSmoke(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	// See cli_test.go TestTask14_AC1: Windows os.UserHomeDir needs USERPROFILE.
	t.Setenv("USERPROFILE", home)

	if code := cli.Execute([]string{"init"}, io.Discard, io.Discard); code != 0 {
		t.Fatalf("contextforge init exit = %d, want 0", code)
	}
	if _, err := os.Stat(filepath.Join(home, ".contextforge", "config.toml")); err != nil {
		t.Fatalf("init did not create ~/.contextforge/config.toml: %v", err)
	}

	addr := freeAddr(t)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	d, err := Start(ctx, Options{CoreBinPath: coreBin, ListenAddr: addr})
	if err != nil {
		t.Fatalf("Start: %v", err)
	}
	defer d.Stop()

	if s := pollHealth(t, d, 15*time.Second); s != "SERVING" {
		t.Fatalf("end-to-end HealthCheck = %q, want SERVING", s)
	}
}
