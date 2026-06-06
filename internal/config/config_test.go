package config

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

// SCEN-1.2.1 / AC1: contextforge 能生成默认 ~/.contextforge/config.toml 与目录骨架
// (collections/ logs/ runtime/)，且写入的 config 可被 Load 往返读回。
func TestConfigInitAndLoad(t *testing.T) {
	t.Run("TEST-1.2.1: Init 生成 config.toml + collections/ logs/ runtime/ 并 Load 往返一致", func(t *testing.T) {
		root := t.TempDir()

		cfg, err := Init(root)
		if err != nil {
			t.Fatalf("Init(%q) error = %v", root, err)
		}
		if cfg.SchemaVersion != SchemaVersion {
			t.Errorf("Init cfg.SchemaVersion = %q, want %q", cfg.SchemaVersion, SchemaVersion)
		}

		cfgPath := filepath.Join(root, "config.toml")
		if _, err := os.Stat(cfgPath); err != nil {
			t.Fatalf("config.toml not created: %v", err)
		}
		for _, d := range []string{"collections", "logs", "runtime"} {
			fi, err := os.Stat(filepath.Join(root, d))
			if err != nil || !fi.IsDir() {
				t.Errorf("data-dir scaffold %q missing (err=%v)", d, err)
			}
		}

		loaded, err := Load(root)
		if err != nil {
			t.Fatalf("Load(%q) error = %v", root, err)
		}
		if loaded.SchemaVersion != cfg.SchemaVersion {
			t.Errorf("round-trip SchemaVersion = %q, want %q", loaded.SchemaVersion, cfg.SchemaVersion)
		}
		if len(loaded.Denylist) != len(cfg.Denylist) {
			t.Errorf("round-trip Denylist len = %d, want %d", len(loaded.Denylist), len(cfg.Denylist))
		}
	})
}

// SCEN-1.2.2 / AC2: 默认 denylist 含 PRD §Constraints 安全列出的全部敏感路径。
func TestDefaultDenylistComplete(t *testing.T) {
	t.Run("TEST-1.2.2: DefaultDenylist 含全部 16 项敏感路径且 DefaultConfig 带上", func(t *testing.T) {
		want := []string{
			".env", ".env.*", "*.pem", "*.key", "*.p12", "*.pfx",
			"id_rsa", "id_ed25519", ".ssh/", ".git/objects/",
			"node_modules/", "target/", "dist/", "build/", ".cache/", "vendor/",
		}

		got := DefaultDenylist()
		set := make(map[string]bool, len(got))
		for _, p := range got {
			set[p] = true
		}
		for _, w := range want {
			if !set[w] {
				t.Errorf("默认 denylist 缺敏感路径 %q (got=%v)", w, got)
			}
		}

		if dc := DefaultConfig(); len(dc.Denylist) < len(want) {
			t.Errorf("DefaultConfig().Denylist len = %d, want >= %d", len(dc.Denylist), len(want))
		}
	})
}

// SCEN-1.2.3 / AC3: collection 采用 allowlist 路径导入模型；用户覆盖 denylist 需显式确认。
func TestAllowlistImportModel(t *testing.T) {
	t.Run("TEST-1.2.3: collection allowlist 模型 + AllowDenylistOverride 默认 false 且 Save/Load 保真", func(t *testing.T) {
		if DefaultConfig().AllowDenylistOverride {
			t.Errorf("AllowDenylistOverride 默认应为 false（覆盖默认 denylist 须显式确认）")
		}

		root := t.TempDir()
		c := DefaultConfig()
		c.DataDir = root
		c.Collections = []CollectionConfig{{
			ID:         "proj_x",
			Allowlist:  []string{"/home/u/proj_x/src", "/home/u/proj_x/docs"},
			AgentScope: []string{"openclaw", "hermes"},
		}}
		c.AllowDenylistOverride = true // 用户显式确认覆盖默认 denylist

		if err := Save(root, c); err != nil {
			t.Fatalf("Save error = %v", err)
		}
		got, err := Load(root)
		if err != nil {
			t.Fatalf("Load error = %v", err)
		}
		if len(got.Collections) != 1 || got.Collections[0].ID != "proj_x" {
			t.Fatalf("collection 未保真: %+v", got.Collections)
		}
		if len(got.Collections[0].Allowlist) != 2 || len(got.Collections[0].AgentScope) != 2 {
			t.Errorf("allowlist/agent_scope 未保真: %+v", got.Collections[0])
		}
		if !got.AllowDenylistOverride {
			t.Errorf("AllowDenylistOverride 显式置 true 后应被持久化")
		}
	})
}

