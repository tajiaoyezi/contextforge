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
  Scenario: SCEN-4.2.1 — 对应 AC1（可解释字段完整，12 字段 PRESENT 契约）
    Given task-2.4 indexer 已写入 ≥1 chunk（scanner 路径）
    When  Retriever::search(query="explainmarker42", explain=false)
    Then  返回 SearchResult 含 PRD §search response 全部 12 字段：chunk_id / context_id / source_type / file_path / line_start / line_end / score / retrieval_method / reason / agent_scope / redaction_status / provenance；v0.1 schema gap 字段（context_id="" / source_type="" / agent_scope=[] / redaction_status="applied"）按 §2A 决策返默认值

  Scenario: SCEN-4.2.2 — 对应 AC2（定位回原文行号 file_path + line_start/end 精确）
    Given fixture 文件含多行可分块 markdown（headings 切多 chunk，line_start/end 跨多行）
    When  Retriever::search 命中其中一条
    Then  result.file_path 精确为 fixture 路径；line_start / line_end 落在 fixture 实际行数范围内（line_start ≤ line_end ≤ 总行数）；按 file_path + line_start/end 可定位回原始内容（不模糊不偏移）

  Scenario: SCEN-4.2.3 — 对应 AC3（schema coverage 100% + 反指标 provenance≥1 黑盒守护）
    Given 多文件 fixture（scanner 路径，无 importer provenance 行）
    When  Retriever::search 返回 N 条结果
    Then  每条 result.provenance.len() ≥ 1（合成 scanner-default：importer="scanner" / original_path=file_path / imported_at=indexed_at）；schema coverage 100%（12 字段 PRESENT，struct 强制）；反指标：不允许任一结果出现 provenance 为空的"黑盒高分"

  Scenario: SCEN-4.2.4 — 对应 AC4（Rust public API 调试入口 Retriever::explain）
    Given 索引非空 fixture
    When  Retriever::explain(opts) 调用（v0.1 调试入口 — 等价 search(opts with explain=true)）
    Then  返回 Ok(Vec<SearchResult>)，且每条 result.reason 非空（含 "bm25 hit" 或 "matched terms" 词）+ matched_terms 非空（task-4.2 enrichment）；CLI / REST / MCP / gRPC 在 Phase 6/7 wrap 本方法即可

  Scenario: SCEN-4.2.5 — 对应 AC5（Phase 4 端到端 smoke 落点 core/tests/phase4_smoke.rs）
    Given core/tests/phase4_smoke.rs 含 #[test] fn phase_4_end_to_end_smoke
    When  cargo test --test phase4_smoke 运行（主 agent §4 Gate 3 phase-4 §6 端到端 smoke 触发点）
    Then  跑完整链路 scanner→parser→chunker→indexer→retriever→explain，断言：12 字段全 PRESENT / 每条 provenance ≥1 / file_path+lines 精确 / 空 query 安全（不 panic）/ explain() reason 非空
