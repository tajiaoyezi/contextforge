# language: en
# Maps to:
#   - docs/specs/phases/phase-21-retrieval-quality.md
#   - docs/specs/tasks/task-21.1-hybrid-scoring.md
#   - docs/specs/tasks/task-21.2-reranker-pipeline.md
#   - docs/specs/tasks/task-21.3-closeout-v0.14.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 21 retrieval-quality。Scenario ID 在各 task spec §7 追踪表映射到具体测试。

Feature: phase-21-retrieval-quality
  In order to 在 BM25 / 语义双路之上提升 top-k 排序质量（hybrid 融合 + reranker 重排）
  As Phase 21 内核（hybrid scoring + reranker + v0.14.0 收口）
  I want 融合与重排管道确定性可 CI 验证、real 模型真实质量据真实 eval 如实记录，且默认 BM25 行为不退化

  # ---
  # Maps to: docs/specs/tasks/task-21.1-hybrid-scoring.md (TEST-21.1.1/21.1.2)
  Scenario: SCEN-21.1.1 — 对应 AC1（hybrid RRF 融合序确定性 + retrieval_method="hybrid" + hybrid_score）
    Given Retriever::search_hybrid（RRF k=60 融合 BM25 与向量两路）+ proto SearchRequest.hybrid=8 / RetrievalResult.hybrid_score=15（add-only）
    When  固定 BM25/vector 分数经 fusion.rs::fuse 融合，或 req.hybrid=true 经 server.rs CoreService 分派
    Then  融合按 hybrid_score 降序 + chunk_id 升序确定性 tie-break，retrieval_method == "hybrid" 且 hybrid_score 填实（test_21_1_hybrid_dispatches_fusion_path）；单路缺失时降级不 panic；既有 search()/search_semantic() 逐字段不变（默认 BM25 baseline，ADR-025 D1/D4）

  # ---
  # Maps to: docs/specs/tasks/task-21.2-reranker-pipeline.md (TEST-21.2.1/21.2.2/21.2.3)
  Scenario: SCEN-21.2.1 — 对应 AC2（Reranker trait + 确定性 IdentityReranker 管道 + real cross-encoder 真实质量）
    Given Reranker trait（Send+Sync+Debug，#[non_exhaustive] RerankError）+ 确定性 IdentityReranker（0 模型依赖）+ feature-gated CrossEncoderReranker + Retriever::with_reranker seam
    When  IdentityReranker 重排固定候选集（CI 默认构建），或 real CrossEncoderReranker 经 reranker-fastembed 对 (query,doc) 对联合打分（本地 real model run）
    Then  确定性路径按 score 降序 + chunk_id 升序稳定重排、不丢候选、标注 reason provenance（test_21_2_1/test_21_2_2，CI 可断言）；real 路径按联合相关性重排（test_21_2_3 real BGE 模型）；real top-1/MRR 提升真实质量据 task-21.3 dogfood eval 记录或受阻如实 defer（ADR-013，ADR-026 D5）

  # ---
  # Maps to: docs/specs/tasks/task-21.3-closeout-v0.14.0.md (TEST-21.3.1/21.3.2/21.3.3)
  Scenario: SCEN-21.3.1 — 对应 AC3/AC4（eval hybrid/reranked 列 + smoke v11 + v0.14.0 收口 + ADR-025/026 ratify）
    Given internal/eval Report add-only hybrid/reranked 列 + SummarizePasses + internal/cli/eval.go --hybrid/--rerank flag + scripts/console_smoke.sh v11 + v0.14.0 release docs
    When  eval run --semantic --hybrid --rerank 多路汇报 + smoke step30 断言多路 report shape + gate（ADR-013 不预判召回阈值）+ dogfood eval 经生产 Retriever 跑 hybrid/reranked vs baseline 真实召回
    Then  无 hybrid/rerank pass 时 Report byte-equivalent 既有（add-only）；smoke 既有 step 不退化 + bash -n exit 0；ADR-025 据真实 dogfood 数据 ratify、ADR-026 据真实 cross-encoder uplift ratify 或受阻如实维持 Proposed（ADR-013）；phase-21 §6 AC1-5 全 met
