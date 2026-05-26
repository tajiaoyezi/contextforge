# Task `15.1`: `memstore-chunk-trace-cache — MemStore fallback 模式补 chunkCache + traceCache 兜底 GET /v1/source-chunks/<id> + GET /v1/search/<query_id>/trace`

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 15 (console-functional-gap-closure)
**Dependencies**: task-12.2 (Source chunk by id REST) + task-12.3 (Search trace by query id REST) + task-11.4 (existing MemStore baseline)

## 1. Background

ContextForge-Console PR #91/#93 backlog 列 P0 #1：

> `CONSOLE_API_FALLBACK_INMEM=1` 模式下：
> 1. `POST /v1/search` 返 200 + 一个 chunk-1 / query-1 / trace-1 占位项 (MemStore.Search stub at memstore.go:272-317) [SPEC-OWNER:task-15.1]
> 2. 用户接着 `GET /v1/source-chunks/chunk-1` → 返 503（MemStore.GetSourceChunk 返 ErrDataPlaneUnavailable at memstore.go:257-262）
> 3. 用户接着 `GET /v1/search/query-1/trace` → 同样 503
> 4. UI 流程被打断 — 用户看到 SearchResult 但点击 chunk 详情 / trace 详情时 fallback 整个崩盘

**根因**：MemStore.Search 返了 stub，但 MemStore.GetSourceChunk / GetSearchTrace 没缓存 stub 数据 → 第二次请求时找不到。fallback 模式应该是"自洽 in-memory demo"，不是"半残骨架"。 [SPEC-OWNER:task-15.1]

**Fix 策略**：MemStore 加 `chunkCache map[string]contractv1.SourceChunk` + `traceCache map[string]contractv1.RetrievalTrace`；MemStore.Search 内同步写入两 map；GetSourceChunk / GetSearchTrace 查 map → 命中返 200 + cached 数据，未命中（cache miss）→ 沿用既有 ErrDataPlaneUnavailable / ErrNotFound。

## 2. Goal

`internal/consoleapi/memstore.go` 修改 MemStore struct 加 `chunkCache` / `traceCache` 两个 map；`MemStore.Search` 返 stub 后同步把 stub 写入两 cache；`GetSourceChunk` / `GetSearchTrace` 内先查 cache，命中返 200，未命中沿用既有 ErrDataPlaneUnavailable。≥3 unit test PASS（hit / miss / eviction）；`go test ./internal/consoleapi/...` 不退化。 [SPEC-OWNER:task-15.1]

## 3. Scope

### In Scope

- **修改 `internal/consoleapi/memstore.go`**：
  - `MemStore` struct 加：
    ```go
    chunkCache map[string]contractv1.SourceChunk  // key = chunk_id
    traceCache map[string]contractv1.RetrievalTrace // key = query_id
    cacheCapacity int  // default 256
    cacheOrder    []string // FIFO eviction key order
    ```
  - `NewMemStore()` 内初始化两 map + cacheCapacity = 256
  - `MemStore.Search` 内（既有 line 272-317）返 stub 前同步：[SPEC-OWNER:task-15.1]
    ```go
    s.mu.Lock()
    s.cacheChunkUnlocked(res.ChunkID, /* SourceChunk built from res */)
    s.cacheTraceUnlocked(trace.TraceID, trace)  // key by trace_id (= query_id pattern)
    s.mu.Unlock()
    ```
  - 新增 `cacheChunkUnlocked(chunkID string, sc contractv1.SourceChunk)` + `cacheTraceUnlocked(traceID string, t contractv1.RetrievalTrace)` 私有 helper（FIFO eviction，cap = cacheCapacity）
  - `GetSourceChunk` (既有 line 257-262) 改：
    ```go
    func (s *MemStore) GetSourceChunk(chunkID string) (contractv1.SourceChunk, error) {
        s.mu.Lock()
        if sc, ok := s.chunkCache[chunkID]; ok {
            s.mu.Unlock()
            return sc, nil
        }
        s.mu.Unlock()
        if s.SearchBackend != nil {
            return s.SearchBackend.GetSourceChunk(chunkID)
        }
        return contractv1.SourceChunk{}, ErrDataPlaneUnavailable
    }
    ```
  - `GetSearchTrace` (既有 line 265-270) 改同款 cache 优先模式 — 注意：MemStore.Search 写入 trace 时 key 用 trace.TraceID（"trace-1"），但 GetSearchTrace 调用方传的是 query_id（"query-1"）；需双 key 索引或者 stub 时让 trace_id == query_id [SPEC-OWNER:task-15.1]
    - 实施选择：stub Search 时 set `trace.TraceID = res.QueryID` 让两 key 对齐；或保留两 key（traceByQueryID + traceByTraceID） — task 实施时选简单方案 [SPEC-OWNER:task-15.1]

