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

// Mark — RED stub. AC1+AC2+AC3 实施待 GREEN commit.
//
// 行为（RED）：原样透传 Records；StaleMarks 与 Conflicts 返空 -> AC1/AC2 测试 fail.
// 行为（GREEN）：见 §5.3 函数签名描述。
func Mark(records []*contextforgev1.ContextRecord, oracle Oracle) Result {
	return Result{
		Records:    records,
		StaleMarks: nil,
		Conflicts:  nil,
	}
}

// FilterStale — RED stub. AC4 实施待 GREEN commit.
//
// 行为（RED）：原样返回 records 不剔除 -> AC4 测试 fail.
// 行为（GREEN）：剔除 RecordID 在 marks 集合内的所有 record，保留顺序。
func FilterStale(records []*contextforgev1.ContextRecord, marks []StaleMark) []*contextforgev1.ContextRecord {
	return records
}
