# language: en
# Maps to:
#   - docs/specs/phases/phase-26-observability-hardening.md
#   - docs/specs/tasks/task-26.1-tracestore-fts-and-vacuum.md
#   - docs/specs/tasks/task-26.2-events-sse-push-and-replay.md
#   - docs/specs/tasks/task-26.3-closeout-v0.19.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 26 observability-hardening。Scenario ID 在各 task spec §7 追踪表映射到具体测试。

Feature: phase-26-observability-hardening
  In order to 让 trace 持久面可按内容检索且不无界膨胀、events 实时面无遗漏可重放、event-bus 可配
  As Phase 26 内核（TraceStore FTS+VACUUM + events SSE+重放 + event-bus 配置 + v0.19.0 收口）
  I want trace 可 FTS5 检索 + 周期 VACUUM、events SSE 实时推送 + 从 audit log 重放，且默认构建恒 0 新 dep / 0 network、既有 long-poll/22-endpoint 契约不退化、受阻态如实记录不伪造

  # ---
  # Maps to: docs/specs/tasks/task-26.1-tracestore-fts-and-vacuum.md (TEST-26.1.1/26.1.2/26.1.3/26.1.4)
  Scenario: SCEN-26.1.1 — 对应 AC1（trace FTS5 全文检索 + 周期 VACUUM；既有 put/get/list/load_warm 不变）
    Given core/src/data_plane/search_persist.rs SqliteTracePersist + core/migrations/0016 FTS5 影子表（rusqlite bundled SQLite，0 新 dep）
    When  put 若干含确定 term 的 trace → search_fts("known-term", k)；或 search_fts("absent-term")；或 prune_older_than(cutoff)+vacuum()
    Then  FTS 命中含该 term 的 trace 投影序 + limit clamp 1..=100（TEST-26.1.1）；miss 返 Ok(vec![]) 不误命中（TEST-26.1.2）；VACUUM/prune 后 row_count 与剩余行一致 + 保留行 get/list 仍正确不破坏数据（TEST-26.1.3）；既有 put/get/list/load_warm 签名语义不变 + 0016 IF NOT EXISTS 幂等回填旧库（TEST-26.1.4）；默认构建 0 新依赖

  # ---
  # Maps to: docs/specs/tasks/task-26.2-events-sse-push-and-replay.md (TEST-26.2.1/26.2.2/26.2.3/26.2.4)
  Scenario: SCEN-26.2.1 — 对应 AC2（events SSE 实时推送 + 从 audit log 重放；add-only 不退化 long-poll）
    Given internal/consoleapi SSE endpoint（text/event-stream，http.Flusher，旁挂既有 GET /v1/observability/events long-poll）+ 从 core/src/memoryops/audit.rs AuditSink audit_log 重放（id ASC）
    When  注入确定事件序经 SSE 推；或 ?since_ts= 先重放 memory state-op 历史再接实时流；或 client 断开 r.Context().Done()
    Then  响应体含正确 SSE 帧 id:/event:/data: + 顺序与注入序一致 + data 合法 JSON ObservabilityEvent（TEST-26.2.1）；既有 long-poll endpoint + Recent(limit,wait) 签名 + 22-endpoint 不退化（add-only，TEST-26.2.2）；重放段 audit id ASC 升序 + 拼接边界以 event_id/ts_unix 去重不重复不乱序 deterministic 不依赖墙钟（TEST-26.2.3）；断开释放 gRPC 订阅不泄漏 goroutine + indexing 事件重放 [SPEC-DEFER:phase-future.indexing-event-persistence]（TEST-26.2.4）；SSE 用标准库 0 新 dep

  # ---
  # Maps to: docs/specs/tasks/task-26.3-closeout-v0.19.0.md (TEST-26.3.1/26.3.2/26.3.3/26.3.5)
  Scenario: SCEN-26.3.1 — 对应 AC1/AC3/AC5（event-bus 配置 + smoke v16 + v0.19.0 收口 + ADR-031 ratify）
    Given event-bus 配置（event-bus-capacity / event-bus-partition / events-drain-timeout-config，复用 core/src/data_plane/events.rs:35 with_capacity seam）+ scripts/console_smoke.sh v16 + v0.19.0 release docs + ADR-031（observability-hardening）
    When  EventBus 读配置容量（默认 1000）/ memory.*-indexing.* 分区（默认不分区）/ grpcclient phase-2 drain 可配（默认 ~100ms）；smoke v16 加 SSE/FTS/event-bus 断言；ADR-031 据 task-26.1/26.2 真实结果 ratify
    Then  配置生效 + 默认等价于既有 broadcast::channel(1000) 单 channel（TEST-26.3.1）；smoke 既有 step 25/26 不退化 + bash -n exit 0；ADR-031 D1-D6 经真实非合成验证 Proposed→Accepted + ADR-021 add-only Amendment（events-replay 兑现 [SPEC-DEFER:phase-future.events-replay-from-audit] + event-bus 容量/分区兑现 Rollback path 预见）；phase-26 §6 全 met；ADR-014 D1-D5（第十七次激活）全通过
