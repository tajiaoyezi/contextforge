package main

import (
	"os"
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
