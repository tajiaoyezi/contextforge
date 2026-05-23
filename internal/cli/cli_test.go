package cli

import (
	"bytes"
	"io"
	"os"
	"path/filepath"
	"runtime"
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
// SCEN-1.4.4 / AC4: 8 子命令注册齐全；未实现子命令返回非 0 + stderr "not implemented"，绝不 panic。
func TestTask14_AC4_SubcommandsRegisteredUnimplementedNoPanic(t *testing.T) {
	want := []string{"init", "import", "index", "search", "serve", "mcp", "eval", "export"}
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
	// task-8.2 makes `index` real. Only import remains a Phase 2+ placeholder.
	for _, sub := range []string{"import"} {
		var stdout, stderr bytes.Buffer
		code := mustNotPanic(t, func() int { return Execute([]string{sub}, &stdout, &stderr) })
		if code == 0 {
			t.Fatalf("Execute(%q) exit=0, want non-zero (not-implemented)", sub)
		}
		if !bytes.Contains(stderr.Bytes(), []byte("not implemented")) {
			t.Fatalf("Execute(%q) stderr=%q, want substring %q", sub, stderr.String(), "not implemented")
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
