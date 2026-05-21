package openclaw

import (
	"bytes"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// TEST-3.3.1 / SCEN-3.3.1 / AC1: OpenClaw workspace 通用导入 file/markdown/config/log/memory-like 文件。
func TestImportWorkspaceGenericFiles(t *testing.T) {
	ws := t.TempDir()
	writeFile(t, ws, "notes/plan.md", "# Plan\n")
	writeFile(t, ws, "config/settings.toml", "mode = \"local\"\n")
	writeFile(t, ws, "logs/session.log", "started\n")
	writeFile(t, ws, "memory/session.json", `{"unknown":"schema"}`)

	recs, err := ImportWorkspace(ws, "openclaw/proj-a", "openclaw")
	if err != nil {
		t.Fatalf("ImportWorkspace failed: %v", err)
	}
	if len(recs) != 4 {
		t.Fatalf("expected 4 imported records, got %d", len(recs))
	}

	typesByRel := sourceTypesByRelPath(t, ws, recs)
	want := map[string]string{
		"notes/plan.md":        "file",
		"config/settings.toml": "config",
		"logs/session.log":     "log",
		"memory/session.json":  "memory",
	}
	for rel, sourceType := range want {
		if typesByRel[rel] != sourceType {
			t.Errorf("%s source_type = %q, want %q", rel, typesByRel[rel], sourceType)
		}
	}
	for _, rec := range recs {
		if rec.SourceProvider != Provider {
			t.Errorf("SourceProvider = %q, want %q", rec.SourceProvider, Provider)
		}
	}
}

// TEST-3.3.2 / SCEN-3.3.2 / AC2: collection id 和 file_path/source_modified_at/source_type/agent_scope 保留。
func TestCollectionAndProvenanceFields(t *testing.T) {
	root := t.TempDir()
	ws := filepath.Join(root, "proj-a")
	writeFile(t, ws, "config/settings.yaml", "memory: true\n")

	recs, err := ImportWorkspace(ws, "", "openclaw")
	if err != nil {
		t.Fatalf("ImportWorkspace failed: %v", err)
	}
	if len(recs) != 1 {
		t.Fatalf("expected 1 imported record, got %d", len(recs))
	}

	rec := recs[0]
	if rec.CollectionId != "openclaw/proj-a" {
		t.Errorf("CollectionId = %q, want openclaw/proj-a", rec.CollectionId)
	}
	if rec.FilePath == "" {
		t.Error("FilePath must be preserved")
	}
	if rec.SourceType != "config" {
		t.Errorf("SourceType = %q, want config", rec.SourceType)
	}
	if !contains(rec.AgentScope, "openclaw") || !contains(rec.AgentScope, "workspace:proj-a") {
		t.Errorf("AgentScope = %#v, want openclaw and workspace:proj-a", rec.AgentScope)
	}
	if len(rec.Provenance) != 1 || rec.Provenance[0].SourceModifiedAt == nil {
		t.Fatalf("source_modified_at must be preserved in provenance, got %#v", rec.Provenance)
	}
}

// TEST-3.3.3 / SCEN-3.3.3 / AC3: importer 只读导入，不复刻 OpenClaw backend 或写回 workspace。
func TestImportIsReadOnly(t *testing.T) {
	ws := t.TempDir()
	source := writeFile(t, ws, "memory/current.md", "keep me\n")
	beforeInfo, err := os.Stat(source)
	if err != nil {
		t.Fatal(err)
	}
	beforeFiles := relFileList(t, ws)
	beforeContent, err := os.ReadFile(source)
	if err != nil {
		t.Fatal(err)
	}

	recs, err := ImportWorkspace(ws, "openclaw/read-only", "openclaw")
	if err != nil {
		t.Fatalf("ImportWorkspace failed: %v", err)
	}
	if len(recs) != 1 {
		t.Fatalf("expected 1 imported record, got %d", len(recs))
	}

	afterContent, err := os.ReadFile(source)
	if err != nil {
		t.Fatal(err)
	}
	afterInfo, err := os.Stat(source)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(beforeContent, afterContent) {
		t.Fatal("source content changed during import")
	}
	if !beforeInfo.ModTime().Equal(afterInfo.ModTime()) {
		t.Fatalf("source mtime changed during import: before=%s after=%s", beforeInfo.ModTime(), afterInfo.ModTime())
	}
	if got := relFileList(t, ws); strings.Join(got, "\n") != strings.Join(beforeFiles, "\n") {
		t.Fatalf("workspace file set changed: before=%v after=%v", beforeFiles, got)
	}
}

// TEST-3.3.4 / SCEN-3.3.4 / AC4: OpenClaw schema TBD 时走通用 fallback 并输出 warning。
func TestUnknownOpenClawSchemaUsesFallbackWarning(t *testing.T) {
	var buf bytes.Buffer
	log.SetOutput(&buf)
	defer log.SetOutput(os.Stderr)

	ws := t.TempDir()
	writeFile(t, ws, "memory/raw.json", `{"future_openclaw_schema":true}`)

	recs, err := ImportWorkspace(ws, "openclaw/fallback", "openclaw")
	if err != nil {
		t.Fatalf("ImportWorkspace failed: %v", err)
	}
	if len(recs) != 1 {
		t.Fatalf("expected 1 imported fallback record, got %d", len(recs))
	}
	rec := recs[0]
	if rec.SourceProvider != Provider {
		t.Errorf("SourceProvider = %q, want %q", rec.SourceProvider, Provider)
	}
	if rec.SourceType != "memory" {
		t.Errorf("SourceType = %q, want memory", rec.SourceType)
	}
	out := buf.String()
	if !strings.Contains(out, "warning") || !strings.Contains(out, "fallback") {
		t.Fatalf("expected fallback warning, got %q", out)
	}
}

func writeFile(t *testing.T, root string, rel string, content string) string {
	t.Helper()
	path := filepath.Join(root, rel)
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
	return path
}

func sourceTypesByRelPath(t *testing.T, root string, recs []*contextforgev1.ContextRecord) map[string]string {
	t.Helper()
	got := make(map[string]string, len(recs))
	for _, rec := range recs {
		rel, err := filepath.Rel(root, rec.FilePath)
		if err != nil {
			t.Fatal(err)
		}
		got[filepath.ToSlash(rel)] = rec.SourceType
	}
	return got
}

func contains(values []string, want string) bool {
	for _, value := range values {
		if value == want {
			return true
		}
	}
	return false
}

func relFileList(t *testing.T, root string) []string {
	t.Helper()
	var files []string
	err := filepath.WalkDir(root, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			return nil
		}
		rel, err := filepath.Rel(root, path)
		if err != nil {
			return err
		}
		files = append(files, filepath.ToSlash(rel))
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	sort.Strings(files)
	return files
}
