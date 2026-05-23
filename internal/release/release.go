// Package release contains v0.1 release contract checks.
package release

import (
	"archive/tar"
	"compress/gzip"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

const (
	StepUnpack      = "unpack"
	StepInit        = "init"
	StepImport      = "import"
	StepIndex       = "index"
	StepSearch      = "search"
	StepMCP         = "mcp"
	StepExport      = "export"
	StepEval        = "eval"
	StepExplain     = "explain"
	StepReliability = "reliability"

	StepPassed = "passed"
	StepFailed = "failed"
)

var requiredTarballEntries = []string{
	"contextforge",
	"contextforge-core",
	"contextforge.example.toml",
	"README.md",
	"LICENSE",
}

var requiredSmokeSteps = []string{
	StepUnpack,
	StepInit,
	StepImport,
	StepIndex,
	StepSearch,
	StepMCP,
	StepExport,
	StepEval,
}

var requiredClosureSteps = []string{
	StepImport,
	StepIndex,
	StepSearch,
	StepMCP,
	StepExplain,
	StepEval,
	StepReliability,
}

type TarballReport struct {
	Name    string
	Entries []string
	Modes   map[string]int64
}

type StepResult struct {
	Name     string
	Status   string
	Evidence string
}

type BenchmarkReport struct {
	ChunkCount    int
	BM25P95MS     int
	MetadataP95MS int
	FilterP95MS   int
}

type PhaseSmokeReport struct {
	Tarball   TarballReport
	Smoke     []StepResult
	Closure   []StepResult
	Benchmark BenchmarkReport
}

type Asset struct {
	Name string
	Path string
	Mode int64
}

func RequiredTarballEntries() []string {
	return append([]string(nil), requiredTarballEntries...)
}

func RequiredSteps() []string {
	return append([]string(nil), requiredSmokeSteps...)
}

func BuildTarball(output string, assets []Asset) error {
	if output == "" {
		return errors.New("release: output tarball path is required")
	}
	if err := os.MkdirAll(filepath.Dir(output), 0o755); err != nil {
		return err
	}
	f, err := os.Create(output)
	if err != nil {
		return err
	}
	defer f.Close()
	gz := gzip.NewWriter(f)
	defer gz.Close()
	tw := tar.NewWriter(gz)
	defer tw.Close()

	for _, asset := range assets {
		if strings.TrimSpace(asset.Name) == "" {
			return errors.New("release: asset name is required")
		}
		if strings.Contains(asset.Name, "..") || filepath.IsAbs(asset.Name) {
			return fmt.Errorf("release: unsafe asset name %q", asset.Name)
		}
		body, err := os.ReadFile(asset.Path)
		if err != nil {
			return fmt.Errorf("release: read %s: %w", asset.Path, err)
		}
		mode := asset.Mode
		if mode == 0 {
			mode = 0o644
		}
		if err := tw.WriteHeader(&tar.Header{
			Name:    filepath.ToSlash(asset.Name),
			Mode:    mode,
			Size:    int64(len(body)),
			ModTime: time.Unix(0, 0).UTC(),
		}); err != nil {
			return fmt.Errorf("release: tar header %s: %w", asset.Name, err)
		}
		if _, err := tw.Write(body); err != nil {
			return fmt.Errorf("release: tar body %s: %w", asset.Name, err)
		}
	}
	return nil
}

func ValidateTarball(path string) (*TarballReport, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	gz, err := gzip.NewReader(f)
	if err != nil {
		return nil, err
	}
	defer gz.Close()
	tr := tar.NewReader(gz)

	report := &TarballReport{
		Name:  filepath.Base(path),
		Modes: map[string]int64{},
	}
	for {
		h, err := tr.Next()
		if errors.Is(err, io.EOF) {
			break
		}
		if err != nil {
			return nil, err
		}
		if h.Typeflag != tar.TypeReg && h.Typeflag != tar.TypeRegA {
			continue
		}
		name := filepath.ToSlash(strings.TrimPrefix(h.Name, "./"))
		report.Entries = append(report.Entries, name)
		report.Modes[name] = h.Mode
	}
	sort.Strings(report.Entries)
	if err := validateTarballReport(*report); err != nil {
		return report, err
	}
	return report, nil
}

func ValidateSmokeEvidence(results []StepResult) error {
	pos := 0
	for _, want := range requiredSmokeSteps {
		found := false
		for pos < len(results) {
			step := results[pos]
			pos++
			if step.Name != want {
				continue
			}
			if step.Status != StepPassed {
				return fmt.Errorf("release smoke step %q status=%q", want, step.Status)
			}
			if strings.TrimSpace(step.Evidence) == "" {
				return fmt.Errorf("release smoke step %q missing evidence", want)
			}
			found = true
			break
		}
		if !found {
			return fmt.Errorf("release smoke missing ordered step %q", want)
		}
	}
	return nil
}

func CheckBenchmark(report BenchmarkReport) error {
	if report.ChunkCount < 100000 {
		return fmt.Errorf("benchmark chunk_count=%d below 100000", report.ChunkCount)
	}
	for name, p95 := range map[string]int{
		"bm25":     report.BM25P95MS,
		"metadata": report.MetadataP95MS,
		"filter":   report.FilterP95MS,
	} {
		if p95 <= 0 {
			return fmt.Errorf("%s p95 must be >0ms", name)
		}
		if p95 >= 500 {
			return fmt.Errorf("%s p95=%dms exceeds v0.1 gate <500ms", name, p95)
		}
	}
	return nil
}

func ValidateV01Closure(results []StepResult) error {
	byName := map[string]StepResult{}
	for _, step := range results {
		byName[step.Name] = step
	}
	for _, want := range requiredClosureSteps {
		got, ok := byName[want]
		if !ok {
			return fmt.Errorf("v0.1 closure missing %q evidence", want)
		}
		if got.Status != StepPassed {
			return fmt.Errorf("v0.1 closure %q status=%q", want, got.Status)
		}
		if strings.TrimSpace(got.Evidence) == "" {
			return fmt.Errorf("v0.1 closure %q missing evidence", want)
		}
	}
	return nil
}

func ValidatePhaseSmoke(report PhaseSmokeReport) error {
	if err := validateTarballReport(report.Tarball); err != nil {
		return err
	}
	if err := ValidateSmokeEvidence(report.Smoke); err != nil {
		return err
	}
	if err := ValidateV01Closure(report.Closure); err != nil {
		return err
	}
	return CheckBenchmark(report.Benchmark)
}

func validateTarballReport(report TarballReport) error {
	entries := map[string]bool{}
	for _, entry := range report.Entries {
		entries[entry] = true
	}
	for _, required := range requiredTarballEntries {
		if !entries[required] {
			return fmt.Errorf("release tarball missing %s", required)
		}
	}
	if len(report.Modes) > 0 {
		for _, bin := range []string{"contextforge", "contextforge-core"} {
			mode, ok := report.Modes[bin]
			if !ok {
				return fmt.Errorf("release tarball missing mode for %s", bin)
			}
			if mode&0o111 == 0 {
				return fmt.Errorf("release tarball %s mode %#o is not executable", bin, mode)
			}
		}
	}
	return nil
}
