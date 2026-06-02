package lifecycle

import (
	"sort"
	"testing"
	"time"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
	"google.golang.org/protobuf/types/known/timestamppb"
)

// ---- Test helpers ----

// fakeOracle — 测试可注入的确定性 Oracle（避免依赖墙钟与真实 FS）.
type fakeOracle struct {
	now      time.Time
	exists   map[string]bool
	modTimes map[string]time.Time
}

func (f fakeOracle) Now() time.Time { return f.now }
func (f fakeOracle) Exists(path string) bool {
	v, ok := f.exists[path]
	return ok && v
}
func (f fakeOracle) ModTime(path string) (time.Time, bool) {
	t, ok := f.modTimes[path]
	return t, ok
}

// newRecord 构造测试用 ContextRecord（最小字段集合 — 测试关注的字段单独覆盖）.
func newRecord(id, hash, sourceURI, filePath string, expiresAt *time.Time, prov []*contextforgev1.Provenance) *contextforgev1.ContextRecord {
	rec := &contextforgev1.ContextRecord{
		Id:            id,
		SchemaVersion: "0.1",
		CollectionId:  "project-x",
		SourceType:    "memory",
		SourceUri:     sourceURI,
		Content:       "body for " + id,
		ContentHash:   hash,
		FilePath:      filePath,
		Provenance:    prov,
	}
	if expiresAt != nil {
		rec.ExpiresAt = timestamppb.New(*expiresAt)
	}
	return rec
}

func newProvenance(importer, originalPath string, sourceMod *time.Time) *contextforgev1.Provenance {
	p := &contextforgev1.Provenance{
		Importer:     importer,
		OriginalPath: originalPath,
		ImportedAt:   timestamppb.New(time.Unix(1, 0).UTC()),
	}
	if sourceMod != nil {
		p.SourceModifiedAt = timestamppb.New(*sourceMod)
	}
	return p
}

// findStaleMark — 在 marks 中查 RecordID + Reason 组合（顺序无关）.
func findStaleMark(marks []StaleMark, recordID string, reason StaleReason) *StaleMark {
	for i := range marks {
		if marks[i].RecordID == recordID && marks[i].Reason == reason {
			return &marks[i]
		}
	}
	return nil
}

// findConflict — 在 conflicts 中查 Key + KeyType 组合（顺序无关）.
func findConflict(conflicts []ConflictReport, key string, keyType ConflictKeyType) *ConflictReport {
	for i := range conflicts {
		if conflicts[i].Key == key && conflicts[i].KeyType == keyType {
			return &conflicts[i]
		}
	}
	return nil
}

// ---- TEST-5.2.1 / SCEN-5.2.1 (AC1) — stale 三触发可设/检索 ----
//
// 4 record 覆盖：expired / source-deleted / source-modified / 健康 — 前三应被
// StaleMark 标记（reason 各正确），第 4 不应被标。
func TestMark_StaleThreeTriggers(t *testing.T) {
	now := time.Date(2026, 5, 23, 12, 0, 0, 0, time.UTC)
	pastTime := now.Add(-24 * time.Hour) // expired threshold
	futureTime := now.Add(24 * time.Hour)
	oldModTime := now.Add(-48 * time.Hour)
	newerModTime := now.Add(-1 * time.Hour) // fs 比 record 记录新

	oracle := fakeOracle{
		now: now,
		exists: map[string]bool{
			"/repo/expired.md":  true,  // 文件还在，但 record expires_at 过期
			"/repo/deleted.md":  false, // 文件已删
			"/repo/modified.md": true,
			"/repo/healthy.md":  true,
		},
		modTimes: map[string]time.Time{
			"/repo/expired.md":  oldModTime,   // mtime 不影响
			"/repo/modified.md": newerModTime, // fs mtime 比 record source_modified 新
			"/repo/healthy.md":  oldModTime,   // mtime 等于 record source_modified
		},
	}

	records := []*contextforgev1.ContextRecord{
		newRecord("rec-expired", "h1", "file:///repo/expired.md", "/repo/expired.md", &pastTime,
			[]*contextforgev1.Provenance{newProvenance("scanner", "/repo/expired.md", &oldModTime)}),
		newRecord("rec-deleted", "h2", "file:///repo/deleted.md", "/repo/deleted.md", &futureTime,
			[]*contextforgev1.Provenance{newProvenance("scanner", "/repo/deleted.md", &oldModTime)}),
		newRecord("rec-modified", "h3", "file:///repo/modified.md", "/repo/modified.md", &futureTime,
			[]*contextforgev1.Provenance{newProvenance("scanner", "/repo/modified.md", &oldModTime)}),
		newRecord("rec-healthy", "h4", "file:///repo/healthy.md", "/repo/healthy.md", &futureTime,
			[]*contextforgev1.Provenance{newProvenance("scanner", "/repo/healthy.md", &oldModTime)}),
	}

	result := Mark(records, oracle)

	if len(result.Records) != 4 {
		t.Fatalf("AC1: Records 应原样透传 4 条, got %d", len(result.Records))
	}
	if len(result.StaleMarks) < 3 {
		t.Fatalf("AC1: 应有 >=3 StaleMark (expired/deleted/modified), got %d: %+v",
			len(result.StaleMarks), result.StaleMarks)
	}

	if findStaleMark(result.StaleMarks, "rec-expired", StaleReasonExpired) == nil {
		t.Errorf("AC1 expired: rec-expired 应被标 StaleReasonExpired")
	}
	if findStaleMark(result.StaleMarks, "rec-deleted", StaleReasonSourceDeleted) == nil {
		t.Errorf("AC1 source-deleted: rec-deleted 应被标 StaleReasonSourceDeleted")
	}
	if findStaleMark(result.StaleMarks, "rec-modified", StaleReasonSourceModified) == nil {
		t.Errorf("AC1 source-modified: rec-modified 应被标 StaleReasonSourceModified")
	}

	// 健康 record 不应有任何 stale mark
	for _, m := range result.StaleMarks {
		if m.RecordID == "rec-healthy" {
			t.Errorf("AC1 healthy: rec-healthy 不应被标, got %+v", m)
		}
	}
}

