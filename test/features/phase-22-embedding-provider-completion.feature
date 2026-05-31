# language: en
# Maps to:
#   - docs/specs/phases/phase-22-embedding-provider-completion.md
#   - docs/specs/tasks/task-22.1-provider-config-selection.md
#   - docs/specs/tasks/task-22.2-embedding-cache.md
#   - docs/specs/tasks/task-22.3-remote-provider-skeleton.md
#   - docs/specs/tasks/task-22.4-closeout-v0.15.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 22 embedding-provider-completion。Scenario ID 在各 task spec §7 追踪表映射到具体测试。

Feature: phase-22-embedding-provider-completion
  In order to 把 Phase 19 的「硬编码确定性缺省 + 单一 fastembed real provider」扩成可配置的完整 embedding provider 层
  As Phase 22 内核（配置选择 + dim 协商 + 缓存 + 远程骨架 + health 探针 + v0.15.0 收口）
  I want provider 经 [embedding] 配置选择、重复内容缓存、远程 HTTP 骨架契约可验证，且默认构建恒本地、0 网络 dep、缺省行为不变

  # ---
  # Maps to: docs/specs/tasks/task-22.1-provider-config-selection.md (TEST-22.1.1–22.1.5)
  Scenario: SCEN-22.1.1 — 对应 AC1（[embedding] 配置 + select_provider 工厂 + dim 协商）
    Given internal/config 加 add-only [embedding](provider/dim) 段 + core/src/embedding/factory.rs select_provider + server.rs 语义路径走工厂
    When  select_provider 在 "deterministic"/""（缺省）/ "fastembed"（feature）/ "remote"（骨架）间选择，并对非 0 请求 dim 与 provider dim() 协商
    Then  缺省 select_provider("deterministic",0) 与 Phase 19 default() 逐字节等价（语义路径不退化，TEST-22.1.2/22.1.5）；dim 冲突返 EmbeddingError::DimMismatch 不静默（TEST-22.1.3）；"remote"/未知/fastembed-未启用 返明确 Err（TEST-22.1.4）；[embedding] 段 TOML round-trip 含/不含均合法、既有段不受影响（TEST-22.1.1）

  # ---
  # Maps to: docs/specs/tasks/task-22.2-embedding-cache.md (TEST-22.2.1–22.2.4)
  Scenario: SCEN-22.2.1 — 对应 AC2（CachingEmbeddingProvider content-hash 缓存）
    Given core/src/embedding/cache.rs CachingEmbeddingProvider（Sha256(text)→embedding 装饰器；内存 L1 + 可选 SQLite L2 承 ADR-002）
    When  同一 text 二次 embed、批量混合命中/未命中、或经 with_sqlite 重载同一缓存文件
    Then  命中跳过底层 embed（计数 wrapper 断言）且向量逐字节相同（TEST-22.2.1）；新 text 未命中、批量仅未命中调底层且按输入顺序组装（TEST-22.2.2）；SQLite 往返底层 0 调用、内存缺省不落盘（TEST-22.2.3）；dim()/name() 透传 + 可作 Arc<dyn EmbeddingProvider> 接入（TEST-22.2.4）；0 新 dep（sha2/rusqlite 已 direct）

  # ---
  # Maps to: docs/specs/tasks/task-22.3-remote-provider-skeleton.md (TEST-22.3.1–22.3.4)
  Scenario: SCEN-22.3.1 — 对应 AC3（RemoteEmbeddingProvider 骨架 + 契约测试，不打网络）
    Given core/src/embedding/remote_provider.rs RemoteEmbeddingProvider（OpenAI/Cohere HTTP，embedding-remote feature-gated，ureq rustls）+ 纯函数 build_request_body/parse_response
    When  build_request_body 构造请求 / parse_response 解析 fixture 响应 / 工厂 "remote" 分支在 feature 下返回 provider（全 fixture，不打真实网络）
    Then  请求体含 model/input/dimensions（TEST-22.3.1）；fixture 响应解析出有序向量（TEST-22.3.2）；malformed/空 data/缺字段返明确 EmbeddingError（TEST-22.3.3）；默认构建 0 网络 dep、feature 下 name/dim 正确（TEST-22.3.4）；真实网络联调/密钥/召回质量按 ADR-013 如实 defer（§8 R1 stop-condition，不伪造）

  # ---
  # Maps to: docs/specs/tasks/task-22.4-closeout-v0.15.0.md (TEST-22.4.1–22.4.5)
  Scenario: SCEN-22.4.1 — 对应 AC4/AC5/AC6（health 探针 opt-in + smoke v12 + v0.15.0 收口 + ADR-027 ratify）
    Given core/src/health.rs probe_embed feature-gated opt-in 远程探针 + scripts/console_smoke.sh v12 step 31 + v0.15.0 release docs + ADR-027
    When  默认构建 probe_embed 维持 config-only（opt-in inert）、smoke step 31 断言 init 生成 [embedding] 段、ADR-027 据 task-22.1/22.2/22.3 真实非合成验证 ratify
    Then  config-only 缺省行为逐字节不变（ADR-020 D1，TEST-22.4.1）；真实远程探针命中按 ADR-013 如实 defer（CI 无 endpoint/keys）；release docs 齐备 + ADR-027 Proposed→Accepted（架构经真实单测/契约验证，远程真实联调 defer）+ phase-22 §6 全 met；ADR-014 D1-D5（第十三次激活）全通过