- **新增 SourceChunk 构造 helper** `buildSourceChunkFromResult(res contractv1.SearchResult) contractv1.SourceChunk`：
  - 把 SearchResult 字段 (ChunkID / SourceFilePath / SourceFileType / ChunkTextPreview / LineStart / LineEnd / WorkspaceID) 映射到 SourceChunk
  - SourceChunk 必填字段：chunk_id / workspace_id / source_file_path / source_file_type / chunk_text / line_start / line_end / availability
  - chunk_text 用 ChunkTextPreview 兜底（fallback 没有真 chunk 内容）

- **单元测试 ≥3**（`internal/consoleapi/memstore_test.go`）：
  - `TestMemStore_ChunkCacheHit_AfterSearch` — Search → GetSourceChunk 命中返 200
  - `TestMemStore_TraceCacheHit_AfterSearch` — Search → GetSearchTrace 命中返 200
  - `TestMemStore_CacheEviction_FIFO` — 257 次 Search 后 cache cap=256 触发 FIFO 驱逐 oldest
  - 加分项 `TestMemStore_CacheMiss_Returns503` — 直接 GetSourceChunk 未 Search 过的 id → ErrDataPlaneUnavailable

- **不修改**：
  - SearchBackend 注入逻辑 (`SearchBackend != nil` 分支保留 — 真接 grpc backend 不走 cache)
  - degradedDeps / buildDeps 上游 wiring
  - REST handlers / router
  - proto / contractv1.go schema

- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **真接 retriever 端 chunk cache**（task-15.1 仅 in-memory fallback；真 grpc backend 通过 task-12.2/12.3 实现）
- **chunkCache 持久化**（in-memory map；进程重启失效；与 fallback 行为一致）
- **LRU eviction（vs FIFO）**：v0.8 ship FIFO 简单；LRU 留 [SPEC-DEFER:phase-future.cache-lru]
- **cache 容量参数化（env / flag）**：v0.8 ship hardcode 256；可配置留 [SPEC-DEFER:phase-future.cache-cap-configurable]
- **缓存 TTL / 过期清理**：永不过期直到进程退出（fallback 单进程短生命周期）

## 4. Users / Actors

