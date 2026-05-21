package hermes_test

import (
	"bytes"
	"log"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/tajiaoyezi/contextforge/internal/importer/hermes"
)

// TEST-3.2.1 / SCEN-3.2.1 / AC1: Hermes MEMORY.md 能导入为 ContextRecord
func TestHermesImportMemoryMd(t *testing.T) {
	tmp := t.TempDir()
	fpath := filepath.Join(tmp, "MEMORY.md")
	content := "# Project memories\n- rule 1\n"
	if err := os.WriteFile(fpath, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}

	imp := hermes.New()
	// AC1: Detect filename
	conf, ok := imp.Detect(fpath)
	if !ok {
		t.Errorf("AC1: Detect(MEMORY.md) must report ok=true (got conf=%v ok=%v)", conf, ok)
	}
	if conf < 0.5 {
		t.Errorf("AC1: Detect confidence should be > fallback's 0.1 (got %v)", conf)
	}

	recs, err := imp.Import(fpath, "default")
	if err != nil {
		t.Fatalf("AC1: Import failed: %v", err)
	}
	if len(recs) != 1 {
		t.Fatalf("AC1: expected 1 ContextRecord, got %d", len(recs))
	}
	r := recs[0]
	if r.Content != content {
		t.Errorf("AC1: Content should preserve markdown verbatim, got %q", r.Content)
	}
	if r.SchemaVersion != "0.1" {
		t.Errorf("AC1: SchemaVersion must be 0.1, got %q", r.SchemaVersion)
	}
	if r.CollectionId != "default" {
		t.Errorf("AC1: CollectionId mismatch, got %q", r.CollectionId)
	}
}

// TEST-3.2.2 / SCEN-3.2.2 / AC2: provider=hermes / scope 含 hermes / provenance.importer=hermes-memory + source_modified_at 保留
func TestHermesProviderScopeProvenance(t *testing.T) {
	tmp := t.TempDir()
	fpath := filepath.Join(tmp, "MEMORY.md")
	if err := os.WriteFile(fpath, []byte("# heading\nbody"), 0o644); err != nil {
		t.Fatal(err)
	}
	// 设一个明确 mtime 以验证 source_modified_at
	wantMtime := time.Date(2026, 4, 1, 12, 0, 0, 0, time.UTC)
	if err := os.Chtimes(fpath, wantMtime, wantMtime); err != nil {
		t.Fatal(err)
	}

	imp := hermes.New()
	recs, err := imp.Import(fpath, "proj-a")
	if err != nil {
		t.Fatalf("AC2: Import failed: %v", err)
	}
	if len(recs) != 1 {
		t.Fatalf("AC2: expected 1 record, got %d", len(recs))
	}
	r := recs[0]

	// Provider / source_type / language
	if r.SourceProvider != "hermes" {
		t.Errorf("AC2: SourceProvider must be 'hermes', got %q", r.SourceProvider)
	}
	if r.SourceType != "memory" {
		t.Errorf("AC2: SourceType must be 'memory', got %q", r.SourceType)
	}
	if r.Language != "markdown" {
		t.Errorf("AC2: Language must be 'markdown', got %q", r.Language)
	}

	// BINDING: redaction_status="pending"（task-3.1 §10 Waiver）
	if r.RedactionStatus != "pending" {
		t.Errorf("AC2 BINDING: RedactionStatus must be 'pending' (task-3.1 §10 Waiver), got %q", r.RedactionStatus)
	}

	// AgentScope 含 hermes
	hasHermes := false
	for _, s := range r.AgentScope {
		if s == "hermes" {
			hasHermes = true
			break
		}
	}
	if !hasHermes {
		t.Errorf("AC2: AgentScope must contain 'hermes', got %v", r.AgentScope)
	}

	// Provenance 字段透传
	if len(r.Provenance) != 1 {
		t.Fatalf("AC2: expected 1 Provenance, got %d", len(r.Provenance))
	}
	p := r.Provenance[0]
	if p.Importer != "hermes-memory" {
		t.Errorf("AC2: Provenance.Importer must be 'hermes-memory', got %q", p.Importer)
	}
	if p.OriginalPath == "" {
		t.Errorf("AC2: Provenance.OriginalPath must be non-empty")
	}
	if p.SourceModifiedAt == nil {
		t.Errorf("AC2: Provenance.SourceModifiedAt must be set (file mtime)")
	} else if !p.SourceModifiedAt.AsTime().Equal(wantMtime) {
		t.Errorf("AC2: Provenance.SourceModifiedAt mismatch: want %v got %v",
			wantMtime, p.SourceModifiedAt.AsTime())
	}
}

// TEST-3.2.3 / SCEN-3.2.3 / AC3: 只读导入 — Import 不写回 Hermes 文件
func TestHermesReadOnlyNoWriteback(t *testing.T) {
	tmp := t.TempDir()
	fpath := filepath.Join(tmp, "USER.md")
	originalBytes := []byte("# User preferences\n")
	if err := os.WriteFile(fpath, originalBytes, 0o644); err != nil {
		t.Fatal(err)
	}
	preInfo, err := os.Stat(fpath)
	if err != nil {
		t.Fatal(err)
	}
	preMtime := preInfo.ModTime()
	preSize := preInfo.Size()

	imp := hermes.New()
	recs, err := imp.Import(fpath, "default")
	if err != nil {
		t.Fatalf("AC3: Import failed: %v", err)
	}
	// 不写回 + 必须真产 record（防 stub 通过 "什么也不做" 假绿）
	if len(recs) == 0 {
		t.Fatalf("AC3: Import must produce ≥1 record (defense against do-nothing stub)")
	}

	postInfo, err := os.Stat(fpath)
	if err != nil {
		t.Fatal(err)
	}
	if !postInfo.ModTime().Equal(preMtime) {
		t.Errorf("AC3: file mtime changed (was %v, now %v) — Hermes file was modified",
			preMtime, postInfo.ModTime())
	}
	if postInfo.Size() != preSize {
		t.Errorf("AC3: file size changed (was %d, now %d) — Hermes file was modified",
			preSize, postInfo.Size())
	}
	// 原内容字节级一致
	postBytes, err := os.ReadFile(fpath)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(postBytes, originalBytes) {
		t.Errorf("AC3: file content modified — bytes differ")
	}
}

// TEST-3.2.4 / SCEN-3.2.4 / AC4: 空内容 → 降级 task-3.1 fallback + warning，不中断
func TestHermesEmptyContentFallback(t *testing.T) {
	var buf bytes.Buffer
	log.SetOutput(&buf)
	defer log.SetOutput(os.Stderr)

	tmp := t.TempDir()
	fpath := filepath.Join(tmp, "MEMORY.md")
	// 仅含空白 → §2A 决策 AC4 触发条件
	if err := os.WriteFile(fpath, []byte("   \n\n  "), 0o644); err != nil {
		t.Fatal(err)
	}

	imp := hermes.New()
	recs, err := imp.Import(fpath, "default")
	if err != nil {
		t.Fatalf("AC4: empty content should fall back, not error: %v", err)
	}
	if len(recs) != 1 {
		t.Fatalf("AC4: fallback must produce 1 record (via task-3.1 FileFallbackImporter), got %d", len(recs))
	}
	out := buf.String()
	if !strings.Contains(out, "warning") {
		t.Errorf("AC4: expected explicit warning log for fallback path, got: %q", out)
	}
	if !strings.Contains(strings.ToLower(out), "fallback") {
		t.Errorf("AC4: expected 'fallback' mention in warning log, got: %q", out)
	}
}
