# ContextForge Roadmap — post-v0.12.0 候选版本规划

> **本文件性质（必读）**：这是一份**规划文档**，把仓库中**真实存在**的 `[SPEC-OWNER]` /
> `[SPEC-DEFER]` 标记按既有版本节奏映射为后续候选版本，作为后续 Phase spec / task spec /
> `/goal` 推进的依据。它**不是**已闭合的承诺：
>
> - 表中召回率 / 性能等**数值一律不预填**（ADR-013 禁伪造凭据）；验收数值在对应 task 真实跑出后写进 task §10 + release evidence。
> - 每个版本的 AC 标注了**可验证性分级**（🟢🟡🔴，见 §1.3）——凡需外部服务 / 密钥 / 网络 / 特定平台才能验证的项，按 ADR-013 做成 feature-gated 管道 + deterministic 测试 + 如实 defer，**不**在无人值守下编造结果。
> - 版本切分 / task 拆分**可据实施时的真实数据微调**（ADR-012 主 agent 自治）；本文件随之 add-only 更新。
> - 每个 `v0.x.0` 的 tag push 前**必须经用户明确授权**（沿用历史 release 流的 stop-condition）。
>
> 与既有索引的关系：本文件给「跨版本视图」；单版本的契约细节以对应 `docs/specs/phases/phase-N-*.md` +
> `docs/specs/tasks/task-N.M-*.md` 为单一事实源；ADR 决策以 `docs/decisions/` 为准；
> `docs/s2v-adapter.md` 仍是 Phase / task / ADR 的状态索引。

---

## 1. 规则与约束

### 1.1 版本节奏

历史落定的节奏是「**1 Phase ≈ 1 个 minor 版本**」：

| Phase | 版本 | Phase | 版本 |
|---|---|---|---|
| 10 console-contract-v1 | v0.3.0 | 15 console-functional-gap-closure | v0.8.0 |
| 11 console-real-data-plane | v0.4.0 | 16 v0.9.0-backlog-completion | v0.9.0 |
| 12 console-contract-completion | v0.5.0 | 17 is-pinned-amendment | v0.10.0 |
| 13 memory-rest-surface | v0.6.0 | 18 vector-backend-selection | v0.11.0 |
| 14 eval-rest-surface | v0.7.0 | 19 vector-retrieval-integration | v0.12.0 |

外推：**Phase N → v0.(N−7).0**。故后续候选版本起点为 Phase 20 / v0.13.0。

### 1.2 未来工作的真实来源

仓库无独立 roadmap（本文件即首份），未来工作以 `[SPEC-OWNER:phase-future.<name>]`（已归属、承诺度高）与
`[SPEC-DEFER:phase-future.<name>]`（已延后、承诺度低）两类标记散落于 `docs/prds/` / `docs/decisions/` /
`docs/specs/` / 源码注释 / `docs/releases/`。本文件把这些标记聚合、归类、排期。

post-v0.12.0 仍开放的 `[SPEC-OWNER]`：

| marker | 含义 | 出处 |
|---|---|---|
| `phase-future.vector-retrieval-integration` | 向量检索集成 | **已由 Phase 19 / v0.12.0 兑现（关闭）** |
| `phase-future.embedding-provider-full` | 完整 embedding provider（选择 / 配置 / 缓存 / 远程） | `task-19.1` / `task-19.5` / `adr-006` |
| `phase-future.console-semantic-explain` | Console UI 语义召回 explain 面板 | `task-19.7` / `v0.12.0-artifacts`（跨仓库 Console 域） |

### 1.3 可验证性分级（ADR-013 诚实口径）

每条候选 AC 在对应 Phase / task spec 中按此分级，决定无人值守下能否真正验证：

- 🟢 **可无人值守 CI 真实验证**：纯 wiring / 算法 / 协议 / 配置，deterministic provider 或固定 fixture 即可断言；CI 三门覆盖。
- 🟡 **需 feature / real provider 本地真实验证**：如 real fastembed 召回、hnsw 图持久化往返——默认构建不验证，需 `--features` 本地真实跑（承 Phase 19 real-recall 模式），数值写进 spec §10 + evidence。
- 🔴 **需外部服务 / 密钥 / 网络 / 受阻平台**：如 remote embedding provider（OpenAI/Cohere）、sqlite-vec Windows MSVC、reranker 模型真实质量——**做成 feature-gated 管道 + deterministic 契约测试**，真实联调 / 跨平台 / 模型质量**如实 defer 不伪造**；若整体受阻则文档化 stop-condition。

---

## 2. 候选版本总览

