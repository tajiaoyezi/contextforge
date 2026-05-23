// Package cli (sub-file serve.go) — task-6.2 `contextforge serve` subcommand.
//
// Contract: task-6.2 §5.3. Parses flags + loads/generates the local token
// + delegates to the injected ServeBackend (production: long-running
// daemon + REST server, lives in `cmd/contextforge/main.go`). Like
// task-6.1's SearchBackend pattern, `internal/cli` deliberately does NOT
// import `internal/daemon` — that would resurrect the test-time import
// cycle with `daemon_test.go` (which imports `cli` for the task-1.4 §6
// end-to-end smoke). All daemon-coupled work lives behind ServeBackend.
package cli

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"flag"
	"fmt"
	"io"
	"net"
	"os"
	"path/filepath"
	"strings"

	"github.com/tajiaoyezi/contextforge/internal/config"
)

// ServeOpts is the parsed `contextforge serve` invocation state. `Token`
// and `TokenPath` are filled in by `runServe` before the backend is
// invoked (so the backend stays daemon-focused and never touches
// crypto/rand or the data root).
type ServeOpts struct {
	Addr      string // --addr <host:port>; empty → auto-select free loopback port
	Unix      string // --unix <path>; mutex with --addr
	DataDir   string // --data-dir; empty → config.DefaultRootDir()
	Token     string // filled by runServe via loadOrGenerateToken
	TokenPath string // absolute path of the on-disk token file
}

// ServeBackend is the production-injected long-running serve callable.
// `cmd/contextforge/main.go` wires the real daemon-spawning implementation;
// tests don't usually exercise the production path (they unit-test the
// pre-backend pieces: parseServeOpts, loadOrGenerateToken).
type ServeBackend func(ctx context.Context, opts *ServeOpts, stdout, stderr io.Writer) error

var serveBackend ServeBackend

// SetServeBackend wires the production serve backend. Called once from
// `cmd/contextforge/main.go` before Execute. Panics on nil so wiring
// mistakes are caught at startup, not on first `contextforge serve` use.
func SetServeBackend(b ServeBackend) {
	if b == nil {
		panic("cli.SetServeBackend: nil backend")
	}
	serveBackend = b
}

// runServe dispatches `contextforge serve`: validate flags, resolve the
// data root, load (or generate) the token, then delegate to the injected
// backend. Returns the process exit code (0=ok / 1=runtime / 2=usage).
func runServe(args []string, stdout, stderr io.Writer) int {
	if serveBackend == nil {
		fmt.Fprintln(stderr,
			"contextforge serve: serve backend not wired "+
				"(cmd/contextforge/main.go must call cli.SetServeBackend)")
		return 1
	}

	opts, err := parseServeOpts(args, stderr)
	if err != nil {
		return 2
	}

	if opts.DataDir == "" {
		d, derr := config.DefaultRootDir()
		if derr != nil {
			fmt.Fprintf(stderr, "contextforge serve: resolve data dir: %v\n", derr)
			return 1
		}
		opts.DataDir = d
	}

	token, tokenPath, terr := loadOrGenerateToken(opts.DataDir)
	if terr != nil {
		fmt.Fprintf(stderr, "contextforge serve: token: %v\n", terr)
		return 1
	}
	opts.Token = token
	opts.TokenPath = tokenPath

	if err := serveBackend(context.Background(), opts, stdout, stderr); err != nil {
		fmt.Fprintf(stderr, "contextforge serve: %v\n", err)
		return 1
	}
	return 0
}