// SCEN-1.2.4 / AC4: config.toml 与 token 文件权限为 0600（数据目录 0700）。
func TestFilePermissions(t *testing.T) {
	t.Run("TEST-1.2.4: Init 后 config.toml 权限 0600、数据目录 0700", func(t *testing.T) {
		if runtime.GOOS == "windows" {
			t.Skip("POSIX 权限语义不适用 Windows（v0.1 P0 = Linux x86_64）")
		}
		root := t.TempDir()
		if _, err := Init(root); err != nil {
			t.Fatalf("Init error = %v", err)
		}

		fi, err := os.Stat(filepath.Join(root, "config.toml"))
		if err != nil {
			t.Fatalf("stat config.toml: %v", err)
		}
		if perm := fi.Mode().Perm(); perm != FileMode {
			t.Errorf("config.toml perm = %#o, want %#o", perm, FileMode)
		}

		di, err := os.Stat(root)
		if err != nil {
			t.Fatalf("stat data dir: %v", err)
		}
		if perm := di.Mode().Perm(); perm != DirMode {
			t.Errorf("data dir perm = %#o, want %#o", perm, DirMode)
		}
	})
}

// SCEN-1.2.5 / AC5: 远程 provider 配置默认关闭，须显式 opt-in 字段才启用。
func TestRemoteProviderDefaultOff(t *testing.T) {
	t.Run("TEST-1.2.5: 远程 provider 默认关，显式 opt-in 后 RemoteEnabled()==true 并 Save/Load 保真", func(t *testing.T) {
		dc := DefaultConfig()
		if dc.Remote.Enabled {
			t.Errorf("远程 provider 默认应关闭，但 Remote.Enabled=true")
		}
		if dc.RemoteEnabled() {
			t.Errorf("RemoteEnabled() 默认应为 false")
		}

		root := t.TempDir()
		c := dc
		c.DataDir = root
		c.Remote = RemoteProviderConfig{
			Enabled:  true,
			Provider: "openai-compatible",
			Endpoint: "http://127.0.0.1:1234",
		}
		if !c.RemoteEnabled() {
			t.Errorf("显式 opt-in (Enabled=true) 后 RemoteEnabled() 应为 true")
		}
		if err := Save(root, c); err != nil {
			t.Fatalf("Save error = %v", err)
		}
		got, err := Load(root)
		if err != nil {
			t.Fatalf("Load error = %v", err)
		}
		if !got.Remote.Enabled || got.Remote.Provider != "openai-compatible" {
			t.Errorf("远程 provider opt-in 未保真: %+v", got.Remote)
		}
	})
}

