package reliability

import (
	"path/filepath"
	"strings"
	"testing"
)

// TEST-8.2.1 / SCEN-8.2.1 / AC1
func TestTask82_AC1_ResumeManifestContinuesIncompleteWork(t *testing.T) {
	path := filepath.Join(t.TempDir(), "index-default.resume.json")
	opts := ManifestOptions{SourcePath: "/repo", DataDir: "/data", Collection: "default", TotalItems: 100}

	first, resumed, err := StartOrResumeManifest(path, opts)
	if err != nil {
		t.Fatalf("StartOrResumeManifest first: %v", err)
	}
	if resumed {
		t.Fatalf("first run should not be resumed")
	}
	if first.ProcessedItems != 0 || first.Completed {
		t.Fatalf("new manifest should start incomplete: %+v", first)
	}
	if err := MarkProgress(path, 42); err != nil {
		t.Fatalf("MarkProgress: %v", err)
	}

	second, resumed, err := StartOrResumeManifest(path, opts)
	if err != nil {
		t.Fatalf("StartOrResumeManifest second: %v", err)
	}
	if !resumed || second.ProcessedItems != 42 || second.Completed {
		t.Fatalf("second run should resume processed=42 incomplete: resumed=%v manifest=%+v", resumed, second)
	}
	if err := MarkComplete(path); err != nil {
		t.Fatalf("MarkComplete: %v", err)
	}
	third, resumed, err := StartOrResumeManifest(path, opts)
	if err != nil {
		t.Fatalf("StartOrResumeManifest third: %v", err)
	}
	if resumed || third.ProcessedItems != 0 || third.Completed {
		t.Fatalf("completed manifest should trigger safe rebuild: resumed=%v manifest=%+v", resumed, third)
	}
}

// TEST-8.2.2 / SCEN-8.2.2 / AC2
func TestTask82_AC2_ResourceBudgetRejectsOverBudgetSamples(t *testing.T) {
	if err := CheckResourceBudget(ResourceSample{DaemonIdleMB: 299, IndexingMB: 2048, SearchExtraMB: 199}); err != nil {
		t.Fatalf("within budget should pass: %v", err)
	}
	err := CheckResourceBudget(ResourceSample{DaemonIdleMB: 301, IndexingMB: 2048, SearchExtraMB: 199})
	if err == nil || !strings.Contains(err.Error(), "daemon idle") {
		t.Fatalf("daemon over-budget should name daemon idle, got %v", err)
	}
	err = CheckResourceBudget(ResourceSample{DaemonIdleMB: 299, IndexingMB: 2049, SearchExtraMB: 199})
	if err == nil || !strings.Contains(err.Error(), "indexing") {
		t.Fatalf("indexing over-budget should name indexing, got %v", err)
	}
	err = CheckResourceBudget(ResourceSample{DaemonIdleMB: 299, IndexingMB: 2048, SearchExtraMB: 201})
	if err == nil || !strings.Contains(err.Error(), "search extra") {
		t.Fatalf("search over-budget should name search extra, got %v", err)
	}
}

// TEST-8.2.3 / SCEN-8.2.3 / AC3
func TestTask82_AC3_SafetyRegressionRequiresRedactionExportAuditSignals(t *testing.T) {
	ok := SafetySignals{
		RedactionRegressionPassed: true,
		ExportSecretScanPassed:    true,
		AuditMetadataOnlyPassed:   true,
	}
	if err := CheckSafetyRegression(ok); err != nil {
		t.Fatalf("all safety signals should pass: %v", err)
	}
	for name, signals := range map[string]SafetySignals{
		"redaction": {ExportSecretScanPassed: true, AuditMetadataOnlyPassed: true},
		"export":    {RedactionRegressionPassed: true, AuditMetadataOnlyPassed: true},
		"audit":     {RedactionRegressionPassed: true, ExportSecretScanPassed: true},
	} {
		err := CheckSafetyRegression(signals)
		if err == nil || !strings.Contains(err.Error(), name) {
			t.Fatalf("missing %s signal should fail with signal name, got %v", name, err)
		}
	}
}