// parseServeOpts parses the serve subcommand flags. AC3 (defense in depth):
// refuse wildcard / non-loopback `--addr` early, before any listener bind.
// `--addr` and `--unix` are mutually exclusive.
func parseServeOpts(args []string, stderr io.Writer) (*ServeOpts, error) {
	fs := flag.NewFlagSet("serve", flag.ContinueOnError)
	fs.SetOutput(stderr)
	var (
		addr    = fs.String("addr", "", "TCP listen address (loopback only; default = auto-select free 127.0.0.1 port)")
		unix    = fs.String("unix", "", "Unix socket path (mutex with --addr; Windows falls back to loopback TCP)")
		dataDir = fs.String("data-dir", "", "data root (default = config.DefaultRootDir)")
	)
	if err := fs.Parse(args); err != nil {
		return nil, err
	}
	if *addr != "" && *unix != "" {
		fmt.Fprintln(stderr, "contextforge serve: --addr and --unix are mutually exclusive")
		return nil, errors.New("addr/unix mutex")
	}
	if *addr != "" {
		if err := validateLoopbackAddr(*addr, stderr); err != nil {
			return nil, err
		}
	}
	return &ServeOpts{
		Addr:    *addr,
		Unix:    *unix,
		DataDir: *dataDir,
	}, nil
}

// validateLoopbackAddr rejects wildcard (0.0.0.0 / ::), non-loopback IPs,
// and non-IP hosts (defense in depth — daemon-side ensureLoopback also
// rejects these for the gRPC listener; same baseline as task-1.4 §AC1).
func validateLoopbackAddr(addr string, stderr io.Writer) error {
	host, _, err := net.SplitHostPort(addr)
	if err != nil {
		fmt.Fprintf(stderr, "contextforge serve: invalid --addr %q: %v\n", addr, err)
		return err
	}
	ip := net.ParseIP(host)
	if ip == nil {
		fmt.Fprintf(stderr,
			"contextforge serve: --addr host %q is not an IP literal "+
				"(use 127.0.0.1 / [::1])\n", host)
		return errors.New("non-IP host")
	}
	if ip.IsUnspecified() {
		fmt.Fprintf(stderr,
			"contextforge serve: refusing wildcard bind %q "+
				"(0.0.0.0/:: forbidden; loopback only — use 127.0.0.1 / [::1])\n", addr)
		return errors.New("wildcard bind")
	}
	if !ip.IsLoopback() {
		fmt.Fprintf(stderr,
			"contextforge serve: --addr %q is not loopback "+
				"(use 127.0.0.1 / [::1])\n", addr)
		return errors.New("non-loopback")
	}
	return nil
}

// loadOrGenerateToken reads `<dataDir>/token`; if absent or malformed,
// generates 32 random bytes via crypto/rand, hex-encodes, writes the
// file with 0600 (config.FileMode), and returns the token + absolute
// path. AC4 §2A 决策 D — soft-random generated on first run, reused on
// subsequent starts.
func loadOrGenerateToken(dataDir string) (string, string, error) {
	if dataDir == "" {
		return "", "", errors.New("loadOrGenerateToken: empty dataDir")
	}
	if err := os.MkdirAll(dataDir, config.DirMode); err != nil {
		return "", "", fmt.Errorf("token dir %q: %w", dataDir, err)
	}
	tokenPath := filepath.Join(dataDir, "token")

	if existing, err := os.ReadFile(tokenPath); err == nil {
		token := strings.TrimSpace(string(existing))
		// 32-byte hex == 64 chars; reuse iff well-formed
		if len(token) == 64 {
			if _, derr := hex.DecodeString(token); derr == nil {
				return token, tokenPath, nil
			}
		}
		// Existing token corrupt — regenerate
	}

	raw := make([]byte, 32)
	if _, err := rand.Read(raw); err != nil {
		return "", "", fmt.Errorf("crypto/rand: %w", err)
	}
	token := hex.EncodeToString(raw)
	if err := os.WriteFile(tokenPath, []byte(token), config.FileMode); err != nil {
		return "", "", fmt.Errorf("write token %q: %w", tokenPath, err)
	}
	// MkdirAll honours umask, and WriteFile does not always force the bits — chmod explicitly.
	if err := os.Chmod(tokenPath, config.FileMode); err != nil {
		return "", "", fmt.Errorf("chmod token %q: %w", tokenPath, err)
	}
	return token, tokenPath, nil
}
