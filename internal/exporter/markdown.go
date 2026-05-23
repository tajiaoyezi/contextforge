package exporter

import (
	"archive/tar"
	"bytes"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"io"
	"regexp"
	"sort"
	"strings"
	"time"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/protobuf/types/known/timestamppb"
)

type manifest struct {
	Format  string         `json:"format"`
	Records int            `json:"records"`
	Files   []manifestFile `json:"files"`
}

type manifestFile struct {
	ID   string `json:"id"`
	Path string `json:"path"`
}

func writeMarkdownBundle(records []*contextforgev1.ContextRecord, w io.Writer) error {
	var payload bytes.Buffer
	gzw := gzip.NewWriter(&payload)
	// defer Close 增强稳健性：早 return 路径（writeTarFile err / recordMarkdown
	// err / manifest marshal err 等）也能保证 writer GC 之前关闭。tw/gzw 写
	// bytes.Buffer 无 OS fd 仅 GC-leak，但 PR #44 review FIX-3 防御性加固。
	defer gzw.Close()
	tw := tar.NewWriter(gzw)
	defer tw.Close()

	files := make([]manifestFile, 0, len(records))
	for i, rec := range records {
		if rec == nil {
			continue
		}
		name := fmt.Sprintf("records/%03d-%s.md", i, safeName(rec.GetId()))
		body, err := recordMarkdown(rec)
		if err != nil {
			return err
		}
		if err := writeTarFile(tw, name, body); err != nil {
			return err
		}
		files = append(files, manifestFile{ID: rec.GetId(), Path: name})
	}

	man := manifest{Format: string(FormatMarkdownBundle), Records: len(files), Files: files}
	body, err := json.MarshalIndent(man, "", "  ")
	if err != nil {
		return err
	}
	body = append(body, '\n')
	if err := writeTarFile(tw, "manifest.json", body); err != nil {
		return err
	}
	// 显式 Close 让 tar/gzip flush trailer 在 Write(payload) 之前完成（defer
	// 也会关，重复 close 无害但 trailer 时机决定 payload 完整性）。
	if err := tw.Close(); err != nil {
		return err
	}
	if err := gzw.Close(); err != nil {
		return err
	}
	_, err = w.Write(payload.Bytes())
	return err
}

func writeTarFile(tw *tar.Writer, name string, body []byte) error {
	hdr := &tar.Header{
		Name:    name,
		Mode:    0o600,
		Size:    int64(len(body)),
		ModTime: time.Unix(0, 0).UTC(),
	}
	if err := tw.WriteHeader(hdr); err != nil {
		return err
	}
	_, err := tw.Write(body)
	return err
}

func recordMarkdown(rec *contextforgev1.ContextRecord) ([]byte, error) {
	var b strings.Builder
	b.WriteString("---\n")
	writeYAMLRecord(&b, rec)
	b.WriteString("---\n\n")
	title := rec.GetTitle()
	if title == "" {
		title = rec.GetId()
	}
	b.WriteString("# ")
	b.WriteString(escapeMarkdownLine(title))
	b.WriteString("\n\n")
	if rec.GetContent() != "" {
		b.WriteString(rec.GetContent())
		if !strings.HasSuffix(rec.GetContent(), "\n") {
			b.WriteByte('\n')
		}
		b.WriteByte('\n')
	}
	b.WriteString("## Provenance\n\n")
	for _, p := range rec.GetProvenance() {
		b.WriteString("- ")
		b.WriteString(p.GetImporter())
		if p.GetOriginalPath() != "" {
			b.WriteString(": ")
			b.WriteString(p.GetOriginalPath())
		}
		b.WriteByte('\n')
	}
	if len(rec.GetProvenance()) == 0 {
		b.WriteString("- none\n")
	}
	b.WriteByte('\n')
	jsonRecord, err := json.Marshal(rec)
	if err != nil {
		return nil, err
	}
	b.WriteString("<!-- contextforge-record-json\n")
	b.Write(jsonRecord)
	b.WriteString("\n-->\n")
	return []byte(b.String()), nil
}