// SCEN-22.1.1 / AC1 (task-22.1): add-only [embedding] 段（provider/dim）round-trip——含段时保真、
// 不含段时仍合法加载（缺省 Provider=""/Dim=0），且既有 [remote]/[[collections]] 段互不影响。
func TestTask221EmbeddingConfig(t *testing.T) {
	t.Run("TEST-22.1.1: 默认 Embedding 空 provider/0 维，显式设置后 Save/Load 保真（既有段不受影响）", func(t *testing.T) {
		dc := DefaultConfig()
		if dc.Embedding.Provider != "" || dc.Embedding.Dim != 0 {
			t.Errorf("默认 Embedding 应为空 provider/0 维，got %+v", dc.Embedding)
		}

		root := t.TempDir()
		c := dc
		c.DataDir = root
		c.Embedding = EmbeddingConfig{Provider: "fastembed", Dim: 512}
		// 既有 [remote] / [[collections]] 段同时存在，验证 [embedding] 加入后互不影响。
		c.Remote = RemoteProviderConfig{Enabled: true, Provider: "openai-compatible", Endpoint: "http://127.0.0.1:1234"}
		c.Collections = []CollectionConfig{{ID: "proj_x", Allowlist: []string{"/home/u/proj_x"}, AgentScope: []string{"hermes"}}}

		if err := Save(root, c); err != nil {
			t.Fatalf("Save error = %v", err)
		}
		got, err := Load(root)
		if err != nil {
			t.Fatalf("Load error = %v", err)
		}
		if got.Embedding.Provider != "fastembed" || got.Embedding.Dim != 512 {
			t.Errorf("[embedding] 段未保真: %+v", got.Embedding)
		}
		if !got.Remote.Enabled || got.Remote.Provider != "openai-compatible" {
			t.Errorf("[embedding] 加入后 [remote] 段受影响: %+v", got.Remote)
		}
		if len(got.Collections) != 1 || got.Collections[0].ID != "proj_x" {
			t.Errorf("[embedding] 加入后 [[collections]] 段受影响: %+v", got.Collections)
		}
	})

	t.Run("TEST-22.1.1: 不含 [embedding] 段的旧 config 仍合法加载（向后兼容 Provider=\"\"/Dim=0）", func(t *testing.T) {
		root := t.TempDir()
		legacy := "schema_version = \"0.1\"\n" +
			"data_dir = " + tomlQuote(root) + "\n" +
			"allow_denylist_override = false\n" +
			"denylist = []\n" +
			"\n[remote]\n" +
			"enabled = false\n" +
			"provider = \"\"\n" +
			"endpoint = \"\"\n"
		if err := os.WriteFile(filepath.Join(root, "config.toml"), []byte(legacy), 0o600); err != nil {
			t.Fatalf("write legacy config: %v", err)
		}
		got, err := Load(root)
		if err != nil {
			t.Fatalf("Load legacy (no [embedding]) error = %v", err)
		}
		if got.Embedding.Provider != "" || got.Embedding.Dim != 0 {
			t.Errorf("不含 [embedding] 段应得空 provider/0 维，got %+v", got.Embedding)
		}
	})
}

// TEST-37.2.1 (task-37.2 / ADR-042 D3): add-only [remote] model field Save/Load round-trip; absent
// model line ⇒ zero value (backward-compatible legacy config); existing [remote] Enabled/Provider/
// Endpoint + [embedding]/[vector]/[[collections]] sections are unaffected by adding model. The API
// key is NOT a config field and never round-trips through config.toml (security baseline).
func TestTask372RemoteModelConfig(t *testing.T) {
	t.Run("TEST-37.2.1: [remote] model 显式设置后 Save/Load 保真（既有字段/段不受影响）", func(t *testing.T) {
		dc := DefaultConfig()
		if dc.Remote.Model != "" {
			t.Errorf("默认 Remote.Model 应为空，got %q", dc.Remote.Model)
		}
		root := t.TempDir()
		c := dc
		c.DataDir = root
		c.Remote = RemoteProviderConfig{Enabled: true, Provider: "openai-compatible", Endpoint: "https://api.example.com/v1/embeddings", Model: "Qwen/Qwen3-Embedding-8B"}
		c.Embedding = EmbeddingConfig{Provider: "remote", Dim: 1024}
		c.Collections = []CollectionConfig{{ID: "proj_x", Allowlist: []string{"/home/u/proj_x"}}}
		if err := Save(root, c); err != nil {
			t.Fatalf("Save error = %v", err)
		}
		got, err := Load(root)
		if err != nil {
			t.Fatalf("Load error = %v", err)
		}
		if got.Remote.Model != "Qwen/Qwen3-Embedding-8B" {
			t.Errorf("[remote] model 未保真: %+v", got.Remote)
		}
		if !got.Remote.Enabled || got.Remote.Provider != "openai-compatible" || got.Remote.Endpoint != "https://api.example.com/v1/embeddings" {
			t.Errorf("加 model 后 [remote] 既有字段受影响: %+v", got.Remote)
		}
		if got.Embedding.Provider != "remote" || got.Embedding.Dim != 1024 {
			t.Errorf("加 model 后 [embedding] 段受影响: %+v", got.Embedding)
		}
		if len(got.Collections) != 1 || got.Collections[0].ID != "proj_x" {
			t.Errorf("加 model 后 [[collections]] 段受影响: %+v", got.Collections)
		}
	})

	t.Run("TEST-37.2.1: 不含 model 行的旧 config 仍合法加载（向后兼容 Model=\"\"）", func(t *testing.T) {
		root := t.TempDir()
		legacy := "schema_version = \"0.1\"\n" +
			"data_dir = " + tomlQuote(root) + "\n" +
			"allow_denylist_override = false\n" +
			"denylist = []\n" +
			"\n[remote]\n" +
			"enabled = true\n" +
			"provider = \"openai-compatible\"\n" +
			"endpoint = \"https://api.example.com/v1/embeddings\"\n"
		if err := os.WriteFile(filepath.Join(root, "config.toml"), []byte(legacy), 0o600); err != nil {
			t.Fatalf("write legacy config: %v", err)
		}
		got, err := Load(root)
		if err != nil {
			t.Fatalf("Load legacy (no model) error = %v", err)
		}
		if got.Remote.Model != "" {
			t.Errorf("不含 model 行应得 Model=\"\"，got %q", got.Remote.Model)
		}
		if got.Remote.Provider != "openai-compatible" || got.Remote.Endpoint != "https://api.example.com/v1/embeddings" {
			t.Errorf("不含 model 行的既有字段须保真，got %+v", got.Remote)
		}
	})
}

