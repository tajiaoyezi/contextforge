package exporter

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TEST-6.3.2 / SCEN-6.3.2 / AC2
func TestTask63_AC2_AgentDraftWritesFourFilesAndRejectsProtectedPaths(t *testing.T) {
	records := sampleRecords(t, 4)
	records[0].AgentScope = []string{"memory"}
	records[0].Content = "memory scope rule"
	records[1].AgentScope = []string{"user"}
	records[1].Content = "user scope preference"
	records[2].AgentScope = []string{"agents"}
	records[2].Content = "agents scope topology"
	records[3].AgentScope = []string{"claude"}
	records[3].Content = "claude scope setting"

	outDir := filepath.Join(t.TempDir(), "draft")
	if err := writeAgentDraft(records, outDir); err != nil {
		t.Fatalf("writeAgentDraft: %v", err)
	}

	wantFiles := map[string]string{
		"MEMORY.md": "memory scope rule",
		"USER.md":   "user scope preference",
		"AGENTS.md": "agents scope topology",
		"CLAUDE.md": "claude scope setting",
	}
	for name, want := range wantFiles {
		path := filepath.Join(outDir, name)
		body, err := os.ReadFile(path)
		if err != nil {
			t.Fatalf("draft file %s missing: %v", name, err)
		}
		if !strings.Contains(string(body), want) {
			t.Fatalf("%s missing scoped content %q\n%s", name, want, body)
		}
	}

	home := t.TempDir()
	t.Setenv("HOME", home)
	t.Setenv("USERPROFILE", home)
	protected := filepath.Join(home, ".claude")
	if err := writeAgentDraft(records, protected); err == nil {
		t.Fatalf("writeAgentDraft should reject protected agent path %s", protected)
	}
}
