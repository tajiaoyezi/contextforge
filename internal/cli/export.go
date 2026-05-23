package cli

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/config"
	"github.com/tajiaoyezi/contextforge/internal/exporter"
)

type exportOpts struct {
	Format       string
	Collection   string
	DataDir      string
	Output       string
	IncludeStale bool
}

// runExport implements `contextforge export`.
func runExport(args []string, stdout, stderr io.Writer) int {
	opts, err := parseExportOpts(args, stderr)
	if err != nil {
		return 2
	}
	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	result, err := exporter.Export(ctx, exporter.Options{
		Format:       exporter.Format(opts.Format),
		Collection:   opts.Collection,
		DataDir:      opts.DataDir,
		Output:       opts.Output,
		IncludeStale: opts.IncludeStale,
	})
	if err != nil {
		if errors.Is(err, exporter.ErrSecretHits) {
			fmt.Fprintf(stderr, "contextforge export: secret scan rejected export (%d hits)\n",
				len(result.SecretHits))
			for _, hit := range result.SecretHits {
				fmt.Fprintf(stderr, "  - %s at byte %d: %s\n",
					hit.PatternName, hit.Offset, hit.Match)
			}
			return 3
		}
		fmt.Fprintf(stderr, "contextforge export: %v\n", err)
		return 1
	}

	fmt.Fprintf(stdout, "exported %d records to %s (format=%s fidelity=%.3f)\n",
		result.RecordsExported, result.OutputPath, opts.Format, result.FidelityScore)
	return 0
}

func parseExportOpts(args []string, stderr io.Writer) (*exportOpts, error) {
	fs := flag.NewFlagSet("export", flag.ContinueOnError)
	fs.SetOutput(stderr)
	format := fs.String("format", "", "required: jsonl, markdown-bundle, or agent-draft")
	collection := fs.String("collection", "default", "collection ID")
	dataDir := fs.String("data-dir", "", "data root (default ~/.contextforge)")
	output := fs.String("output", "", "output file path or directory")
	includeStale := fs.Bool("include-stale", false, "include records marked stale")
	if err := fs.Parse(args); err != nil {
		return nil, err
	}
	if *format == "" {
		fmt.Fprintln(stderr, "contextforge export: --format is required")
		return nil, fmt.Errorf("missing --format")
	}
	if *output == "" {
		fmt.Fprintln(stderr, "contextforge export: --output is required")
		return nil, fmt.Errorf("missing --output")
	}
	if *dataDir == "" {
		root, err := config.DefaultRootDir()
		if err != nil {
			return nil, err
		}
		*dataDir = root
	}
	return &exportOpts{
		Format:       *format,
		Collection:   *collection,
		DataDir:      *dataDir,
		Output:       *output,
		IncludeStale: *includeStale,
	}, nil
}
