// Package config implements ContextForge's local-first TOML configuration:
// default config.toml + data-dir scaffold, the default denylist, the allowlist
// import model, 0600/0700 permission policy, and remote-provider opt-in.
//
// task-1.2 (Phase 1 foundation). Contract: docs/specs/tasks/task-1.2-config.md §5.3.
// v0.1 uses a hand-rolled minimal TOML codec (stdlib only, no third-party
// dependency — §2A user decision, avoids R7).
//
// NOTE: this file is the §2.5.1 RED skeleton — signatures compile, bodies
// deliberately panic("unimplemented") so TEST-1.2.* fail functionally (not by
// compile error). Step 7 (GREEN) replaces the bodies with the real impl.
package config

const (
	// SchemaVersion is the canonical config/record schema version (frozen by task-1.1).
	SchemaVersion = "0.1"
	// FileMode is the permission for config.toml and the runtime token file (AC4).
	FileMode = 0o600
	// DirMode is the permission for the ~/.contextforge data directory tree (AC4).
	DirMode = 0o700
)

// Config is the root ContextForge configuration (one ~/.contextforge/config.toml).
type Config struct {
	SchemaVersion         string             // "0.1"
	DataDir               string             // ~/.contextforge
	Denylist              []string           // default contains AC2's full sensitive-path set
	AllowDenylistOverride bool               // AC3: explicit confirmation to override default denylist (default false)
	Collections           []CollectionConfig // AC3: allowlist path import model
	Remote                RemoteProviderConfig
}

// CollectionConfig is one indexed collection's allowlist import scope.
type CollectionConfig struct {
	ID         string
	Allowlist  []string // allowed import path prefixes
	AgentScope []string
}

// RemoteProviderConfig holds remote embedding/reranker provider settings.
// Disabled by default; only an explicit opt-in (Enabled=true) activates it (AC5).
type RemoteProviderConfig struct {
	Enabled  bool
	Provider string
	Endpoint string
}

// DefaultRootDir returns the default data root (~/.contextforge).
func DefaultRootDir() (string, error) {
	panic("unimplemented: config.DefaultRootDir (task-1.2 RED skeleton)")
}

// DefaultDenylist returns the built-in sensitive-path denylist (AC2).
func DefaultDenylist() []string {
	panic("unimplemented: config.DefaultDenylist (task-1.2 RED skeleton)")
}

// DefaultConfig returns the default Config (default denylist, remote disabled).
func DefaultConfig() Config {
	panic("unimplemented: config.DefaultConfig (task-1.2 RED skeleton)")
}

// Init generates root/config.toml + the collections/ logs/ runtime/ scaffold
// (files 0600, dirs 0700); if config.toml already exists it is loaded, not
// overwritten (AC1/AC4).
func Init(root string) (Config, error) {
	panic("unimplemented: config.Init (task-1.2 RED skeleton)")
}

// Load reads root/config.toml (round-trips with Save) (AC1).
func Load(root string) (Config, error) {
	panic("unimplemented: config.Load (task-1.2 RED skeleton)")
}

// Save writes c to root/config.toml with permission 0600 (AC4).
func Save(root string, c Config) error {
	panic("unimplemented: config.Save (task-1.2 RED skeleton)")
}

// RemoteEnabled reports whether the remote provider has been explicitly enabled (AC5).
func (c Config) RemoteEnabled() bool {
	panic("unimplemented: config.Config.RemoteEnabled (task-1.2 RED skeleton)")
}
