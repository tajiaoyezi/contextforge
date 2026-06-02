# language: en
# Maps to:
#   - docs/specs/phases/phase-29-live-vector-recall.md
#   - docs/specs/tasks/task-29.1-vector-backend-factory-and-hotpath-injection.md
#   - docs/specs/tasks/task-29.2-qdrant-live-knn-and-recall-harness.md
#   - docs/specs/tasks/task-29.3-lancedb-ann-index-tuning-and-backend-matrix.md
#   - docs/specs/tasks/task-29.4-closeout-v0.22.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 29 live-vector-recall。Scenario ID 在各 task spec §7 追踪表映射到测试 / 真实 run。

Feature: phase-29-live-vector-recall
  In order to 把 Phase 25 已落地的契约层 / 参数校验层转为真实 live 向量召回，并把真实 backend 接入生产热路径
  As Phase 29 内核（vector-backend 工厂 + server.rs 热路径注入 + qdrant live KNN + lancedb 真实 ANN 索引 + 多 backend 选择矩阵真实测量）
  I want select_vector_backend 工厂消除 server.rs 硬编码 BruteForce + qdrant connect→ensure-create→upsert→KNN live 端到端（无 server 时诚实延后）+ lancedb 真实 IVF_PQ/HNSW 索引建索引并实测召回，且默认构建仍 0 vector dep + BruteForce semantic baseline 不变、受阻维度（live-server / 大语料）如实记录不伪造（ADR-013）

  # ---
  # Maps to: docs/specs/tasks/task-29.1-vector-backend-factory-and-hotpath-injection.md (TEST-29.1.1)
  Scenario: SCEN-29.1.1 — 对应 AC1（vector-backend 工厂默认 ""/"brute" → BruteForce）
    Given select_vector_backend(name, dim) 工厂仿 core/src/embedding/factory.rs::select_provider（factory.rs:27-30），默认构建不开任何 vector feature
    When  以空串 "" 或 "brute" 调 select_vector_backend，并以 "qdrant"/"lancedb" 在其 feature 关闭时调用
    Then  ""/"brute" 返回 BruteForceVectorBackend（始终可用，0-dep，deterministic 可单测）+ "qdrant"/"lancedb" 在 feature 关闭时返回诚实 Err（不伪造成功，ADR-013）；默认构建 0 新 vector dep（ADR-004 local-first，验证 D5）（TEST-29.1.1 / TEST-29.1.2）

  # ---
  # Maps to: docs/specs/tasks/task-29.1-vector-backend-factory-and-hotpath-injection.md (TEST-29.1.3)
  Scenario: SCEN-29.1.3 — 对应 AC2（server.rs 热路径经工厂注入；默认构建 semantic+hybrid 仍工作）
    Given server.rs:302（hybrid 路径）+ server.rs:341（semantic 路径）的硬编码 BruteForceVectorBackend::new() 改为经 select_vector_backend 工厂注入，兑现 [SPEC-DEFER:phase-future.vector-retrieval-integration]（phase-25 spec line 44）
    When  在默认构建（无 vector feature）下跑 cargo test --workspace 走 semantic + hybrid 热路径
    Then  热路径经工厂取得 backend + 默认回落 BruteForce → semantic + hybrid 召回仍工作 + cargo test --workspace 不受影响（deterministic CI-verifiable）（TEST-29.1.3）

  # ---
  # Maps to: docs/specs/tasks/task-29.2-qdrant-live-knn-and-recall-harness.md (TEST-29.2.1/29.2.2)
  Scenario: SCEN-29.2.1 — 对应 AC1（qdrant live KNN 端到端；无 server 诚实延后）
    Given phase29 harness 自 core/examples/phase20_recall_via_retriever.rs 克隆，把 BruteForce 换为 QdrantBackend::connect(QdrantConnConfig::from_env())，gated 于 feature vector-qdrant + embedding-fastembed，首次真实兑现 [SPEC-DEFER:phase-future.qdrant-server-lifecycle]
    When  对真实 qdrant server（manual / dev-box）跑 connect→ensure-create→upsert→KNN（live 读路径 qdrant.rs:330-371），CI 无 server 时探测 backend.health()
    Then  health()==Unreachable 时 eprintln + exit 0 诚实延后（CI 无 server，ADR-013，不伪造召回），harness 仍编译通过 deterministic 兑现接线（TEST-29.2.2）；有 server 时 connect→ensure-create→upsert→KNN 全通过，真实召回数值待实测回填 §10 + v0.22.0 evidence（绝不预填，TEST-29.2.1）；single-node 部署基线文档化，cluster/replication [SPEC-DEFER:phase-future.qdrant-deployment-topology]（TEST-29.2.3）

  # ---
  # Maps to: docs/specs/tasks/task-29.3-lancedb-ann-index-tuning-and-backend-matrix.md (TEST-29.3.1/29.3.2)
  Scenario: SCEN-29.3.1 — 对应 AC1（lancedb 真实 IVF_PQ/HNSW 索引建索引 + 实测召回）
    Given LanceIndexTuning / LanceAnnIndex 参数契约层（lance_db.rs:33-108）之上，在 feature vector-lancedb 下对内嵌 Lance dataset 真实建 IVF_PQ/HNSW 索引（in-process，n 仍适度），兑现 [SPEC-DEFER:phase-future.lancedb-index-tuning]
    When  经 LanceIndexTuning 建真实 ANN 索引并对 live 读路径（lance_db.rs:270-332）实测召回，再跑多 backend（brute / sqlite-vec / lancedb / qdrant where runnable）选择矩阵真实测量
    Then  真实 IVF_PQ/HNSW 索引建成 + 召回实测回填（不预填，TEST-29.3.1）；多 backend 选择矩阵真实测量喂入 ADR-030 D3 + ADR-023 tiers 的 add-only Amendment（不溯改其 D-body，ADR-014 D5，TEST-29.3.2）；lancedb feature build caveat（rustc ICE on broad cargo test → cargo build + --lib scoped tests）[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]；schema compaction 真实执行或诚实延后 [SPEC-DEFER:phase-future.lancedb-schema-compaction]（TEST-29.3.3）
