# language: en
# Maps to:
#   - docs/specs/phases/phase-33-governance-debt-cleanup-2.md
#   - docs/specs/tasks/task-33.1-l2-embedding-cache-bound.md
#   - docs/specs/tasks/task-33.2-memstore-lru-and-harddelete-invariant.md
#   - docs/specs/tasks/task-33.3-observability-indexing-replay-and-trace-isolation.md
#   - docs/specs/tasks/task-33.4-export-timeout-and-closeout-v0.26.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 33 governance-debt-cleanup-2（第二轮治理债清扫，镜像 Phase 31 / ADR-036）。Scenario ID 在各 task spec §7 追踪表映射到测试 / 真实 run。
# 受阻 / 据实不实现维度均以 [SPEC-DEFER:phase-future.<name>] 标注（L2 created_at 真 LRU 须对既有用户库 ALTER；memory hard-delete cascade 当前无可级联表为非问题；indexing replay e2e 须运行 daemon/job runner；TraceStore 多 workspace 严格隔离 e2e 须运行 console），据真实测试回填，绝不预填数值（ADR-013）。

Feature: phase-33-governance-debt-cleanup-2
  In order to 清扫第二轮跨 Phase 累积的治理债（§4 长尾 backlog + 调研发现的若干 survey 夸大项），让缓存/内存/观测/导出各面契约据实硬化且可单测验证，并据实记录非问题与延后维度（ADR-013 的诚实价值）
  As Phase 33 内核（L2 embedding cache 上界 + memstore access-order LRU + hard-delete 不变式 + indexing 事件持久化与回放 + TraceStore workspace 隔离 + export --timeout + v0.26.0 closeout）
  I want L2 SQLite embedding cache 加 row-count cap + rowid-FIFO 淘汰（0 dep / 0 schema migration，true-LRU honest-defer）+ console-api memstore 两缓存由 FIFO 升 access-order LRU（命中/覆盖均 move-to-front，hard-delete 不变式 + cascade 非问题 honest-defer + handleMemoryPin 宽松契约据 ADR-022 D2 保持）+ indexing.* 事件经 add-only migration 0019 持久化并扩展回放 mapper + TraceStore 经 add-only proto workspace_id 字段 + SQL WHERE 过滤实现多 workspace 隔离（空 workspace_id 保持聚合全量）+ events-drain-timeout verify-only 复核（Phase 26 已交付）+ export --timeout add-only flag（默认 60s 字节等价），且默认构建仍 0 new dep / 0 网络 / 默认行为不变（ADR-004），受阻 / 非问题维度（true-LRU / hard-delete cascade / indexing replay e2e / trace 隔离 e2e / datadir Options / %v→%w 非 bug）如实记录不伪造（ADR-013）

  # ---
  # Maps to: docs/specs/tasks/task-33.1-l2-embedding-cache-bound.md (TEST-33.1.1 / TEST-33.1.2)
  Scenario: SCEN-33.1.1 — 对应 AC1（L2 embedding cache row-count cap + rowid-FIFO 淘汰）
    Given core/src/embedding/cache.rs 的 L1 已是 BoundedCache FIFO（Phase 31），但 L2 sqlite_put（:153-161，INSERT OR REPLACE INTO embedding_cache at :155）无上界 → 行只增不减（表经 CREATE TABLE IF NOT EXISTS cache.rs:110-120 建，非编号 migration，列 content_hash/provider/dim/vector，PK(content_hash,provider)，含隐式 rowid），且 with_sqlite 当前无生产调用点（test-only cache.rs:331/337，发布 daemon 走 memory-only L1 → 此为 opt-in-path 纵深防御非已证实泄漏，据实陈述不夸大）
    When  为 L2 加 row-count cap（默认常量置于 DEFAULT_EMBEDDING_CACHE_CAP :23 旁）+ sqlite_put 后 COUNT(*)，超 cap 时 DELETE ... WHERE rowid IN (SELECT rowid ORDER BY rowid ASC LIMIT overflow)，并向 L2 连续 put 超过 cap 的 key
    Then  最旧 rowid 条目被淘汰 + L2 行数稳定在 cap 上界内 + 命中已淘汰 key 回源 L1/inner（cache miss）+ 0 new dep / 0 schema migration（用隐式 rowid；created_at 列真 LRU 须对既有用户库 ALTER → honest-defer [SPEC-DEFER:phase-future.l2-cache-true-lru]）+ 公共 ctor（new/with_sqlite/with_capacity）保持源兼容（镜像 TEST-31.2.1 cache.rs:345）（TEST-33.1.1，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-33.1-l2-embedding-cache-bound.md (TEST-33.1.2)
  Scenario: SCEN-33.1.2 — 对应 AC2（默认不变 + 既有 22.2.* / 31.2.1 保持绿）
    Given task-33.1 仅对 L2 加上界（cap 默认值不破坏既有 with_capacity 语义），既有 embed/cache 契约与 L1 BoundedCache 行为不应受影响
    When  以默认配置跑既有 cache / embedding 测试（TEST-22.2.* + TEST-31.2.1）并复核 public ctor 源兼容
    Then  既有 TEST-22.2.* / TEST-31.2.1 保持绿 + 默认 embed 契约不变（ADR-004）+ new/with_sqlite/with_capacity 签名源兼容（TEST-33.1.2，真实跑出后回填）

  # ---
  # Maps to: docs/specs/tasks/task-33.2-memstore-lru-and-harddelete-invariant.md (TEST-33.2.1 / TEST-33.2.2)
  Scenario: SCEN-33.2.1 — 对应 AC1（console-api memstore 两缓存 FIFO → access-order LRU，命中/覆盖 move-to-front）
    Given internal/consoleapi/memstore.go 的 cacheChunkUnlocked（:76-91）/ cacheTraceUnlocked（:96-111）为 FIFO（已存在 key 覆盖不重排，淘汰弹 [0]），读路径 GetSourceChunk（:341-352）/ GetSearchTrace（:357-368）命中不 move-to-front；cap = resolveCacheCapacity（Phase 31，env CONTEXTFORGE_CONSOLEAPI_CACHE_CAP，默认 256）；types.go:69 / handlers.go:277 注释已称 'in-memory LRU'（术语已 ahead）
    When  在 fallback 模式下将两缓存升为 access-order LRU（读路径命中 move-to-front + 已存在 key 覆盖 move-to-front），并向缓存连续访问 / 覆盖 / 插入超过 cap 的 key
    Then  命中最近被访问的 key 不被淘汰 + 最久未访问条目优先淘汰（access-order，非纯插入序）+ 既有 TestMemStore_CacheEviction_FIFO（memstore_test.go:209-243 硬断言 FIFO）须改写为 LRU + 0-dep / 仅 fallback 模式（生产接真实 backend）（TEST-33.2.1 chunk + TEST-33.2.2 trace，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-33.2-memstore-lru-and-harddelete-invariant.md (TEST-33.2.3)
  Scenario: SCEN-33.2.2 — 对应 AC2（memory hard-delete 无悬挂引用不变式 + cascade 非问题 honest-defer + handleMemoryPin 宽松契约保持）
    Given core/src/memory/store.rs 的 hard_delete（:235-246）只对 memory_items 做单条 DELETE；全 schema 审计（6 表 migration 0010-0018）显示 memory_id 仅是 memory_items 的 PK，无其它表含 memory_id，无 memory-vector/embedding 表（grep=0），vector-store 层与 memory 零耦合 → 当前无可级联表，写 cascade 代码即推测性/不可能场景（CLAUDE.md Simplicity-First）；handleMemoryPin（handlers.go:521-549）对 malformed/empty/absent body 故意回落 pin=true 以保 v0.7 宽松契约（doc-comment :519-524，task-17.1 / ADR-022 D2）
    When  以 schema introspection 写一条不变式测试（memory_items 是唯一含 memory_id 的表 + hard_delete(id) 后 get(id)=None），并据实将 handleMemoryPin 记为 HONEST 非改动（不改 400，保持宽松契约）
    Then  hard-delete 后无悬挂引用（不变式测试绿，未来若加 memory_id FK 将令其失败 → 强制届时真决策）+ cascade 据实标 [SPEC-DEFER:phase-future.memory-harddelete-cascade]（only-if-future-FK 非问题，不写推测代码 ADR-013 + Simplicity-First）+ handleMemoryPin 宽松契约保持（不改 400，ADR-022 D2 + ADR-004，记于 ADR-038 D4）（TEST-33.2.3，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-33.3-observability-indexing-replay-and-trace-isolation.md (TEST-33.3.1 / TEST-33.3.2)
  Scenario: SCEN-33.3.1 — 对应 AC1（indexing.* 事件持久化 migration 0019 + 回放 mapper rebuild）
    Given indexing 事件（indexing.progress/.cancelled/.error）现仅 broadcast 入内存 EventBus（core/src/jobs/index_session_backend.rs:157-218 build_*_event→eb.send，best-effort 不持久化），replay_events_from_audit（core/src/data_plane/events.rs:391-420）+ audit_op_str_to_event（:369-377）仅处理 memory_*（marker [SPEC-DEFER:phase-future.indexing-event-persistence] at events.rs:389），AuditOperation enum（core/src/memoryops/audit.rs:12-20）无 indexing variant + AuditLogEntry 缺 job_id/processed/total
    When  加 add-only migration 0019_indexing_events（专表，比 audit 复用更干净），在 emit 点持久化 indexing job 生命周期（job_id/stage/processed/total/ts），并扩展回放 mapper 以 id/ts ASC 重建 indexing.* PbEvent
    Then  indexing.* 事件可经专表持久 roundtrip + 回放 mapper 据真实 durable 行（非合成）重建 PbEvent（真实 job_id/processed/total，ADR-013）+ migration 纯 add-only（不溯改 0001-0018）+ mapper 单测 🟢（纯函数，镜像 TEST-26.2.3 events.rs:493）；端到端 restart 回放 🟡 须运行 daemon/job runner → [SPEC-DEFER:phase-future.indexing-replay-e2e]（TEST-33.3.1 mapper + TEST-33.3.2 persist-roundtrip，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-33.3-observability-indexing-replay-and-trace-isolation.md (TEST-33.3.3 / TEST-33.3.4)
  Scenario: SCEN-33.3.2 — 对应 AC2（TraceStore workspace 隔离：add-only proto field + SQL WHERE 过滤，空=聚合全量）
    Given core/src/data_plane/search_persist.rs 的 get（:129-142）/ list（:147-174）/ search_fts（:219-259）/ load_warm（:184-213）无 WHERE workspace_id；内存 TraceStore search.rs get/list 亦未过滤；handlers get_search_trace（:460-480）/ list_queries（:486-502）忽略 workspace；GetSearchTraceRequest（proto:237-239）+ ListQueriesRequest（:255-257）缺 workspace_id 字段（marker [SPEC-DEFER:phase-future.tracestore-multi-workspace-strict] at task-16.1:288）
    When  对该两 request message 加 add-only workspace_id 字段（buf generate proto regen）+ 透传至 TraceStore / SqliteTracePersist + 加 WHERE workspace_id 过滤，并以非空 / 空 workspace_id 分别查询
    Then  非空 workspace_id 严格隔离（仅返回该 workspace 的 trace）+ 空 workspace_id 保持当前聚合全量行为（ADR-004 后向兼容）+ proto 为 add-only field 不破既有契约 + SQL/handler 测试 🟢；e2e console 🟡 → [SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]（TEST-33.3.3 SQL + TEST-33.3.4 handler，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-33.3-observability-indexing-replay-and-trace-isolation.md (TEST-33.3.5)
  Scenario: SCEN-33.3.3 — 对应 AC3（events-drain-timeout-config verify-only：Phase 26 已交付）
    Given CONSOLE_EVENTS_DRAIN_TIMEOUT 已真实存在（internal/consoleapi/grpcclient/grpcclient.go:405-419 drainTimeoutFromEnv，默认 100ms）且 TestDrainTimeoutFromEnv（grpcclient_test.go:867-895）已绿，而调研曾误列为待加项（survey 夸大）
    When  以 verify-only 复核既有 TestDrainTimeoutFromEnv 仍绿，并将 add 重构为 verify（恰如 Phase 31 event-bus-partition verify-only 更正），不重新实现
    Then  TestDrainTimeoutFromEnv 保持绿 + 引用既有测试据实更正为 verify-only（不重复实现，net-zero，ADR-013 诚实 + ADR-031 add-only Amendment）（TEST-33.3.5，真实跑出后回填）

  # ---
  # Maps to: docs/specs/tasks/task-33.4-export-timeout-and-closeout-v0.26.0.md (TEST-33.4.1)
  Scenario: SCEN-33.4.1 — 对应 AC1（export --timeout add-only flag，默认 60s 字节等价）
    Given internal/cli/export.go:29 硬编码 context.WithTimeout(..., 60*time.Second)，parseExportOpts（:58-91）无 --timeout；而 task-31.3 export 在单一 60s 上限下做两次顺序 daemon spawn（search.go source.go:91 spawn#1 + listChunks source.go:104-109 spawn#2，每次等至多 daemonHealthDeadline=15s）→ 大库可能偏紧
    When  加 add-only --timeout flag（默认 60s）并以默认 / 自定义值解析 parseExportOpts
    Then  --timeout 可配置导出超时 + 未设时默认 60s 与既有字节等价（ADR-004）+ flag 为 add-only 不破既有 CLI 契约 + parse 单测 🟢（TEST-33.4.1，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-33.4-export-timeout-and-closeout-v0.26.0.md (TEST-33.4.2)
  Scenario: SCEN-33.4.2 — 对应 AC2（默认行为不变 + v0.26.0 closeout + dropped-nits 诚实）
    Given task-33.1 + task-33.2 + task-33.3 全 Done（L2 cap / memstore LRU + hard-delete 不变式 / indexing replay + trace 隔离 + drain verify-only），current Phase 32 smoke v22[41/41]；调研更正若干 dropped-nits 须据实记录（%v→%w 在 internal/daemon/search.go 为非 bug——该文件无 fmt，真实 %v 在 internal/cli/search.go:88 终端 Fprintf 处 %w 无效/vet-error 且 err.Error() 已携完整 grpc Status → Status 未丢失；tracestore-fts 跨版本 migration 已修并测过 search_persist.rs:84-90 + backfill_fts_if_empty :304 / TEST-26.1.4/.4b → no-op；datadir env-global→daemon.Options.DataDir 为真但 🟡 须改 spawn 契约）
    When  跑 scripts/console_smoke.sh banner v22→v23 + 新增 step → [42/42]（smoke_syntax_test.go TestTask334 镜像 TestTask324，no-regression [37/37]..[41/41]），产出 v0.26.0 release docs（evidence/artifacts/README/RELEASE_NOTES，<backfill> markers），ADR-038 逐 D Proposed→Accepted + Ratification + ADR-031 add-only Amendment（indexing replay + drain verify-only）+ ADR-027 add-only Amendment（L2 bound）+ roadmap §3.15/§4 add-only + s2v-adapter add-only + phase §6 闭合
    Then  默认行为 / proto / 既有契约不变（ADR-004——L2 cap 默认字节等价、memstore LRU 仅 fallback、proto add-only field、migration 0019 add-only、export --timeout 默认 60s 字节等价）+ smoke v23[42/42]（既有 step 不退化，denominators 不溯改 ADR-014 D5）+ ADR-038 逐 D 如实 ratify（cascade / true-LRU / indexing-replay-e2e / trace-isolation-e2e / datadir-Options honest-defer，%v→%w 非 bug、tracestore-fts already-fixed 据实记于 D4）+ ADR-014 D1-D5 第 24 次激活全通过（TEST-33.4.2 + 各 task LAST TEST TEST-33.1.3 / TEST-33.2.4 / TEST-33.3.6 / TEST-33.4.3 D2-lint，真实跑出后回填）
