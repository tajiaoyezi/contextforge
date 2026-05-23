package exporter

import "testing"

// TEST-6.3.3 / SCEN-6.3.3 / AC3
func TestTask63_AC3_SecretScanHitsAndCleanContent(t *testing.T) {
	unsafe := []byte(`
AWS_ACCESS_KEY_ID=AKIA1234567890ABCDEF
Authorization: Bearer abcdefghijklmnopqrstuvwxyz123456
-----BEGIN PRIVATE KEY-----
fake
-----END PRIVATE KEY-----
GITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuvwxyzAB
api_key = sk_1234567890abcdefghijklmnop
password = never-print-this-secret
`)
	hits := ScanForSecrets(unsafe)
	for _, name := range []string{
		"aws_access_key",
		"bearer_token",
		"pem_private_key",
		"github_token",
		"generic_api_key",
		"password_literal",
	} {
		if !hasSecretHit(hits, name) {
			t.Fatalf("ScanForSecrets missing %s in hits %#v", name, hits)
		}
	}

	clean := []byte("normal redacted content with [REDACTED:GITHUB_TOKEN] placeholder")
	if got := ScanForSecrets(clean); len(got) != 0 {
		t.Fatalf("clean redacted content should have 0 hits, got %#v", got)
	}
}

func hasSecretHit(hits []SecretHit, name string) bool {
	for _, h := range hits {
		if h.PatternName == name {
			return true
		}
	}
	return false
}
