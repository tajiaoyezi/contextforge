# language: en
# Maps to:
#   - docs/specs/phases/phase-31-governance-debt-cleanup.md
#   - docs/specs/tasks/task-31.1-observability-memstore-event-parity.md
#   - docs/specs/tasks/task-31.2-cache-and-deploy-hardening.md
#   - docs/specs/tasks/task-31.3-eval-exporter-and-mcp-nits.md
#   - docs/specs/tasks/task-31.4-closeout-v0.24.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 31 governance-debt-cleanup。Scenario ID 在各 task spec §7 追踪表映射到测试 / 真实 run。

Feature: phase-31-governance-debt-cleanup
  In order to 清理跨 Phase 累积的治理债（§4 长尾 backlog + Phase 28 follow-up + 旧 nits），让观测/缓存/部署/eval 各面契约与既有 Rust 路径对齐且可单测验证
  As Phase 31 内核（memstore event parity + cache LRU + compose hardening + eval 子表 + exporter 全文 + MCP nits + 诚实重申延后）
  I want Go fallback memory ops 发 memory.* 事件与 workspace/job + Rust 路径对齐 + embedding-cache 加 LRU/cap 上界 + Go memstore cap 可配置 + 生产 compose 加资源限/可选 TLS proxy + eval case 结果升为可查询子表 + exporter 经新 ListAllChunks RPC 取真实全文 + 3 个 MCP nits 修，且事件总线 partition/capacity 经核 Phase 26 已交付 → 仅 verify-only + roadmap §4 add-only 更正不重复实现，受阻维度（真实 TLS cert / native arm64 runner / 私有仓库 attestation）如实记录不伪造

  # ---
  # Maps to: docs/specs/tasks/task-31.1-observability-memstore-event-parity.md (TEST-31.1.1)
  Scenario: SCEN-31.1.1 — 对应 AC1（memstore-event-emit：Go fallback memory ops 发 memory.* 事件）
    Given internal/consoleapi/memstore.go 的 emitEvent helper 已用于 workspace/job 变更（CreateWorkspace/UpdateWorkspaceConfig/EnqueueJob/CancelJob）但 MemMemoryStore.Pin/Deprecate/SoftDelete/Unpin/HardDelete 从未调用它（GET /v1/observability/events fallback ring 缺 memory 类事件），而 Rust data-plane MemoryServer 已发 memory.* 事件（core/src/data_plane/memory.rs:52-106 不动）
    When  在 fallback 模式下对 MemMemoryStore 执行 Pin（及 Deprecate/SoftDelete/Unpin/HardDelete），令其将 memory.pin / memory.deprecate / memory.soft_delete / memory.unpin / memory.hard_delete 写入 capped 1000 fallback ring
    Then  Pin 后 fallback ring 增长（Go 单测断言 emitEvent 被 memory ops 触发，与 workspace/job + Rust 路径 event 对齐）+ 既有 workspace/job 事件不退化 + Rust 侧零改动（TEST-31.1.1，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-31.1-observability-memstore-event-parity.md (TEST-31.1.2)
  Scenario: SCEN-31.1.2 — 对应 AC2（event-bus partition/capacity verify-only + roadmap §4 add-only 更正）
    Given 经核 event-bus partition + capacity 已于 Phase 26 / ADR-031 D5 交付（core/src/data_plane/events.rs:24-203 EventBusConfig/Partition/from_config + server.rs:602-603 production-wired + TEST-26.3.1a/b/c at events.rs:549-605），而 roadmap §4 backlog 仍将其列为开放项（陈旧条目）
    When  以 verify-only 复核既有 core 测试 TEST-26.3.1a/b/c 仍绿 + 对 roadmap §4 做 add-only 更正注记其经 Phase 26 已交付（不重复实现，ADR-013 诚实）
    Then  TEST-26.3.1a/b/c 保持绿 + roadmap §4 add-only 更正剔除 event-bus-partition/capacity 开放 backlog + 无任何重复实现改动（net-zero core）+ events.rs/server.rs 不被本 phase 触及（TEST-31.1.2，真实跑出后回填）

  # ---
  # Maps to: docs/specs/tasks/task-31.2-cache-and-deploy-hardening.md (TEST-31.2.1)
  Scenario: SCEN-31.2.1 — 对应 AC1（embedding-cache LRU/cap 淘汰）
    Given core/src/embedding/cache.rs 的 CachingEmbeddingProvider.mem（Mutex<HashMap<String,Vec<f32>>> at :23）无界增长（insert :154/:170 不设上限；L2 SQLite INSERT OR REPLACE :99-104 亦无界），长跑 daemon 存内存无界增长风险
    When  为 L1（及可选 L2）加 LRU / capacity-cap 淘汰策略，并向缓存连续 insert 超过容量上限的 key
    Then  最旧条目被淘汰 + 命中已淘汰 key 时内层 provider 被重新调用（cache miss 回源）+ 容量上界内条目稳定命中 + 既有 embed 契约不变（TEST-31.2.1，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-31.3-eval-exporter-and-mcp-nits.md (TEST-31.3.1)
  Scenario: SCEN-31.3.1 — 对应 AC1（eval case-results 子表 eval_case_results）
    Given core/src/eval/store.rs 的 CaseResult（:17-25）以序列化 JSON 存于单 eval_runs 表的 case_results_json 列（update_case_results :177-193 写 UPDATE ... SET case_results_json；row_to_run :285 读 serde_json::from_str；INSERT seed [] :118），per-case 结果不可 SQL 查询
    When  将 per-case 结果升为可查询子表 eval_case_results（FK eval_run_id）+ add-only migration 0018（当前最新为 0017，Phase 27 已交付）
    Then  per-case 结果可经 SQL 过滤/聚合查询 + 既有 eval_runs 读路径不受影响 + migration 为纯 add-only（不溯改 0001-0017）（TEST-31.3.1，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-31.3-eval-exporter-and-mcp-nits.md (TEST-31.3.2)
  Scenario: SCEN-31.3.2 — 对应 AC2（exporter 全文非空 via ListAllChunks RPC）
    Given internal/exporter/source.go 的 loadRecords 将 content 设为 ""（:85）后对空串算 ContentHash = contentHash(content)（:96），根因 v1 search proto SearchResponse 不携 chunk 全文（proto/contextforge/v1/search.proto），exporter 导出记录全文为空、ContentHash 为 sha256-of-empty（fidelity.go CalcFidelity 一并失真）
    When  新增 add-only ListAllChunks(collection_id) RPC（返回 chunk 全文，task-6.3 §10:335-368 documented path B）并据其填充 content + 真实 ContentHash
    Then  导出 record.content 非空 + ContentHash 匹配真实全文（非 sha256-of-empty）+ CalcFidelity 据真实全文计算 + proto 为 add-only RPC 不破既有契约（TEST-31.3.2，真实测试通过后回填）
