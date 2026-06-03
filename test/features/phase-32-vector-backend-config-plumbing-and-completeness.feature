# language: en
# Maps to:
#   - docs/specs/phases/phase-32-vector-backend-config-plumbing-and-completeness.md
#   - docs/specs/tasks/task-32.1-vector-backend-config-plumbing.md
#   - docs/specs/tasks/task-32.2-sqlite-vec-factory-arm-and-selection-matrix.md
#   - docs/specs/tasks/task-32.3-console-provenance-and-retrieval-filter-honesty.md
#   - docs/specs/tasks/task-32.4-closeout-v0.25.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 32 vector-backend-config-plumbing-and-completeness。Scenario ID 在各 task spec §7 追踪表映射到测试 / 真实 run。
# 受阻 / 延后维度均以 [SPEC-DEFER:phase-future.<name>] 标注（sqlite-vec in-process 矩阵 cell 须本机 MSVC feature build；real chunk source_type/agent_scope filter 须 importer-side tagging + schema migration），据真实测试回填，绝不预填数值（ADR-013）。

Feature: phase-32-vector-backend-config-plumbing-and-completeness
  In order to 把 Phase 29 已落地的 select_vector_backend 工厂从「仅默认接线」补全为「经 env/config 选 backend + 工厂后端覆盖齐全 + console provenance 与契约诚实化」
  As Phase 32 内核（backend config plumbing 两热路径 + sqlite-vec factory arm + console_data_plane vector_score add-only + retrieval-filter 契约诚实化）
  I want server.rs hybrid + semantic 两热路径经 env/config 选 backend（未设/"" → BruteForce byte-equivalent 默认行为不变）+ factory 加 sqlite-vec arm（feat on→SqliteVecBackend / feat off→honest Err naming vector-sqlite）+ console_data_plane SearchResultItem add-only vector_score=16 携 provenance（parity v1 search proto）+ retrieval-filter 误导性 WARN 改为准确 no-op 契约，且默认构建仍 0 vector dep / 0 网络 / 默认行为不变（ADR-004），受阻维度（sqlite-vec 矩阵 cell / real chunk filter feature）如实记录不伪造（ADR-013）

  # ---
  # Maps to: docs/specs/tasks/task-32.1-vector-backend-config-plumbing.md (TEST-32.1.1 / TEST-32.1.2)
  Scenario: SCEN-32.1.1 — 对应 AC1（vector backend config plumbing：env→server.rs 两热路径，default 保形）
    Given server.rs hybrid 路径（~server.rs:340）+ semantic 路径（~server.rs:367+）现仅注入默认 select_vector_backend("", 0)（无 vector config 接线），而 CoreService 已持 data_dir: PathBuf（server.rs:52，main 经 resolve_data_dir / env CONTEXTFORGE_DATA_DIR 构造 server.rs:504-521）
    When  以 env/config（仿 CONTEXTFORGE_DATA_DIR pattern）解析 backend name 经 select_vector_backend 注入 hybrid + semantic 两热路径，并在未设 / "" 时跑默认构建
    Then  两热路径经 env/config 取得 backend + 未设 / "" 回落 BruteForce（与 TEST-29.1.3 + 既有语义 / hybrid 行为 byte-equivalent，默认行为不变，ADR-004）+ cargo test --workspace 不受影响（deterministic CI-verifiable，TEST-32.1.1 + TEST-32.1.2，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-32.2-sqlite-vec-factory-arm-and-selection-matrix.md (TEST-32.2.1 / TEST-32.2.2)
  Scenario: SCEN-32.2.1 — 对应 AC2（sqlite-vec factory arm 双半 gating + 选择矩阵 wiring）
    Given core/src/retriever/vector/sqlite_vec.rs 的 SqliteVecBackend（feature vector-sqlite，name()="sqlite-vec"，in-memory vec0 vtable，task-23.2 已验 MSVC-buildable）已 impl VectorStore，但 factory.rs（match name 现仅 ""/"brute"/"qdrant"/"lancedb"）无 sqlite-vec arm
    When  为 factory 加 sqlite-vec arm（feature vector-sqlite on → SqliteVecBackend::new() / feature off → 诚实 Err naming "vector-sqlite"，仿 qdrant/lancedb 双半 gating），并经 in-process 选择矩阵 wiring 选择 sqlite-vec backend
    Then  feature off → 诚实 Err 命名 vector-sqlite（不静默回落 BruteForce、不伪造成功，ADR-013）+ feature on → factory 返回 name()=="sqlite-vec" backend + 选择矩阵 wiring 🟢 deterministic 兑现接线（TEST-32.2.1 + TEST-32.2.2，真实测试通过后回填）；矩阵 recall/latency CELL 须本机 MSVC feature build → honest-defer [SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]，不预填数值（ADR-013）

  # ---
  # Maps to: docs/specs/tasks/task-32.3-console-provenance-and-retrieval-filter-honesty.md (TEST-32.3.1 / TEST-32.3.2)
  Scenario: SCEN-32.3.1 — 对应 AC3（console_data_plane vector_score add-only provenance + retrieval-filter 契约诚实 no-op）
    Given v1 search proto SearchResultItem 已有 vector_score=13 + retrieval_method=8，而 console data-plane proto SearchResultItem（console_data_plane.proto:185-201）有 retrieval_method=13 但缺 vector_score（字段至 citation=15）；retriever mod.rs:325 现发误导性 WARN（自称 source_type/agent_scope filter 尚未落地）（mod.rs:135 SearchFilters 有 source_type/agent_scope，但 chunks 表 FROZEN §5.3 indexer/mod.rs:117 无对应列；SearchResult.source_type 硬编码 DEFAULT_SOURCE_TYPE mod.rs:452、agent_scope 硬编码 Vec::new() mod.rs:459），real chunk filter 须 importer-side source_type tagging + schema migration [SPEC-DEFER:phase-future.chunk-source-type-filter] [SPEC-DEFER:phase-future.chunk-agent-scope-filter]
    When  对 console_data_plane SearchResultItem 加 add-only vector_score = field 16（parity v1 search proto provenance），并把 mod.rs:325 误导性 WARN 改为准确 no-op 契约（说明真实 chunk filter 须 importer-side source_type tagging + schema migration）
    Then  console 数据面 / 控制面经新 vector_score=16 携 provenance（add-only，既有字段 1-15 不动，不破契约 ADR-004）+ retrieval-filter 契约诚实化为准确 no-op（默认空 filter 结果与既有完全一致）+ 新 backlog 标注 [SPEC-DEFER:phase-future.chunk-source-type-filter] + [SPEC-DEFER:phase-future.chunk-agent-scope-filter]（real chunk filter 为 import-path feature，本 phase 据实不实现、不伪造完成，ADR-013）（TEST-32.3.1 + TEST-32.3.2，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-32.4-closeout-v0.25.0.md (TEST-32.4.1 / TEST-32.4.2)
  Scenario: SCEN-32.4.1 — 对应 AC4（默认行为不变 + v0.25.0 closeout）
    Given task-32.1 + task-32.2 + task-32.3 全 Done（config plumbing 两热路径 / sqlite-vec arm 双半 gating / console vector_score add-only + filter no-op），current Phase 31 smoke [40/40]
    When  跑 scripts/console_smoke.sh 新增 step → [41/41]，产出 v0.25.0 release docs（evidence/artifacts/README/RELEASE_NOTES），ADR-037 据真实测试 ratify（sqlite-vec 矩阵 cell honest-defer [SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix] 部分 ratify）+ ADR-034 add-only Amendment（sqlite-vec arm 补全工厂）+ roadmap §3/§4 add-only + s2v-adapter add-only + phase §6 闭合
    Then  默认行为 / proto / 既有契约不变（ADR-004——console proto add-only field 16、factory arm add-only、filter no-op 不破契约）+ smoke [41/41]（既有 step 不退化，denominators 不溯改 ADR-014 D5）+ ADR-037 Proposed→Accepted（逐 D 如实，sqlite-vec 矩阵 cell 受阻维度部分 ratify）+ ADR-034 add-only Amendment + roadmap/adapter add-only + ADR-014 D1-D5 第 23 次激活全通过（TEST-32.4.1 + TEST-32.4.2 + 各 task LAST TEST TEST-32.1.3 / TEST-32.2.3 / TEST-32.3.3 / TEST-32.4.3，真实跑出后回填）
