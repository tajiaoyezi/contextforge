package mcpadapter

import (
	"encoding/json"
	"os"
	"strconv"
	"strings"
)

// AllowlistEntry is one entry in <data_dir>/mcp-allowlist.json.
type AllowlistEntry struct {
	Name    string `json:"name"`
	Version string `json:"version,omitempty"`
}

// LoadAllowlist reads a JSON array of MCP client allowlist entries. A missing
// file is intentionally interpreted as an empty allowlist, which rejects every
// client until the user explicitly opts in.
func LoadAllowlist(path string) ([]AllowlistEntry, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}
		return nil, err
	}
	var entries []AllowlistEntry
	if err := json.Unmarshal(b, &entries); err != nil {
		return nil, err
	}
	out := entries[:0]
	for _, entry := range entries {
		entry.Name = strings.TrimSpace(entry.Name)
		entry.Version = strings.TrimSpace(entry.Version)
		if entry.Name != "" {
			out = append(out, entry)
		}
	}
	return out, nil
}

// IsAllowlisted matches client name exactly and supports either an empty
// version (any client version), exact version equality, or >=X.Y.Z.
func IsAllowlisted(entry AllowlistEntry, allowlist []AllowlistEntry) bool {
	if len(allowlist) == 0 || strings.TrimSpace(entry.Name) == "" {
		return false
	}
	for _, allowed := range allowlist {
		if allowed.Name != entry.Name {
			continue
		}
		if allowed.Version == "" {
			return true
		}
		if entry.Version == "" {
			return false
		}
		if strings.HasPrefix(allowed.Version, ">=") {
			min := strings.TrimSpace(strings.TrimPrefix(allowed.Version, ">="))
			return compareSemver(entry.Version, min) >= 0
		}
		return normalizeVersion(entry.Version) == normalizeVersion(allowed.Version)
	}
	return false
}

func normalizeVersion(v string) string {
	return strings.TrimPrefix(strings.TrimSpace(v), "v")
}

func compareSemver(a, b string) int {
	av := parseSemver(normalizeVersion(a))
	bv := parseSemver(normalizeVersion(b))
	for i := 0; i < 3; i++ {
		if av[i] < bv[i] {
			return -1
		}
		if av[i] > bv[i] {
			return 1
		}
	}
	return 0
}

func parseSemver(v string) [3]int {
	parts := strings.Split(v, ".")
	var out [3]int
	for i := 0; i < len(parts) && i < 3; i++ {
		n, err := strconv.Atoi(parts[i])
		if err != nil {
			return [3]int{-1, -1, -1}
		}
		out[i] = n
	}
	return out
}
