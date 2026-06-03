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
| **v0.17.0** | 24 | 检索 tokenizer + eval 加固 | `cjk-and-code-tokenizer` / `eval-dataset-validation` / `semantic-golden-dataset` / `rust-native-eval-runner` | 🟢 tokenizer opt-in + 校验器 / 🟡 真实 recall delta |
| **v0.18.0** | 25 | 生产向量 backend（qdrant / lancedb） | `qdrant-server-lifecycle` / `lancedb-build-prereq-ci` / `lancedb-index-tuning` | 🟢 qdrant 契约生命周期层 / 🔴 live server + lancedb protoc 构建 |
| **v0.19.0** | 26 | 可观测性硬化（trace FTS/VACUUM + events SSE/replay） | `tracestore-fts` / `tracestore-sqlite-vacuum` / `events-sse-push` / `events-replay-from-audit` | 🟢 FTS + VACUUM + audit 重放 / 🟡 SSE live e2e |
| **v0.20.0** | 27 | memory-ops 硬化（pin actor/timestamp + unpin/hard-delete） | `memory-pin-actor` / `memory-pinned-at-timestamp` / `memory-pin-unpin-split` / `hard-delete-policy` / `is-pinned-backfill-from-audit` | 🟢 proto add-only + 写穿 + X-Confirm hard-delete |
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

### 3.5 v0.17.0 / Phase 24 — code-and-cjk-tokenizer-and-eval-hardening

**目标**：解决两块直接影响核心代码检索用例可信度的检索质量债：`content` 字段**代码/CJK 分词偏弱**（`core/src/indexer/mod.rs:148` 用默认 `TEXT` analyzer，对 camelCase / snake_case / dotted.path / kebab-case / CJK 切分弱）与 **eval 标尺未加固**（`internal/eval/eval.go::ValidateDataset` 仅基本校验、golden 无代码/CJK case、`core/src/eval/runner.rs` 为 placeholder）。opt-in 的代码/CJK tokenizer 提升代码符号 + CJK 召回；eval 数据集校验器 + golden 扩充让召回声明可信。

**来源 marker（§4 backlog 本版兑现）**：
- `[SPEC-DEFER:phase-future.cjk-and-code-tokenizer]`（`phase-19` §2 / 检索 tokenizer 段）。
- `eval-dataset-validation` / `semantic-golden-dataset` / `rust-native-eval-runner`（§4 eval 段三 marker）。

**候选 task 拆分**：
- **task-24.1** code/CJK tokenizer：`core/src/indexer/mod.rs` 注册自定义 `TextAnalyzer`（代码符号拆分 + 保留原 token + CJK bigram），opt-in via config + 默认 tokenization 不变（既有索引不失效）+ index/query 侧对称（`RetrieverConfig.tokenizer` 接入点）。🟢 分词单测（优先 std-only 0 新 dep）。
- **task-24.2** eval 数据集加固：`internal/eval/eval.go` 独立校验器（schema 良构 + 重复 + 覆盖，add-only）+ `test/fixtures/eval/golden-semantic.jsonl` 含代码/CJK annotated query case。🟢
- **task-24.3** tokenizer recall delta + runner 评估 + v0.17.0 closeout：真实 before/after recall delta（小语料 delta 不显著则如实记录，ADR-013）+ `core/src/eval/runner.rs` promote 最小 runner 或诚实延后。🟡 真实 delta / 🟢 wiring。

**ADR**：**ADR-029 code-and-cjk-tokenizer-and-eval-hardening**（Proposed，D1 tokenizer opt-in / D2 校验器 / D3 数据集扩充 / D4 runner 评估 / D5 默认不变；真实 recall delta + runner 评估出来才 Accepted）。

### 3.6 v0.18.0 / Phase 25 — production-vector-backend

**目标**：把 ADR-023 列为生产规模 ANN 两档的 **qdrant**（外部 gRPC server，hosted/scale-out）与 **lancedb**（嵌入式列存）从 Phase 18 spike 态推向生产：qdrant 加 connect / health-probe / collection ensure-create / 连接配置的**生命周期层**（契约层不需 live server 即可 deterministic 验证）；lancedb 做**真实可构建性调查**（dev-box `cargo build --features vector-lancedb`，protoc 前置，仿 task-23.2 sqlite-vec MSVC pattern）+ 索引调参参数；产出**生产 backend 选择矩阵**。

**来源 marker（§4 向量 backend 细化段）**：
- `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` / `qdrant-deployment-topology` / `multi-backend-production`。
- `[SPEC-DEFER:phase-future.lancedb-build-prereq-ci]` / `lancedb-index-tuning` / `lancedb-schema-compaction`。

**候选 task 拆分**：
- **task-25.1** qdrant server lifecycle：connection-config validate + health-probe（unreachable shape）+ `decide_ensure`（reuse / create / error 纯函数 3 分支，替 spike 盲目 drop+create）契约层 deterministic 单测（不需 live server）。🟢 契约层 / 🔴 live KNN（CI 无 server，如实 defer）。
- **task-25.2** lancedb 可构建性 + 索引调参：dev-box 真实 `cargo build --features vector-lancedb`（protoc 前置）三态如实标 + 索引调参参数 struct 校验（不建大索引）。🔴 构建 / 🟢 参数校验。
- **task-25.3** v0.18.0 closeout + 生产 backend 选择矩阵。🟢 / 🔴 视调查结果。

