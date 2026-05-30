# language: en
# Maps to:
#   - docs/specs/phases/phase-19-vector-retrieval-integration.md
#   - docs/specs/tasks/task-19.1-spike-embedding-provider.md
#   - docs/specs/tasks/task-19.2-default-backend-wiring.md
#   - docs/specs/tasks/task-19.3-semantic-search-api.md
#   - docs/specs/tasks/task-19.5-real-recall-eval.md
#
# 轻量 BDD（s2v §9.2）；Phase 19 vector-retrieval-integration。Given/When/Then 由各 task agent 实施时落实。

Feature: phase-19-vector-retrieval-integration
  In order to 让 ContextForge 把 Phase 18 的向量基础设施推到生产语义检索
  As Phase 19 内核（embedding provider + 默认 backend 生产 wiring + 语义 API + 真实召回评测）
  I want 端到端语义召回 + ADR-023 据真实数据 ratify，且既有 BM25 不退化

  # ---
  # Maps to: docs/specs/tasks/task-19.1-spike-embedding-provider.md
  Scenario: SCEN-19.1.1 — 对应 AC1（deterministic embedding provider 确定性）
    Given DeterministicEmbeddingProvider（无模型依赖，默认构建启用）
    When  对同一组文本两次调用 embed(texts)
    Then  两次输出逐字节相同的 Vec<Vec<f32>>；维度 == dim()；不同文本得不同向量

  # ---
  # Maps to: docs/specs/tasks/task-19.2-default-backend-wiring.md + task-19.3-semantic-search-api.md
  Scenario: SCEN-19.3.1 — 对应 AC2/AC3（index → semantic search roundtrip）
    Given Retriever 接入选定默认 backend + EmbeddingProvider，索引含 ≥2 chunk
    When  /v1/search?semantic=true&q="<自然语言查询>" 经 Go→Rust gRPC 语义路径
    Then  返回非空结果，含 vector_score + embedding_provider provenance；既有 BM25-only 路径（无 semantic flag）行为不变

  # ---
  # Maps to: docs/specs/tasks/task-19.5-real-recall-eval.md + task-19.4-smoke-v9.md
  Scenario: SCEN-19.5.1 — 对应 AC5（real-recall eval gate）
    Given 真实 embedding provider 对 dogfood 语料生成 embedding
    When  eval --semantic 跑 SemanticRecall@K（K=5,10）
    Then  产出真实 SemanticRecall@5/10 数值（非合成、非伪造）；MeetsRecallGate 据实测判定；结果 feed ADR-023 ratify
