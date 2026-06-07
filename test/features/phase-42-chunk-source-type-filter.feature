# Phase 42 · chunk-source-type-filter
# 把 chunk 检索的 source_type 过滤从 Phase 32（task-32.3 / ADR-037）据实记的 documented no-op 落地为真实过滤：
# source_type 由 file_path 确定性派生（classify_source_type 纯函数，0 schema migration，§5.3 FROZEN），
# v1 retriever 真实过滤 + 三路径 populate，console proto add-only source_type=9 请求侧 forward。
# agent_scope 据 grounding 为 memory 层概念续 documented no-op（不伪造，ADR-013）。
# ADR-047（Proposed→Accepted @ task-42.3）。0 新 dep（classify_source_type 纯 std）/ 0 网络 / 0 migration
# / 空 filter byte-equiv（ADR-004/008/013/015/037/044）。

Feature: chunk-source-type-filter — chunk 检索 source_type 过滤落地
  作为 ContextForge 维护者
  我希望把 v1 / console 检索 API 早有契约但 Phase 32 据实记为 no-op 的 source_type 过滤落地为真实能力
  以便用户可按来源类型（code/doc/config/other）筛选检索结果、关闭一个诚实缺口
  且空 filter byte-equiv、0 schema migration（source_type 由 file_path 派生）、agent_scope 据实 honest-defer

  # ---- task-42.1: source_type 派生 + 真实过滤 + populate（ADR-047 D1/D2）----

  Scenario: classify_source_type 扩展名确定性映射 code/doc/config/other
    Given core/src/retriever/mod.rs classify_source_type(file_path)（镜像 indexer::lang_hint_from_path）
    When file_path 扩展名为 .rs/.go/.py/.ts 等源码 / .md/.txt/.rst 等文档 / .toml/.yaml/.json 等配置 / 无扩展名或未知
    Then 分别返回 "code" / "doc" / "config" / "other"（确定性、纯 std、大小写不敏感）
    And 无扩展名（Makefile/LICENSE）/ 未知扩展名 → "other"（确定性优先、不做 basename 特例）

  Scenario: 三路径 populate 真实 source_type（填补 v0.1 schema gap）
    Given search() BM25 / get_chunk / search_semantic 三构造点原 source_type: DEFAULT_SOURCE_TYPE=""
    When 改为 classify_source_type(&file_path).to_string()
    Then source_type value 三路径真实可见（填补 task-4.2 §2A v0.1 schema gap）
    And ADR-047 D1 据实记可观测字段变化（非破坏性默认变更、空 filter 下过滤行为 byte-equiv）

  Scenario: source_type 真实过滤（非空 filter 仅留匹配桶）
    Given search() BM25 加 source_type post-filter（镜像 :386 language post-filter）
    And 索引含 .rs + .md + .toml 混合 fixture
    When filters.source_type = ["doc"]
    Then 仅返回 .md chunk（source_type == "doc"）
    And filters.source_type = ["code"] 仅返回 .rs chunk

  Scenario: 空 source_type filter byte-equiv（ADR-004）
    Given search() source_type post-filter 仅 !filters.source_type.is_empty() 时生效
    When filters.source_type 为空
    Then 不过滤、结果与改动前 byte-identical（仅 source_type value 由 "" 变派生值）

  Scenario: v1 读路径已就绪、retriever 真实过滤后立即生效
    Given v1 server.rs:440-453 已映射 proto filters.source_type → RetrieverFilters.source_type
    And v1 REST rest.go:137 解码完整 proto SearchRequest（含 filters）
    When retriever 真实派生 + 过滤
    Then v1 gRPC / v1 REST body {"filters":{"source_type":["doc"]}} 路径立即生效（无须改 v1 server/proto）

  Scenario: agent_scope 据实 honest-defer（memory 层概念，不伪造）
    Given agent_scope 经 grounding 为 memory 层概念（memory_items 0013 / ListMemory scope / memstore.go:629-635）
    And chunks 无 agent 关联、无可派生维度
    When 我据实分级
    Then agent_scope 续 documented no-op（窄化 retriever no-op 块仅覆盖 agent_scope，非空 → byte-equiv）
    And 不伪造 chunk-level agent_scope 过滤 [SPEC-DEFER:phase-future.chunk-agent-scope-filter]（须 ingest-path schema 工程）

  Scenario: 0 schema migration（§5.3 FROZEN）
    Given source_type 由 file_path query 时确定性派生（与 language 同源信号）
    When 落地真实过滤
    Then chunks/files/provenance 三表 §5.3 保持 FROZEN（0 schema migration）
    And 确定性派生 == 存储值（等价正确、更 surgical）
    And importer 显式 source_type 打标续 [SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]（须 §5.3 解冻）

  # ---- task-42.2: console-api source_type 请求侧 forward（ADR-047 D3）----

  Scenario: console_data_plane SearchRequest add-only source_type=9（proto add-only）
    Given console_data_plane.proto SearchRequest 既有字段 1-8（semantic=7/hybrid=8）
    When add-only repeated string source_type = 9
    Then 既有字段 1-8 号冻结（ADR-015 add-only）
    And prost wire-tag 字段号 9 in-crate 断言
    And 空 source_type → 不过滤 backward-compat（既有 client 不传 → 行为不变）

  Scenario: console 响应侧已就绪、populate 后立即显示真实 source_type
    Given console SearchResultItem.source_file_type=5 + data_plane/search.rs:378 source_file_type: h.source_type
    When task-42.1 populate h.source_type
    Then console 响应 source_file_type 立即显示真实派生值（无须改响应侧）

  Scenario: handleSearch ?source_type= 请求侧 forward（query param + body 并集，镜像 ?semantic/?hybrid）
    Given internal/contractv1.SearchRequest add-only SourceType []string
    When handleSearch 收到 ?source_type=code&source_type=doc（repeated query param）+ body source_type
    Then 并集合并到 SearchRequest.SourceType（镜像 ?semantic/?hybrid OR-merge）
    And grpcclient 映射 → console_data_plane SearchRequest.source_type

  Scenario: data_plane 统一 post-filter 覆盖 BM25/semantic/hybrid 三路径
    Given data_plane/search.rs 按 req.source_type 对汇总后的 hit post-filter（利用 populate 的 h.source_type）
    When req.source_type 非空
    Then 仅留 req.source_type.contains(&h.source_type) 的 hit（三检索路径一致）
    And req.source_type 空 → 不过滤 byte-equiv

  # ---- task-42.3: v0.35.0 收口 + agent_scope honest-defer + 0-dep/0-migration 守线 ----

  Scenario: v0.35.0 收口 + 默认零依赖零迁移守线
    Given task-42.1 + task-42.2 全 Done
    When task-42.3 收口
    Then scripts/console_smoke.sh v32[51/51]（REAL source_type 真实过滤端到端：.rs+.md fixture / ?source_type=doc 仅 doc / ?source_type=code 仅 code，distinguishing）+ TestTask423 无 [37/37]..[50/50] 回归
    And ADR-047 据 D1-D4 真实测试 ratify Proposed→Accepted
    And ADR-037 add-only Phase-42 Amendment（source_type no-op 被真实过滤 supersede / agent_scope no-op 据实保持）
    And ADR-015（proto add-only）/ ADR-024 / ADR-044（console 请求侧 forward 范式）/ ADR-004（空 filter byte-equiv）守线
    And 0 新 dep（classify_source_type 纯 std）+ 0 网络 + 0 schema migration（§5.3 FROZEN）
    And 真实 v0.35.0 tag/run/digest/tlog post-tag-push 回填（ADR-013 不预填）