| 版本 | Phase | 主题 | 主要来源 marker | 验收姿态 |
|---|---|---|---|---|
| **v0.13.0** | 20 | 语义检索贯通 console-api + 经 Retriever 真实召回 | `console-semantic-rest-forward`（新）/ `real-recall-via-retriever` / `embedding-provider-full`(部分) | 🟢 主体 + 🟡 真实召回确认 |
| **v0.14.0** | 21 | 检索质量：hybrid scoring + reranker | `hybrid-scoring` / `reranker` | 🟢 hybrid + 管道 / 🔴 reranker 真实质量 |
| **v0.15.0** | 22 | embedding provider 完整化 | `embedding-provider-full` / `embedding-provider-remote` / `embedding-cache` | 🟢 缓存+配置 / 🔴 remote 联调 |
| **v0.16.0** | 23 | 向量持久化与跨平台 | `hnsw-graph-persistence` / `sqlite-vec-cross-platform` / `vector-incremental-index` | 🟡 hnsw 持久化 / 🔴 sqlite-vec MSVC |
| **（穿插）** | — | 发布 / CI 硬化 | `multi-arch-image` / `image-signing-and-sbom` / `ci-strict-lint` | 🟢 CI 配置（release run 验证） |
| **（跨仓库）** | — | Console 语义 explain | `console-semantic-explain` | 🔴 Console 独立仓库，本仓仅协调 / 文档 |

> 每个版本一组 PR：Phase spec（Draft）+ 全量 task spec（Draft，§1-§10）+ 需要的 ADR 草稿（Proposed）+
> `s2v-adapter.md` add-only 索引行。规划合入后再按版本顺序 S2V 实现。

---

## 3. 各候选版本详述

### 3.1 v0.13.0 / Phase 20 — semantic-retrieval-throughline

**目标**：把 v0.12.0 已落地但「opt-in + 仅 CLI / 仅 `internal/daemon/rest.go`」的语义检索，贯通到
**console-api（`internal/consoleapi`）的 `/v1/search`**，并让真实召回评测**经生产 `Retriever` 热路径**跑（而非 v0.12 的独立 example 谐波）。这是 v0.12.0 evidence §3b / task-19.4 §10 已诚实记录的两条 caveat 的闭环。

**来源 marker / caveat**：
- v0.12.0 已记：console-api `/v1/search` **未转发** `?semantic=true` 到 gRPC（仅 `internal/daemon/rest.go` 转发）——见 `docs/releases/v0.12.0-evidence.md` §3b 末。
- `[SPEC-DEFER:phase-future.real-recall-via-retriever]`（`task-14.2` / `RELEASE_NOTES`）——真实召回经 Retriever 而非旁路。
- `[SPEC-OWNER:phase-future.embedding-provider-full]`（部分）——本版补「provider 经配置选择 + dim 协商」最小子集。

**候选 task 拆分**：
- **task-20.1** console-api semantic 转发：`contractv1.SearchRequest` add-only `Semantic` 字段 + `internal/consoleapi/handlers.go::handleSearch` 转发 `?semantic=true` / body `semantic` 到 gRPC `SearchRequest.Semantic` + grpcclient 透传。🟢
- **task-20.2** real-recall-via-retriever：eval / smoke 经真实 `Retriever::search_semantic` 热路径（而非 `phase19_real_recall` 独立 example）跑召回；deterministic provider 下 wiring 可验证，real fastembed 召回数值 🟡 本地复跑。
- **task-20.3** smoke v10 + closeout + v0.13.0 release docs：smoke 加 console-api semantic REST 真实断言（非仅保形）；README/RELEASE_NOTES/evidence/artifacts。🟢

**ADR**：可能需 **ADR-024 console-api-semantic-forward**（Proposed）记 contractv1 add-only 字段 + console-api↔daemon 两条 REST surface 的语义对齐口径（仿 ADR-015/022 add-only pattern）。

### 3.2 v0.14.0 / Phase 21 — retrieval-quality（hybrid + reranker）

**目标**：在「BM25 单路 / 语义单路 + BM25 fallback」之上，提供 **hybrid scoring（BM25 + 向量分数融合）**，并引入 **reranker（cross-encoder）** 提升 top-k 排序质量。

**来源 marker**：
- `[SPEC-DEFER:phase-future.hybrid-scoring]`（`core/src/retriever/mod.rs:450/640` / `phase-19` §2 / `v0.12.0-artifacts`）。
- `[SPEC-DEFER:phase-future.reranker]`（`phase-19` §2 / `v0.12.0-artifacts`）。