**ADR**：**ADR-030 production-vector-backend**（Proposed，qdrant 生命周期 + lancedb 可构建性 + 选择矩阵 + 默认 0-dep；ADR-023 D3/D4 tier add-only Amendment 推进）。

### 3.7 v0.19.0 / Phase 26 — observability-hardening

**目标**：硬化 Phase 16 落地的两条可观测性信号路径：**TraceStore 持久化**（`core/src/data_plane/search_persist.rs`，`search_traces` 表无按内容检索 + 无清理路径无界膨胀）与 **events 实时面**（`internal/consoleapi` `GET /v1/observability/events` long-poll + `EventBus`）。trace 全文检索（FTS5）+ 周期 VACUUM；events SSE 实时推送（替 long-poll 重订阅）+ 从 audit log 重放漏失事件 + event-bus 容量 / 分区 / drain 超时配置。全部 local-first（默认 0 新 dep / 0 network，ADR-004）。

**来源 marker（§4 trace/events 段）**：
- `[SPEC-DEFER:phase-future.tracestore-fts]` / `tracestore-sqlite-vacuum`。
- `[SPEC-DEFER:phase-future.events-sse-push]` / `events-replay-from-audit`（`adr-021:115`）/ `events-drain-timeout-config` / `event-bus-capacity` / `event-bus-partition`。

**候选 task 拆分**：
- **task-26.1** TraceStore FTS + VACUUM：`search_traces_fts` shadow 表（FTS5，bundled SQLite 0 新 dep）+ `prune_older_than` / VACUUM（新 migration `0016`，add-only 方法，既有签名不变）。🟢
- **task-26.2** events SSE 推送 + 重放：`GET /v1/observability/events/stream`（Go stdlib `http.Flusher`，add-only side-by-side 既有 long-poll）+ 从 `audit_log` 重放漏失事件 + event-bus 容量 / drain 配置（复用 `EventBus::with_capacity` seam，兑现 ADR-021 `events-replay-from-audit` + Rollback path）。🟢 契约 + 重放 id-ASC 序 / 🟡 SSE live e2e（需 running daemon，如实 defer）。
- **task-26.3** v0.19.0 closeout。🟢 / 🟡。

**ADR**：**ADR-031 observability-hardening**（Proposed，D1 FTS / D2 VACUUM / D3 SSE / D4 audit 重放 / D5 event-bus 配置 / D6 默认不变；ADR-021 / ADR-015 add-only Amendment）。

### 3.8 v0.20.0 / Phase 27 — memory-ops-hardening

**目标**：硬化 Phase 13 / Phase 17 落地的 Memory 生命周期 / pin 语义：记录 **pin-actor + pinned-at-timestamp**、**Pin/Unpin 显式拆分**（vs 既有 `bool pin` toggle）、**hard-delete 策略**（vs 仅 soft-delete，X-Confirm gated）、**is_pinned 审计回填**。proto 改动全 **add-only**（新字段在冻结 tag 之后 + 新 Unpin/HardDelete RPC，不破冻结契约，proto-freeze guard 须过）。全部本地（ADR-004，0 网络 / 默认构建 0 新 dep）。

**来源 marker（§4 memory 段；兑现 ADR-022 §Trade-offs 三 marker）**：
- `[SPEC-DEFER:phase-future.memory-pin-actor]` / `memory-pinned-at-timestamp` / `is-pinned-backfill-from-audit`（ADR-022 §Trade-offs 缩范围延后）。
- `[SPEC-DEFER:phase-future.memory-pin-unpin-split]` / `hard-delete-policy` / `handle-memory-pin-strict-body`。

**候选 task 拆分**：
- **task-27.1** pin-actor + pinned-at-timestamp：proto add-only `pinned_by`(string) + `pinned_at_unix`(int64)（tag 10 之后）+ `core/src/memory/store.rs` 写穿 + 从 audit 回填。🟢（console-api `source` 硬编码，真实 per-user actor 上游传播如实 defer）。
- **task-27.2** Pin/Unpin 显式拆分 + hard-delete：add-only `Unpin` / `HardDelete` RPC（hard-delete 复用 `confirmMiddleware` X-Confirm，ADR-017 D2）+ Pin toggle 向后兼容。🟢（hard-delete 仅删 `memory_items` 行，vector-index/trace 级联如实 defer）。
- **task-27.3** v0.20.0 closeout。🟢。

**ADR**：**ADR-032 memory-ops-hardening**（Proposed，proto add-only pin-actor/timestamp + Unpin/HardDelete RPC + 审计回填 + X-Confirm hard-delete；兑现 ADR-022 三 marker）。

### 3.9 发布 / CI 硬化（穿插，可单列 Phase 或并入某版 closeout）