// ---- TEST-5.2.2 / SCEN-5.2.2 (AC2) — 基础冲突检测提示 ----
//
// 4 record:
//   - rec-a + rec-b 共享 source_uri "file:///doc1.md", content_hash 不同 -> conflict
//   - rec-c + rec-d 共享 file_path "/dup-path.md", content_hash 不同 -> conflict
//   - rec-e 独立（不应在任何 conflict 内）
//
// 期待 2 ConflictReport：一 source_uri group，一 file_path group。
func TestMark_BasicConflictDetection(t *testing.T) {
	oracle := fakeOracle{now: time.Date(2026, 5, 23, 12, 0, 0, 0, time.UTC)}

	records := []*contextforgev1.ContextRecord{
		newRecord("rec-a", "hash-A", "file:///doc1.md", "/doc1.md", nil, nil),
		newRecord("rec-b", "hash-B", "file:///doc1.md", "/doc1.md", nil, nil), // 同 source_uri + 不同 content_hash
		newRecord("rec-c", "hash-C", "file:///x.md", "/dup-path.md", nil, nil),
		newRecord("rec-d", "hash-D", "file:///y.md", "/dup-path.md", nil, nil), // 同 file_path + 不同 content_hash
		newRecord("rec-e", "hash-E", "file:///lonely.md", "/lonely.md", nil, nil),
	}

	result := Mark(records, oracle)

	if len(result.Conflicts) < 2 {
		t.Fatalf("AC2: 应有 >=2 ConflictReport (source_uri group + file_path group), got %d: %+v",
			len(result.Conflicts), result.Conflicts)
	}

	srcConflict := findConflict(result.Conflicts, "file:///doc1.md", ConflictKeySourceURI)
	if srcConflict == nil {
		t.Fatalf("AC2 source_uri: 应有 source_uri='file:///doc1.md' 的 ConflictReport, got: %+v",
			result.Conflicts)
	}
	sort.Strings(srcConflict.RecordIDs)
	if len(srcConflict.RecordIDs) != 2 || srcConflict.RecordIDs[0] != "rec-a" || srcConflict.RecordIDs[1] != "rec-b" {
		t.Errorf("AC2 source_uri: RecordIDs 应含 [rec-a rec-b], got %v", srcConflict.RecordIDs)
	}

	fpConflict := findConflict(result.Conflicts, "/dup-path.md", ConflictKeyFilePath)
	if fpConflict == nil {
		t.Fatalf("AC2 file_path: 应有 file_path='/dup-path.md' 的 ConflictReport, got: %+v",
			result.Conflicts)
	}
	sort.Strings(fpConflict.RecordIDs)
	if len(fpConflict.RecordIDs) != 2 || fpConflict.RecordIDs[0] != "rec-c" || fpConflict.RecordIDs[1] != "rec-d" {
		t.Errorf("AC2 file_path: RecordIDs 应含 [rec-c rec-d], got %v", fpConflict.RecordIDs)
	}

	// rec-e 独立，不应在任何 conflict
	for _, c := range result.Conflicts {
		for _, id := range c.RecordIDs {
			if id == "rec-e" {
				t.Errorf("AC2: rec-e 独立 source_uri+file_path, 不应在 conflict 内, got: %+v", c)
			}
		}
	}
}

