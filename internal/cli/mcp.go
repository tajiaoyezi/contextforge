package cli

import (
	"context"
	"flag"
	"fmt"
	"io"
	"os"
	"os/signal"
	"path/filepath"
	"syscall"

	"github.com/tajiaoyezi/contextforge/internal/config"
)

// MCPOpts is the parsed `contextforge mcp` invocation state.
type MCPOpts struct {
	DataDir   string
	Allowlist string
}

// MCPBackend is the production-injected MCP server callable. The real backend
// lives in cmd/contextforge/main.go so internal/cli does not import daemon or
// mcpadapter directly.
type MCPBackend func(ctx context.Context, opts MCPOpts, stdin io.Reader, stdout, stderr io.Writer) error

var mcpBackend MCPBackend

// SetMCPBackend wires the production MCP backend. Called by main at startup.
func SetMCPBackend(b MCPBackend) {
	if b == nil {
		panic("cli.SetMCPBackend: nil backend")
	}
	mcpBackend = b
}

// runMCP implements `contextforge mcp`.
func runMCP(args []string, stdin io.Reader, stdout, stderr io.Writer) int {
	if mcpBackend == nil {
		fmt.Fprintln(stderr,
			"contextforge mcp: mcp backend not wired "+
				"(cmd/contextforge/main.go must call cli.SetMCPBackend)")
		return 1
	}
	opts, err := parseMCPOpts(args, stderr)
	if err != nil {
		return 2
	}
	if stdin == nil {
		stdin = os.Stdin
	}
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	if err := mcpBackend(ctx, *opts, stdin, stdout, stderr); err != nil {
		fmt.Fprintf(stderr, "contextforge mcp: %v\n", err)
		return 1
	}
	return 0
}

func parseMCPOpts(args []string, stderr io.Writer) (*MCPOpts, error) {
	fs := flag.NewFlagSet("mcp", flag.ContinueOnError)
	fs.SetOutput(stderr)
	dataDir := fs.String("data-dir", "", "data root (default ~/.contextforge)")
	allowlist := fs.String("allowlist", "", "allowlist file (default <data-dir>/mcp-allowlist.json)")
	if err := fs.Parse(args); err != nil {
		return nil, err
	}
	if *dataDir == "" {
		root, err := config.DefaultRootDir()
		if err != nil {
			return nil, err
		}
		*dataDir = root
	}
	if *allowlist == "" {
		*allowlist = filepath.Join(*dataDir, "mcp-allowlist.json")
	}
	return &MCPOpts{DataDir: *dataDir, Allowlist: *allowlist}, nil
}