**来源 marker**：
- `[SPEC-DEFER:phase-future.multi-arch-image]`（`prd:524` / `release.yml` 现 linux/amd64 only / 多处 v0.9-v0.11 artifacts）。
- `[SPEC-DEFER:phase-future.image-signing-and-sbom]`（v0.9-v0.11 artifacts）。
- `[SPEC-DEFER:phase-future.ci-strict-lint]`（`prd:524`——clippy / gofmt 卡红，现非阻断）。
- `[SPEC-DEFER:phase-future.verify-image-anonymous-pull]`（`RELEASE_NOTES` / `v0.10.0-artifacts`）。

**性质**：均为 CI / release.yml 配置，🟢 可在 CI / release run 验证（multi-arch 需 buildx+QEMU；签名需 cosign/syft）。建议作为**发布硬化小 Phase**（Phase 24-27 已分配给 v0.17-v0.20，故此项落后续小 Phase，如 Phase 28+）单列，或逐项并入上述版本的 closeout PR。ci-strict-lint 须先评估存量 clippy/gofmt 告警量再决定卡红时机（避免一次性大面积变红）。

### 3.10 Console 语义 explain（跨仓库，本仓不实现）

- `[SPEC-OWNER:phase-future.console-semantic-explain]`：ContextForge-Console 是**独立仓库**，语义召回 explain 面板属 Console 领域。本仓职责限于：(a) 确保 `/v1/search` 语义响应携带 `vector_score` / `embedding_provider` provenance（v0.12 已加，v0.13 贯通 console-api）；(b) 跨仓库通知 + 契约对齐文档（仿 ADR-022 D4 cross-repo signal 模式）。🔴 本仓不实现 UI，规划中仅记协调项。

### 3.11 v0.22.0 / Phase 29 — live-vector-recall（承 Phase 25，post-v0.20.0 add-only 排期）

**目标**：把 Phase 25（production-vector-backend, Done）推到「契约层 / 参数校验层」的 qdrant / lancedb backend **真实跑通为 live 向量召回**，并把真实 backend 工厂化注入生产热路径 `core/src/server.rs`（现 `:302` hybrid / `:341` semantic 仍硬编码 `BruteForceVectorBackend`）。