// ---- TEST-5.2.3 / SCEN-5.2.3 (AC3) — 反指标：不做语义冲突判断 ----
//
// 2 record:
//   - content_hash 不同
//   - 不共享 source_uri (file:///alpha.md vs file:///beta.md)
//   - 不共享 file_path (/alpha.md vs /beta.md)
//   - 内容语义"相同事实"（"Use QMD reranker" vs "Prefer QMD reranking"）
//
// AC3 反指标守护：Mark 必须 NOT 报告它们冲突（任何 LLM/embedding 检查都会误判它们冲突）.
// 当前 RED stub 返空 Conflicts -> 此测试 vacuously pass；GREEN 实际实施 source_uri/file_path
// 分组也应 pass。本测试主要价值是未来 regression guard（防止有人加 LLM hook）.
func TestMark_DoesNotPerformSemanticAnalysis(t *testing.T) {
	oracle := fakeOracle{now: time.Date(2026, 5, 23, 12, 0, 0, 0, time.UTC)}

	records := []*contextforgev1.ContextRecord{
		newRecord("rec-a", "sha256:aaaa", "file:///alpha.md", "/alpha.md", nil, nil),
		newRecord("rec-b", "sha256:bbbb", "file:///beta.md", "/beta.md", nil, nil),
	}
	records[0].Content = "Use the QMD reranker by default"
	records[1].Content = "Prefer QMD reranking over baseline"

	result := Mark(records, oracle)

	// AC3 反指标：不报告这两条为冲突（语义相同但字面 key 不重叠）
	for _, c := range result.Conflicts {
		ids := make(map[string]bool, len(c.RecordIDs))
		for _, id := range c.RecordIDs {
			ids[id] = true
		}
		if ids["rec-a"] && ids["rec-b"] {
			t.Errorf("AC3 反指标: 语义相同但 source_uri/file_path 不重叠的 records 不应被报告为冲突 — Mark 似乎引入了语义分析, got %+v", c)
		}
	}

	// 同时 sanity: 两条都不应被 stale 标（oracle 时间正常，无 expires_at，无 fs 操作）
	if len(result.StaleMarks) != 0 {
		t.Errorf("AC3 sanity: 健康 record 不应被 stale 标, got %+v", result.StaleMarks)
	}
}

// ---- TEST-5.2.4 / SCEN-5.2.4 (AC4) — FilterStale pre-filter ----
//
// 4 record + 2 StaleMark -> FilterStale 应剔除 2 条，留 2 条；保留 records 顺序.
func TestFilterStale_RemovesMarkedRecords(t *testing.T) {
	records := []*contextforgev1.ContextRecord{
		newRecord("rec-1", "h1", "file:///1.md", "/1.md", nil, nil),
		newRecord("rec-2", "h2", "file:///2.md", "/2.md", nil, nil),
		newRecord("rec-3", "h3", "file:///3.md", "/3.md", nil, nil),
		newRecord("rec-4", "h4", "file:///4.md", "/4.md", nil, nil),
	}
	marks := []StaleMark{
		{RecordID: "rec-2", Reason: StaleReasonExpired, MarkedAt: time.Unix(100, 0)},
		{RecordID: "rec-4", Reason: StaleReasonSourceDeleted, MarkedAt: time.Unix(100, 0)},
	}

	clean := FilterStale(records, marks)

	if len(clean) != 2 {
		t.Fatalf("AC4: FilterStale 应剔除 2 条 stale, 留 2 条, got %d: %+v",
			len(clean), recordIDs(clean))
	}

	gotIDs := recordIDs(clean)
	wantIDs := []string{"rec-1", "rec-3"}
	for i, want := range wantIDs {
		if gotIDs[i] != want {
			t.Errorf("AC4: clean[%d] 应保留 %s（顺序保持）, got %s", i, want, gotIDs[i])
		}
	}

	// 不修改入参 records
	if len(records) != 4 {
		t.Errorf("AC4: FilterStale 不应修改入参, records 应仍为 4 条, got %d", len(records))
	}
}

func recordIDs(rs []*contextforgev1.ContextRecord) []string {
	out := make([]string, 0, len(rs))
	for _, r := range rs {
		out = append(out, r.GetId())
	}
	return out
}
