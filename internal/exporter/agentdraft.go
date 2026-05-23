package exporter

import (
	"bytes"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

var draftFiles = []string{"MEMORY.md", "USER.md", "AGENTS.md", "CLAUDE.md"}

func writeAgentDraft(records []*contextforgev1.ContextRecord, dir string) error {
	files, err := renderAgentDraft(records)
	if err != nil {
		return err
	}
	return writeAgentDraftFiles(dir, files)
}

func renderAgentDraft(records []*contextforgev1.ContextRecord) (map[string][]byte, error) {
	files := map[string][]byte{
		"MEMORY.md": []byte("# Project memories\n\n"),
		"USER.md":   []byte("# User context\n\n"),
		"AGENTS.md": []byte("# Agent rules\n\n"),
		"CLAUDE.md": []byte("# Claude context\n\n"),
	}
	for _, rec := range records {
		if rec == nil {
			continue
		}
		target := draftFileForRecord(rec)
		md, err := recordMarkdown(rec)
		if err != nil {
			return nil, err
		}
		var b bytes.Buffer
		b.Write(files[target])
		b.WriteString("\n```bash\n")
		b.WriteString("# Apply this draft manually after review; ContextForge never writes agent homes.\n")
		b.WriteString("```\n\n")
		b.Write(md)
		files[target] = b.Bytes()
	}
	return files, nil
}

func writeAgentDraftFiles(dir string, files map[string][]byte) error {
	if isProtectedAgentPath(dir) {
		return fmt.Errorf("agent-draft output %q is a protected agent path", dir)
	}
	if err := os.MkdirAll(dir, 0o700); err != nil {
		return fmt.Errorf("create agent-draft dir %s: %w", dir, err)
	}
	for _, name := range draftFiles {
		body := files[name]
		if body == nil {
			body = []byte("# " + strings.TrimSuffix(name, ".md") + "\n")
		}
		if err := os.WriteFile(filepath.Join(dir, name), body, 0o600); err != nil {
			return err
		}
	}
	return nil
}

func concatDraftFiles(files map[string][]byte) []byte {
	var out bytes.Buffer
	for _, name := range sortedKeys(files) {
		out.WriteString("\n--- contextforge-draft-file: ")
		out.WriteString(name)
		out.WriteString(" ---\n")
		out.Write(files[name])
		out.WriteByte('\n')
	}
	return out.Bytes()
}

func draftFileForRecord(rec *contextforgev1.ContextRecord) string {
	for _, scope := range rec.GetAgentScope() {
		switch strings.ToLower(scope) {
		case "memory":
			return "MEMORY.md"
		case "user":
			return "USER.md"
		case "agents", "agent", "rules":
			return "AGENTS.md"
		case "claude":
			return "CLAUDE.md"
		}
	}
	if strings.Contains(strings.ToLower(rec.GetFilePath()), "claude") {
		return "CLAUDE.md"
	}
	return "MEMORY.md"
}

func isProtectedAgentPath(path string) bool {
	abs, err := filepath.Abs(path)
	if err != nil {
		return true
	}
	abs = cleanComparablePath(abs)
	home := userHomeDir()
	if home == "" {
		return false
	}
	home = cleanComparablePath(home)
	protected := []string{
		// Unix-style agent config dirs
		filepath.Join(home, ".cursor"),
		filepath.Join(home, ".claude"),
		filepath.Join(home, ".cursor-claude"),
		filepath.Join(home, ".config", "claude"),
		filepath.Join(home, ".config", "cursor"),
	}
	// Windows-style agent config dirs (PR #44 review FIX-2)
	// Project primary env per docs/s2v-adapter.md §Constraints platform 是
	// Windows + WSL2；agent 软件 (Claude Desktop / Cursor) 在 Windows 用
	// %APPDATA% / %LOCALAPPDATA% 而非 ~/.config — 必须覆盖
	if appData := os.Getenv("APPDATA"); appData != "" {
		appData = cleanComparablePath(appData)
		protected = append(protected,
			filepath.Join(appData, "Claude"),
			filepath.Join(appData, "Cursor"),
			filepath.Join(appData, "anthropic"),
		)
	}
	if localAppData := os.Getenv("LOCALAPPDATA"); localAppData != "" {
		localAppData = cleanComparablePath(localAppData)
		protected = append(protected,
			filepath.Join(localAppData, "Claude"),
			filepath.Join(localAppData, "Cursor"),
			filepath.Join(localAppData, "Programs", "Claude"),
		)
	}
	for _, p := range protected {
		p = cleanComparablePath(p)
		if abs == p || strings.HasPrefix(abs, p+string(filepath.Separator)) {
			return true
		}
	}
	return false
}

func cleanComparablePath(path string) string {
	clean := filepath.Clean(path)
	if volume := filepath.VolumeName(clean); volume != "" {
		return strings.ToLower(clean)
	}
	return clean
}

func userHomeDir() string {
	if h := os.Getenv("HOME"); h != "" {
		return h
	}
	if h := os.Getenv("USERPROFILE"); h != "" {
		return h
	}
	home, _ := os.UserHomeDir()
	return home
}

func sortedDraftNames() []string {
	names := append([]string(nil), draftFiles...)
	sort.Strings(names)
	return names
}
