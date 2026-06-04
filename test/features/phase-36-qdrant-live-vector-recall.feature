# language: en
# Maps to:
#   - docs/specs/phases/phase-36-qdrant-live-vector-recall.md
#   - docs/specs/tasks/task-36.1-qdrant-live-recall-harness.md
#   - docs/specs/tasks/task-36.2-qdrant-recall-ci-service.md
#   - docs/specs/tasks/task-36.3-closeout-v0.29.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 36 qdrant-live-vector-recall（兑现 Phase 25/29 已实现的 qdrant VectorBackend 之真实端到端 KNN 召回——把 ADR-034 D2 honest-defer 的 [SPEC-DEFER:phase-future.qdrant-server-lifecycle] 关闭：env-gated live recall harness + 真实 qdrant server + CI service-container 每跑必验。ADR-013 诚实优先：唯一在仓召回数 eval_integration.rs 0.7/0.85 为 SYNTHETIC fixture，非真实）。Scenario ID 在各 task spec §7 追踪表映射到测试 / 真实 run。
# Status: Draft（规划稿，本文件随 phase/task spec 一并 Draft）。
# 默认构建不变：vector-qdrant 为 opt-in，默认构建 0-vector-dep / 0-网络（ADR-004/008）；0 NEW dep（qdrant-client 自 task-18.4 已 optional）；0 schema migration；0 默认行为变更。harness 复用 QdrantConnConfig::from_env()（读 QDRANT_URL + 可选 QDRANT_API_KEY，tls 由 https scheme 推断）。
# 受阻 / 据实非问题维度均以 [SPEC-DEFER:phase-future.<name>] 标注（无 server 时 honest-defer 干净 skip 不 fail；语义 golden 召回需真实 embedding model 延后；部署拓扑 / 多后端生产化延后），据真实 CI run 回填，绝不预填 release 数值或召回数（ADR-013，placeholder 待回填 / floor-only）。