**候选 task 拆分**：
- **task-21.1** hybrid scoring：融合函数（如 RRF / 加权归一）+ `retrieval_method=hybrid` + add-only `hybrid_score` 字段 + deterministic 单测（固定 BM25/vector 分数 → 期望融合序）。🟢
- **task-21.2** reranker 管道：`Reranker` trait + deterministic identity-reranker（默认构建 0 模型 dep，供 CI/测试）+ real cross-encoder provider（feature-gated）。管道 + deterministic 🟢；real 模型**质量** 🔴（需模型 + 真实 eval，如实 defer 数值）。
- **task-21.3** eval 扩展 + smoke + v0.14.0 closeout：eval 报告加 hybrid / reranked 召回列；release docs。🟢（hybrid）/ 🟡（reranked 真实召回本地复跑）。

**ADR**：**ADR-025 hybrid-scoring-fusion**（Proposed，融合策略选型，仿 ADR-006/023 数据驱动 ratify 模式——真实数据出来才 Accepted）。reranker 选型可并入或单列 **ADR-026 reranker-provider**（Proposed）。

### 3.3 v0.15.0 / Phase 22 — embedding-provider-completion

**目标**：把 v0.12.0 的「deterministic 缺省 + 单一 fastembed real provider」扩成**完整 provider 层**：运行时 / 配置选择、embedding 缓存、远程 provider（OpenAI / Cohere）骨架。

**来源 marker**：
- `[SPEC-OWNER:phase-future.embedding-provider-full]`（`task-19.1` / `task-19.5` / `adr-006`）。
- `[SPEC-DEFER:phase-future.embedding-provider-remote]`（`adr-008:56` / `phase-19` §2 / `phase-18-spike`）。
- `[SPEC-DEFER:phase-future.embedding-cache]`（`phase-19-embedding-{candidates,fastembed}` spike）。
- `[SPEC-DEFER:phase-future.embed-remote-probe]`（`adr-020:103`）——health 探针含远程 embedding。

**候选 task 拆分**：
- **task-22.1** provider 配置 + 选择：config 增 `embedding.provider` / `embedding.dim` + 工厂选择（deterministic / fastembed / remote）+ dim 协商校验。🟢
- **task-22.2** embedding cache：content-hash → embedding 缓存（内存 + 可选 SQLite 持久化，承 ADR-002）+ deterministic 命中/失效单测。🟢
- **task-22.3** remote provider 骨架：`RemoteEmbeddingProvider`（OpenAI/Cohere HTTP，feature-gated）+ 契约级 deterministic 测试（请求构造 / 响应解析 / 错误路径，**不打真实网络**）。骨架 🟢；真实联调 + 密钥 🔴（如实 defer，记 stop-condition）。
- **task-22.4** health 远程探针 + smoke + v0.15.0 closeout。🟢 / remote 探针真实命中 🔴。

**ADR**：**ADR-027 embedding-provider-abstraction**（Proposed，provider 层 + 远程 opt-in + 本地优先红线，承 ADR-004 local-first / ADR-008）。

### 3.4 v0.16.0 / Phase 23 — vector-persistence-and-cross-platform

**目标**：解决 Phase 18/19 留下的两块向量持久化 / 跨平台债：**hnsw 图持久化**（避免重启重建，Phase 18 记 100k 28s 重建）与 **sqlite-vec Windows MSVC 跨平台**；并评估**向量增量索引**。

**来源 marker**：
- `[SPEC-DEFER:phase-future.hnsw-graph-persistence]`（`adr-023:60/100` / `server.rs:296` / `phase-18-hnsw`）。
- `[SPEC-DEFER:phase-future.sqlite-vec-cross-platform]`（`adr-023:101` / `phase-18-sqlite-vec` / `v0.11.0-evidence`）。
- `[SPEC-DEFER:phase-future.sqlite-vec-on-disk]` / `phase-future.sqlite-vec-blob-encoding`（`phase-18-sqlite-vec`）。
- `[SPEC-DEFER:phase-future.vector-incremental-index]`（`phase-19` §2）——承 Phase 18 默认全量 reindex。

**候选 task 拆分**：
- **task-23.1** hnsw 图持久化：图序列化 / 反序列化到磁盘 + rebuild-on-load fallback + roundtrip 测试。管道 🟢 / feature 下真实持久化往返 🟡。
- **task-23.2** sqlite-vec 跨平台调查 + 落地或文档化 blocker：尝试 MSVC 可构建路径（bundled / 预编译 / 替代绑定）；若仍受阻则**诚实文档化 stop-condition**（承 spike 既有结论），不伪造跨平台通过。🔴
- **task-23.3** 向量增量索引（评估 + 最小实现或 defer）+ smoke + v0.16.0 closeout。🟡 / 🔴 视调查结果。

**ADR**：**ADR-028 vector-persistence-strategy**（Proposed，hnsw 持久化格式 + sqlite-vec 跨平台结论）。

### 3.5 发布 / CI 硬化（穿插，可单列 Phase 或并入某版 closeout）

