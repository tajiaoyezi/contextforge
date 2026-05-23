# language: en
# Maps to:
#   - docs/specs/tasks/task-1.4-cli-init.md
#   - docs/specs/tasks/task-6.1-cli-search.md
#
# 轻量 BDD（s2v §9.2）；module=cli 跨 task 1.4 / 6.1，本文件追加各 task 的 Scenario 组。
# 占位场景由 task agent 实施时填 Given/When/Then。

Feature: cli
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want CLI 入口——命令解析、配置加载、子命令编排（init/import/index/search/serve/mcp/eval/export）

  # ---
  # Maps to: docs/specs/tasks/task-1.4-cli-init.md
  Scenario: SCEN-1.4.1 — 对应 AC1（init 生成配置/目录）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.4.2 — 对应 AC2（daemon 拉起 core + gRPC health）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.4.3 — 对应 AC3（core 崩溃自动重启）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.4.4 — 对应 AC4（CLI 子命令注册）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-1.4.5 — 对应 AC5（Phase1 端到端 smoke）
    Given <TBD>
    When <TBD>
    Then <TBD>

  # ---
  # Maps to: docs/specs/tasks/task-6.1-cli-search.md
  Scenario: SCEN-6.1.1 — 对应 AC1（search 返回 Top-K）
    Given 数据目录中存在已索引的含 trigger token 的 fixture collection
    And contextforge-core 数据平面可被 daemon 自启
    When 用户在终端执行 `contextforge search "<trigger>" --collections=<coll>`
    Then CLI 经 daemon 调用 gRPC ContextService.Search 返非空 Top-K RetrievalResult 列表
    And 每条结果含 chunk_id / file_path / line_start / line_end / score / retrieval_method 等字段

  Scenario: SCEN-6.1.2 — 对应 AC2（flags 契约一致）
    Given 用户已知 SearchRequest proto 契约（query / collections / agent_scope / top_k / filters / explain）
    When 执行 `contextforge search "<q>" --collections=c1 --agent-scope=a1 --top-k=5 --source-type=markdown --language=go --explain`
    Then CLI 解析后构造的 SearchRequest 字段与 flag 取值 1:1 映射（含 SearchFilters.source_type / language）
    And `--top-k=0` 或缺省时回退默认 top_k=10

  Scenario: SCEN-6.1.3 — 对应 AC3（可解释字段 + --json）
    Given retriever 返回含全部 12 字段的 RetrievalResult
    When 用户加 --json 标志
    Then stdout 输出 JSON 序列化 SearchResponse（含 12 字段 + provenance 数组）
    And 缺省（不加 --json）输出人类可读 text 块（每结果一块，含 chunk_id / score / redaction_status / reason）

  Scenario: SCEN-6.1.4 — 对应 AC4（不展示完整 secret）
    Given retriever 返回的 RetrievalResult.redaction_status="applied"（上游 scanner+indexer 已 redact）
    When CLI 渲染 text/JSON 输出
    Then redaction_status 字段值原样透传到 stdout（无论 text 或 JSON 模式）
    And CLI 不在 content 字段上二次执行 secret scan

  Scenario: SCEN-6.1.5 — 对应 AC5（与 export 共享结果模型）
    Given 索引 fixture 含 trigger token 且 Rust tonic Search server 已启
    When grpc 客户端发起 ContextService.Search RPC
    Then 返回的 *contextforgev1.RetrievalResult 含全部 12 explainable 字段
    And provenance.len() ≥ 1（黑盒守护，沿 task-4.2 AC3 反指标）
    And 该 proto-generated 类型未来由 task-6.3 exporter 直接消费（ADR-003 单一源）
