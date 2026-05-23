package exporter

import (
	"regexp"
	"sort"
)

// SecretHit is a safe summary of one sanity secret scanner hit.
type SecretHit struct {
	PatternName string
	Match       string
	Offset      int
}

type secretPattern struct {
	name string
	re   *regexp.Regexp
}

var secretPatterns = []secretPattern{
	{name: "pem_private_key", re: regexp.MustCompile(`-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----`)},
	{name: "aws_access_key", re: regexp.MustCompile(`AKIA[0-9A-Z]{16}`)},
	{name: "bearer_token", re: regexp.MustCompile(`Bearer[[:space:]]+[A-Za-z0-9+/=_-]{20,}`)},
	{name: "github_token", re: regexp.MustCompile(`gh[ouspr]_[A-Za-z0-9_]{36,}|github_pat_[A-Za-z0-9_]{22,}`)},
	{name: "generic_api_key", re: regexp.MustCompile(`(?i)(api[_-]?key|token|x-api-key)[[:space:]]*[:=][[:space:]]*[A-Za-z0-9_.:/+=-]{20,}`)},
	{name: "password_literal", re: regexp.MustCompile(`(?i)password[[:space:]]*[:=][[:space:]]*[^[:space:]]{8,}`)},
}

// ScanForSecrets performs the task-6.3 Go inline sanity hit-count check.
func ScanForSecrets(content []byte) []SecretHit {
	hits := make([]SecretHit, 0)
	for _, p := range secretPatterns {
		matches := p.re.FindAllIndex(content, -1)
		for _, m := range matches {
			hits = append(hits, SecretHit{
				PatternName: p.name,
				Match:       safeSnippet(content[m[0]:m[1]]),
				Offset:      m[0],
			})
		}
	}
	sort.SliceStable(hits, func(i, j int) bool {
		if hits[i].Offset != hits[j].Offset {
			return hits[i].Offset < hits[j].Offset
		}
		return hits[i].PatternName < hits[j].PatternName
	})
	return hits
}

func safeSnippet(b []byte) string {
	const max = 20
	if len(b) <= max {
		return string(b)
	}
	return string(b[:max]) + "..."
}
