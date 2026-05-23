package exporter

import (
	"archive/tar"
	"bufio"
	"bytes"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"io"
	"reflect"
	"strings"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/protobuf/proto"
)

const fidelityFieldCount = 23

// CalcFidelity reparses exported bytes and scores ContextRecord field parity.
func CalcFidelity(original []*contextforgev1.ContextRecord, exported []byte, format Format) (float64, error) {
	parsed, err := parseExportedRecords(exported, format)
	if err != nil {
		return 0, err
	}
	if len(original) == 0 {
		return 1, nil
	}
	byID := make(map[string]*contextforgev1.ContextRecord, len(parsed))
	for _, rec := range parsed {
		byID[rec.GetId()] = rec
	}

	matched := 0
	total := len(original) * fidelityFieldCount
	for _, orig := range original {
		if orig == nil {
			continue
		}
		exp := byID[orig.GetId()]
		matched += compareRecordFields(orig, exp)
	}
	return float64(matched) / float64(total), nil
}

func parseExportedRecords(exported []byte, format Format) ([]*contextforgev1.ContextRecord, error) {
	switch format {
	case FormatJSONL:
		return parseJSONL(exported)
	case FormatMarkdownBundle:
		return parseMarkdownBundle(exported)
	case FormatAgentDraft:
		return parseRecordJSONComments(exported)
	default:
		return nil, fmt.Errorf("unsupported export format %q", format)
	}
}

func parseJSONL(exported []byte) ([]*contextforgev1.ContextRecord, error) {
	out := make([]*contextforgev1.ContextRecord, 0)
	scanner := bufio.NewScanner(bytes.NewReader(exported))
	for scanner.Scan() {
		line := bytes.TrimSpace(scanner.Bytes())
		if len(line) == 0 {
			continue
		}
		var rec contextforgev1.ContextRecord
		if err := json.Unmarshal(line, &rec); err != nil {
			return nil, err
		}
		out = append(out, &rec)
	}
	return out, scanner.Err()
}

func parseMarkdownBundle(exported []byte) ([]*contextforgev1.ContextRecord, error) {
	gzr, err := gzip.NewReader(bytes.NewReader(exported))
	if err != nil {
		return nil, err
	}
	defer gzr.Close()
	tr := tar.NewReader(gzr)
	all := make([]byte, 0)
	for {
		hdr, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, err
		}
		if !strings.HasSuffix(hdr.Name, ".md") {
			continue
		}
		body, err := io.ReadAll(tr)
		if err != nil {
			return nil, err
		}
		all = append(all, body...)
		all = append(all, '\n')
	}
	return parseRecordJSONComments(all)
}

func parseRecordJSONComments(exported []byte) ([]*contextforgev1.ContextRecord, error) {
	const start = "<!-- contextforge-record-json\n"
	const end = "\n-->"
	out := make([]*contextforgev1.ContextRecord, 0)
	rest := string(exported)
	for {
		i := strings.Index(rest, start)
		if i < 0 {
			break
		}
		rest = rest[i+len(start):]
		j := strings.Index(rest, end)
		if j < 0 {
			return nil, fmt.Errorf("unterminated contextforge-record-json block")
		}
		var rec contextforgev1.ContextRecord
		if err := json.Unmarshal([]byte(rest[:j]), &rec); err != nil {
			return nil, err
		}
		out = append(out, &rec)
		rest = rest[j+len(end):]
	}
	return out, nil
}

func compareRecordFields(a, b *contextforgev1.ContextRecord) int {
	if a == nil || b == nil {
		return 0
	}
	score := 0
	add := func(ok bool) {
		if ok {
			score++
		}
	}
	add(a.GetId() == b.GetId())
	add(a.GetSchemaVersion() == b.GetSchemaVersion())
	add(a.GetCollectionId() == b.GetCollectionId())
	add(a.GetSourceType() == b.GetSourceType())
	add(a.GetSourceProvider() == b.GetSourceProvider())
	add(a.GetSourceUri() == b.GetSourceUri())
	add(reflect.DeepEqual(a.GetAgentScope(), b.GetAgentScope()))
	add(a.GetTitle() == b.GetTitle())
	add(a.GetContent() == b.GetContent())
	add(a.GetContentHash() == b.GetContentHash())
	add(a.GetRedactionStatus() == b.GetRedactionStatus())
	add(a.GetLanguage() == b.GetLanguage())
	add(a.GetFilePath() == b.GetFilePath())
	add(a.GetLineStart() == b.GetLineStart())
	add(a.GetLineEnd() == b.GetLineEnd())
	add(reflect.DeepEqual(a.GetTags(), b.GetTags()))
	add(provenanceEqual(a.GetProvenance(), b.GetProvenance()))
	add(reflect.DeepEqual(a.GetSecurityLabels(), b.GetSecurityLabels()))
	add(proto.Equal(a.GetCreatedAt(), b.GetCreatedAt()))
	add(proto.Equal(a.GetUpdatedAt(), b.GetUpdatedAt()))
	add(proto.Equal(a.GetExpiresAt(), b.GetExpiresAt()))
	add(a.GetVersion() == b.GetVersion())
	add(proto.Equal(a.GetMetadata(), b.GetMetadata()))
	return score
}

func provenanceEqual(a, b []*contextforgev1.Provenance) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if !proto.Equal(a[i], b[i]) {
			return false
		}
	}
	return true
}
