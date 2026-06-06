package main

import (
	"bytes"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/tajiaoyezi/contextforge/internal/config"
)

// TEST-34.2.2 (task-34.2 / ADR-039 D2): setVectorEnv bridges config.toml [vector] → the core's
// CONTEXTFORGE_VECTOR_BACKEND/_DIM env; an explicitly-set env var wins over the config file; an
// empty [vector] exports nothing (core env path unchanged → BruteForce byte-equivalent).
func TestSetVectorEnv(t *testing.T) {
	writeCfg := func(t *testing.T, v config.VectorConfig) string {
		t.Helper()
		root := t.TempDir()
		c := config.DefaultConfig()
		c.DataDir = root
		c.Vector = v
		if err := config.Save(root, c); err != nil {
			t.Fatalf("save config: %v", err)
		}
		return root
	}

	t.Run("[vector] present → env exported, restore unsets", func(t *testing.T) {
		root := writeCfg(t, config.VectorConfig{Backend: "qdrant", Dim: 384})
		os.Unsetenv("CONTEXTFORGE_VECTOR_BACKEND")
		os.Unsetenv("CONTEXTFORGE_VECTOR_DIM")
		restore := setVectorEnv(root)
		if got := os.Getenv("CONTEXTFORGE_VECTOR_BACKEND"); got != "qdrant" {
			t.Errorf("backend env = %q want qdrant", got)
		}
		if got := os.Getenv("CONTEXTFORGE_VECTOR_DIM"); got != "384" {
			t.Errorf("dim env = %q want 384", got)
		}
		restore()
		if _, had := os.LookupEnv("CONTEXTFORGE_VECTOR_BACKEND"); had {
			t.Errorf("restore should unset backend env")
		}
		if _, had := os.LookupEnv("CONTEXTFORGE_VECTOR_DIM"); had {
			t.Errorf("restore should unset dim env")
		}
	})

	t.Run("explicit env wins over config file", func(t *testing.T) {
		root := writeCfg(t, config.VectorConfig{Backend: "qdrant", Dim: 384})
		t.Setenv("CONTEXTFORGE_VECTOR_BACKEND", "lancedb")
		restore := setVectorEnv(root)
		defer restore()
		if got := os.Getenv("CONTEXTFORGE_VECTOR_BACKEND"); got != "lancedb" {
			t.Errorf("env-wins broken: backend = %q want lancedb (explicit env must not be overridden)", got)
		}
	})

	t.Run("empty [vector] → nothing exported (byte-equivalent)", func(t *testing.T) {
		root := writeCfg(t, config.VectorConfig{})
		os.Unsetenv("CONTEXTFORGE_VECTOR_BACKEND")
		os.Unsetenv("CONTEXTFORGE_VECTOR_DIM")
		restore := setVectorEnv(root)
		defer restore()
		if _, had := os.LookupEnv("CONTEXTFORGE_VECTOR_BACKEND"); had {
			t.Errorf("empty [vector] must export no backend env")
		}
		if _, had := os.LookupEnv("CONTEXTFORGE_VECTOR_DIM"); had {
			t.Errorf("empty [vector] must export no dim env")
		}
	})
}

// captureStderr redirects os.Stderr to a pipe for the duration of fn and returns what was written.
func captureStderr(t *testing.T, fn func()) string {
	t.Helper()
	old := os.Stderr
	r, w, err := os.Pipe()
	if err != nil {
		t.Fatalf("os.Pipe: %v", err)
	}
	os.Stderr = w
	fn()
	_ = w.Close()
	os.Stderr = old
	var buf bytes.Buffer
	_, _ = io.Copy(&buf, r)
	_ = r.Close()
	return buf.String()
}

// TEST-35.2.1 (task-35.2 / ADR-040 D2): setVectorEnv surfaces a malformed/unreadable config.toml
// error to stderr (was a silent `return restore`) while staying best-effort (env-only path unchanged,
// daemon not blocked). A MISSING config.toml is the normal default → no WARN (os.ErrNotExist guard).
// A valid config likewise emits no load-failed WARN. stderr is captured via os.Pipe (genuine RED→GREEN).
func TestSetVectorEnv_LoadErrorSurfacing(t *testing.T) {
	t.Run("malformed config.toml → WARN surfaced, env-only path unchanged", func(t *testing.T) {
		root := t.TempDir()
		// dim = <non-int> makes decodeTOML reject it → config.Load parse error (not NotExist).
		if err := os.WriteFile(filepath.Join(root, "config.toml"), []byte("[vector]\ndim = notanumber\n"), 0o644); err != nil {
			t.Fatalf("write malformed config: %v", err)
		}
		os.Unsetenv("CONTEXTFORGE_VECTOR_BACKEND")
		os.Unsetenv("CONTEXTFORGE_VECTOR_DIM")
		var restore func()
		out := captureStderr(t, func() { restore = setVectorEnv(root) })
		defer restore()
		if !strings.Contains(out, "vector config load failed") {
			t.Errorf("malformed config must surface a WARN to stderr; got %q", out)
		}
		// best-effort: a load failure exports no vector env (env-only path unchanged).
		if _, had := os.LookupEnv("CONTEXTFORGE_VECTOR_BACKEND"); had {
			t.Errorf("malformed config must not export vector env (env-only path unchanged)")
		}
	})

	t.Run("missing config.toml → no WARN (normal default)", func(t *testing.T) {
		root := t.TempDir() // no config.toml written
		out := captureStderr(t, func() {
			restore := setVectorEnv(root)
			restore()
		})
		if strings.Contains(out, "vector config load failed") {
			t.Errorf("missing config.toml is the normal default → no WARN; got %q", out)
		}
	})

	t.Run("valid config → no load-failed WARN", func(t *testing.T) {
		root := t.TempDir()
		c := config.DefaultConfig()
		c.DataDir = root
		c.Vector = config.VectorConfig{Backend: "qdrant", Dim: 384}
		if err := config.Save(root, c); err != nil {
			t.Fatalf("save config: %v", err)
		}
		os.Unsetenv("CONTEXTFORGE_VECTOR_BACKEND")
		os.Unsetenv("CONTEXTFORGE_VECTOR_DIM")
		out := captureStderr(t, func() {
			restore := setVectorEnv(root)
			restore()
		})
		if strings.Contains(out, "vector config load failed") {
			t.Errorf("valid config must not emit a load-failed WARN; got %q", out)
		}
	})
}

