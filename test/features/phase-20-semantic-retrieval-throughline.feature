# language: en
# Maps to:
#   - docs/specs/phases/phase-20-semantic-retrieval-throughline.md
#   - docs/specs/tasks/task-20.1-console-api-semantic-forward.md
#   - docs/specs/tasks/task-20.2-real-recall-via-retriever.md
#   - docs/specs/tasks/task-20.3-closeout-v0.13.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 20 semantic-retrieval-throughline。Scenario ID 在各 task spec §7 追踪表映射到具体测试。

Feature: phase-20-semantic-retrieval-throughline
  In order to 让 console-api 的语义检索真正端到端生效 + 真实召回经生产 Retriever 热路径
  As Phase 20 内核（console-api ?semantic=true 贯通 + recall-via-Retriever + v0.13.0 收口）
  I want 闭合 v0.12.0 evidence §3b 两条 caveat，且默认 BM25 行为不退化

  # ---
  # Maps to: docs/specs/tasks/task-20.1-console-api-semantic-forward.md (TEST-20.1.2/20.1.3)
  Scenario: SCEN-20.1.1 — 对应 AC1（console-api ?semantic=true 转发 → gRPC 语义分派）
    Given console-api /v1/search 与 console_data_plane SearchRequest（add-only semantic 字段）
    When  POST /v1/search?semantic=true（或 body semantic:true）经 handleSearch OR-merge + grpcclient 透传到 Rust SearchService.Query
    Then  SearchServer.query 走语义分派分支，结果 retrieval_method == "vector"；既有不带 semantic 的请求逐字节不变（BM25）；{result, trace} 响应 shape 与 22-endpoint conformance 不破坏

  # ---
  # Maps to: docs/specs/tasks/task-20.2-real-recall-via-retriever.md (TEST-20.2.1/20.2.2)
  Scenario: SCEN-20.2.1 — 对应 AC2（真实召回经生产 Retriever::search_semantic 热路径）
    Given 生产 Retriever（real scanner+chunker 索引）+ 0-dep BruteForceVectorBackend，deterministic provider 守 CI wiring / real fastembed 守真实召回
    When  经 Retriever::search_semantic 对 30 golden 查询检索 top-10
    Then  deterministic 路径命中预期 chunk（retrieval_method=vector，CI 可断言）；real fastembed 路径产真实 SemanticRecall@K（@10 ≥ 0.70 gate PASS），数据源 ADR-013 如实记录（含 uncapped-chunk 膨胀 caveat + top1/MRR 区分度）

  # ---
  # Maps to: docs/specs/tasks/task-20.3-closeout-v0.13.0.md (TEST-20.3.1)
  Scenario: SCEN-20.3.1 — 对应 AC3/AC4（smoke v10 真实语义断言 + v0.13.0 收口）
    Given scripts/console_smoke.sh v10 step 29 + v0.13.0 release docs + ADR-024
    When  REAL 模式 POST /v1/search?semantic=true 经 console-api（合规 Linux host / CI）
    Then  step 29 断言 {result, trace} 保形 AND trace candidate_generation_steps 含 vector-bruteforce（语义路径经 console-api 真生效）；ADR-024 据真实非合成 Proposed→Accepted；phase-20 §6 AC1-5 全 met；ADR-013 不预判召回阈值
