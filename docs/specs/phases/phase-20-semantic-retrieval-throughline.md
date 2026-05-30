# Phase 20 · semantic-retrieval-throughline

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。本 phase 把 Phase 19（v0.12.0）落地的**语义检索能力**从「opt-in + 仅 CLI / 仅 `internal/daemon/rest.go` REST surface」**贯通到 console-api（`internal/consoleapi`）的 `/v1/search`**，并让真实召回评测**经生产 `Retriever` 热路径**跑通（而非 v0.12.0 的独立 `core/examples/phase19_real_recall.rs` 谐波）。这是 `docs/releases/v0.12.0-evidence.md` §3b 与 `docs/specs/tasks/task-19.4-smoke-v9.md` §10 已**诚实记录**的两条 caveat 的闭环。v0.13.0 收口。对应 `docs/roadmap.md` §3.1。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md` §3.1 → `docs/releases/v0.12.0-evidence.md` §3b（两条 caveat）→ `docs/specs/tasks/task-19.4-smoke-v9.md` §10（console-api 未转发 semantic 的诚实记录）→ `docs/specs/tasks/task-19.3-semantic-search-api.md`（`SearchRequest.semantic` proto + `internal/daemon/rest.go` 参考实现）→ `internal/consoleapi/handlers.go::handleSearch` + `internal/consoleapi/grpcclient/grpcclient.go::searchClient.Search` + `internal/contractv1/contractv1.go::SearchRequest` → AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第十一次激活）→ `docs/decisions/adr-015-console-contract-v1-compatibility.md` + `docs/decisions/adr-017-console-contract-completion-22-endpoint.md`（add-only contract 演进 + 22-endpoint conformance）。
>
> **ADR 影响面（已识别）**：
> - **ADR-024 console-api-semantic-forward（新，Proposed）**：记 `contractv1.SearchRequest` add-only `Semantic` 字段 + console-api↔daemon 两条 REST surface 的语义对齐口径（仿 ADR-015/022 add-only pattern）。
> - 可能触及 **ADR-017（22-endpoint conformance）**：`/v1/search` 响应 shape 不变（add-only 请求字段），conformance 不破坏——以 amendment / 记录方式确认，不溯改正文（D5）。

## 1. 阶段目标

v0.13.0 ship 后，ContextForge 的语义检索从「v0.12.0 仅经 CLI `eval run --semantic` + `internal/daemon/rest.go ?semantic=true`」**贯通到 console-api `/v1/search`**：Console（及任何走 console-api 的客户端）可经 `?semantic=true`（或 body `semantic` 字段）请求语义检索，响应携带 v0.12.0 已加的 `vector_score` / `embedding_provider` provenance；真实召回评测经生产 `Retriever::search_semantic` 热路径跑（非旁路 example）。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. `contractv1.SearchRequest` add-only `Semantic` 字段 + `handleSearch` 转发 `?semantic=true` / body `semantic` 到 gRPC `SearchRequest.Semantic`（仿 `internal/daemon/rest.go` 参考实现）+ grpcclient 透传；既有 `{result, trace}` 响应 shape 与 22-endpoint conformance 不破坏（AC1）
2. 真实召回评测经生产 `Retriever` 热路径（`search_semantic`）跑通，deterministic provider 下 wiring 可断言；real fastembed 召回数值经 feature 本地复跑记录（AC2）
3. smoke v10：console-api `/v1/search?semantic=true` 真实语义断言（非仅 add-only 保形）+ 既有 step 不退化（AC3）
4. v0.13.0 release docs + phase §6 闭合 + ADR-024 ratify 或据实测记录（AC4）
5. ADR-014 D1-D5（第十一次激活）全通过（AC5）

**v0.x 版本号决策**：v0.13.0 minor release（语义检索贯通 console-api；默认构建仍 BM25-only baseline——semantic 仍 opt-in，add-only 请求字段不破坏既有客户端）。

## 2. 业务价值

直接闭合 v0.12.0 收口时**诚实记录**的两条 caveat（`docs/releases/v0.12.0-evidence.md` §3b / `docs/specs/tasks/task-19.4-smoke-v9.md` §10）：

- **console-api 语义贯通**：v0.12.0 时 console-api `/v1/search` 仅解码 JSON body、不转发 `?semantic=true`（仅 `internal/daemon/rest.go` 转发）。本 phase 让经 console-api 的语义检索真正生效——这是 Console UI 语义召回（cross-repo `[SPEC-OWNER:phase-future.console-semantic-explain]`）的前置数据通路。
- **真实召回经 Retriever 热路径**：v0.12.0 的 real recall 经独立 example（`phase19_real_recall.rs`）测得，未经生产 `Retriever`。本 phase 让评测走真实热路径，提升 evidence 代表性（`[SPEC-DEFER:phase-future.real-recall-via-retriever]`，承 `task-14.2` / `RELEASE_NOTES`）。
- **PRD §Core Capabilities #1（可解释召回）**：语义结果经 console-api 仍保留 `vector_score` + `embedding_provider` provenance，可解释性不退化。

**不在本 phase scope**：

- Hybrid scoring（BM25 + Vector 融合）[SPEC-DEFER:phase-future.hybrid-scoring]——v0.14.0 / Phase 21
- Reranker（cross-encoder）[SPEC-DEFER:phase-future.reranker]——v0.14.0 / Phase 21
- Remote embedding provider（OpenAI / Cohere）[SPEC-DEFER:phase-future.embedding-provider-remote]——v0.15.0 / Phase 22
- Embedding 缓存 [SPEC-DEFER:phase-future.embedding-cache]——v0.15.0 / Phase 22
- Console UI 语义 explain 面板（cross-repo Console 领域）[SPEC-OWNER:phase-future.console-semantic-explain]
- 向量增量索引 [SPEC-DEFER:phase-future.vector-incremental-index]——承 Phase 18/19 默认全量 reindex

## 3. 涉及模块

### 20.1 console-api semantic 转发（task-20.1）

- 修改 `internal/contractv1/contractv1.go`——`SearchRequest` 加 `Semantic bool`（add-only field，`json:"semantic"`）
- 修改 `internal/consoleapi/handlers.go`——`handleSearch` 读 `?semantic=true` query param 并 OR-merge 到 body `Semantic`（仿 `internal/daemon/rest.go:142-146` 参考实现）
- 修改 `internal/consoleapi/grpcclient/grpcclient.go`——`searchClient.Search` 把 `req.Semantic` 透传到 `pb.SearchRequest.Semantic`
- 同源 Go tests（≥3：contractv1 字段 round-trip JSON + handleSearch query param/ body OR-merge → grpcclient 透传 + 既有 BM25 请求不退化）

### 20.2 real-recall-via-retriever（task-20.2）

- 修改/扩 `core/src/retriever/mod.rs` 或新增评测入口——让真实召回经生产 `Retriever::search_semantic` 热路径产生（替代/补充 `core/examples/phase19_real_recall.rs` 旁路）
- deterministic provider 下 index→search_semantic roundtrip wiring 可断言；real fastembed（feature-gated）召回数值本地复跑记录
- 修改 `docs/spikes/phase-19-real-recall.md` 或新增 `docs/spikes/phase-20-recall-via-retriever.md`——记录经 Retriever 热路径的真实召回（与 v0.12.0 example 口径对比）
- 同源 Rust tests（≥2：deterministic roundtrip via Retriever + fixture 格式校验）

### 20.3 smoke v10 + closeout（task-20.3）

- 修改 `scripts/console_smoke.sh`——v10：console-api `/v1/search?semantic=true` 真实语义断言（响应 `retrieval_method` 反映语义路径 / `vector_score` 在 result item 出现），既有 step 不退化
- 新增 `docs/releases/v0.13.0-{evidence,artifacts}.md` + `README.md` v0.13 段 + `RELEASE_NOTES.md` v0.13.0 段
- 修改 `docs/decisions/adr-024-console-api-semantic-forward.md`——据实测 Proposed→Accepted 或记录维持
- 修改 `docs/s2v-adapter.md`（Phase 20 Draft→Done + Tasks 0→3；ADR-024 状态；v0.12.0 caveat 解除记录）

### BDD feature

- 新增 `test/features/phase-20-semantic-retrieval-throughline.feature`（≥3 scenario：console-api semantic 请求转发 / 真实召回经 Retriever / smoke v10 语义断言）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 20.1 | `internal/contractv1` `SearchRequest.Semantic` + `internal/consoleapi/handlers.go` `handleSearch` query-param 转发 + `grpcclient` 透传 | `../tasks/task-20.1-console-api-semantic-forward.md` |
| 20.2 | real-recall 经生产 `Retriever::search_semantic` 热路径 + spike 记录 | `../tasks/task-20.2-real-recall-via-retriever.md` |
| 20.3 | smoke v10 console-api semantic 真实断言 + v0.13.0 closeout + ADR-024 ratify | `../tasks/task-20.3-closeout-v0.13.0.md` |

## 5. 依赖关系

- **task-20.1**（console-api 转发）= 首项，提供经 console-api 的语义通路；解锁 20.3 smoke 真实断言。
- **task-20.2**（recall via Retriever）dep Phase 19 `Retriever::search_semantic`（已落地）+ embedding provider（19.1）；可与 20.1 并行（写路径不相交：Go vs Rust）。
- **task-20.3**（closeout）dep 20.1 + 20.2 全 Done。
- 外部：ADR-024（本 phase 新 Proposed）/ ADR-015/017（contract add-only + 22-endpoint conformance）/ ADR-014 第十一次激活 / Phase 19 task-19.3（proto `SearchRequest.semantic` + daemon 参考实现）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [ ] **AC1**：`contractv1.SearchRequest` add-only `Semantic` + `handleSearch` 转发 `?semantic=true` / body → gRPC `SearchRequest.Semantic` + grpcclient 透传；`{result, trace}` 响应 shape + 22-endpoint conformance 不破坏 — verified by task-20.1 §6 AC1-3 + phase-smoke step 1
- [ ] **AC2**：真实召回评测经生产 `Retriever::search_semantic` 热路径跑通；deterministic provider wiring 可断言，real fastembed 召回数值本地复跑记录（禁伪造，ADR-013）— verified by task-20.2 §6 AC1-2 + phase-smoke step 2
- [ ] **AC3**：smoke v10 console-api `/v1/search?semantic=true` 真实语义断言（非仅保形）+ 既有 step 不退化 — verified by task-20.3 §6 AC1 + phase-smoke step 3
- [ ] **AC4**：v0.13.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ ADR-024 据实测 ratify 或记录 + phase §6 闭合 — verified by task-20.3 §6 AC2-3
- [ ] **AC5**：ADR-014 cross-validation gate 全套通过（第十一次激活）— D1 mapping + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-19 不溯改 — verified by task-20.3 closeout PR body

**端到端 smoke（C1 集成兜底）**：(1) console-api `/v1/search?semantic=true` 转发 roundtrip；(2) 真实召回经 `Retriever::search_semantic`；(3) smoke v10 console-api 语义断言全 PASS。

## 7. 阶段级风险

- **R1（中）console-api 语义响应 provenance 缺失**：若 semantic 路径下 `vector_score` / `embedding_provider` 未填到 console-api 响应 item，可解释性退化。
  - **缓解**：task-20.1 测试断言 semantic 响应 item 携带 provenance；`protoToSearchResult` mapping 复核。
- **R2（中）真实召回经 Retriever 热路径需 real provider，CI 不验证**：承 phase-19 §7 R1。
  - **缓解**：deterministic provider 下 wiring 可 CI 验证；real fastembed 召回数值 🟡 本地 feature 复跑记录（ADR-013 不伪造）。stop-condition：real provider 两平台均不可构建 → deterministic wiring 跑通 + 真实数值如实 defer，继续 closeout。
- **R3（低）22-endpoint conformance 因 add-only 字段误判**：add-only 请求字段不应破坏 conformance。
  - **缓解**：conformance test + proto-freeze 守护复跑；响应 shape 不变（仅请求加字段）。

## 8. Definition of Done

- 3 task spec（20.1-20.3）顶部 `**Status**: Done`
- §6 阶段级 AC1-5 全 `[x]`
- 端到端 smoke 3 step 全 PASS
- **ADR**：ADR-024 `Proposed → Accepted`（或据实测记录维持 + 文档化）
- **adapter**：§Phase 索引 Phase 20 `Draft → Done` + `Tasks 0 → 3`；§ADR 索引 ADR-024；§BDD 追加 phase-20 feature 行；v0.12.0 console-api semantic caveat 解除记录
- **spike evidence**：`docs/spikes/phase-20-recall-via-retriever.md`（或扩 `phase-19-real-recall.md`）
- **release**：`docs/releases/v0.13.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.13 段 + README v0.13 段
- **cross-repo follow-up**：console-api 语义贯通后，通知 Console 语义 explain（`[SPEC-OWNER:phase-future.console-semantic-explain]`）的数据通路就绪