// TEST-37.2.2 (task-37.2 / ADR-042 D3): setRemoteEnv bridges config.toml [remote] → the core's
// CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_PROVIDER env; an explicitly-set env var wins over the config
// file; an empty [remote] exports nothing (default provider unchanged); and the API key is NEVER
// bridged — CONTEXTFORGE_REMOTE_API_KEY is env-only, never sourced from config (security baseline).
func TestSetRemoteEnv(t *testing.T) {
	writeCfg := func(t *testing.T, r config.RemoteProviderConfig) string {
		t.Helper()
		root := t.TempDir()
		c := config.DefaultConfig()
		c.DataDir = root
		c.Remote = r
		if err := config.Save(root, c); err != nil {
			t.Fatalf("save config: %v", err)
		}
		return root
	}
	const (
		ep = "https://api.example.com/v1/embeddings"
		md = "Qwen/Qwen3-Embedding-8B"
		pv = "openai-compatible"
	)
	keys := []string{"CONTEXTFORGE_REMOTE_ENDPOINT", "CONTEXTFORGE_REMOTE_MODEL", "CONTEXTFORGE_REMOTE_PROVIDER"}

	t.Run("[remote] present → endpoint/model/provider env exported, restore unsets", func(t *testing.T) {
		root := writeCfg(t, config.RemoteProviderConfig{Enabled: true, Provider: pv, Endpoint: ep, Model: md})
		for _, k := range keys {
			os.Unsetenv(k)
		}
		restore := setRemoteEnv(root)
		if got := os.Getenv("CONTEXTFORGE_REMOTE_ENDPOINT"); got != ep {
			t.Errorf("endpoint env = %q want %q", got, ep)
		}
		if got := os.Getenv("CONTEXTFORGE_REMOTE_MODEL"); got != md {
			t.Errorf("model env = %q want %q", got, md)
		}
		if got := os.Getenv("CONTEXTFORGE_REMOTE_PROVIDER"); got != pv {
			t.Errorf("provider env = %q want %q", got, pv)
		}
		restore()
		for _, k := range keys {
			if _, had := os.LookupEnv(k); had {
				t.Errorf("restore should unset %s", k)
			}
		}
	})

	t.Run("explicit env wins over config file", func(t *testing.T) {
		root := writeCfg(t, config.RemoteProviderConfig{Enabled: true, Provider: pv, Endpoint: ep, Model: md})
		t.Setenv("CONTEXTFORGE_REMOTE_MODEL", "explicit-model")
		restore := setRemoteEnv(root)
		defer restore()
		if got := os.Getenv("CONTEXTFORGE_REMOTE_MODEL"); got != "explicit-model" {
			t.Errorf("env-wins broken: model = %q want explicit-model (explicit env must not be overridden)", got)
		}
	})

	t.Run("empty [remote] → nothing exported (default provider unchanged)", func(t *testing.T) {
		root := writeCfg(t, config.RemoteProviderConfig{})
		for _, k := range keys {
			os.Unsetenv(k)
		}
		restore := setRemoteEnv(root)
		defer restore()
		for _, k := range keys {
			if _, had := os.LookupEnv(k); had {
				t.Errorf("empty [remote] must export no %s", k)
			}
		}
	})

	t.Run("API key is NEVER bridged (security baseline)", func(t *testing.T) {
		// even with a fully-populated [remote] config, setRemoteEnv must not set the api-key env.
		root := writeCfg(t, config.RemoteProviderConfig{Enabled: true, Provider: pv, Endpoint: ep, Model: md})
		os.Unsetenv("CONTEXTFORGE_REMOTE_API_KEY")
		restore := setRemoteEnv(root)
		defer restore()
		if _, had := os.LookupEnv("CONTEXTFORGE_REMOTE_API_KEY"); had {
			t.Errorf("setRemoteEnv must NEVER set CONTEXTFORGE_REMOTE_API_KEY (api key is env-only, never from config)")
		}
	})
}