**来源 marker**：
- `[SPEC-DEFER:phase-future.multi-arch-image]`（`prd:524` / `release.yml` 现 linux/amd64 only / 多处 v0.9-v0.11 artifacts）。
- `[SPEC-DEFER:phase-future.image-signing-and-sbom]`（v0.9-v0.11 artifacts）。
- `[SPEC-DEFER:phase-future.ci-strict-lint]`（`prd:524`——clippy / gofmt 卡红，现非阻断）。
- `[SPEC-DEFER:phase-future.verify-image-anonymous-pull]`（`RELEASE_NOTES` / `v0.10.0-artifacts`）。

**性质**：均为 CI / release.yml 配置，🟢 可在 CI / release run 验证（multi-arch 需 buildx+QEMU；签名需 cosign/syft）。建议作为**发布硬化小 Phase**（如 Phase 24）单列，或逐项并入上述版本的 closeout PR。ci-strict-lint 须先评估存量 clippy/gofmt 告警量再决定卡红时机（避免一次性大面积变红）。

### 3.6 Console 语义 explain（跨仓库，本仓不实现）

- `[SPEC-OWNER:phase-future.console-semantic-explain]`：ContextForge-Console 是**独立仓库**，语义召回 explain 面板属 Console 领域。本仓职责限于：(a) 确保 `/v1/search` 语义响应携带 `vector_score` / `embedding_provider` provenance（v0.12 已加，v0.13 贯通 console-api）；(b) 跨仓库通知 + 契约对齐文档（仿 ADR-022 D4 cross-repo signal 模式）。🔴 本仓不实现 UI，规划中仅记协调项。

---

## 4. 长尾 backlog（尚未归入上述版本，留 vNext）

下列 `[SPEC-DEFER]` 标记承诺度低 / 范围小 / 依赖未明，暂不排入 v0.13–v0.16，待对应版本启动时据数据决定纳入或继续延后：

- **向量 backend 细化**：`multi-backend-production`、`qdrant-server-lifecycle`、`qdrant-deployment-topology`、`lancedb-index-tuning`、`lancedb-schema-compaction`、`lancedb-build-prereq-ci`。
- **eval**：`rust-native-eval-runner`（现 Go runner，承 `task-14.1`）、`eval-dataset-validation`、`case-results-subtable`、`semantic-golden-dataset`（语义近邻标注扩充）。
- **检索 tokenizer**：`cjk-and-code-tokenizer`（CJK + 代码符号分词，`phase-19` §2）。
- **trace / events**：`tracestore-sqlite-vacuum`、`tracestore-fts`、`tracestore-multi-workspace-strict`、`events-sse-push`、`events-replay-from-audit`、`events-drain-timeout-config`、`event-bus-partition`、`event-bus-capacity`、`memstore-event-emit`。
- **memory**：`memory-pinned-at-timestamp`、`memory-pin-actor`、`handle-memory-pin-strict-body`、`is-pinned-backfill-from-audit`、`memory-pin-unpin-split`、`hard-delete-policy`。
- **cache / deploy**：`cache-lru`、`cache-cap-configurable`、`compose-resource-limits`、`compose-tls-termination`。

> 该清单由对应版本启动时的 marker 复扫刷新（`rg 'SPEC-(DEFER|OWNER):phase-future'`）；本文件 add-only 更新，不删历史条目。

---

## 5. 执行协议（承项目契约）

每个候选版本的实现遵循既有规则，无例外：

1. **S2V 四步**逐 task：spec（已在规划阶段起草，Draft）→ RED → GREEN → REFACTOR。
2. **一 task 一 PR**；CI 三门（cargo-test / go-test / spec-lint）全绿后自主 merge + 删分支；**红灯绝不合**（已知 phase9 Tantivy `LockBusy` flake → `gh run rerun --failed` 复跑至绿）。
3. **ADR-014 D1-D5** 逐 task：D1 mapping、D2 lint 0 未标注命中、D3 verified-by、D4 自治、**D5 不溯改已闭合 Phase 1-19 spec（ADR 改动 add-only amendment）**。
4. **ADR-013 禁伪造**：🟡/🔴 项的真实数值 / 联调 / 跨平台只在真实证据下记录；未达不标 `[x]`，provisional ADR 不在缺真实数据时翻 Accepted。
5. 版本全部 task 合入后起 release docs（README/RELEASE_NOTES/evidence/artifacts）+ phase §6 ACs `[x]` + Status Done + adapter 行 Done；**tag push 前停下等用户明确授权**；授权后 push → release.yml → post-tag-push backfill PR 填实 tag SHA / run ID / 镜像 digest。
6. 治理承**单驱动 + 内部 Agent subagent**（ADR-011 / ADR-012），不外派 worker。
