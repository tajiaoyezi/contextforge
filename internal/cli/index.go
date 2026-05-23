package cli

import (
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"

	"github.com/tajiaoyezi/contextforge/internal/config"
	"github.com/tajiaoyezi/contextforge/internal/reliability"
)

type indexOpts struct {
	Source       string
	DataDir      string
	Collection   string
	Resume       bool
	ChangedItems int64
}

func runIndex(args []string, stdout, stderr io.Writer) int {
	opts, err := parseIndexOpts(args, stderr)
	if err != nil {
		return 2
	}
	if opts.Source == "" {
		fmt.Fprintln(stderr, "contextforge index: --source is required")
		return 2
	}
	if opts.ChangedItems < 0 {
		fmt.Fprintln(stderr, "contextforge index: --changed-items must be >=0")
		return 2
	}

	manifestPath := indexManifestPath(opts.DataDir, opts.Collection)
	manifest, resumed, err := reliability.StartOrResumeManifest(manifestPath, reliability.ManifestOptions{
		SourcePath: opts.Source,
		DataDir:    opts.DataDir,
		Collection: opts.Collection,
		TotalItems: opts.ChangedItems,
	})
	if err != nil {
		fmt.Fprintf(stderr, "contextforge index: resume manifest: %v\n", err)
		return 1
	}
	mode := "long-task mode"
	if resumed {
		mode = "resuming long-task mode"
	}
	if !opts.Resume {
		mode = "safe rebuild mode"
	}
	fmt.Fprintf(stdout, "contextforge index: %s collection=%s processed=%d total=%d\n",
		mode, manifest.Collection, manifest.ProcessedItems, manifest.TotalItems)
	fmt.Fprintf(stdout, "resume_manifest=%s\n", manifestPath)
	return 0
}

func parseIndexOpts(args []string, stderr io.Writer) (*indexOpts, error) {
	fs := flag.NewFlagSet("index", flag.ContinueOnError)
	fs.SetOutput(stderr)
	source := fs.String("source", "", "source root to index")
	dataDir := fs.String("data-dir", "", "data root (default ~/.contextforge)")
	collection := fs.String("collection", "default", "collection ID")
	resume := fs.Bool("resume", false, "resume from an incomplete long-task manifest when available")
	changedItems := fs.Int64("changed-items", 0, "estimated changed item count for long-task mode")
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
	if _, err := os.Stat(*source); *source != "" && err != nil {
		return nil, fmt.Errorf("source %q: %w", *source, err)
	}
	return &indexOpts{
		Source:       *source,
		DataDir:      *dataDir,
		Collection:   *collection,
		Resume:       *resume,
		ChangedItems: *changedItems,
	}, nil
}

func indexManifestPath(dataDir, collection string) string {
	if collection == "" {
		collection = "default"
	}
	return filepath.Join(dataDir, "runtime", "index-"+collection+".resume.json")
}
