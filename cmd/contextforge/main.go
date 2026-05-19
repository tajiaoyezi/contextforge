// Command contextforge is the Go control-plane binary (task-1.4, Phase 1).
// It delegates to the internal/cli stdlib subcommand dispatcher.
package main

import (
	"os"

	"github.com/tajiaoyezi/contextforge/internal/cli"
)

func main() {
	os.Exit(cli.Execute(os.Args[1:], os.Stdout, os.Stderr))
}