// TEST-38.2.1 (task-38.2 / ADR-043 D2): add-only [reranker] section Save/Load round-trip; absent
// section ⇒ zero value (backward-compatible); existing [remote]/[embedding]/[vector]/[[collections]]
// sections are unaffected by adding [reranker]. The API key is never a [reranker] field.
func TestTask382RerankerConfig(t *testing.T) {
	t.Run("TEST-38.2.1: [reranker] 段显式设置后 Save/Load 保真（既有段不受影响）", func(t *testing.T) {
		dc := DefaultConfig()
		if dc.Reranker.Enabled || dc.Reranker.Provider != "" || dc.Reranker.Endpoint != "" || dc.Reranker.Model != "" {
			t.Errorf("默认 Reranker 应为 zero value，got %+v", dc.Reranker)
		}
		root := t.TempDir()
		c := dc
		c.DataDir = root
		c.Reranker = RerankerConfig{Enabled: true, Provider: "siliconflow", Endpoint: "https://api.siliconflow.cn/v1/rerank", Model: "Qwen/Qwen3-VL-Reranker-8B"}
		c.Remote = RemoteProviderConfig{Enabled: true, Provider: "openai-compatible", Endpoint: "https://api.example.com/v1/embeddings", Model: "Qwen/Qwen3-Embedding-8B"}
		c.Embedding = EmbeddingConfig{Provider: "remote", Dim: 1024}
		c.Vector = VectorConfig{Backend: "qdrant", Dim: 384}
		c.Collections = []CollectionConfig{{ID: "proj_x", Allowlist: []string{"/home/u/proj_x"}}}
		if err := Save(root, c); err != nil {
			t.Fatalf("Save error = %v", err)
		}
		got, err := Load(root)
		if err != nil {
			t.Fatalf("Load error = %v", err)
		}
		if !got.Reranker.Enabled || got.Reranker.Provider != "siliconflow" || got.Reranker.Endpoint != "https://api.siliconflow.cn/v1/rerank" || got.Reranker.Model != "Qwen/Qwen3-VL-Reranker-8B" {
			t.Errorf("[reranker] 段未保真: %+v", got.Reranker)
		}
		if !got.Remote.Enabled || got.Remote.Model != "Qwen/Qwen3-Embedding-8B" {
			t.Errorf("加 [reranker] 后 [remote] 段受影响: %+v", got.Remote)
		}
		if got.Embedding.Provider != "remote" || got.Embedding.Dim != 1024 {
			t.Errorf("加 [reranker] 后 [embedding] 段受影响: %+v", got.Embedding)
		}
		if got.Vector.Backend != "qdrant" || got.Vector.Dim != 384 {
			t.Errorf("加 [reranker] 后 [vector] 段受影响: %+v", got.Vector)
		}
		if len(got.Collections) != 1 || got.Collections[0].ID != "proj_x" {
			t.Errorf("加 [reranker] 后 [[collections]] 段受影响: %+v", got.Collections)
		}
	})

	t.Run("TEST-38.2.1: 不含 [reranker] 段的旧 config 仍合法加载（向后兼容 zero value）", func(t *testing.T) {
		root := t.TempDir()
		legacy := "schema_version = \"0.1\"\n" +
			"data_dir = " + tomlQuote(root) + "\n" +
			"allow_denylist_override = false\n" +
			"denylist = []\n" +
			"\n[remote]\n" +
			"enabled = true\n" +
			"provider = \"openai-compatible\"\n" +
			"endpoint = \"https://api.example.com/v1/embeddings\"\n"
		if err := os.WriteFile(filepath.Join(root, "config.toml"), []byte(legacy), 0o600); err != nil {
			t.Fatalf("write legacy config: %v", err)
		}
		got, err := Load(root)
		if err != nil {
			t.Fatalf("Load legacy (no [reranker]) error = %v", err)
		}
		if got.Reranker.Enabled || got.Reranker.Provider != "" || got.Reranker.Endpoint != "" || got.Reranker.Model != "" {
			t.Errorf("不含 [reranker] 段应得 zero value，got %+v", got.Reranker)
		}
		if got.Remote.Provider != "openai-compatible" || got.Remote.Endpoint != "https://api.example.com/v1/embeddings" {
			t.Errorf("不含 [reranker] 段的既有 [remote] 字段须保真，got %+v", got.Remote)
		}
	})
}

