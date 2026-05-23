package exporter

import (
	"encoding/json"
	"io"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

func writeJSONL(records []*contextforgev1.ContextRecord, w io.Writer) error {
	enc := json.NewEncoder(w)
	enc.SetEscapeHTML(false)
	for _, rec := range records {
		if rec == nil {
			continue
		}
		if err := enc.Encode(rec); err != nil {
			return err
		}
	}
	return nil
}
