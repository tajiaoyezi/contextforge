package cli

import (
	"bytes"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// TEST-1.4.1
// SCEN-1.4.1 / AC1: `contextforge init` 生成 ~/.contextforge/ 配置与目录（不联网），幂等可重跑。
func TestTask14_AC1_InitGeneratesConfigIdempotent(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	// Windows os.UserHomeDir reads USERPROFILE first; HOME-only setenv leaks
	// into the real %USERPROFILE%\.contextforge and breaks the sandbox.
	t.Setenv("USERPROFILE", home)

	if err := runInit("", io.Discard); err != nil {
		t.Fatalf("runInit #1: %v", err)
	}
	root := filepath.Join(home, ".contextforge")
	cfg := filepath.Join(root, "config.toml")
	cfgFI, err := os.Stat(cfg)
	if err != nil {
		t.Fatalf("config.toml not generated: %v", err)
	}
	// §5.3 SCEN-1.4.1: config.toml == 0600 (guards the task-1.2 config.Init
	// 0600/0700 enforcement against future regression — ADR-004 baseline).
	// Windows Go stdlib does not implement POSIX chmod; Stat().Mode().Perm()
	// reports 0666/0777 regardless. ACL-equivalent enforcement is Phase 8 /
	// v0.3 Windows-preview scope; skip the bit check here.
	if runtime.GOOS == "windows" {
		t.Logf("SCEN-1.4.1: POSIX perm bits not asserted on Windows " +
			"(Go stdlib reports 0666/0777); ACL enforcement deferred to Phase 8 / v0.3")
	} else {
		if got := cfgFI.Mode().Perm(); got != 0o600 {
			t.Fatalf("config.toml perm = %o, want 0600", got)
		}
	}
	for _, d := range []string{"collections", "logs", "runtime"} {
		fi, err := os.Stat(filepath.Join(root, d))
		if err != nil || !fi.IsDir() {
			t.Fatalf("scaffold dir %q missing: %v", d, err)
		}
		// §5.3 SCEN-1.4.1: scaffold dirs == 0700 (see Windows note above).
		if runtime.GOOS != "windows" {
			if got := fi.Mode().Perm(); got != 0o700 {
				t.Fatalf("scaffold dir %q perm = %o, want 0700", d, got)
			}
		}
	}

	// idempotent re-run: no error, config still present.
	if err := runInit("", io.Discard); err != nil {
		t.Fatalf("runInit #2 (idempotent): %v", err)
	}
	if _, err := os.Stat(cfg); err != nil {
		t.Fatalf("config.toml gone after idempotent re-run: %v", err)
	}
}

// TEST-1.4.4
// SCEN-1.4.4 / AC4: 9 子命令注册齐全；未实现子命令返回非 0 + stderr "not implemented"，绝不 panic。
// task-10.6 (Phase 10) 新增 "console-api-serve" 子命令 → 列表从 8 扩到 9。
// task-45.3 (Phase 45 / ADR-050 D2) 新增 "version" 子命令 → 列表从 9 扩到 10。
func TestTask14_AC4_SubcommandsRegisteredUnimplementedNoPanic(t *testing.T) {
	want := []string{"init", "import", "index", "search", "serve", "mcp", "eval", "export", "console-api-serve", "version"}
	got := SubcommandNames()
	if len(got) != len(want) {
		t.Fatalf("SubcommandNames()=%v (len %d), want %v (len %d)", got, len(got), want, len(want))
	}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("SubcommandNames()[%d]=%q, want %q", i, got[i], want[i])
		}
	}

	// task-6.1 / task-6.2 / task-6.3: `search`, `serve`, `export` are now real
	// subcommands and dispatch through runSearch / runServe / runExport. The
	// task-7.1: `mcp` is now a real subcommand. task-8.1 makes `eval` real.
	// task-8.2 makes `index` real. task-9.4 (Phase 9) makes `import` real —
	// every registered subcommand is now wired. The legacy `not implemented`
	// branch in cli.go remains for forward-compat (any future subcommand name
	// registered without dispatch wiring), but no current subcommand should
	// surface it; assert empty-args returns a non-zero usage exit (no panic).
	for _, sub := range []string{"import"} {
		var stdout, stderr bytes.Buffer
		code := mustNotPanic(t, func() int { return Execute([]string{sub}, &stdout, &stderr) })
		if code == 0 {
			t.Fatalf("Execute(%q) exit=0, want non-zero (usage)", sub)
		}
		if bytes.Contains(stderr.Bytes(), []byte("not implemented")) {
			t.Fatalf("Execute(%q) should be wired (no 'not implemented'), stderr=%q", sub, stderr.String())
		}
	}

	// unknown subcommand: also non-zero, no panic.
	var o, e bytes.Buffer
	if code := mustNotPanic(t, func() int { return Execute([]string{"bogus"}, &o, &e) }); code == 0 {
		t.Fatalf("Execute(bogus) exit=0, want non-zero for unknown subcommand")
	}
}

