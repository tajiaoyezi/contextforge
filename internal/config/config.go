// Package config implements ContextForge's local-first TOML configuration:
// default config.toml + data-dir scaffold, the default denylist, the allowlist
// import model, 0600/0700 permission policy, and remote-provider opt-in.
//
// task-1.2 (Phase 1 foundation). Contract: docs/specs/tasks/task-1.2-config.md §5.3.
// v0.1 uses a hand-rolled minimal TOML codec (stdlib only, no third-party
// dependency — §2A user decision, avoids R7). The codec covers exactly the
// fixed config schema below (top-level scalars/string-arrays, a [remote] table,
// and [[collections]] array-of-tables) and is an exact Save/Load inverse.
package config

import (
	"bufio"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

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
	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("config: resolve home dir: %w", err)
	}
	return filepath.Join(home, ".contextforge"), nil
}

// DefaultDenylist returns the built-in sensitive-path denylist (AC2), exactly
// the set enumerated in PRD §Constraints 安全 (16 entries).
func DefaultDenylist() []string {
	return []string{
		".env", ".env.*", "*.pem", "*.key", "*.p12", "*.pfx",
		"id_rsa", "id_ed25519", ".ssh/", ".git/objects/",
		"node_modules/", "target/", "dist/", "build/", ".cache/", "vendor/",
	}
}

// DefaultConfig returns the default Config: full default denylist, override
// confirmation off, and the remote provider disabled (AC2/AC3/AC5).
func DefaultConfig() Config {
	return Config{
		SchemaVersion:         SchemaVersion,
		Denylist:              DefaultDenylist(),
		AllowDenylistOverride: false,
		Remote:                RemoteProviderConfig{Enabled: false},
	}
}

// RemoteEnabled reports whether the remote provider has been explicitly enabled (AC5).
func (c Config) RemoteEnabled() bool { return c.Remote.Enabled }

func configPath(root string) string { return filepath.Join(root, "config.toml") }

// Init generates root/config.toml + the collections/ logs/ runtime/ scaffold
// (files 0600, dirs 0700). If config.toml already exists it is loaded, not
// overwritten (AC1/AC4).
func Init(root string) (Config, error) {
	if root == "" {
		return Config{}, errors.New("config: Init: empty root")
	}
	if err := mkdirMode(root); err != nil {
		return Config{}, err
	}
	for _, d := range []string{"collections", "logs", "runtime"} {
		if err := mkdirMode(filepath.Join(root, d)); err != nil {
			return Config{}, err
		}
	}
	if _, err := os.Stat(configPath(root)); err == nil {
		return Load(root) // 已存在 → 不覆盖
	} else if !errors.Is(err, os.ErrNotExist) {
		return Config{}, fmt.Errorf("config: stat config.toml: %w", err)
	}
	c := DefaultConfig()
	c.DataDir = root
	if err := Save(root, c); err != nil {
		return Config{}, err
	}
	return c, nil
}

// Save writes c to root/config.toml with permission 0600 (AC4).
func Save(root string, c Config) error {
	if root == "" {
		return errors.New("config: Save: empty root")
	}
	if err := mkdirMode(root); err != nil {
		return err
	}
	p := configPath(root)
	if err := os.WriteFile(p, []byte(encodeTOML(c)), FileMode); err != nil {
		return fmt.Errorf("config: write %q: %w", p, err)
	}
	if err := os.Chmod(p, FileMode); err != nil { // 抵消 umask，保证恰为 0600
		return fmt.Errorf("config: chmod %q: %w", p, err)
	}
	return nil
}

// Load reads root/config.toml; round-trips with Save (AC1).
func Load(root string) (Config, error) {
	if root == "" {
		return Config{}, errors.New("config: Load: empty root")
	}
	p := configPath(root)
	f, err := os.Open(p)
	if err != nil {
		return Config{}, fmt.Errorf("config: open %q: %w", p, err)
	}
	defer f.Close()

	c, err := decodeTOML(bufio.NewScanner(f))
	if err != nil {
		return Config{}, fmt.Errorf("config: parse %q: %w", p, err)
	}
	if c.DataDir == "" {
		c.DataDir = root
	}
	return c, nil
}

