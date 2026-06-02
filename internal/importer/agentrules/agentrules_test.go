package agentrules_test

import (
	"bytes"
	"log"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/tajiaoyezi/contextforge/internal/importer"
	_ "github.com/tajiaoyezi/contextforge/internal/importer/agentrules"  // ensure registry has it for Resolve tests
	ar "github.com/tajiaoyezi/contextforge/internal/importer/agentrules" // for New + init side-effect
)

// TEST-3.4.1 / SCEN-3.4.1 / AC1: AGENTS.md / CLAUDE.md 作为 agent_rule 导入
func TestAgentRulesImporter_AGENTS_CL_AUDE(t *testing.T) {
	tmp := t.TempDir()
	agents := filepath.Join(tmp, "AGENTS.md")
	content := "# Project Rules\n\n- Always run tests before commit\n- Review AGENTS.md on entry"
	if err := os.WriteFile(agents, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}

	imp := ar.NewAgentRulesImporter() // direct from subpackage (RED will fail on err)
	recs, err := imp.Import(agents, "default")
	if err != nil {
		t.Fatalf("Import AGENTS.md failed in RED: %v", err)
	}
	if len(recs) != 1 {
		t.Fatalf("expected 1 record, got %d", len(recs))
	}
	r := recs[0]
	if r.SourceType != "agent_rule" {
		t.Errorf("TEST-3.4.1: expected SourceType=agent_rule, got %q", r.SourceType)
	}
	if r.SourceProvider != "claude-code" {
		t.Errorf("TEST-3.4.1: expected provider=claude-code for AGENTS.md, got %q", r.SourceProvider)
	}
	if !strings.Contains(r.Content, "Project Rules") {
		t.Error("TEST-3.4.1: content should contain markdown header")
	}
	if len(r.Tags) == 0 || r.Tags[0] != "agent_rule" {
		t.Errorf("TEST-3.4.1: expected tags to include agent_rule, got %v", r.Tags)
	}
	if r.RedactionStatus != "pending" {
		t.Errorf("TEST-3.4.1: expected redaction_status=pending (from 3.1 FIX-4), got %q", r.RedactionStatus)
	}
}

// TEST-3.4.2 / SCEN-3.4.2 / AC2: Cursor/Zed rules 直接用 agent-rules importer 导入并标记 agent_rule
func TestAgentRulesImporter_CursorZedDirect(t *testing.T) {
	tmp := t.TempDir()
	cursorRule := filepath.Join(tmp, ".cursorrules")
	content := "# Cursor Rules\n\n- Use goimports on save"
	if err := os.WriteFile(cursorRule, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}

	imp := ar.NewAgentRulesImporter()
	recs, err := imp.Import(cursorRule, "proj-cursor")
	if err != nil {
		t.Fatalf("direct Import cursor rule failed in RED: %v", err)
	}
	if len(recs) != 1 {
		t.Fatalf("expected 1 record, got %d", len(recs))
	}
	r := recs[0]
	if r.SourceType != "agent_rule" {
		t.Errorf("TEST-3.4.2: expected agent_rule even for TBD cursor path, got %q", r.SourceType)
	}
	if r.SourceProvider != "cursor" {
		t.Errorf("TEST-3.4.2: expected provider=cursor for direct TBD path, got %q", r.SourceProvider)
	}
}

// TEST-3.4.3 / SCEN-3.4.3 / AC3: 只读导入，不写回原文件
func TestAgentRulesImporter_ReadOnlyNoWriteback(t *testing.T) {
	tmp := t.TempDir()
	claude := filepath.Join(tmp, "CLAUDE.md")
	orig := "# Claude Memory\n\n- foo"
	if err := os.WriteFile(claude, []byte(orig), 0o644); err != nil {
		t.Fatal(err)
	}
	before, _ := os.ReadFile(claude)

	imp := ar.NewAgentRulesImporter()
	_, err := imp.Import(claude, "mem")
	if err != nil {
		t.Fatalf("Import failed in RED: %v", err)
	}

	after, _ := os.ReadFile(claude)
	if string(before) != string(after) {
		t.Error("TEST-3.4.3: original CLAUDE.md must be unchanged (只读, AC3/ADR-005)")
	}
}

// TEST-3.4.4 / SCEN-3.4.4 / AC4: Cursor/Zed TBD 路径走 Resolve fallback + warning
func TestAgentRulesImporter_TBDPathFallsToFallback(t *testing.T) {
	var buf bytes.Buffer
	log.SetOutput(&buf)
	defer log.SetOutput(os.Stderr)

	tmp := t.TempDir()
	zedRule := filepath.Join(tmp, "zed", "project-rules.md")
	os.MkdirAll(filepath.Dir(zedRule), 0o755)
	if err := os.WriteFile(zedRule, []byte("# Zed rules TBD"), 0o644); err != nil {
		t.Fatal(err)
	}

	imp, err := importer.Resolve(zedRule)
	if err != nil {
		t.Fatalf("Resolve TBD path should not error: %v", err)
	}
	if imp.Name() != "fallback" {
		t.Errorf("TEST-3.4.4: expected fallback for unrecognized TBD path, got %s", imp.Name())
	}
	_, err = imp.Import(zedRule, "default")
	if err != nil {
		t.Fatalf("fallback Import failed: %v", err)
	}
	out := buf.String()
	if !strings.Contains(out, "warning") && !strings.Contains(out, "fallback") {
		t.Errorf("TEST-3.4.4: expected explicit warning log for TBD path, got: %q", out)
	}
}