func mustNotPanic(t *testing.T, f func() int) (code int) {
	t.Helper()
	defer func() {
		if r := recover(); r != nil {
			t.Fatalf("Execute panicked (AC4 requires explicit not-implemented, never panic): %v", r)
		}
	}()
	return f()
}

// TEST-45.3.1 (task-45.3 / ADR-050 D2): `contextforge version` prints the version.
func TestTask453_VersionPrintsVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := ExecuteWithIO([]string{"version"}, nil, &stdout, &stderr)
	if code != 0 {
		t.Fatalf("version: exit=%d want 0 (stderr=%q)", code, stderr.String())
	}
	out := stdout.String()
	if !strings.Contains(out, "contextforge ") {
		t.Fatalf("version output missing 'contextforge ' prefix: %q", out)
	}
	if !strings.Contains(out, Version) {
		t.Fatalf("version output missing Version=%q: %q", Version, out)
	}
}

// TEST-45.3.2 (task-45.3 / ADR-050 D2): top-level `--help`/`-h`/`help` does NOT
// exit 2 (previously `-h` fell through to "unknown subcommand"). It prints the
// subcommand list + usage and exits 0.
func TestTask453_TopLevelHelpDoesNotExit2(t *testing.T) {
	for _, arg := range []string{"--help", "-h", "help"} {
		var stdout, stderr bytes.Buffer
		code := ExecuteWithIO([]string{arg}, nil, &stdout, &stderr)
		if code != 0 {
			t.Fatalf("%s: exit=%d want 0 (stderr=%q)", arg, code, stderr.String())
		}
		out := stdout.String()
		if !strings.Contains(out, "Subcommands:") {
			t.Fatalf("%s: stdout missing 'Subcommands:' usage: %q", arg, out)
		}
		// sanity: the subcommand list includes the canonical subcommands.
		for _, want := range []string{"init", "search", "version"} {
			if !strings.Contains(out, want) {
				t.Fatalf("%s: stdout missing subcommand %q: %q", arg, want, out)
			}
		}
	}
}

// TEST-45.3.3 (task-45.3 / ADR-050 D2): contextforge.example.toml documents all 4
// retrieval sections introduced by task-22.1/34.2/38.2/41.2 (embedding/vector/
// reranker/retrieval) so users can configure the full retrieval stack from the example.
func TestTask453_ExampleTomlCoversRetrievalSections(t *testing.T) {
	path := filepath.Join("..", "..", "contextforge.example.toml")
	body, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	s := string(body)
	for _, section := range []string{"[embedding]", "[vector]", "[reranker]", "[retrieval]"} {
		if !strings.Contains(s, section) {
			t.Fatalf("contextforge.example.toml missing %q section (task-45.3: document all retrieval sections)", section)
		}
	}
}
