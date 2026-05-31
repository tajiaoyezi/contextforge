# language: en
# Maps to:
#   - docs/specs/phases/phase-23-vector-persistence-and-cross-platform.md
#   - docs/specs/tasks/task-23.1-hnsw-graph-persistence.md
#   - docs/specs/tasks/task-23.2-sqlite-vec-cross-platform.md
#   - docs/specs/tasks/task-23.3-closeout-v0.16.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 23 vector-persistence-and-cross-platform。Scenario ID 在各 task spec §7 追踪表映射到具体测试。

Feature: phase-23-vector-persistence-and-cross-platform
  In order to 让向量索引可持久化、sqlite-vec 跨平台可用、并务实评估增量索引
  As Phase 23 内核（hnsw 图持久化 + sqlite-vec 跨平台 + 增量索引评估 + v0.16.0 收口）
  I want hnsw 图可序列化往返 + sqlite-vec 在 Windows MSVC 真实可构建，且默认构建恒 0-vector-dep BM25 baseline、受阻态如实记录不伪造

  # ---
  # Maps to: docs/specs/tasks/task-23.1-hnsw-graph-persistence.md (TEST-23.1.1/23.1.2/23.1.3)
  Scenario: SCEN-23.1.1 — 对应 AC1（hnsw 图持久化往返 + rebuild-on-load fallback）
    Given core/src/retriever/vector/hnsw.rs HnswBackend save/load（路径 B：持久化 (normalized embedding, chunk_id) 输入集到 VectorIndexConfig.persistence_path）+ open 接通 persistence_path
    When  index→save→新实例 load→search（feature vector-hnsw），或 load 缺失/损坏/版本不匹配文件
    Then  重载后 search 命中与原实例等价的 chunk_id 序（路径 B 重建确定性，TEST-23.1.1）；load 失败返 Ok(false) rebuild-on-load 不 panic 不静默成功（TEST-23.1.2）；persistence_path:None 纯内存等价 + 不破坏三 trait 签名（TEST-23.1.3）；默认构建 0 新 dep（serde/serde_json 已 direct）

  # ---
  # Maps to: docs/specs/tasks/task-23.2-sqlite-vec-cross-platform.md (TEST-23.2.3) + docs/spikes/phase-23-sqlite-vec-cross-platform.md
  Scenario: SCEN-23.2.1 — 对应 AC2/AC3（sqlite-vec Windows MSVC 跨平台真实调查 + backend 契约不退化）
    Given core/Cargo.toml vector-sqlite + sqlite_vec.rs（sqlite-vec 0.1.9 vec0）+ 三路径调查（bundled amalgamation / 预编译扩展 / 替代绑定）
    When  在 x86_64-pc-windows-msvc（rustc 1.95.0）真实跑 cargo build --features vector-sqlite + 契约测试（open→index→KNN + dim mismatch）
    Then  路径(a) bundled amalgamation 真实构建+运行通过（exit 0 + 2/2 PASS），解除 Phase 18 MSVC-blocked stop-condition（ADR-013 真实非合成，不伪造）；既有 Linux gcc 路径不退化；诚实 caveat：单机/CI 默认不构建该 feature（spike 记录）

  # ---
  # Maps to: docs/specs/tasks/task-23.3-closeout-v0.16.0.md (TEST-23.3.1/23.3.2/23.3.3/23.3.5)
  Scenario: SCEN-23.3.1 — 对应 AC1/AC3/AC5（增量索引评估 + smoke v13 + v0.16.0 收口 + ADR-028 ratify）
    Given 向量增量索引评估 + scripts/console_smoke.sh v13 step 32 + v0.16.0 release docs + ADR-028（vector-persistence-strategy）
    When  brute-force/sqlite-vec 行级追加（index_batch 累积不 reindex）落最小增量 + deterministic 单测、hnsw 全量建图增量如实延后；smoke v13 文档化 Phase 23 状态；ADR-028 据 task-23.1/23.2 真实结果 ratify
    Then  增量追加 search 反映变更不全量重建（TEST-23.3.1）；hnsw 增量 [SPEC-DEFER:phase-future.vector-incremental-index]；smoke 既有 step 不退化 + bash -n exit 0；ADR-028 D1-D4 经真实非合成验证 Proposed→Accepted + ADR-023 Follow-ups add-only Amendment（rebuild-on-restart 前提经 hnsw 持久化解除）；phase-23 §6 全 met；ADR-014 D1-D5（第十四次激活）全通过
