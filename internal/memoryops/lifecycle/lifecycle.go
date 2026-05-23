// Package lifecycle implements v0.1 MemoryOps stale 三触发 + 基础冲突检测.
//
// 同 task-5.1 dedup pattern：纯 transform（input -> Result），不持久化；
// Phase 6 daemon 决定 in-memory cache / SQLite 持久化层归宿。
//
// AC1 stale 三触发：expires_at 到期 / source deleted / source modified（OR 关系，
// 任一命中即追加 StaleMark；同一 record 多触发各加一条供 audit/debug 看清原因）。
// AC2 基础冲突检测：source_uri 或 file_path 任一 group 内 >=2 条且 content_hash
// 不全相同 -> ConflictReport。tags overlap v0.1 不参与（噪音过大；§2A 决策）。
// AC3 反指标硬约束：不调用任何 LLM / embedding API；仅 oracle + 字面字段比较。
// AC4 FilterStale pre-filter：retriever 不改；Phase 6 caller 显式 wrap。
package lifecycle

import (
	"os"
	"sort"
	"time"

	contextforgev1 "github.com/tajiaoyezi/contextforge/proto/contextforge/v1"
)

// Result — Mark 一次过的全部输出。
//
// Records 为入参原样透传（不去重 / 不删 stale），由 caller 决定后续步骤
// （FilterStale / 直接展示 / 写 audit log）。
type Result struct {
	Records    []*contextforgev1.ContextRecord
	StaleMarks []StaleMark
	Conflicts  []ConflictReport
}

// StaleMark — AC1 三触发任一命中即标记一条。
type StaleMark struct {
	RecordID string
	Reason   StaleReason
	MarkedAt time.Time
}

// StaleReason 枚举（v0.1 三种触发，按 AC1）。
type StaleReason string

const (
	StaleReasonExpired        StaleReason = "expired"
	StaleReasonSourceDeleted  StaleReason = "source-deleted"
	StaleReasonSourceModified StaleReason = "source-modified"
)

// ConflictReport — AC2 同 source_uri 或同 file_path 但 content_hash 不全相同。
type ConflictReport struct {
	Key       string
	KeyType   ConflictKeyType
	RecordIDs []string
}

// ConflictKeyType 枚举（v0.1 两种 group key，按 §2A 决策）。
type ConflictKeyType string

const (
	ConflictKeySourceURI ConflictKeyType = "source_uri"
	ConflictKeyFilePath  ConflictKeyType = "file_path"
)

// Oracle 抽象环境依赖（Clock + FS），让测试可注入确定性 fake — AC1 三触发依赖.
type Oracle interface {
	Now() time.Time
	Exists(path string) bool
	ModTime(path string) (time.Time, bool)
}

// SystemOracle — 生产默认 Oracle（time.Now + os.Stat）.
type SystemOracle struct{}

// Now returns the current wall-clock time.
func (SystemOracle) Now() time.Time { return time.Now() }

// Exists returns true if the given path can be stat'd (file or directory).
func (SystemOracle) Exists(path string) bool {
	if path == "" {
		return false
	}
	_, err := os.Stat(path)
	return err == nil
}

// ModTime returns the modification time of the given path, or (zero, false) when unavailable.
func (SystemOracle) ModTime(path string) (time.Time, bool) {
	if path == "" {
		return time.Time{}, false
	}
	info, err := os.Stat(path)
	if err != nil {
		return time.Time{}, false
	}
	return info.ModTime(), true
}

// Mark — 主入口（AC1 stale 三触发 + AC2 基础冲突检测 + AC3 反指标守护）.
//
// AC1: 对每条 record 跑 expires_at / source-deleted / source-modified 三触发；
//      任一命中即追加 StaleMark（同一 record 每种 reason 至多 1 条，避免多 provenance
//      行重复刷屏；时间用 oracle.Now() 统一）.
// AC2: 跨全集按 source_uri 与 file_path 两种 group key 分组；
//      group 内 >=2 条且 content_hash 不全相同 → ConflictReport 追加；
//      group key 为空字符串的 record 不参与分组（避免空键聚成巨型 conflict）.
// AC3: 不调用任何 LLM / embedding API（仅 oracle + 字面字段比较；
//      TestMark_DoesNotPerformSemanticAnalysis regression guard）.
func Mark(records []*contextforgev1.ContextRecord, oracle Oracle) Result {
	now := oracle.Now()
	staleMarks := make([]StaleMark, 0)

	for _, rec := range records {
		if rec == nil {
			continue
		}

		// 同 record 每种 reason 仅一条（dedup）
		reasons := make(map[StaleReason]bool)
		addMark := func(reason StaleReason) {
			if reasons[reason] {
				return
			}
			reasons[reason] = true
			staleMarks = append(staleMarks, StaleMark{
				RecordID: rec.GetId(),
				Reason:   reason,
				MarkedAt: now,
			})
		}

		// AC1 trigger 1: expires_at 到期
		if exp := rec.GetExpiresAt(); exp != nil {
			if !exp.AsTime().After(now) {
				addMark(StaleReasonExpired)
			}
		}

		// AC1 trigger 2/3: 遍历 provenance 检查 source-deleted / source-modified
		for _, p := range rec.GetProvenance() {
			if p == nil {
				continue
			}
			path := p.GetOriginalPath()
			if path == "" {
				continue
			}

			// trigger 2: source deleted — 文件不存在
			if !oracle.Exists(path) {
				addMark(StaleReasonSourceDeleted)
				continue // 文件不存在就不再查 mtime
			}

			// trigger 3: source modified — fs mtime > record 记录的 source_modified_at
			fsMtime, ok := oracle.ModTime(path)
			if !ok {
				continue
			}
			recordedMtime := p.GetSourceModifiedAt()
			if recordedMtime == nil {
				continue // 没有 baseline mtime 无法判定，不算 stale
			}
			if fsMtime.After(recordedMtime.AsTime()) {
				addMark(StaleReasonSourceModified)
			}
		}
	}

	conflicts := detectConflicts(records)

	return Result{
		Records:    records,
		StaleMarks: staleMarks,
		Conflicts:  conflicts,
	}
}

