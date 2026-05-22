# language: en
# Maps to:
#   - docs/specs/tasks/task-4.1-retriever.md
#   - docs/specs/tasks/task-4.2-explain.md
#
# 轻量 BDD（s2v §9.2）；module=retriever 跨 task 4.1/4.2，本文件追加各 task 的 Scenario 组。
# 占位场景由 task agent 实施时填 Given/When/Then。

Feature: retriever
  In order to 让用户与下游 CLI/REST/MCP/eval 在 Phase 2 索引上做可解释的 BM25 + metadata + filter 检索
  As Phase 4 retrieval-explain 内核（task-4.2 / task-6.1/6.2 / task-7.1 / task-8.1 上游依赖）
  I want BM25/metadata/filter 检索 + explainable retrieval trace + 可解释 result schema

  # ---
  # Maps to: docs/specs/tasks/task-4.1-retriever.md
  Scenario: SCEN-4.1.1 — 对应 AC1（BM25+metadata Top-K）
    Given task-2.4 indexer 已写入 ≥2 chunk（不同文件、不同 content）
    When  Retriever::open + search(query="uniquetoken", top_k=10)
    Then  返回非空 Vec<SearchResult>，结果含 chunk_id / file_path / line_start / line_end / language / score / content；score > 0；retrieval_method="bm25"

  Scenario: SCEN-4.1.2 — 对应 AC2（filter 契约一致）
    Given fixture 含 .md (markdown) 与 .rs (rust) 文件
    When  search 带 SearchFilters { language: ["rust"] }
    Then  结果全部 language="rust"；non-rust 文件全部被过滤掉

  Scenario: SCEN-4.1.3 — 对应 AC3（空/错误 query 不 panic）
    Given Retriever 已 open，索引非空
    When  search 传入 query="" / "   " / 非法 QueryParser 语法 "??!!"
    Then  返回 Ok(Vec::new()) 不 panic，不返回 Err

  Scenario: SCEN-4.1.4 — 对应 AC4（性能 P95<500ms）
    Given task-8.1 eval-harness（本 task 不跑大规模 benchmark）
    When  Phase 8 真实大仓库压测
    Then  10 万 chunk BM25/metadata/filter P95 < 500ms；本 task 仅架构支持，不硬测

  Scenario: SCEN-4.1.5 — 对应 AC5（tokenizer/boost/exact）
    Given fixture 含 "alpha beta gamma" 跨多文件
    When  search "alpha beta" → 不要求完全相邻；search "\"alpha beta\"" → 要求 phrase 相邻；boost 让命中 file_path 含相关词的文档分数 > 仅 content 命中的文档
    Then  exact phrase 收紧命中集；boost 影响排名顺序；RetrieverConfig.tokenizer 默认 "default"，CJK / n-gram 留接入点（PRD §O11 R8）

  # ---
  # Maps to: docs/specs/tasks/task-4.2-explain.md
  Scenario: SCEN-4.2.1 — 对应 AC1（可解释字段完整）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.2.2 — 对应 AC2（定位回原文行号）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.2.3 — 对应 AC3（覆盖率≥90%/禁黑盒）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.2.4 — 对应 AC4（gRPC/CLI 调试入口）
    Given <TBD>
    When <TBD>
    Then <TBD>

  Scenario: SCEN-4.2.5 — 对应 AC5（Phase4 端到端 smoke）
    Given <TBD>
    When <TBD>
    Then <TBD>
