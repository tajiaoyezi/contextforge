# language: en
# Maps to:
#   - docs/specs/phases/phase-34-vector-config-completeness.md
#   - docs/specs/tasks/task-34.1-vector-dim-auto-negotiation.md
#   - docs/specs/tasks/task-34.2-vector-backend-config-file.md
#   - docs/specs/tasks/task-34.3-closeout-v0.27.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 34 vector-config-completeness（补完 Phase 32 起的 vector-backend 配置故事，刻意小版本——Phase 31/33 后绿色 backlog 已薄，诚实优先于凑数，ADR-013 + ADR-039）。Scenario ID 在各 task spec §7 追踪表映射到测试 / 真实 run。
# 受阻 / 据实非问题维度均以 [SPEC-DEFER:phase-future.<name>] 标注（默认 BruteForce dim-agnostic 故默认构建 dim 协商接受任意 dim，真正强制仅咬声明 dim 的 feature backend，其 live 演练须 feature build；daemon.Options.DataDir 字段重构仍延后，本版沿用已验证的 CONTEXTFORGE_DATA_DIR 同款跨进程 env 桥），据真实测试回填，绝不预填数值（ADR-013）。

Feature: phase-34-vector-config-completeness
  In order to 补完 Phase 32 起步的 vector-backend 配置链路（select_vector_backend 丢弃 CONTEXTFORGE_VECTOR_DIM + backend 仅 env 无 config-file 入口），让 dim 协商与配置文件桥接据实可单测验证，并据实记录默认 no-op 诚实 caveat 与已存在的隔离（ADR-013 的诚实价值 + ADR-039）
  As Phase 34 内核（factory negotiate_vector_dim + VectorBackend::expected_dim 默认 None + Go [vector] section → env 桥（env-wins）+ get_source_chunk workspace 隔离 verify-only grounding-correction + v0.27.0 closeout）
  I want core/src/retriever/vector/factory.rs select_vector_backend 不再静默丢弃配置的 CONTEXTFORGE_VECTOR_DIM（以纯函数 negotiate_vector_dim(dim, backend.expected_dim()) 协商，镜像 embedding::factory::negotiate_dim，复用既有 VectorError::DimMismatch）+ VectorBackend trait 加 expected_dim(self)->Option<usize> 默认 None（dim-agnostic，BruteForce 保持 None）+ Go config 加 [vector] section（Backend/Dim toml 标签）与 setVectorEnv helper（镜像 setDataDirEnv，section 存在且对应 env 未设时才导出 CONTEXTFORGE_VECTOR_BACKEND/_DIM 供 spawn 的 core daemon 经既有 resolve_vector_backend env 路径拾取，env-wins）+ get_source_chunk 的 req.workspace_id scoping（search.rs:421-423 自 task-12.2 已存在）以 verify-only guard 测试据实固化，且默认构建仍 0 new dep / 0 网络 / 默认行为字节等价（ADR-004/008——默认 BruteForce expected_dim()=None 接受任意 dim、无 [vector] section 不导出 = unset = BruteForce 字节等价、Rust core 仍无 toml dep），受阻 / 非问题维度（feature backend dim 强制 live / daemon.Options.DataDir / get_source_chunk 隔离实为已存在）如实记录不伪造（ADR-013）

  # ---
  # Maps to: docs/specs/tasks/task-34.1-vector-dim-auto-negotiation.md (TEST-34.1.1 / TEST-34.1.2)
  Scenario: SCEN-34.1.1 — 对应 AC1（factory 纯函数 negotiate_vector_dim + VectorBackend::expected_dim，dim 不再静默丢弃）
    Given core/src/retriever/vector/factory.rs:33-39 的 select_vector_backend(name, dim) 以 `let _ = dim;` 静默丢弃 server.rs resolve_vector_backend(:540) 解析并传入的 CONTEXTFORGE_VECTOR_DIM；而 embedding 侧 core/src/embedding/factory.rs:81-96 的 negotiate_dim(provider_dim, requested) 已是 requested!=0 且不等时返回 DimMismatch{expected,got} 的成熟范式；VectorError::DimMismatch{expected,got} 已存在（core/src/retriever/vector/types.rs:83），VectorBackend trait（core/src/retriever/vector/traits.rs）尚无 dim 声明出口
    When  为 VectorBackend trait 加 expected_dim(self)->Option<usize> 默认实现返回 None（dim-agnostic），BruteForceVectorBackend 保持 None；factory 以纯函数 negotiate_vector_dim(dim, backend.expected_dim()) 替换 `let _ = dim`（requested==0 OR 声明==None → Ok；非零 requested != Some(declared) → DimMismatch），并以 0 / None-declared / 匹配 / 不匹配 四类入参直接调用该纯函数
    Then  negotiate_vector_dim 纯函数：requested==0 给 Ok + 声明 None 给 Ok + 匹配给 Ok + 不匹配给 VectorError::DimMismatch{expected,got}（expected=requested，got=declared，绝不静默截断/补齐，镜像 embedding negotiate_dim）+ CONTEXTFORGE_VECTOR_DIM 不再被静默丢弃 + 0 new dep（TEST-34.1.1，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-34.1-vector-dim-auto-negotiation.md (TEST-34.1.2)
  Scenario: SCEN-34.1.2 — 对应 AC2（默认 BruteForce dim-agnostic：任意 dim 接受，字节等价 + feature 强制 honest-defer）
    Given 默认构建 select_vector_backend("" / "brute", dim) 返回 BruteForceVectorBackend，其 expected_dim()=None（dim-agnostic）→ negotiate_vector_dim(任意 dim, None) 恒 Ok，故默认构建 dim 协商接受任意 dim，无强制（honest-caveat：默认行为字节等价，ADR-004）；真正的 dim 强制仅咬声明 dim 的 feature backend（qdrant/lancedb/sqlite-vec），其 live 演练须 feature build
    When  以默认（无 vector feature）构建对 BruteForce 路径传入各非零 dim 调用 select_vector_backend，并复核既有 TEST-29.1.* / TEST-32.2.* 选择矩阵
    Then  默认 BruteForce 路径任意 dim 均被接受（字节等价于 Phase 32 既有行为，ADR-004）+ 既有选择矩阵保持绿 + 声明 dim 的 feature backend live 强制 honest-defer [SPEC-DEFER:phase-future.vector-dim-feature-enforce]（须 feature build，不伪造数值 ADR-013）（TEST-34.1.2，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-34.2-vector-backend-config-file.md (TEST-34.2.1)
  Scenario: SCEN-34.2.1 — 对应 AC1（Go config 加 [vector] section：TOML round-trip，既有 section 不受影响）
    Given vector backend 当前仅 env 入口（CONTEXTFORGE_VECTOR_BACKEND/_DIM 由 Rust server.rs 读取），而 Rust core 有 serde/serde_json 但无 toml dep（Rust 端配置读取会破 0-dep，ADR-008）；Go internal/config/config.go 已以手写 codec 解析 config.toml 的 [collections]/[remote]/[embedding] section（EmbeddingConfig 范式 :61-64：Provider/Dim 加 toml 标签）
    When  为 Go config.Config 加 [vector] section（Backend string 带 toml backend 标签，Dim int 带 toml dim 标签，镜像 EmbeddingConfig），并以含 / 不含 [vector] section 两种 config.toml 做 Save/Load round-trip
    Then  [vector] section 含值时 round-trip 保真（Backend/Dim 往返一致）+ 不含 [vector] section 时为零值（Backend="" / Dim=0）+ 既有 [collections]/[remote]/[embedding] section 解析不受影响 + 0 new dep（沿用手写 TOML codec）（TEST-34.2.1，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-34.2-vector-backend-config-file.md (TEST-34.2.2)
  Scenario: SCEN-34.2.2 — 对应 AC2（setVectorEnv 跨进程 env 桥：section 导出 / env-wins / 无 section 字节等价）
    Given spawn 的 Rust daemon 继承 Go 进程 env（internal/daemon/daemon.go:202 exec.Command 继承 env；cmd/contextforge/main.go:255 setDataDirEnv 以同款方式导出 CONTEXTFORGE_DATA_DIR），这是已验证的跨进程 env 桥（非延后的 daemon.Options.DataDir 字段重构）
    When  加 setVectorEnv helper（镜像 setDataDirEnv :254-268）：当 [vector] section 存在且对应 env 未预设时导出 CONTEXTFORGE_VECTOR_BACKEND / CONTEXTFORGE_VECTOR_DIM，使 spawn 的 core daemon 经既有 resolve_vector_backend env 路径拾取；并分别以 [section 存在] / [env 已预设] / [无 section] 三态调用
    Then  [vector] section 存在 → env 被导出 + 已显式设置的 env 覆盖 config（env-wins，向后兼容）+ 无 [vector] section → 不导出 → unset → BruteForce 字节等价（ADR-004 默认不变）+ daemon.Options.DataDir 字段重构仍 honest-defer [SPEC-DEFER:phase-future.daemon-options-datadir]（沿用 env 桥，不改 spawn 契约）（TEST-34.2.2，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-34.3-closeout-v0.27.0.md (TEST-34.3.1)
  Scenario: SCEN-34.3.1 — 对应 AC1（get_source_chunk workspace 隔离 verify-only：自 task-12.2 已存在的 grounding-correction）
    Given core/src/data_plane/search.rs:421-423 自 task-12.2（ADR-017 D1 Wave 2）已将候选 scope 到 req.workspace_id（非空 → 仅该 workspace；空 → 聚合全量探测已知 workspace）；调研曾把此项夸大为 gap，实为已存在的隔离（grounding-correction，须记于 ADR-039 D3）
    When  以 verify-only guard 测试覆盖三态（workspace_id 设置 → 仅该 workspace 的 chunk；跨 workspace 的 chunk_id → not_found；空 workspace_id → 聚合全量探测），据实记录已存在的隔离对称性（无新代码）
    Then  workspace_id 设置时仅返回该 workspace chunk + 跨 workspace chunk_id 给 not_found + 空 workspace_id 保持聚合全量（ADR-004 后向兼容）+ 据实更正为 verify-only / already-present，调研夸大记于 ADR-039 D3（无新代码，net-zero，ADR-013 诚实）（TEST-34.3.1，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-34.3-closeout-v0.27.0.md (TEST-34.3.2)
  Scenario: SCEN-34.3.2 — 对应 AC2（默认行为不变 + v0.27.0 closeout + grounding-correction 诚实）
    Given task-34.1 + task-34.2 全 Done（dim 协商 + config-file env 桥），current Phase 32/33 smoke v23[42/42]；ADR-039 据 D1-D4 须逐 D Proposed→Accepted（D1 dim-negotiation 默认 no-op honest-caveat / feature-enforce SPEC-DEFER；D2 config-file env-wins + Rust 0-dep 保持；D3 get_source_chunk 隔离已存在 verify-only grounding-correction；D4 默认 + 0-dep + 0-网络 + 既有契约不变）
    When  跑 scripts/console_smoke.sh banner v23→v24 + 新增 step → [43/43]（smoke_syntax_test.go TestTask343 镜像 TestTask334，no-regression [37/37]..[42/42]，staging dir cf-v26-cfg offset +2），产出 v0.27.0 release docs（docs/releases/v0.27.0-evidence.md + v0.27.0-artifacts.md + README v0.27 + RELEASE_NOTES v0.27.0，tag/run/digest 为 <backfill> 待回填 markers），ADR-039 逐 D Proposed→Accepted + ADR-037 add-only Phase 34 Amendment（dim-negotiation + config-file 补完 Phase 32 起的 env-plumbing，不溯改正文 ADR-014 D5）+ roadmap §3.16/§4 add-only + s2v-adapter add-only + phase §6 闭合
    Then  默认行为 / proto / 既有契约不变（ADR-004——默认 BruteForce expected_dim()=None 接受任意 dim、无 [vector] section 不导出 = BruteForce 字节等价、Rust core 仍无 toml dep、get_source_chunk 隔离无新代码）+ smoke v24[43/43]（既有 step 不退化，denominators 不溯改 ADR-014 D5）+ ADR-039 逐 D 如实 ratify（feature-enforce / daemon-options-datadir honest-defer，get_source_chunk 隔离 already-present 据实记于 D3）+ ADR-014 D1-D5 第 25 次激活全通过（TEST-34.3.2 + 各 task LAST TEST TEST-34.1.3 / TEST-34.2.3 / TEST-34.3.3 D2-lint，真实跑出后回填）