**来源 marker（§4 向量 backend 细化段 + phase-25 spec line 44）**：
- `[SPEC-DEFER:phase-future.vector-retrieval-integration]`（phase-25 spec line 44——qdrant/lancedb 接入 server.rs 语义热路径）。
- `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（残留 live-server KNN 维度）/ `qdrant-deployment-topology`。
- `[SPEC-DEFER:phase-future.lancedb-index-tuning]`（真实 ANN 索引建图 + 性能）/ `lancedb-schema-compaction` / `lancedb-build-prereq-ci`。
- `multi-backend-production`（选择矩阵真实测量校准）。

**候选 task 拆分**：
- **task-29.1** vector backend 工厂 + server.rs 热路径注入：`select_vector_backend(name, dim) -> Result<Arc<dyn VectorSearcher>, VectorError>`（仿 `embedding/factory.rs::select_provider`）+ 替换 `server.rs:302/341` 硬编码 BruteForce；默认仍 BruteForce、feature 关闭诚实 Err。🟢 deterministic（不连 server）。
- **task-29.2** qdrant live KNN + 真实召回 harness：克隆 `phase20_recall_via_retriever.rs`，对真实 qdrant server 跑 connect→ensure-create→upsert→KNN；CI 无 server `health()==Unreachable` 时 honest-defer。🔴 live server（真实召回数真实跑出后回填，不伪造）/ 🟢 wiring。
- **task-29.3** lancedb 真实 ANN 索引调参 + 多 backend 选择矩阵：用 `LanceIndexTuning` 在内嵌 dataset 真建 IVF_PQ/HNSW 索引并实测召回 + 真实跨 backend 选择矩阵测量 → ADR-030 D3 / ADR-023 tier add-only Amendment。🟡 feature build / 🔴 大语料 / compaction 诚实延后。
- **task-29.4** v0.22.0 closeout：smoke v19 + release docs + ADR-034 ratify + ADR-030/023 add-only Amendment + adapter + feature。🟢。

**ADR**：**ADR-034 production-vector-live-recall**（Proposed，D1 backend 工厂 + 热路径注入 / D2 qdrant live KNN 无 server 诚实延后 / D3 lancedb 真实 ANN 索引 / D4 选择矩阵真实测量 add-only Amendment / D5 默认 0 vector dep baseline 不变；真实 live KNN / 真实索引召回 / 真实矩阵测量出来才 ratify，受阻维度据已达维度 ratify 不强翻）。

### 3.12 v0.23.0 / Phase 30 — cjk-true-segmenter（承 Phase 24，post-v0.20.0 add-only 排期）

**目标**：把 Phase 24（code-and-cjk-tokenizer, Done）的务实 **CJK 重叠 bigram**（`配置加载` → `配置`/`置加`/`加载`）升级为**真分词器**（true word segmenter，`配置加载` → `配置`/`加载`），评估把 tokenizer 由 opt-in 翻为**默认开启**（含既有索引 reindex/migration 工具），并在扩展后的 CJK golden 上量出**真实召回 delta**。真分词器 feature-gated（默认 0-dep，bigram 保留作 fallback）。

**来源 marker（ADR-029:54 Follow-ups + phase-24 spec :41/:42）**：
- `[SPEC-DEFER:phase-future.cjk-true-segmenter]`（CJK 真正分词器替/补 bigram）。
- `[SPEC-DEFER:phase-future.tokenizer-default-on]`（tokenizer 默认开启 + 既有索引迁移工具）。

**候选 task 拆分**：
- **task-30.1** cjk-true-segmenter：新 `cjk-segmenter` feature（默认 off，镜像 `vector-lancedb` gating）+ optional dep（jieba-rs/lindera，经主 agent R7 chore + ADR-008 add-only）+ 并行 analyzer 名 `cjk_segmenter` + 双站点注册（index `:442` + query `:250`）对称；bigram 保留 0-dep fallback；deterministic 真词边界单测。🟢 分词单测 / 🔴 重词典 dep。
- **task-30.2** tokenizer-default-on + 既有索引迁移 + 真实 CJK recall delta：reindex/migration 工具 + `RetrieverConfig.tokenizer`（现 vestigial）路由接线或文档化 schema-driven + 扩展 CJK golden（Go `ValidateGoldenSemantic` 校验）+ phase24-harness 量 default vs bigram vs 真分词真实 delta（不预填，ADR-013）；迁移过重则诚实延后 default flip。🟡 recall / 🟢 wiring。
- **task-30.3** v0.23.0 closeout：smoke v20 step + release docs + ADR-035 ratify + ADR-029 add-only Amendment + adapter + feature。🟢。

**ADR**：**ADR-035 cjk-true-segmenter-and-tokenizer-default**（Proposed，D1 真分词 feature-gated / D2 并行 analyzer 名 + 双站点注册对称 / D3 tokenizer-default-on 评估 + 迁移工具 + config 路由接线 / D4 扩展 CJK golden 真实 recall delta 不预填 / D5 默认 tokenization 不变；真实分词单测 / 真实 recall delta 出来才 ratify，重词典 dep / 小语料受阻维度据已达维度 ratify）。

### 3.13 v0.24.0 / Phase 31 — governance-debt-cleanup（清长尾 backlog + Phase 28 follow-up + 旧 nits，post-v0.20.0 add-only 排期）

**目标**：清理跨 Phase 累积的治理债——§4 长尾 backlog 中真正开放的 code-local 项 + Phase 28 follow-up + 旧 nits。多为 🟢 可单测；compose TLS 真实 cert 🟡；github-native-attestation 私有仓库 🔴。含一项**诚实校正**：经核 `event-bus-partition` / `event-bus-capacity` 已在 Phase 26 交付（非债），本 phase verify-only + §4 add-only 更正剔除（ADR-013，不重复实现）。

**来源 marker（§4 backlog 真实开放项 + Phase 28 follow-up + 旧 nits）**：
- `memstore-event-emit`（Go fallback memory 变更未 emit event）/ `cache-lru` / `cache-cap-configurable` / `compose-resource-limits` / `compose-tls-termination` / `case-results-subtable`。
- Phase 28 follow-up 诚实重申：`multi-arch-native-runner` / `github-native-attestation`（私有仓库受阻）/ `rust-native-eval-runner`（无 consumer）。
- 旧 nits：task-6.3 exporter `content=""`（根因 v1 search proto 无 chunk 全文）+ PR #48 3 个 MCP nits。

**候选 task 拆分**：
- **task-31.1** observability + memstore event parity：Go fallback `MemMemoryStore` Pin/Deprecate/SoftDelete/Unpin/HardDelete emit `memory.*` event（与 workspace/job + Rust 路径对齐）+ event-bus partition/capacity verify-only + §4 add-only 更正（经核 Phase 26 已交付）。🟢。
- **task-31.2** cache + deploy hardening：embedding-cache LRU（`cache.rs:23` 无界）+ Go memstore cap 可配置（`memstore.go:49` 硬编码 256）+ compose `mem_limit`/`cpus` + 可选 TLS proxy。🟢 / 🟡（真实 cert）。
- **task-31.3** eval case-results 子表（add-only migration 0018）+ exporter `content=""` 经新 `ListAllChunks` RPC 真实全文 + 3 MCP nits（protocolVersion 白名单 / audit.Write err 不吞 / allowlist 文件 mode warn）+ C2/C3/C4 诚实延后重申。🟢。
- **task-31.4** v0.24.0 closeout：smoke v21 + release docs + ADR-036 ratify + ADR-021/027/029/033 add-only Amendment + §4 event-bus 更正 + adapter + feature。🟢。

**ADR**：**ADR-036 governance-debt-cleanup**（Proposed，D1 memstore-event-emit Go parity + event-bus verify-only 更正 / D2 cache + deploy 硬化 / D3 eval 子表 + exporter 全文 + MCP nits / D4 honest defer 重申 / D5 默认行为 + 既有契约不变；真实测试 / 实测出来才 ratify，TLS cert / native runner / attestation 受阻维度据已达维度 ratify）。

### 3.14 v0.25.0 / Phase 32 — vector-backend-config-plumbing-and-completeness（承 Phase 29，post-v0.24.0 add-only 排期）

**目标**：把 Phase 29（live-vector-recall, Done）落地的 `select_vector_backend` 工厂从「仅默认接线」补全为「经 env/config 选 backend + 工厂后端覆盖齐全 + 控制面 provenance 对齐 + 检索 filter 契约诚实化」。`core/src/server.rs` 两热路径（hybrid `:340` / semantic `:382`）今天硬注入 `select_vector_backend("", 0)`（注释明记「No vector config is plumbed」）；`factory.rs` 有 brute/qdrant/lancedb arm 却无 sqlite-vec arm（`SqliteVecBackend` 已实存、已 re-export、task-23.2 已验 MSVC 可构建）；控制面 `console_data_plane.proto` `SearchResultItem` 缺 `vector_score`（数据面 v1 search proto 已有 `vector_score=13`）；`retriever/mod.rs:325` 对 source_type/agent_scope filter emit 措辞误导的 WARN。

**来源 marker（grounded 校正）**：
- `[SPEC-DEFER:phase-future.vector-backend-config-file]`（backend 经 env/config 选用，承 task-29.1 工厂）。
- `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`（sqlite-vec in-process 选择矩阵 recall/latency cell，须本机 MSVC feature build，🟡）。
- **关键诚实校正**：`source_type`/`agent_scope` chunk filter 经核 chunks 表（§5.3 FROZEN）无该列、`SearchResult` 二者硬编码、`agent_scope` 属 memory 层 — real chunk filter 是 import-path feature 非确定性 nit，本 phase 仅令契约诚实（准确 no-op）+ 开新 backlog `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`，不伪造已实现（ADR-013）。

**候选 task 拆分**：
- **task-32.1** backend config plumbing：`server.rs` hybrid + semantic 两热路径经 env（仿 `CONTEXTFORGE_DATA_DIR` pattern）选 backend，未设/"" → BruteForce byte-equivalent。🟢
- **task-32.2** sqlite-vec factory arm：`select_vector_backend` 加 `"sqlite-vec"` arm（feature 双半 gating，镜像 qdrant/lancedb）+ in-process 选择矩阵 wiring 🟢；矩阵 recall/latency cell 🟡 honest-defer `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`（须 MSVC feature build，不伪造数值 ADR-013）。
- **task-32.3** console provenance + filter 契约诚实化：`console_data_plane.proto` `SearchResultItem` add-only `vector_score=16`（parity v1 search proto）+ `mod.rs:325` 误导性 WARN → 准确 no-op 契约 + 新 chunk filter backlog。🟢
- **task-32.4** v0.25.0 closeout：smoke v22 + release docs + ADR-037 ratify + ADR-034 add-only Amendment（sqlite-vec arm 补全工厂）+ roadmap/adapter add-only。🟢

**ADR**：**ADR-037 vector-backend-config-plumbing-and-completeness**（Proposed，D1 config plumbing / D2 sqlite-vec arm（矩阵 cell honest-defer）/ D3 console provenance add-only + filter 契约诚实化 / D4 honest-defer 边界 / D5 默认 0-vector-dep baseline + 既有契约不变；真实测试 / 实测出来才 ratify，sqlite-vec 矩阵 cell / real chunk filter feature 受阻维度据已达维度 ratify）。

### 3.15 v0.26.0 / Phase 33 — governance-debt-cleanup-2（第二轮治理债清扫，承 Phase 31，post-v0.25.0 add-only 排期）

**目标**：清第二轮 code-local 治理债，镜像 Phase 31 / ADR-036；grounding 校正后据实下修多处 survey 过陈（ADR-013）。

**候选 task 拆分**：
- **task-33.1** L2 SQLite embedding-cache rowid-FIFO 有界（Phase 31 仅界 L1；L2 `INSERT OR REPLACE` 无界）。**0 新 dep / 0 migration**（用既有 implicit rowid）。诚实校正：`with_sqlite` 无生产调用点（test-only）→ opt-in-path 防御非确证 live 泄漏；true-LRU 须 ALTER → `[SPEC-DEFER:phase-future.l2-cache-true-lru]`。🟢
- **task-33.2** console-api memstore FIFO→access-order LRU（move-to-front on hit）+ memory hard-delete no-dangling-ref 不变式测试（cascade 经核非问题——`memory_id` 仅 memory_items PK、无 memory-vector 表 → `[SPEC-DEFER:phase-future.memory-harddelete-cascade]`）。剔除 handleMemoryPin strict-400（ADR-022 D2 蓄意 lenient 契约，据实不改）。🟢
- **task-33.3** observability：indexing.* 事件持久化（add-only migration 0019）+ replay mapper 扩展（mapper 🟢 / e2e 🟡 `[SPEC-DEFER:phase-future.indexing-replay-e2e]`）+ TraceStore 多 workspace 严格隔离（add-only `workspace_id` proto 字段 + SQL WHERE，空=aggregate-all 兼容）+ events-drain-timeout verify-only（经核 Phase 26 已交付）。🟢/🟡
- **task-33.4** `internal/cli/export.go` add-only `--timeout` flag（默认 60s）+ v0.26.0 closeout。剔除/honest-defer：`%v→%w`（non-bug，Status 未丢）/ tracestore-fts（已修复）/ datadir env→Options（🟡 `[SPEC-DEFER:phase-future.daemon-options-datadir]`）。🟢

**ADR**：**ADR-038 governance-debt-cleanup-2**（Proposed，D1-D5；真实测试出来才 ratify，indexing replay e2e / trace isolation e2e 受阻维度据已达维度 ratify）。

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
>
> **v0.17–v0.20 排期更新（add-only，不删上方历史条目）**：上方部分 backlog 已据 §3.5-§3.8 排入对应版本——`cjk-and-code-tokenizer` + eval 三 marker（`eval-dataset-validation` / `semantic-golden-dataset` / `rust-native-eval-runner`）→ **v0.17.0 / Phase 24**；`qdrant-server-lifecycle` / `lancedb-build-prereq-ci` / `lancedb-index-tuning`（+ `qdrant-deployment-topology` / `multi-backend-production` 部分）→ **v0.18.0 / Phase 25**；`tracestore-fts` / `tracestore-sqlite-vacuum` / `events-sse-push` / `events-replay-from-audit`（+ `event-bus-capacity` / `events-drain-timeout-config` 部分）→ **v0.19.0 / Phase 26**；`memory-pin-actor` / `memory-pinned-at-timestamp` / `is-pinned-backfill-from-audit` / `memory-pin-unpin-split` / `hard-delete-policy`（+ `handle-memory-pin-strict-body` 部分）→ **v0.20.0 / Phase 27**。未点名的 marker（如 `lancedb-schema-compaction` / `tracestore-multi-workspace-strict` / `event-bus-partition` / `memstore-event-emit` / `case-results-subtable` / `cache-lru` / `cache-cap-configurable` / `compose-*`）续留 backlog，由各 phase `[SPEC-DEFER]` 如实承接，真正落地或延后以对应版本数据决定。
>
> **v0.17.0 / Phase 24 推进记录（已落地 2026-05-31，add-only）**：`cjk-and-code-tokenizer` → ✅ opt-in code/CJK `TextAnalyzer`（纯 std，task-24.1）；`eval-dataset-validation` → ✅ `ValidateGoldenSemantic`（task-24.2）；`semantic-golden-dataset` → ✅ `golden-semantic.jsonl` 代码/CJK 扩充（task-24.2）；`rust-native-eval-runner` → 🟡 真实评估后**诚实延后**（Go harness 续为单一事实源，`[SPEC-DEFER:phase-future.rust-native-eval-runner]`，task-24.3）。真实 before/after recall delta = +0.0909（default 0.9091 → code/CJK 1.0000）over task-24.2 golden（ADR-029 Accepted；详 `docs/spikes/phase-24-tokenizer-recall.md`）。CJK 真正分词器 `[SPEC-DEFER:phase-future.cjk-true-segmenter]` + tokenizer 默认开启 `[SPEC-DEFER:phase-future.tokenizer-default-on]` 续 backlog。
>
> **v0.22.0 / Phase 29 排期更新（规划中 2026-06-02，add-only，不删上方历史条目）**：§3.11 把上方「向量 backend 细化」段的 `qdrant-deployment-topology`（残留 live-server KNN 维度承 `qdrant-server-lifecycle`）/ `lancedb-index-tuning` / `lancedb-schema-compaction` / `multi-backend-production` + phase-25 spec line 44 的 `[SPEC-DEFER:phase-future.vector-retrieval-integration]` 排入 **v0.22.0 / Phase 29 — live-vector-recall**（task-29.1 工厂 + server.rs 热路径注入 / task-29.2 qdrant live KNN 真实兑现（无 server 诚实延后）/ task-29.3 lancedb 真实 ANN 索引 + 选择矩阵真实测量 → ADR-030/023 add-only Amendment / task-29.4 closeout）。真实 live KNN / 真实索引召回 / 真实矩阵测量数值一律真实跑出后回填（ADR-013，不预填）。`lancedb-schema-compaction` / `qdrant-deployment-topology`（集群拓扑）/ `lancedb-build-prereq-ci`（CI 构建 ICE 前置）很可能续 backlog 诚实延后。ADR-034 Proposed。
>
> **v0.23.0 / Phase 30 排期更新（规划中 2026-06-02，add-only，不删上方历史条目）**：§3.12 把上方「检索 tokenizer」段承 Phase 24 的两项 follow-up marker `[SPEC-DEFER:phase-future.cjk-true-segmenter]` + `[SPEC-DEFER:phase-future.tokenizer-default-on]`（出处 ADR-029:54 + phase-24 spec :41/:42，**非** §4 既有 `cjk-and-code-tokenizer` —— 后者已于 v0.17.0 闭合）排入 **v0.23.0 / Phase 30 — cjk-true-segmenter**（task-30.1 真分词器 feature-gated + 双站点注册 / task-30.2 tokenizer-default-on 评估 + 既有索引迁移 + 真实 CJK recall delta / task-30.3 closeout）。真分词器引入 optional dep（jieba-rs/lindera）经主 agent R7 chore + ADR-008 add-only；默认构建仍 0 新 dep + 默认 tokenization 不变（ADR-004）。真实 recall delta 真实跑出后回填（ADR-013，不预填）；若全量 default-on 迁移过重则诚实延后 default flip 续 backlog。ADR-035 Proposed。
>
> **v0.24.0 / Phase 31 排期更新（规划中 2026-06-02，add-only，不删上方历史条目）**：§3.13 把上方 backlog 中真正开放的 code-local 项排入 **v0.24.0 / Phase 31 — governance-debt-cleanup**：`memstore-event-emit`（task-31.1）/ `cache-lru` + `cache-cap-configurable` + `compose-resource-limits` + `compose-tls-termination`（task-31.2）/ `case-results-subtable` + exporter `content=""`（task-6.3 旧 nit，经新 `ListAllChunks` RPC）+ PR #48 3 MCP nits（task-31.3）。Phase 28 follow-up `multi-arch-native-runner` / `github-native-attestation`（私有仓库受阻）/ `rust-native-eval-runner`（无 consumer）经核受阻 / 无驱动 → task-31.3 诚实重申延后续 backlog（不伪造完成，ADR-013）。**诚实校正（add-only，不删上方条目）**：上方 trace/events 段所列 `event-bus-partition` + `event-bus-capacity` 经核**已在 v0.19.0 / Phase 26（task-26.3 / ADR-031 D5）交付**（`core/src/data_plane/events.rs` `EventBusConfig`/`Partition`/`from_config` + `server.rs` 生产接线 + TEST-26.3.1a/b/c）——非开放债，自开放 backlog 剔除，Phase 31 仅 verify-only（task-31.1）。ADR-036 Proposed。

> **v0.25.0 / Phase 32 排期更新（规划中 2026-06-03，add-only，不删上方历史条目）**：§3.14 把 `select_vector_backend` 工厂的「配置接线 + 后端覆盖补全 + 控制面 provenance + filter 契约诚实化」排入 **v0.25.0 / Phase 32 — vector-backend-config-plumbing-and-completeness**（task-32.1 backend config plumbing / task-32.2 sqlite-vec factory arm + 矩阵 wiring / task-32.3 console `vector_score` add-only + filter 契约诚实化 / task-32.4 closeout）。**新增 backlog 条目（add-only，grounded 校正）**：`chunk-source-type-filter`（real chunk source_type filter，须 importer 侧 source_type 打标 + chunks 表 §5.3 FROZEN schema migration，非确定性 nit）/ `chunk-agent-scope-filter`（agent_scope 属 memory 层 `memory_items` 0013，chunk 检索路径无该维度）/ `sqlite-vec-inprocess-matrix`（sqlite-vec in-process 选择矩阵 recall/latency cell，须本机 MSVC `vector-sqlite` feature build，🟡）/ `vector-backend-config-file`（超 env 的结构化 vector backend 配置）/ `vector-dim-auto-negotiation`（embedder-dim 据 provider 真实维度自动协商，现 `let _ = dim` 占位）。真实数值 / 受阻维度真实跑出后回填（ADR-013，不预填）。ADR-037 Proposed。
>
> **v0.25.0 / Phase 32 推进记录（已落地 2026-06-03，add-only）**：§3.14 全 4 task 合入 master，ADR-037 据 D1-D5 真实 ratify（Proposed → Accepted）：
> - **task-32.1**（PR #212，squash c7358ed）backend config plumbing → ✅：`server.rs` hybrid（`:340`）+ semantic（`:382`）两热路径经 `resolve_vector_backend`/`parse_vector_backend` 读 `CONTEXTFORGE_VECTOR_BACKEND`（+ 可选 `CONTEXTFORGE_VECTOR_DIM`，仿 `resolve_data_dir`）选 backend；未设/"" → BruteForce byte-equivalent（默认行为不变）；unknown/feature-off → 工厂诚实 Err 透出为 `Status::internal`（无静默回退，ADR-013）。TEST-32.1.1/.2 pass；0 新 dep。
> - **task-32.2**（PR #213，squash 76a3137）sqlite-vec factory arm → 🟡 PARTIAL：`factory.rs` `select_vector_backend` 加 `"sqlite-vec"` arm（feature `vector-sqlite` 双半 gating，镜像 qdrant/lancedb）+ in-process 选择矩阵 wiring，default build TEST-32.2.1/.2 factory 6/6；real x86_64-pc-windows-msvc `cargo test --features vector-sqlite` build PASSED（arm wiring 真实验证非仅结构）。矩阵 recall/latency CELL（真实 KNN recall@k + latency）`[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`（须本机 MSVC feature build + 真实语料，不伪造数值，ADR-013）；0 新 dep。
> - **task-32.3**（PR #214，squash eaa37bd）console provenance add-only + filter 契约诚实化 → ✅：`console_data_plane.proto` `SearchResultItem` add-only `vector_score=16`（parity v1 search proto `RetrievalResult.vector_score=13`，buf generate 真实 rawDesc 描述符位移 0xdc→0xff），Rust producer `data_plane/search.rs` 写 `vector_score`（cosine for vector hits / 0 for BM25）→ Go grpcclient `protoToSearchResult` 映射 → `contractv1.SearchResult` add-only `VectorScore`（ADR-015 add-only，跨仓 Console 同字段 cross-repo signal ADR-014 D4）；`retriever/mod.rs:325` 误导性 WARN → 准确 no-op 契约（chunks 表 §5.3 FROZEN 无 source_type/agent_scope 列，`agent_scope` 属 memory 层 `memory_items` 0013）+ 新 backlog `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（real chunk filter 系 import-path feature 非确定性 nit，ADR-013）。TEST-32.3.1/.2 pass。
> - **task-32.4** closeout → ✅：smoke v22 step [41/41]（TestTask324）+ release docs + ADR-037 per-D ratify + ADR-034 add-only Amendment（Phase 32 / v0.25.0：补全工厂 backend 覆盖 brute/qdrant/lancedb/sqlite-vec；sqlite-vec 矩阵 cell 续 `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`）+ roadmap/adapter add-only。
>
> 全 phase 真实验证：`cargo test --workspace` 199 passed / 0 failed；`go test ./...` 全过（含 TestTask323）；`cargo clippy --workspace --all-targets -D warnings` 0 warning；`go vet` clean；gofmt clean；`spec_drift_lint --touched origin/master` 0 unannotated hits。ADR-037 D2 仅 PARTIAL（sqlite-vec 矩阵 cell `[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`，须 MSVC feature build + 语料）；D1/D3/D5 ✅、D4 honest-defer 边界 reaffirmed（含 `[SPEC-DEFER:phase-future.chunk-source-type-filter]` / `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`，ADR-013 不伪造）。默认构建 0 新 dep + 0 network + 既有契约（proto add-only + factory arm + no-op 契约）不变（ADR-004/008/023）。真实 v0.25.0 tag/release 经用户授权（ADR-012）。

> **v0.26.0 / Phase 33 排期更新（规划中 2026-06-03，add-only，不删上方历史条目）**：§3.15 把第二轮 code-local 治理债排入 **v0.26.0 / Phase 33 — governance-debt-cleanup-2**（task-33.1 L2 cache 有界 / task-33.2 memstore LRU + hard-delete invariant / task-33.3 observability indexing replay + trace isolation + drain verify-only / task-33.4 export --timeout + closeout）。**grounding 诚实校正（ADR-013）**：memory hard-delete cascade 经核为非问题（无可级联表）→ 仅不变式测试；handleMemoryPin strict-400 经核为 ADR-022 D2 蓄意 lenient 契约 → 据实不改；`%v→%w` 经核 non-bug；tracestore-fts 经核已修复；events-drain-timeout 经核 Phase 26 已交付 verify-only。**新增 backlog（add-only）**：`l2-cache-true-lru`（须 ALTER）/ `memory-harddelete-cascade`（仅当未来加 FK）/ `indexing-replay-e2e`（须 daemon）/ `tracestore-multi-workspace-strict-e2e`（须 console）/ `daemon-options-datadir`（Go→Rust 跨进程 datadir transport 重构，🟡）。ADR-038 Proposed。

---

## 5. 执行协议（承项目契约）

每个候选版本的实现遵循既有规则，无例外：

1. **S2V 四步**逐 task：spec（已在规划阶段起草，Draft）→ RED → GREEN → REFACTOR。
2. **一 task 一 PR**；CI 三门（cargo-test / go-test / spec-lint）全绿后自主 merge + 删分支；**红灯绝不合**（已知 phase9 Tantivy `LockBusy` flake → `gh run rerun --failed` 复跑至绿）。
3. **ADR-014 D1-D5** 逐 task：D1 mapping、D2 lint 0 未标注命中、D3 verified-by、D4 自治、**D5 不溯改已闭合 Phase 1-19 spec（ADR 改动 add-only amendment）**。
4. **ADR-013 禁伪造**：🟡/🔴 项的真实数值 / 联调 / 跨平台只在真实证据下记录；未达不标 `[x]`，provisional ADR 不在缺真实数据时翻 Accepted。
5. 版本全部 task 合入后起 release docs（README/RELEASE_NOTES/evidence/artifacts）+ phase §6 ACs `[x]` + Status Done + adapter 行 Done；**tag push 前停下等用户明确授权**；授权后 push → release.yml → post-tag-push backfill PR 填实 tag SHA / run ID / 镜像 digest。
6. 治理承**单驱动 + 内部 Agent subagent**（ADR-011 / ADR-012），不外派 worker。