// TEST-34.2.1 (task-34.2 / ADR-039 D2): add-only [vector] section Save/Load round-trip; absent
// section ⇒ zero value (backward-compatible); existing [remote]/[embedding]/[[collections]]
// sections are unaffected by adding [vector].
func TestTask342VectorConfig(t *testing.T) {
	t.Run("TEST-34.2.1: 默认 Vector 空 backend/0 维，显式设置后 Save/Load 保真（既有段不受影响）", func(t *testing.T) {
		dc := DefaultConfig()
		if dc.Vector.Backend != "" || dc.Vector.Dim != 0 {
			t.Errorf("默认 Vector 应为空 backend/0 维，got %+v", dc.Vector)
		}

		root := t.TempDir()
		c := dc
		c.DataDir = root
		c.Vector = VectorConfig{Backend: "qdrant", Dim: 384}
		c.Embedding = EmbeddingConfig{Provider: "fastembed", Dim: 512}
		c.Remote = RemoteProviderConfig{Enabled: true, Provider: "openai-compatible", Endpoint: "http://127.0.0.1:1234"}
		c.Collections = []CollectionConfig{{ID: "proj_x", Allowlist: []string{"/home/u/proj_x"}, AgentScope: []string{"hermes"}}}

		if err := Save(root, c); err != nil {
			t.Fatalf("Save error = %v", err)
		}
		got, err := Load(root)
		if err != nil {
			t.Fatalf("Load error = %v", err)
		}
		if got.Vector.Backend != "qdrant" || got.Vector.Dim != 384 {
			t.Errorf("[vector] 段未保真: %+v", got.Vector)
		}
		if got.Embedding.Provider != "fastembed" || got.Embedding.Dim != 512 {
			t.Errorf("[vector] 加入后 [embedding] 段受影响: %+v", got.Embedding)
		}
		if !got.Remote.Enabled || len(got.Collections) != 1 || got.Collections[0].ID != "proj_x" {
			t.Errorf("[vector] 加入后 [remote]/[[collections]] 段受影响: remote=%+v cols=%+v", got.Remote, got.Collections)
		}
	})

	t.Run("TEST-34.2.1: 不含 [vector] 段的旧 config 仍合法加载（向后兼容 Backend=\"\"/Dim=0）", func(t *testing.T) {
		root := t.TempDir()
		legacy := "schema_version = \"0.1\"\n" +
			"data_dir = " + tomlQuote(root) + "\n" +
			"allow_denylist_override = false\n" +
			"denylist = []\n" +
			"\n[remote]\n" +
			"enabled = false\n" +
			"provider = \"\"\n" +
			"endpoint = \"\"\n"
		if err := os.WriteFile(filepath.Join(root, "config.toml"), []byte(legacy), 0o600); err != nil {
			t.Fatalf("write legacy config: %v", err)
		}
		got, err := Load(root)
		if err != nil {
			t.Fatalf("Load legacy (no [vector]) error = %v", err)
		}
		if got.Vector.Backend != "" || got.Vector.Dim != 0 {
			t.Errorf("不含 [vector] 段应得空 backend/0 维，got %+v", got.Vector)
		}
	})
}