func writeYAMLRecord(b *strings.Builder, rec *contextforgev1.ContextRecord) {
	writeYAMLScalar(b, "id", rec.GetId())
	writeYAMLScalar(b, "schema_version", rec.GetSchemaVersion())
	writeYAMLScalar(b, "collection_id", rec.GetCollectionId())
	writeYAMLScalar(b, "source_type", rec.GetSourceType())
	writeYAMLScalar(b, "source_provider", rec.GetSourceProvider())
	writeYAMLScalar(b, "source_uri", rec.GetSourceUri())
	writeYAMLList(b, "agent_scope", rec.GetAgentScope())
	writeYAMLScalar(b, "title", rec.GetTitle())
	writeYAMLScalar(b, "content_hash", rec.GetContentHash())
	writeYAMLScalar(b, "redaction_status", rec.GetRedactionStatus())
	writeYAMLScalar(b, "language", rec.GetLanguage())
	writeYAMLScalar(b, "file_path", rec.GetFilePath())
	fmt.Fprintf(b, "line_start: %d\n", rec.GetLineStart())
	fmt.Fprintf(b, "line_end: %d\n", rec.GetLineEnd())
	writeYAMLList(b, "tags", rec.GetTags())
	writeYAMLList(b, "security_labels", rec.GetSecurityLabels())
	fmt.Fprintf(b, "version: %d\n", rec.GetVersion())
	writeYAMLScalar(b, "content", rec.GetContent())
	writeYAMLScalar(b, "provenance_json", jsonString(rec.GetProvenance()))
	writeYAMLScalar(b, "created_at", timestampString(rec.GetCreatedAt()))
	writeYAMLScalar(b, "updated_at", timestampString(rec.GetUpdatedAt()))
	writeYAMLScalar(b, "expires_at", timestampString(rec.GetExpiresAt()))
	writeYAMLScalar(b, "metadata_json", jsonString(rec.GetMetadata()))
}

func writeYAMLScalar(b *strings.Builder, key, value string) {
	if value == "" {
		fmt.Fprintf(b, "%s: \"\"\n", key)
		return
	}
	fmt.Fprintf(b, "%s: %s\n", key, quoteYAML(value))
}

func writeYAMLList(b *strings.Builder, key string, values []string) {
	b.WriteString(key)
	b.WriteString(":\n")
	if len(values) == 0 {
		b.WriteString("  []\n")
		return
	}
	for _, v := range values {
		b.WriteString("  - ")
		b.WriteString(quoteYAML(v))
		b.WriteByte('\n')
	}
}

func quoteYAML(s string) string {
	if s == "" {
		return `""`
	}
	if regexp.MustCompile(`^[A-Za-z0-9._:/-]+$`).MatchString(s) {
		return s
	}
	escaped := strings.ReplaceAll(s, `\`, `\\`)
	escaped = strings.ReplaceAll(escaped, `"`, `\"`)
	escaped = strings.ReplaceAll(escaped, "\n", `\n`)
	return `"` + escaped + `"`
}

func jsonString(v any) string {
	body, err := json.Marshal(v)
	if err != nil {
		return ""
	}
	return string(body)
}

func timestampString(ts *timestamppb.Timestamp) string {
	if ts == nil {
		return ""
	}
	return ts.AsTime().UTC().Format(time.RFC3339)
}

var unsafeNameRe = regexp.MustCompile(`[^A-Za-z0-9._-]+`)

func safeName(s string) string {
	s = strings.TrimSpace(s)
	if s == "" {
		return "record"
	}
	s = unsafeNameRe.ReplaceAllString(s, "-")
	s = strings.Trim(s, ".-")
	if s == "" {
		return "record"
	}
	return s
}

func escapeMarkdownLine(s string) string {
	return strings.ReplaceAll(s, "\n", " ")
}

func sortedKeys(m map[string][]byte) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}