Feature: phase-36-qdrant-live-vector-recall
  In order to 把 Phase 25/29 已完整实现的 qdrant VectorBackend（connect/health/open ensure-create via decide_ensure/index_batch upsert/search KNN cosine/delete）的真实端到端 KNN 召回从 ADR-034 D2 的 honest-defer 状态转为「真实测得 + CI 每跑必验」，永久关闭 [SPEC-DEFER:phase-future.qdrant-server-lifecycle]（leader 已 DE-RISK 证明：真实 qdrant + qdrant-client 1.18 端到端往返 KNN 正确——query [1,0,0,0] 返回 [(a,1.0),(c,0.994)] 余弦序正确）
  As Phase 36 内核（env-gated live recall harness 度量 qdrant HNSW ANN recall@k vs BruteForce exact KNN 于同一内嵌语料 + 真实 qdrant server 跑出真实数 + CI service-container 集成每跑必验 + v0.29.0 closeout）
  I want 新增 core/tests/qdrant_live_recall.rs（#![cfg(feature = "vector-qdrant")]，env-gated on QDRANT_URL via QdrantConnConfig::from_env），构造一份 DETERMINISTIC 可复现语料 N=1000 个 VectorChunk（dim D=64 的确定性伪随机单位向量，按 index 播种、无 randomness / 无 clock，ADR-013 可复现），把同一语料同时索引进 QdrantBackend（connect→open ensure-create→index_batch）与 BruteForceVectorBackend（0-dep exact baseline），对 M=50 个确定性查询向量以 BruteForce 算出 exact top-k 作 ground truth、qdrant 算出 top-k，recall@k = mean(|qdrant_topk ∩ exact_topk| / k)，断言 recall@k >= 文档化下限（如 k=10 时 0.90）作 guard、并 eprintln 真实测得数（真实数在 task-36.2/closeout 由真实 CI run 回填，ADR-013 不伪造）；当 QdrantBackend::health() != Ready 时 eprintln 一条 skip 提示并 return（honest-defer，CI/本地无 server 干净 skip 而非 fail）；CI 经 .github/workflows/ci.yml 新增 qdrant-recall job 用 qdrant SERVICE CONTAINER（services: qdrant: image: qdrant/qdrant，ports 6334:6334 + 6333:6333）+ Rust 1.93 + protoc，跑 `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture` 使每个 CI run 都对 live service container 验召回——永久关闭 CI-无-server 的 defer，且默认构建仍 0 NEW dep / 0-vector-dep / 0-网络 / 0 schema migration / 默认行为不变（ADR-004/008——vector-qdrant opt-in、qdrant-client 自 task-18.4 已 optional），harness 度量为 model-free + 可复现的 qdrant-vs-exact-KNN（语义 golden-label 召回需真实 embedding model，作为更干净的主指标外的延后项 [SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]）

  # ---
  # Maps to: docs/specs/tasks/task-36.1-qdrant-live-recall-harness.md (TEST-36.1.1)
  Scenario: SCEN-36.1.1 — 对应 AC1（live recall harness 对 live qdrant 度量 recall@k vs BruteForce exact KNN >= floor + eprintln 真实数）
    Given core/tests/qdrant_live_recall.rs 以 #![cfg(feature = "vector-qdrant")] 门控、经 QdrantConnConfig::from_env() 读 QDRANT_URL（+ 可选 QDRANT_API_KEY，tls 由 https scheme 推断）经 QdrantBackend::connect 连接；当 QdrantBackend::health() == QdrantHealth::Ready 时一个真实 qdrant server 可达；leader 的 DE-RISK 已证明真实 qdrant + qdrant-client 1.18 端到端往返 KNN 正确（query [1,0,0,0] → [(a,1.0),(c,0.994)] 余弦序正确）；BruteForceVectorBackend 为 0-dep exact cosine baseline，与 qdrant 用同一 VectorIndexConfig { dim:64, metric: VectorMetric::Cosine, persistence_path: None, collection_id }
    When  构造 DETERMINISTIC 语料 N=1000 个 VectorChunk { chunk_id: ChunkId(String), embedding: Vec<f32>（dim 64 确定性伪随机单位向量、按 index 播种、无 randomness / 无 clock）, metadata: Option<serde_json::Value> }，把同一语料同时 index 进 QdrantBackend（open ensure-create + index_batch upsert）与 BruteForceVectorBackend，对 M=50 个确定性查询向量分别取 qdrant top-k 与 BruteForce exact top-k（ground truth），算 recall@k = mean(|qdrant_topk ∩ exact_topk| / k)
    Then  recall@k >= 文档化下限（k=10 时 0.90，下限作 guard）+ 真实测得 recall@k 经 eprintln 输出（真实数在 task-36.2/closeout 由真实 CI run 回填，floor-only 不预填，ADR-013 不伪造）+ qdrant HNSW ANN 排序与 BruteForce exact cosine 一致到下限以上 + 0 NEW dep / 0 schema migration / 0 默认构建变更（TEST-36.1.1，env-gated，对 live qdrant 真实测试通过后回填真实数）

  # ---
  # Maps to: docs/specs/tasks/task-36.1-qdrant-live-recall-harness.md (TEST-36.1.2)
  Scenario: SCEN-36.1.2 — 对应 AC2（无 qdrant server 时 honest-defer 干净 skip 不 fail + 确定性语料生成器可复现，runs WITHOUT server）
    Given 本地 / CI 无 qdrant server 可达时 QdrantBackend::health() 返 QdrantHealth::Unreachable（非 Ready）；确定性语料生成器按 index 播种、无 randomness / 无 clock（ADR-013 可复现），不依赖任何 server
    When  harness 在 health() != Ready 时 eprintln 一条 skip 提示并 return（honest-defer，不 fail test）；并在无 server 路径下对同一 seed 跑两次确定性语料生成器，比对产出向量
    Then  无 server → harness 干净 skip（eprintln 提示，return，NOT fail；CI/本地无 server 不红，honest-defer 非伪绿）+ 同一 seed → 同一向量（确定性可复现，runs WITHOUT server 即可断言，behavior-lock guard）+ 无任何 randomness / clock 依赖（ADR-013）+ 0 NEW dep（TEST-36.1.2，无 server 即可跑，确定性可复现断言为真实 RED→GREEN）

  # ---
  # Maps to: docs/specs/tasks/task-36.2-qdrant-recall-ci-service.md (TEST-36.2.1)
  Scenario: SCEN-36.2.1 — 对应 AC3（CI qdrant-recall service-container job 每跑必对 live service 验召回 + 绿 = 永久关闭 CI-无-server defer）
    Given .github/workflows/ci.yml 此前无 live qdrant server，故 ADR-034 D2 把真实端到端 KNN 召回 honest-defer 为 [SPEC-DEFER:phase-future.qdrant-server-lifecycle]；现新增 qdrant-recall job，用 qdrant SERVICE CONTAINER（services: qdrant: image: qdrant/qdrant，ports 6334:6334 + 6333:6333）+ Rust toolchain 1.93 + 安装 protoc
    When  qdrant-recall job 跑 `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`，使 task-36.1 的 harness 对 live service container 真实跑（CI 现 HAS 一个 live server）
    Then  qdrant-recall CI job 对 live service container 真实跑 harness 且绿（验证证据为本 PR 自身的真实 CI run，real evidence，ADR-013 据实记录不伪造）+ recall 自此每个 CI run 必验 → 永久关闭 [SPEC-DEFER:phase-future.qdrant-server-lifecycle]（CI 现 HAS live server）+ CI-only / add-only / 默认构建 + 行为不变（ADR-004）+ 0 NEW dep（TEST-36.2.1，验证证据为 live CI run，run link 真实回填）

  # ---
  # Maps to: docs/specs/phases/phase-36-qdrant-live-vector-recall.md §2 + docs/decisions/adr-041-qdrant-live-vector-recall.md (D1/D4)
  Scenario: SCEN-36.ALT — alternatives 据实记录（A1 synthetic-fixture REJECTED / A2 one-time-local-no-CI REJECTED / A3 semantic-golden DEFERRED，ADR-013 诚实价值）
    Given 备选 A1 synthetic-fixture recall（in-repo eval_integration.rs 0.7/0.85 即此——非真实，违 ADR-013）/ A2 一次性本地 live recall 无 CI（非永久守护）/ A3 recall vs golden semantic labels 需真实 embedding model（非 model-free、不可复现）
    When  对三 alternative 据实定夺并记于 phase spec §2、ADR-041 Alternatives：A1 / A2 REJECTED，A3 因 qdrant-vs-exact-KNN 度量更干净（model-free + 可复现）作主指标而 DEFERRED
    Then  A1 synthetic-fixture REJECTED（非真实召回，ADR-013）+ A2 one-time-local-no-CI REJECTED（非永久守护，本 phase 主张 CI 每跑必验）+ A3 semantic-golden DEFERRED [SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]（需真实 embedding model；qdrant-vs-exact-KNN 为 model-free + 可复现的干净主指标）+ 部署拓扑 / 多后端生产化均延后 [SPEC-DEFER:phase-future.qdrant-deployment-topology] [SPEC-DEFER:phase-future.multi-backend-production]（SCEN-36.ALT 据实记录，不伪造数值，真实 closeout 后回填）

  # ---
  # Maps to: docs/specs/tasks/task-36.3-closeout-v0.29.0.md (TEST-36.3.1)
  Scenario: SCEN-36.3.1 — 对应 AC4（默认 0-vector-dep baseline + 行为不变 + v0.29.0 closeout + ADR-034 D2 fulfilled）
    Given task-36.1 + task-36.2 全 Done（env-gated live recall harness 度量 qdrant HNSW ANN recall@k vs BruteForce exact KNN + CI service-container 每跑必验），current Phase 35 smoke v25[44/44]；ADR-041 据 D1-D4 须逐 D Proposed→Accepted（D1 live recall harness：qdrant HNSW ANN recall@k vs BruteForce exact KNN 方法学 + 确定性可复现语料 + 无 server 时 env-gated honest-defer；D2 真实测得召回数 placeholder 待回填 until task-36.2 run，ADR-013 不伪造；D3 CI service-container 集成：qdrant service 入 ci.yml → 每跑必验、永久关闭 CI-无-server defer；D4 默认 0-vector-dep baseline + 行为不变，vector-qdrant opt-in，ADR-004/008，0 NEW dep）
    When  跑 scripts/console_smoke.sh banner v25→v26 + 新增 step → [45/45]（smoke_syntax_test.go TestTask363_SmokeV26QdrantLiveRecallStep 镜像 TestTask353，no-regression [37/37]..[44/44] 不溯改 ADR-014 D5，staging dir 按 smoke 约定 offset），产出 v0.29.0 release docs（docs/releases/v0.29.0-evidence.md + v0.29.0-artifacts.md + README v0.29 + RELEASE_NOTES v0.29.0，真实 recall 数 + 真实 CI run link，tag/run/digest 为 <backfill> markers 待 post-tag-push），ADR-041 逐 D Proposed→Accepted（per-D ratify with real numbers）+ ADR-034 add-only Phase-36 Amendment（标记其 [SPEC-DEFER:phase-future.qdrant-server-lifecycle] D2 fulfilled：live KNN recall 已测 + CI-guarded，不溯改 ADR-034 D-body ADR-014 D5）+ roadmap §3.11/§4 add-only（qdrant-server-lifecycle progressed）+ s2v-adapter add-only + phase §6 闭合
    Then  默认行为 / proto / 既有契约不变（ADR-004——vector-qdrant opt-in、默认构建 0-vector-dep / 0-网络、0 NEW dep、0 schema migration）+ smoke v26[45/45]（既有 step 不退化，denominators [37/37]..[44/44] 不溯改 ADR-014 D5）+ ADR-041 逐 D 如实 ratify（D2 真实召回数据据实回填，A3 semantic-golden honest-defer）+ ADR-034 D2 [SPEC-DEFER:phase-future.qdrant-server-lifecycle] fulfilled（live KNN recall 已测 + CI 每跑必验）+ ADR-014 D1-D5 第 27 次激活全通过（TEST-36.3.1 + 各 task LAST TEST TEST-36.1.3 / TEST-36.2.2 / TEST-36.3.2 = `bash scripts/spec_drift_lint.sh --touched origin/master` 0 unannotated hits，真实跑出后回填）