// mkdirMode creates dir (and parents) and forces its permission to DirMode
// (MkdirAll honours umask, so an explicit Chmod is needed for exact 0700).
func mkdirMode(dir string) error {
	if err := os.MkdirAll(dir, DirMode); err != nil {
		return fmt.Errorf("config: mkdir %q: %w", dir, err)
	}
	if err := os.Chmod(dir, DirMode); err != nil {
		return fmt.Errorf("config: chmod %q: %w", dir, err)
	}
	return nil
}

// ---- minimal TOML codec (exact inverse pair for the schema above) ----

func encodeTOML(c Config) string {
	var b strings.Builder
	b.WriteString("# ContextForge config (schema_version " + SchemaVersion + "). Managed file — edit with care.\n")
	fmt.Fprintf(&b, "schema_version = %s\n", tomlQuote(c.SchemaVersion))
	fmt.Fprintf(&b, "data_dir = %s\n", tomlQuote(c.DataDir))
	fmt.Fprintf(&b, "allow_denylist_override = %s\n", strconv.FormatBool(c.AllowDenylistOverride))
	fmt.Fprintf(&b, "denylist = %s\n", tomlStringArray(c.Denylist))
	b.WriteString("\n[remote]\n")
	fmt.Fprintf(&b, "enabled = %s\n", strconv.FormatBool(c.Remote.Enabled))
	fmt.Fprintf(&b, "provider = %s\n", tomlQuote(c.Remote.Provider))
	fmt.Fprintf(&b, "endpoint = %s\n", tomlQuote(c.Remote.Endpoint))
	for _, col := range c.Collections {
		b.WriteString("\n[[collections]]\n")
		fmt.Fprintf(&b, "id = %s\n", tomlQuote(col.ID))
		fmt.Fprintf(&b, "allowlist = %s\n", tomlStringArray(col.Allowlist))
		fmt.Fprintf(&b, "agent_scope = %s\n", tomlStringArray(col.AgentScope))
	}
	return b.String()
}

func decodeTOML(sc *bufio.Scanner) (Config, error) {
	var c Config
	section := "" // "" | "remote" | "collections"
	var cur *CollectionConfig
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		switch {
		case line == "[remote]":
			section = "remote"
			continue
		case line == "[[collections]]":
			section = "collections"
			c.Collections = append(c.Collections, CollectionConfig{})
			cur = &c.Collections[len(c.Collections)-1]
			continue
		case strings.HasPrefix(line, "["):
			return Config{}, fmt.Errorf("unknown section %q", line)
		}

		key, raw, ok := strings.Cut(line, "=")
		if !ok {
			return Config{}, fmt.Errorf("malformed line %q", line)
		}
		key = strings.TrimSpace(key)
		raw = strings.TrimSpace(raw)

		switch section {
		case "":
			if err := assignTop(&c, key, raw); err != nil {
				return Config{}, err
			}
		case "remote":
			if err := assignRemote(&c.Remote, key, raw); err != nil {
				return Config{}, err
			}
		case "collections":
			if cur == nil {
				return Config{}, errors.New("collection key before [[collections]] header")
			}
			if err := assignCollection(cur, key, raw); err != nil {
				return Config{}, err
			}
		}
	}
	if err := sc.Err(); err != nil {
		return Config{}, err
	}
	return c, nil
}

func assignTop(c *Config, key, raw string) error {
	switch key {
	case "schema_version":
		s, err := parseTOMLString(raw)
		if err != nil {
			return err
		}
		c.SchemaVersion = s
	case "data_dir":
		s, err := parseTOMLString(raw)
		if err != nil {
			return err
		}
		c.DataDir = s
	case "allow_denylist_override":
		v, err := strconv.ParseBool(raw)
		if err != nil {
			return fmt.Errorf("allow_denylist_override: %w", err)
		}
		c.AllowDenylistOverride = v
	case "denylist":
		arr, err := parseTOMLStringArray(raw)
		if err != nil {
			return err
		}
		c.Denylist = arr
	default:
		// v0.1: 容忍未识别字段（PRD：不影响核心字段）
	}
	return nil
}

