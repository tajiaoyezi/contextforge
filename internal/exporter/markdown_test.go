package exporter

import (
	"archive/tar"
	"bytes"
	"compress/gzip"
	"encoding/json"
	"io"
	"strings"
	"testing"
)

func readTarGz(t *testing.T, data []byte) map[string]string {
	t.Helper()
	gzr, err := gzip.NewReader(bytes.NewReader(data))
	if err != nil {
		t.Fatalf("gzip reader: %v", err)
	}
	defer gzr.Close()

	files := make(map[string]string)
	tr := tar.NewReader(gzr)
	for {
		hdr, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			t.Fatalf("tar next: %v", err)
		}
		body, err := io.ReadAll(tr)
		if err != nil {
			t.Fatalf("read tar entry %s: %v", hdr.Name, err)
		}
		files[hdr.Name] = string(body)
	}
	return files
}

// TEST-6.3.1 / SCEN-6.3.1 / AC1
func TestTask63_AC1_MarkdownBundleTarGzAndManifest(t *testing.T) {
	records := sampleRecords(t, 2)

	var buf bytes.Buffer
	if err := writeMarkdownBundle(records, &buf); err != nil {
		t.Fatalf("writeMarkdownBundle: %v", err)
	}

	files := readTarGz(t, buf.Bytes())
	manifestRaw, ok := files["manifest.json"]
	if !ok {
		t.Fatalf("bundle missing manifest.json; entries=%v", files)
	}
	var manifest struct {
		Format  string `json:"format"`
		Records int    `json:"records"`
	}
	if err := json.Unmarshal([]byte(manifestRaw), &manifest); err != nil {
		t.Fatalf("manifest is not valid JSON: %v\n%s", err, manifestRaw)
	}
	if manifest.Format != "markdown-bundle" {
		t.Fatalf("manifest.format=%q want markdown-bundle", manifest.Format)
	}
	if manifest.Records != len(records) {
		t.Fatalf("manifest.records=%d want %d", manifest.Records, len(records))
	}

	var mdBody string
	for name, body := range files {
		if strings.HasSuffix(name, ".md") {
			mdBody += body
		}
	}
	for _, want := range []string{
		"---",
		"id: ctx-00",
		"collection_id: default",
		"agent_scope:",
		"fixture content 00",
		"## Provenance",
	} {
		if !strings.Contains(mdBody, want) {
			t.Fatalf("markdown bundle missing %q\nfull markdown:\n%s", want, mdBody)
		}
	}
}
