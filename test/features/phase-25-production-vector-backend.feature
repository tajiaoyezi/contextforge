# language: en
# Maps to:
#   - docs/specs/phases/phase-25-production-vector-backend.md
#   - docs/specs/tasks/task-25.1-qdrant-server-lifecycle.md
#   - docs/specs/tasks/task-25.2-lancedb-buildability-and-index-tuning.md
#   - docs/specs/tasks/task-25.3-closeout-v0.18.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 25 production-vector-backend。Scenario ID 在各 task spec §7 追踪表映射到具体测试。

Feature: phase-25-production-vector-backend
  In order to 让生产规模向量 backend（qdrant external server / lancedb embedded columnar）从 spike 推向可生产化
  As Phase 25 内核（qdrant 生命周期层 + lancedb 真实可构建性调查 + 生产 backend 选择矩阵 + v0.18.0 收口）
  I want qdrant 有 connect/health/ensure-create 契约层（不连 live server 可验证）+ lancedb 真实 dev-box 可构建性定论，且默认构建恒 0-vector-dep BM25 baseline、受阻态（live-server / protoc）如实记录不伪造

  # ---
  # Maps to: docs/specs/tasks/task-25.1-qdrant-server-lifecycle.md (TEST-25.1.1/25.1.2/25.1.3/25.1.4)
  Scenario: SCEN-25.1.1 — 对应 AC1（qdrant server 生命周期层契约，不连 live server）
    Given core/src/retriever/vector/qdrant.rs QdrantBackend 连接配置（url/timeout/可选 api-key/TLS）+ health-probe 入口 + collection ensure-create 决策（decide_ensure 纯函数）+ 既有 QDRANT_URL env / Qdrant::from_url
    When  feature vector-qdrant 下不连 live server 跑契约测试：config.validate()、health() 连不存在端点、decide_ensure 喂入构造的 describe 响应
    Then  合法 config Ok / url 空·dim=0·名空 Err（TEST-25.1.1）；无 server health() 返可识别 unreachable 不 panic 不静默成功（TEST-25.1.2）；ensure-create 存在且匹配→Reuse·不存在→Create·dim/metric 不匹配→Error 三分支（TEST-25.1.3）；真实 KNN over live qdrant [SPEC-DEFER:phase-future.qdrant-server-lifecycle] CI 无 server 诚实延后不伪造 + 不破坏三 trait 签名（TEST-25.1.4）；默认构建 0 新 dep

  # ---
  # Maps to: docs/specs/tasks/task-25.2-lancedb-buildability-and-index-tuning.md (TEST-25.2.1/25.2.2/25.2.3/25.2.4) + docs/spikes/phase-25-lancedb-buildability.md
  Scenario: SCEN-25.2.1 — 对应 AC2（lancedb 真实可构建性调查 + 索引调参参数 + backend 契约不退化）
    Given core/Cargo.toml vector-lancedb（lancedb 0.30 + arrow-array 58 + futures 0.3 optional）+ lance_db.rs（protoc 前置构建 + nearest_to Cosine）+ 索引调参参数结构（IVF_PQ/HNSW num_partitions/num_sub_vectors/metric + compaction 口径）
    When  在 dev box 真实跑 cargo build --features vector-lancedb（含 protoc 前置探测/安装，仿 task-23.2 pattern）+ 索引调参参数 validate() + 既有 backend 契约测试
    Then  构建通过则记真实凭据（rustc/protoc/cmake 版本 + 耗时 + arch，TEST-25.2.1）或确证受阻诚实 stop-condition（承 protoc-prereq + sqlite-vec MSVC 先例，ADR-013 不伪造跨平台通过）；docs/spikes/phase-25-lancedb-buildability.md 三态如实标 + 单机 caveat（TEST-25.2.2）；索引调参参数 partitions=0·sub_vectors 不整除 dim → Err（TEST-25.2.3）；既有 open→index→search KNN + dim mismatch 不退化、真实 ANN 索引性能 [SPEC-DEFER:phase-future.lancedb-index-tuning] / compaction [SPEC-DEFER:phase-future.lancedb-schema-compaction] 延后（TEST-25.2.4）

  # ---
  # Maps to: docs/specs/tasks/task-25.3-closeout-v0.18.0.md (TEST-25.3.1/25.3.2/25.3.3/25.3.5)
  Scenario: SCEN-25.3.1 — 对应 AC1/AC3/AC5（生产 backend 选择矩阵 + smoke v15 + v0.18.0 收口 + ADR-030 ratify）
    Given 多 backend 生产选择矩阵（语料规模 × 部署形态 → hnsw/sqlite-vec/lancedb/qdrant + caveat）+ scripts/console_smoke.sh v15 + v0.18.0 release docs + ADR-030（production-vector-backend）
    When  选择矩阵写入 release docs + adapter；smoke v15 文档化 Phase 25 生产 backend 状态（feature 层 + 默认构建 intact）；ADR-030 据 task-25.1/25.2 真实结果 ratify
    Then  选择矩阵每档含 caveat（live-server 依赖 / protoc 前置 / 平台限制，TEST-25.3.1）；smoke 既有 step 不退化 + bash -n exit 0；ADR-030 D1-D4 据真实非合成验证 Proposed→Accepted（受阻维度记录维持）+ ADR-023 D3/D4 tier add-only Amendment（不溯改正文 D5，TEST-25.3.3）；qdrant live-server 集成 [SPEC-DEFER:phase-future.qdrant-server-lifecycle]；phase-25 §6 全 met；ADR-014 D1-D5（第十六次激活）全通过