// recordRef — 私有 group helper 类型（id + content_hash 二元组用于 AC2 conflict 判定）.
type recordRef struct {
	id   string
	hash string
}

// detectConflicts — AC2 基础冲突检测.
//
// 按 source_uri 与 file_path 两种 group key 分别分组；
// group 内 >=2 record 且 content_hash 不全相同 → ConflictReport.
// 输出按 (KeyType, Key) 字典序确定性排序.
func detectConflicts(records []*contextforgev1.ContextRecord) []ConflictReport {
	srcGroups := make(map[string][]recordRef)
	pathGroups := make(map[string][]recordRef)

	for _, rec := range records {
		if rec == nil {
			continue
		}
		ref := recordRef{id: rec.GetId(), hash: rec.GetContentHash()}
		if uri := rec.GetSourceUri(); uri != "" {
			srcGroups[uri] = append(srcGroups[uri], ref)
		}
		if fp := rec.GetFilePath(); fp != "" {
			pathGroups[fp] = append(pathGroups[fp], ref)
		}
	}

	out := make([]ConflictReport, 0)
	out = append(out, collectConflicts(srcGroups, ConflictKeySourceURI)...)
	out = append(out, collectConflicts(pathGroups, ConflictKeyFilePath)...)
	sort.SliceStable(out, func(i, j int) bool {
		if out[i].KeyType != out[j].KeyType {
			return out[i].KeyType < out[j].KeyType
		}
		return out[i].Key < out[j].Key
	})
	return out
}

// collectConflicts — 对一个 group 字典做 conflict 提取（group 内 >=2 且 content_hash 不全相同）.
func collectConflicts(groups map[string][]recordRef, kt ConflictKeyType) []ConflictReport {
	out := make([]ConflictReport, 0)
	for key, refs := range groups {
		if len(refs) < 2 {
			continue
		}
		first := refs[0].hash
		allSame := true
		for _, r := range refs[1:] {
			if r.hash != first {
				allSame = false
				break
			}
		}
		if allSame {
			continue
		}
		ids := make([]string, 0, len(refs))
		seen := make(map[string]bool, len(refs))
		for _, r := range refs {
			if seen[r.id] {
				continue
			}
			seen[r.id] = true
			ids = append(ids, r.id)
		}
		sort.Strings(ids)
		out = append(out, ConflictReport{Key: key, KeyType: kt, RecordIDs: ids})
	}
	return out
}

// FilterStale — AC4 pre-filter for retriever consumers.
//
// 不修改入参；返回 records 中不在 marks RecordID 集合内的子集（保留顺序）.
// Phase 6 CLI / REST / MCP caller 调用模式：
//
//	results := retriever.Search(opts)
//	r := lifecycle.Mark(results, oracle)
//	clean := lifecycle.FilterStale(r.Records, r.StaleMarks)
//	render(clean)
func FilterStale(records []*contextforgev1.ContextRecord, marks []StaleMark) []*contextforgev1.ContextRecord {
	if len(marks) == 0 {
		// 拷贝以保持"不修改入参"语义一致
		out := make([]*contextforgev1.ContextRecord, len(records))
		copy(out, records)
		return out
	}
	staleSet := make(map[string]bool, len(marks))
	for _, m := range marks {
		staleSet[m.RecordID] = true
	}
	out := make([]*contextforgev1.ContextRecord, 0, len(records))
	for _, rec := range records {
		if rec == nil {
			continue
		}
		if staleSet[rec.GetId()] {
			continue
		}
		out = append(out, rec)
	}
	return out
}