- **Console UI 端**（下游，via cross-repo）：Dashboard "最近查询" / Search 详情面板触发 GET /v1/source-chunks/<id> → 期望 fallback 模式不再 503
- **docker single-image 用户**：`docker run contextforge-daemon:v0.8.0 -e CONSOLE_API_FALLBACK_INMEM=1` 跑 Console 反向 conformance 期望 22-endpoint 不退化

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-15-console-functional-gap-closure.md` §3 / §6 AC1
- `docs/specs/tasks/task-12.2-source-chunk-by-id.md` (既有 chunk endpoint)
- `docs/specs/tasks/task-12.3-search-trace-by-query-id.md` (既有 trace endpoint)
- `internal/consoleapi/memstore.go` 既有 line 14-32 (MemStore struct) + line 257-317 (Search/GetSourceChunk/GetSearchTrace)
- `internal/contractv1/contractv1.go::SourceChunk, RetrievalTrace, SearchResult`

### 5.2 Imports

- **Go**: 现有 stdlib `sync` (已 import for mu sync.Mutex)；不引入新 dep
- **不引入新依赖**：R7 不触发

### 5.3 Cache eviction 形状

```go
// helper inside memstore.go (caller holds s.mu)
func (s *MemStore) cacheChunkUnlocked(chunkID string, sc contractv1.SourceChunk) {
    if _, exists := s.chunkCache[chunkID]; exists {
        s.chunkCache[chunkID] = sc  // update; no order change
        return
    }
    s.chunkCache[chunkID] = sc
    s.cacheOrder = append(s.cacheOrder, chunkID)
    if len(s.cacheOrder) > s.cacheCapacity {
        evict := s.cacheOrder[0]
        s.cacheOrder = s.cacheOrder[1:]
        delete(s.chunkCache, evict)
    }
}
```

（trace cache 同款 helper；可统一 cache helper 抽象，但 v0.8 简单实现）

## 6. Acceptance Criteria

- [ ] AC1：MemStore.Search 返 stub 后 chunkCache + traceCache 同步填入；`GetSourceChunk(stubID)` 返 200 + SourceChunk；`GetSearchTrace(stubID)` 返 200 + RetrievalTrace — **verified by `internal/consoleapi/memstore_test.go::TestMemStore_ChunkCacheHit_AfterSearch` + `TestMemStore_TraceCacheHit_AfterSearch` PASS** [SPEC-OWNER:task-15.1]
- [ ] AC2：cache cap = 256 触发 FIFO eviction；第 257 次 Search 写入后 oldest 驱逐，再次 GetSourceChunk(oldestID) → cache miss → ErrDataPlaneUnavailable — **verified by `TestMemStore_CacheEviction_FIFO` PASS**
- [ ] AC3：MemStore.GetSourceChunk / GetSearchTrace 在 cache miss 时沿用既有 ErrDataPlaneUnavailable 或 delegate SearchBackend；不破坏既有行为 — **verified by `TestMemStore_CacheMiss_Returns503` + 现有 `TestRouter_GetSourceChunk_503_when_fallback` 不退化 PASS**
- [ ] AC4：`go test ./internal/consoleapi/...` 全绿；既有 22-endpoint conformance 不退化（task-10.5 / task-12.* test PASS） — **verified by `go test -v ./internal/consoleapi/... ./test/conformance/...` PASS**
- [ ] AC5：fallback 模式实测：`CONSOLE_API_FALLBACK_INMEM=1 go run ./cmd/contextforge console-api-serve` 后 curl POST /v1/search → 拿 chunk_id → curl GET /v1/source-chunks/<chunk_id> → 200 — **verified by 手动 curl 实测 + smoke v6 Step 22**（task-15.6 集成时验证）

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | Search 后 cache 命中 200 | memstore.go + memstore_test.go | Ready |
| AC2 | FIFO eviction 在 cap=256 | memstore.go + test | Ready |
| AC3 | cache miss 沿用 503 | memstore.go + test | Ready |
| AC4 | 既有 test 不退化 | go test | Ready |
| AC5 | 手动 curl 实测 fallback | manual + smoke v6 | Ready |

## 8. Risks

- **并发竞态**：MemStore.Search + GetSourceChunk 同时调用 — mu.Lock 已覆盖；测试时启 goroutine 并发跑验证
- **stub query_id vs trace_id 不对齐**：既有 stub `res.QueryID = "query-1"`, `trace.TraceID = "trace-1"` — 命中需对齐策略；task 实施时选 `trace.TraceID = res.QueryID` 让 GET /v1/search/{query_id}/trace 用 query_id 直接查 traceCache [SPEC-OWNER:task-15.1]
- **cache cap 256 太小**：fallback demo 场景充分；如真用户量大可参数化 [SPEC-DEFER:phase-future.cache-cap-configurable]
- **SourceChunk vs SearchResult 字段差异**：SearchResult 含 ChunkTextPreview（preview，可能截断），SourceChunk 期望全文 chunk_text — fallback 用 preview 兜底；真接 retriever 是另一路径

## 9. Verification Plan

- **install**: `go mod download`
- **lint**: `gofmt -l internal/consoleapi/`
- **typecheck**: `go build ./...`
- **unit-test**: `go test -v ./internal/consoleapi/...`（≥3 新 unit + 既有不退化）
- **integration**: N/A（task-15.1 纯 in-memory；integration 由 task-15.6 smoke v6 覆盖）
- **e2e**: N/A
- **build**: `go build ./cmd/contextforge`
- **coverage**: 不强制（fallback 路径）
- **runtime-smoke**: `CONSOLE_API_FALLBACK_INMEM=1 go run ./cmd/contextforge console-api-serve` + manual curl POST /v1/search + GET /v1/source-chunks/<id> 验证 200
- **manual**: 手动 curl POST + GET 验证不再 503

## 10. Completion Notes

- **完成日期**：<待填>
- **关键决策**：<待填>
- **§9 Verification 结果**：<待填>
- **改动文件**：
  - `internal/consoleapi/memstore.go` (修改 — chunkCache + traceCache + cache helpers + Search/GetSourceChunk/GetSearchTrace 改造)
  - `internal/consoleapi/memstore_test.go` (修改/新增 — ≥3 unit test)
  - `docs/specs/tasks/task-15.1-memstore-chunk-trace-cache.md` (本 spec §6 / §7 / §10 / Status 推进)
- **commit 列表**：<待填>
- **剩余风险 / 未做项**：
  - LRU vs FIFO [SPEC-DEFER:phase-future.cache-lru]
  - Cache cap 配置化 [SPEC-DEFER:phase-future.cache-cap-configurable]
- **下游 task 影响**：task-15.6 smoke v6 Step 22 验证本 task fallback 修复
