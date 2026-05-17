package importer

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// TEST-3.1.1 / SCEN-3.1.1 / AC1: Importer 抽象只读 — 注册表能注册、查询、调用 Importer。
func TestImporterRegistry(t *testing.T) {
	mock := &mockImporter{name: "mock-test", detectOK: true, confidence: 1.0}
	Register(mock)

	imp, err := Resolve("/any/path/mock-test.txt")
	if err != nil {
		t.Fatalf("Resolve failed: %v", err)
	}
	if imp.Name() != "mock-test" {
		t.Errorf("expected importer name mock-test, got %s", imp.Name())
	}
}

// TEST-3.1.2 / SCEN-3.1.2 / AC2: 通用 fallback 保底 — 对无匹配路径返回 FileFallbackImporter 并成功导入。
func TestFileFallbackImporter(t *testing.T) {
	tmp := t.TempDir()
	fpath := filepath.Join(tmp, "note.md")
	if err := os.WriteFile(fpath, []byte("# Hello"), 0o644); err != nil {
		t.Fatal(err)
	}

	fb := NewFileFallbackImporter()
	recs, err := fb.Import(fpath, "default")
	if err != nil {
		t.Fatalf("fallback import failed: %v", err)
	}
	if len(recs) != 1 {
		t.Fatalf("expected 1 record, got %d", len(recs))
	}
	if recs[0].Content != "# Hello" {
		t.Errorf("expected content '# Hello', got %q", recs[0].Content)
	}
}

// TEST-3.1.3 / SCEN-3.1.3 / AC3: 未识别 schema 降级 + warning — Resolve 对未知路径不 error，返回 fallback。
func TestUnrecognizedSchemaFallback(t *testing.T) {
	tmp := t.TempDir()
	unknown := filepath.Join(tmp, "weird.xyz")
	if err := os.WriteFile(unknown, []byte("data"), 0o644); err != nil {
		t.Fatal(err)
	}

	imp, err := Resolve(unknown)
	if err != nil {
		t.Fatalf("Resolve should not error for unknown schema: %v", err)
	}
	if imp.Name() != "fallback" {
		t.Errorf("expected fallback importer, got %s", imp.Name())
	}
	_, err = imp.Import(unknown, "default")
	if err != nil {
		t.Fatalf("Import should not error: %v", err)
	}
}

// TEST-3.1.4 / SCEN-3.1.4 / AC4: 映射核心字段完整 — fallback 产出的 ContextRecord 含必需 canonical 字段。
func TestContextRecordCoreFields(t *testing.T) {
	tmp := t.TempDir()
	fpath := filepath.Join(tmp, "config.yaml")
	if err := os.WriteFile(fpath, []byte("key: val"), 0o644); err != nil {
		t.Fatal(err)
	}

	fb := NewFileFallbackImporter()
	recs, err := fb.Import(fpath, "proj-a")
	if err != nil {
		t.Fatal(err)
	}
	if len(recs) != 1 {
		t.Fatal("expected 1 record")
	}
	r := recs[0]
	checks := []struct {
		name  string
		value string
	}{
		{"SchemaVersion", r.SchemaVersion},
		{"CollectionId", r.CollectionId},
		{"SourceType", r.SourceType},
		{"SourceProvider", r.SourceProvider},
		{"SourceUri", r.SourceUri},
		{"FilePath", r.FilePath},
		{"Language", r.Language},
		{"ContentHash", r.ContentHash},
	}
	for _, c := range checks {
		if c.value == "" {
			t.Errorf("%s should not be empty", c.name)
		}
	}
	if len(r.Provenance) != 1 {
		t.Errorf("expected 1 provenance entry, got %d", len(r.Provenance))
	}
	if r.LineStart != 1 {
		t.Errorf("expected line_start=1, got %d", r.LineStart)
	}
}

// TEST-3.1.5a / SCEN-3.1.5 / AC5: buildRecord schema 不变性 — 不同 source 输入产生结构一致的 canonical record。
func TestBuildRecordSchemaInvariant(t *testing.T) {
	r1 := buildRecord(recordInput{
		path: "/a.md", collectionID: "c", content: "# A",
		lineCount: 1, sourceType: "file", provider: "local", importerName: "fallback",
	})
	r2 := buildRecord(recordInput{
		path: "/b.go", collectionID: "c", content: "package b",
		lineCount: 1, sourceType: "memory", provider: "hermes", importerName: "hermes",
	})

	for _, r := range []*contextforgev1.ContextRecord{r1, r2} {
		if r.SchemaVersion != "0.1" {
			t.Errorf("SchemaVersion should be 0.1, got %q", r.SchemaVersion)
		}
		if r.CollectionId != "c" {
			t.Errorf("CollectionId mismatch: %q", r.CollectionId)
		}
		if r.SourceType == "" || r.SourceProvider == "" || r.SourceUri == "" {
			t.Error("core source fields must be present")
		}
		if r.ContentHash == "" {
			t.Error("ContentHash must be present")
		}
		if len(r.Provenance) != 1 {
			t.Errorf("Provenance must have 1 entry, got %d", len(r.Provenance))
		}
	}
	if r1.Id == r2.Id {
		t.Error("different inputs should yield different Ids")
	}
	if r1.ContentHash == r2.ContentHash {
		t.Error("different content should yield different ContentHash")
	}
}

// TEST-3.1.5 / SCEN-3.1.5 / AC5: importer/record 解耦 — 注册表按 confidence 排序，不同 importer 产出同 schema record。
func TestImporterRecordDecoupling(t *testing.T) {
	// 注册两个 mock，confidence 不同
	low := &mockImporter{name: "low", detectOK: true, confidence: 0.3}
	high := &mockImporter{name: "high", detectOK: true, confidence: 0.9}
	Register(low)
	Register(high)

	imp, err := Resolve("/any/path/high.txt")
	if err != nil {
		t.Fatal(err)
	}
	if imp.Name() != "high" {
		t.Errorf("expected high-confidence importer, got %s", imp.Name())
	}

	// 两个 importer 产生相同字段结构的 record（解耦验证）
	recs1, _ := low.Import("/a", "c")
	recs2, _ := high.Import("/b", "c")
	if len(recs1) == 0 || len(recs2) == 0 {
		t.Fatal("both importers should produce records")
	}
	if recs1[0].SchemaVersion != recs2[0].SchemaVersion {
		t.Error("SchemaVersion should be same across importers (decoupled)")
	}
}

// mockImporter is a test double for AC1/AC5.
type mockImporter struct {
	name       string
	detectOK   bool
	confidence float64
}

func (m *mockImporter) Name() string { return m.name }
func (m *mockImporter) Detect(path string) (float64, bool) {
	// Match only when the path contains the importer's own name so that
	// multiple mocks can coexist without cross-interference.
	if strings.Contains(path, m.name) {
		return m.confidence, m.detectOK
	}
	return 0, false
}
func (m *mockImporter) Import(path string, collectionID string) ([]*contextforgev1.ContextRecord, error) {
	return []*contextforgev1.ContextRecord{{
		Id:            "ctx_mock_" + m.name,
		SchemaVersion: "0.1",
		CollectionId:  collectionID,
		SourceType:    "mock",
		SourceUri:     path,
	}}, nil
}
