// Package reliability contains v0.1 release reliability guards.
package reliability

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"
)

const (
	DaemonIdleBudgetMB  = 300
	IndexingBudgetMB    = 2048
	SearchExtraBudgetMB = 200
)

type ManifestOptions struct {
	SourcePath string `json:"source_path"`
	DataDir    string `json:"data_dir"`
	Collection string `json:"collection"`
	TotalItems int64  `json:"total_items"`
}

type Manifest struct {
	ManifestOptions
	ProcessedItems int64     `json:"processed_items"`
	Completed      bool      `json:"completed"`
	UpdatedAt      time.Time `json:"updated_at"`
}

type ResourceSample struct {
	DaemonIdleMB  int64
	IndexingMB    int64
	SearchExtraMB int64
}

type SafetySignals struct {
	RedactionRegressionPassed bool
	ExportSecretScanPassed    bool
	AuditMetadataOnlyPassed   bool
}

func StartOrResumeManifest(path string, opts ManifestOptions) (*Manifest, bool, error) {
	if existing, err := loadManifest(path); err == nil {
		if !existing.Completed && sameManifestScope(existing.ManifestOptions, opts) {
			return existing, true, nil
		}
	} else if !os.IsNotExist(err) {
		return nil, false, err
	}

	next := &Manifest{
		ManifestOptions: opts,
		ProcessedItems:  0,
		Completed:       false,
		UpdatedAt:       time.Now().UTC(),
	}
	if err := writeManifest(path, next); err != nil {
		return nil, false, err
	}
	return next, false, nil
}

func MarkProgress(path string, processed int64) error {
	m, err := loadManifest(path)
	if err != nil {
		return err
	}
	if processed < 0 {
		processed = 0
	}
	if m.TotalItems > 0 && processed > m.TotalItems {
		processed = m.TotalItems
	}
	m.ProcessedItems = processed
	m.UpdatedAt = time.Now().UTC()
	return writeManifest(path, m)
}

func MarkComplete(path string) error {
	m, err := loadManifest(path)
	if err != nil {
		return err
	}
	m.ProcessedItems = m.TotalItems
	m.Completed = true
	m.UpdatedAt = time.Now().UTC()
	return writeManifest(path, m)
}

func CheckResourceBudget(sample ResourceSample) error {
	if sample.DaemonIdleMB > DaemonIdleBudgetMB {
		return fmt.Errorf("daemon idle memory %dMB exceeds %dMB", sample.DaemonIdleMB, DaemonIdleBudgetMB)
	}
	if sample.IndexingMB > IndexingBudgetMB {
		return fmt.Errorf("indexing memory %dMB exceeds %dMB", sample.IndexingMB, IndexingBudgetMB)
	}
	if sample.SearchExtraMB > SearchExtraBudgetMB {
		return fmt.Errorf("search extra memory %dMB exceeds %dMB", sample.SearchExtraMB, SearchExtraBudgetMB)
	}
	return nil
}

func CheckSafetyRegression(signals SafetySignals) error {
	if !signals.RedactionRegressionPassed {
		return fmt.Errorf("redaction regression signal missing")
	}
	if !signals.ExportSecretScanPassed {
		return fmt.Errorf("export secret-scan signal missing")
	}
	if !signals.AuditMetadataOnlyPassed {
		return fmt.Errorf("audit metadata-only signal missing")
	}
	return nil
}

func loadManifest(path string) (*Manifest, error) {
	body, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var m Manifest
	if err := json.Unmarshal(body, &m); err != nil {
		return nil, err
	}
	return &m, nil
}

func writeManifest(path string, m *Manifest) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		return err
	}
	body, err := json.MarshalIndent(m, "", "  ")
	if err != nil {
		return err
	}
	body = append(body, '\n')
	return os.WriteFile(path, body, 0o600)
}

func sameManifestScope(a, b ManifestOptions) bool {
	return a.SourcePath == b.SourcePath &&
		a.DataDir == b.DataDir &&
		a.Collection == b.Collection &&
		a.TotalItems == b.TotalItems
}