func assignRemote(r *RemoteProviderConfig, key, raw string) error {
	switch key {
	case "enabled":
		v, err := strconv.ParseBool(raw)
		if err != nil {
			return fmt.Errorf("remote.enabled: %w", err)
		}
		r.Enabled = v
	case "provider":
		s, err := parseTOMLString(raw)
		if err != nil {
			return err
		}
		r.Provider = s
	case "endpoint":
		s, err := parseTOMLString(raw)
		if err != nil {
			return err
		}
		r.Endpoint = s
	}
	return nil
}

func assignCollection(col *CollectionConfig, key, raw string) error {
	switch key {
	case "id":
		s, err := parseTOMLString(raw)
		if err != nil {
			return err
		}
		col.ID = s
	case "allowlist":
		arr, err := parseTOMLStringArray(raw)
		if err != nil {
			return err
		}
		col.Allowlist = arr
	case "agent_scope":
		arr, err := parseTOMLStringArray(raw)
		if err != nil {
			return err
		}
		col.AgentScope = arr
	}
	return nil
}

func tomlQuote(s string) string {
	var b strings.Builder
	b.WriteByte('"')
	for _, r := range s {
		switch r {
		case '\\':
			b.WriteString(`\\`)
		case '"':
			b.WriteString(`\"`)
		case '\n':
			b.WriteString(`\n`)
		case '\r':
			b.WriteString(`\r`)
		case '\t':
			b.WriteString(`\t`)
		default:
			b.WriteRune(r)
		}
	}
	b.WriteByte('"')
	return b.String()
}

func tomlStringArray(ss []string) string {
	parts := make([]string, len(ss))
	for i, s := range ss {
		parts[i] = tomlQuote(s)
	}
	return "[" + strings.Join(parts, ", ") + "]"
}

func parseTOMLString(raw string) (string, error) {
	if len(raw) < 2 || raw[0] != '"' || raw[len(raw)-1] != '"' {
		return "", fmt.Errorf("not a quoted string: %q", raw)
	}
	body := raw[1 : len(raw)-1]
	var b strings.Builder
	for i := 0; i < len(body); i++ {
		ch := body[i]
		if ch != '\\' {
			b.WriteByte(ch)
			continue
		}
		i++
		if i >= len(body) {
			return "", fmt.Errorf("dangling escape in %q", raw)
		}
		switch body[i] {
		case '\\':
			b.WriteByte('\\')
		case '"':
			b.WriteByte('"')
		case 'n':
			b.WriteByte('\n')
		case 'r':
			b.WriteByte('\r')
		case 't':
			b.WriteByte('\t')
		default:
			return "", fmt.Errorf("unknown escape \\%c", body[i])
		}
	}
	return b.String(), nil
}

func parseTOMLStringArray(raw string) ([]string, error) {
	raw = strings.TrimSpace(raw)
	if len(raw) < 2 || raw[0] != '[' || raw[len(raw)-1] != ']' {
		return nil, fmt.Errorf("not an array: %q", raw)
	}
	inner := strings.TrimSpace(raw[1 : len(raw)-1])
	if inner == "" {
		return []string{}, nil
	}
	var items []string
	var buf strings.Builder
	inStr, esc := false, false
	for i := 0; i < len(inner); i++ {
		ch := inner[i]
		buf.WriteByte(ch)
		if esc {
			esc = false
			continue
		}
		switch {
		case ch == '\\':
			esc = true
		case ch == '"':
			inStr = !inStr
		case ch == ',' && !inStr:
			s := buf.String()
			items = append(items, strings.TrimSpace(s[:len(s)-1]))
			buf.Reset()
		}
	}
	if last := strings.TrimSpace(buf.String()); last != "" {
		items = append(items, last)
	}
	out := make([]string, 0, len(items))
	for _, it := range items {
		s, err := parseTOMLString(it)
		if err != nil {
			return nil, err
		}
		out = append(out, s)
	}
	return out, nil
}
