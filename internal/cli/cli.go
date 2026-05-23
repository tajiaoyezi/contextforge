// Package cli is the contextforge control-plane CLI entrypoint: a stdlib
// flag-based subcommand dispatcher (init/import/index/search/serve/mcp/eval/
// export). Unimplemented subcommands return an explicit not-implemented error
// (never panic). task-1.4 (Phase 1 foundation). Contract: task-1.4 §5.3.
//
// §2A decision: v0.1 uses the standard library (no third-party CLI framework)
// — zero new dependency, avoids R7 / a go.mod conflict with the parallel
// task-3.1; cobra (PRD §Technical Approach / D8) is deferred to a later dep PR.
package cli

import (
	"flag"
	"fmt"
	"io"
	"os"

	"github.com/tajiaoyezi/contextforge/internal/config"
)

// subcommands is the registered subcommand set in stable order (AC4). Only
// "init" is implemented in task-1.4; the rest are Phase 2+/6/7/8 and return an
// explicit not-implemented message (task-1.4 §3 Out-of-Scope).
var subcommands = []string{"init", "import", "index", "search", "serve", "mcp", "eval", "export"}

// SubcommandNames returns a copy of the registered subcommand names (AC4).
func SubcommandNames() []string {
	out := make([]string, len(subcommands))
	copy(out, subcommands)
	return out
}

func known(sub string) bool {
	for _, s := range subcommands {
		if s == sub {
			return true
		}
	}
	return false
}

// Execute parses args, dispatches the subcommand and returns the process exit
// code. Unknown / unimplemented subcommands write to stderr and return a
// non-zero code — never panic (AC4).
func Execute(args []string, stdout, stderr io.Writer) int {
	return ExecuteWithIO(args, os.Stdin, stdout, stderr)
}

// ExecuteWithIO parses args and dispatches the subcommand with explicit stdin.
// It exists for stdio-native subcommands like `mcp`; Execute preserves the
// original task-1.4 public API for existing callers.
func ExecuteWithIO(args []string, stdin io.Reader, stdout, stderr io.Writer) int {
	if len(args) == 0 {
		fmt.Fprintf(stderr, "contextforge: expected a subcommand, one of %v\n", subcommands)
		return 2
	}
	sub, rest := args[0], args[1:]

	switch sub {
	case "init":
		fs := flag.NewFlagSet("init", flag.ContinueOnError)
		fs.SetOutput(stderr)
		root := fs.String("root", "", "data root (default ~/.contextforge)")
		if err := fs.Parse(rest); err != nil {
			return 2
		}
		if err := runInit(*root, stdout); err != nil {
			fmt.Fprintf(stderr, "contextforge init: %v\n", err)
			return 1
		}
		return 0

	case "search":
		// task-6.1: real subcommand entry; supersedes the task-1.4
		// "not implemented" default-arm response for `search`.
		return runSearch(rest, stdout, stderr)

	case "index":
		// task-8.2: long-task/resume entrypoint. The production data-plane
		// indexing bridge remains Rust-side; this Go entrypoint owns the
		// resumable control-plane manifest.
		return runIndex(rest, stdout, stderr)

	case "serve":
		// task-6.2: real subcommand entry; supersedes the task-1.4
		// "not implemented" default-arm response for `serve`.
		// Backend wired by cmd/contextforge/main.go.
		return runServe(rest, stdout, stderr)

	case "export":
		// task-6.3: real subcommand entry; supersedes the task-1.4
		// "not implemented" default-arm response for `export`.
		return runExport(rest, stdout, stderr)

	case "mcp":
		// task-7.1: stdio JSON-RPC MCP server entry. Backend wired by
		// cmd/contextforge/main.go to avoid cli -> daemon import cycles.
		return runMCP(rest, stdin, stdout, stderr)

	case "eval":
		// task-8.1: recall eval harness entry. Reuses the search backend
		// injection set by cmd/contextforge/main.go.
		return runEval(rest, stdout, stderr)

	default:
		if known(sub) {
			fmt.Fprintf(stderr, "contextforge %s: not implemented "+
				"(Phase 2+/6/7/8; task-1.4 registers the skeleton only)\n", sub)
			return 2
		}
		fmt.Fprintf(stderr, "contextforge: unknown subcommand %q; want one of %v\n", sub, subcommands)
		return 2
	}
}

// runInit orchestrates config.Init to generate the default config + data-dir
// scaffold; root=="" resolves to config.DefaultRootDir(); idempotent because
// config.Init does not overwrite an existing config.toml (AC1).
func runInit(root string, stdout io.Writer) error {
	if root == "" {
		r, err := config.DefaultRootDir()
		if err != nil {
			return err
		}
		root = r
	}
	cfg, err := config.Init(root)
	if err != nil {
		return err
	}
	fmt.Fprintf(stdout, "contextforge: initialized %s (schema_version %s)\n",
		cfg.DataDir, cfg.SchemaVersion)
	return nil
}
