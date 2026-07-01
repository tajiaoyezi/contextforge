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
- `[SPEC-OWNER:phase-future.console-data-plane-surfacing]`（v0.28.0 全面审查记录）：v0.22–v0.27 新增数据面能力（vector backend 选择 / hybrid + reranker 检索 / observability 事件 SSE / `vector_score` 溯源）在本仓已具备 **contract / proto 层暴露**（console-api `/v1/search`、`/v1/observability/events` 等），但**专属 console UI 呈现**仍属 ContextForge-Console 独立仓领域；本仓职责同上（契约层 provenance + 跨仓通知，不实现 UI）。**v0.28.0（observability-hardening）经审查为 daemon stderr-only、零 console 接触面**（无 consoleapi/proto/RPC/字段/endpoint 改动），不新增任何 console 对接义务。🔴 跨仓协调项，非本仓 backlog。

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

### 3.16 v0.27.0 / Phase 34 — vector-config-completeness（承 Phase 32，post-v0.26.0 add-only 排期）

**目标**：把 Phase 32（vector-backend-config-plumbing-and-completeness, Done）落地的「env→server.rs 选 backend」补全为「vector dim 经工厂真实协商 + vector backend 经 config 文件（非仅 env）选用」，闭合 §3.14 排期时新开的两条 grounded backlog（`vector-dim-auto-negotiation` / `vector-backend-config-file`）。这是一个**刻意小**的版本——Phase 31/33 两轮治理债清扫后绿区 backlog 已薄，据实排小版本不凑数（ADR-013，honest over padding）。

**来源 marker（§4 / §3.14 排期时新开 backlog）**：
- `[SPEC-DEFER:phase-future.vector-dim-auto-negotiation]`（`core/src/retriever/vector/factory.rs:33-39` `select_vector_backend(name, dim)` 现 `let _ = dim` 静默丢弃 `server.rs:540 resolve_vector_backend` 解析并传入的 `CONTEXTFORGE_VECTOR_DIM`）。
- `[SPEC-DEFER:phase-future.vector-backend-config-file]`（超 env 的结构化 vector backend 配置——经 Go `config.toml` `[vector]` 段 → env bridge）。

**候选 task 拆分**：
- **task-34.1** vector-dim-auto-negotiation：`factory.rs` 以纯函数 `negotiate_vector_dim(dim, backend.expected_dim())` 替代 `let _ = dim`（仿 `embedding/factory.rs:81-96 negotiate_dim`）；`VectorBackend` trait 加 `expected_dim(self) -> Option<usize>` DEFAULT impl 返 `None`（dim-agnostic），`BruteForceVectorBackend` 保 `None`；`VectorError::DimMismatch{expected,got}` 已实存（`types.rs:83`）。**honest-caveat**：默认 BruteForce `expected_dim()=None` → 默认构建协商接受任意 dim（无强制、byte-equivalent 默认行为，ADR-004），真实强制仅对声明 dim 的 feature backend（qdrant/lancedb/sqlite-vec）生效，其 live 维度 `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`（须 feature build）。🟢 纯函数协商单测。
- **task-34.2** vector-backend-config-file：Go `internal/config/config.go` add-only `[vector]` 段（`Backend string`/`Dim int` toml 标签）+ `setVectorEnv` helper（仿 `cmd/contextforge/main.go:255 setDataDirEnv` 跨进程 env-bridge：`[vector]` 存在且对应 env 未设时 export `CONTEXTFORGE_VECTOR_BACKEND`/`CONTEXTFORGE_VECTOR_DIM`，spawned core daemon 经既有 `resolve_vector_backend` env 路径接收）。**ENV WINS**：显式 env 覆盖 config 文件（向后兼容）；无 `[vector]` 段 → 不 export → unset → BruteForce byte-equivalent（ADR-004 默认不变）。Rust core 无 toml dep → 复用 `CONTEXTFORGE_DATA_DIR` 同款已验证跨进程 env-bridge（**非** `daemon.Options.DataDir` 字段重构，后者续 `[SPEC-DEFER:phase-future.daemon-options-datadir]`）。🟢 Go config round-trip + setVectorEnv 单测，0 新 dep。
- **task-34.3** v0.27.0 closeout：**grounding 诚实校正**——`get_source_chunk` workspace 隔离经核**已实存**（`core/src/data_plane/search.rs:421-423` 自 task-12.2 起按 `req.workspace_id` scope candidates：非空→仅该 workspace / 空→aggregate-all probe），survey 高估为 gap → 仅 verify-only guard 不变式测试（workspace_id 设→仅该 workspace chunk / 跨 workspace chunk_id→not_found / 空→aggregate）记录已存在隔离，无新代码（ADR-039 记此 grounding 校正）。smoke v24 step [43/43]（banner v23→v24，staging `cf-v26-cfg`，offset +2）+ TestTask343（镜像 TestTask334，无 [37/37]..[42/42] 回归）+ v0.27.0 release docs + README v0.27 段 + RELEASE_NOTES v0.27.0 段 + ADR-039 ratify + ADR-037 add-only Phase 34 Amendment（dim-negotiation + config-file 完成 Phase 32 起的 env-plumbing，不溯改正文 D5）+ roadmap/adapter add-only + feature。🟢。

**ADR**：**ADR-039 vector-config-completeness**（Proposed，D1 vector-dim-auto-negotiation（工厂 negotiate + expected_dim，默认 BruteForce no-op honest-caveat，feature-enforce SPEC-DEFER）/ D2 vector-backend-config-file（Go `[vector]`→env bridge，env-wins，无段=byte-equiv，Rust 0-dep 保留）/ D3 get_source_chunk 隔离已实存 verify-only（grounding 校正）+ dropped/honest-defer 边界 / D4 默认行为 + 0-dep + 0-network + 既有契约不变（ADR-004/008）；真实测试出来才 ratify，vector-dim-feature-enforce 受阻维度据已达维度 ratify）。ADR-014 第二十五次激活。

---

### 3.17 v0.28.0 / Phase 35 — observability-hardening（承 Phase 31/33 治理债血脉，post-v0.27.0 add-only 排期）

**目标**：承 Phase 31（governance-debt-cleanup, Done）/ Phase 33（governance-debt-cleanup-2, Done）治理债血脉，把热路径中**被静默吞掉的真实错误**显式化（surface genuinely-swallowed errors），镜像仓库既有 stderr 惯例（Rust `eprintln!` / Go `fmt.Fprintf(os.Stderr)`）。这是一个**刻意小**版本——第三轮债清理性质、边际递减（diminishing returns），据 ADR-013 honest over padding 据实排小不凑数；经 AskUserQuestion（2026-06-04）用户选「A 可观测性硬化（纯绿区）」+「规划+实现+发版（无人值守）」即 v0.28.0 release 授权（ADR-012）。

**来源（backlog grounding workflow 实测 v0.27.0 后 GitHub 0 issue/0 PR，高价值项全 🔴 外部受阻 → 绿区焦点小版本）+ 关键诚实校正（ADR-013，本版核心价值）**：survey 初列 7 处「静默吞错」候选，grounding 据实收敛为 **3-4 处真静默**——其余 4 处 DROP/LEAVE（已显式化 / 设计内有意为之，不改代码记 grounding 校正）：`search.rs:109`（已 `eprintln!` WARN，缺的只是结构化计数器但 core 无 metrics facility 加=过度工程）/ `mcpadapter/server.go:298`（task-31.3 已 `fmt.Fprintf(os.Stderr)`）/ `mcpadapter/allowlist.go:31`（有意 POSIX-only 平台 caveat，改 Windows ACL 须引 `golang.org/x/sys/windows` 破 0-dep）/ `index_session_backend.rs:193` `eb.send`（有意 no-subscribers 正常态）。

**候选 task 拆分**：
- **task-35.1** rust-silent-failure-surfacing：`core/src/jobs/index_session_backend.rs:201` `let _ = store.append(...)`（indexing-event SQLite 持久化真实错误：磁盘满/锁）→ `if let Err(e) = ... { eprintln!("WARN ...: {e}") }`（仍 best-effort 不阻断 indexing）+ `core/src/retriever/mod.rs:415` `Err(_) => continue`（Tantivy/SQLite desync 静默跳过命中）→ `Err(e) => { eprintln!("WARN retriever: ... desync, skipping: {e}"); continue }`（skip 行为保留）；镜像 `search.rs:108-113`；`eb.send:193` LEAVE AS-IS。Rust eprintln! 输出 std 单测难断言（仓库既有 eprintln! 站点亦不断言）→ guard/behavior-preservation 测试，据实不伪造 stderr-assert（ADR-013）。🟢 0 新 dep。
- **task-35.2** go-silent-failure-surfacing：`cmd/contextforge/main.go:297` `setVectorEnv` 内 `config.Load` 错误（malformed/unreadable config.toml 被静默吞）+ `:308` `os.Setenv` 失败 → `fmt.Fprintf(os.Stderr)` 显式化（镜像 `daemon/rest.go:110`，仍 best-effort env-only 路径失败时不变）；`memstore.go:579` `emitMemoryEvent` nil-sink no-op 🟡 实施期 grounding 定夺（production-wired → 一次性 `sync.Once` WARN / fallback-only by-design → honest non-issue `[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]`）。stderr-capture（`os.Pipe`）真实 RED→GREEN。🟢 0 新 dep。
- **task-35.3** v0.28.0 closeout：grounding 校正记录（7→3-4 DROP/LEAVE 4 处）+ smoke v25 step [44/44]（banner v24→v25，staging `cf-v27-cfg`，offset +2）+ TestTask353（镜像 TestTask343，无 [37/37]..[43/43] 回归）+ release docs + README v0.28 段 + RELEASE_NOTES v0.28.0 段 + ADR-040 ratify + ADR-031 add-only Phase 35 Amendment（承 stderr/best-effort surfacing 方向，不溯改正文 D5）+ roadmap/adapter add-only + feature。🟢。

**ADR**：**ADR-040 observability-hardening**（Proposed，D1 rust-silent-failure-surfacing（`index_session_backend.rs:201` + `retriever/mod.rs:415` eprintln! WARN，best-effort 保持，guard 测试）/ D2 go-silent-failure-surfacing（`setVectorEnv` config.Load/Setenv fmt.Fprintf stderr，stderr-capture RED→GREEN；memstore nil-sink 🟡 impl-grounding）/ D3 grounding 校正诚实 7→3-4 收敛（4 处 DROP/LEAVE，不引新 metrics facility）/ D4 默认行为 + 0-dep + 0-network + 既有契约不变（ADR-004/008，best-effort 不转 fail-fast）；真实测试出来才 ratify，memstore nil-sink 🟡 据实施期 grounding 定夺）。ADR-014 第二十六次激活。

### 3.18 v0.29.0 / Phase 36 — qdrant-live-vector-recall（承 Phase 25/29 live 向量召回血脉，post-v0.28.0 add-only 排期）

**目标**：兑现 ADR-034 D2 一路 honest-defer 的 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（真实 live-server 端到端 KNN 召回数）——把「qdrant live KNN wiring 经 honest-defer 证明、但真实召回数从未跑出（in-repo `eval_integration.rs` 0.7/0.85 是合成 fixture）」推进到「对真实 qdrant server 跑出真实 recall@k + 经 CI service container 每次 run 永久验证」。**关键 de-risk 已证明**：真实 qdrant + qdrant-client 1.18 端到端 round-trip 跑通、KNN 余弦序正确（query `[1,0,0,0]`→`[(a,1.0),(c,0.994)]`）。qdrant backend 自 Phase 25/29 已全实现（connect/health/ensure-create/upsert/KNN/delete），本 phase 0 行 backend 改动、只加 harness + CI 接线。经用户 AskUserQuestion（2026-06-04）选「C 解锁高价值项 → qdrant live 向量召回（自起 docker）」+「规划+实现+发版（无人值守）」即 v0.29.0 release 授权（ADR-012）。

**来源 marker（§4 向量 backend 细化段 + ADR-034 D2 Ratification 🟡 PARTIAL）**：
- `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（ADR-034 D2 残留 live-server KNN 召回维度——本 phase 兑现 + CI-guarded 永久关闭）。
- 合成-fixture 召回（`eval_integration.rs` 0.7/0.85 非真实）→ 真实测量取代（ADR-013，A1 synthetic-fixture REJECTED）。

**候选 task 拆分**：
- **task-36.1** qdrant-live-recall-harness：新增 `core/tests/qdrant_live_recall.rs`（`#![cfg(feature = "vector-qdrant")]`，env-gated `QDRANT_URL` 复用 `QdrantConnConfig::from_env()`；`health() != Ready` → honest-defer 干净 skip 不 fail）；确定性可复现语料（N=1000 dim=64 index-seeded 单位向量，无 `rand`/无 clock）双索引进 `QdrantBackend` 与 `BruteForceVectorBackend`（精确 ground truth）；M=50 query recall@k = mean(|∩|/k) 断言 ≥ floor（k=10→0.90）+ eprintln 实测数（真实跑出后回填，绝不预填，ADR-013）。🟢 generator / 🔴 live server。0 新 dep。
- **task-36.2** qdrant-recall-ci-service：`.github/workflows/ci.yml` 加 `qdrant-recall` job（qdrant service container + toolchain 1.93 + protoc + 跑 harness）→ 每次 CI run 对 live service container 验证 recall、永久关闭 `qdrant-server-lifecycle`。CI-only / add-only / 默认构建不变。验证证据 = PR 自身 live CI run（据实记录，ADR-013）。
- **task-36.3** v0.29.0 closeout：smoke v26 step [45/45]（banner v25→v26，staging `cf-v28-cfg`）+ TestTask363（镜像 TestTask353，无 [37/37]..[44/44] 回归）+ release docs（真实召回数 + 真实 CI run 链接）+ README/RELEASE_NOTES v0.29 + ADR-041 ratify + ADR-034 add-only Phase 36 Amendment（标 D2 fulfilled，不溯改 D-body D5）+ roadmap §3.18/§4 + adapter + feature。🟢。

**ADR**：**ADR-041 qdrant-live-vector-recall**（Proposed，D1 live recall harness（qdrant HNSW ANN recall@k vs BruteForce 精确 KNN 方法学；确定性可复现语料；无 server env-gated honest-defer）/ D2 真实测量召回数（`待回填` 直至 CI run 跑出，ADR-013 不伪造）/ D3 CI service-container 集成（每次 run 验证、永久关闭 CI-no-server defer）/ D4 默认 0-vector-dep + 行为不变 + 0 新 dep（ADR-004/008））；ADR-034 add-only Phase-36 Amendment（兑现 D2 `qdrant-server-lifecycle`，不溯改 D-body D5）。ADR-014 第二十七次激活。

**v0.29.0 推进记录（已实现，发版待 tag）**：task-36.1（#236，harness `core/tests/qdrant_live_recall.rs`，本地 recall@10=1.0000）+ task-36.2（#237，`qdrant-recall` CI service-container job，**CI run 26961084355 实测 recall@10=1.0000**，`qdrant-server-lifecycle` 永久关闭）+ task-36.3（closeout：smoke v26[45/45] + ADR-041 Accepted + ADR-034 add-only Phase-36 Amendment 标 D2 fulfilled + release docs）全 Done 三门绿合入 master。诚实：recall=1.0 = qdrant 低于 HNSW indexing_threshold 服务精确 KNN 的 live 正确性证明，HNSW 近似域大语料 recall 续 `[SPEC-DEFER:phase-future.vector-large-corpus-perf]`。

### 3.19 v0.30.0 / Phase 37 — embedding-provider-remote-live（承 Phase 22 embedding provider 抽象血脉，post-v0.29.0 add-only 排期）

**目标**：兑现 ADR-027 一路 honest-defer 的 `[SPEC-DEFER:phase-future.embedding-provider-remote]`（真实远程 embedding 端点端到端联调 + 实测语义召回）——把「remote provider 纯函数契约层（`build_request_body`/`parse_response`）已测、但 live 端点从未联调、真实召回从未跑出」推进到「对真实 OpenAI-compatible 端点跑出真实 recall@k + Go `[remote]` config 经 env-bridge 接通」。**关键 de-risk 已由主 agent 本机真实证明**：SiliconFlow（OpenAI-compatible）+ `Qwen/Qwen3-Embedding-8B` 端到端 round-trip 跑通（native dim=4096，OpenAI 风格 `dimensions` 参数生效——MRL 用 1024，CJK 正常），Rust `RemoteEmbeddingProvider`→ureq→parse 在 Windows MSVC `--features embedding-remote` 真实编译跑通。embedding provider 抽象自 Phase 22 已全实现——本 phase 0 行 provider 核心改动、只加 harness + Go config env-bridge + closeout。默认构建不变（`embedding-remote` opt-in，0 网络 / 0 新 dep——`ureq` 自 task-22.3 已 optional，ADR-004/008）；API key env-only 永不进 config。经用户 AskUserQuestion（2026-06-06）选「解锁高价值项 → 远程 embedding live 召回（提供 SiliconFlow key）」+「完整 S2V phase + 发版 v0.30.0（无人值守）」即 v0.30.0 release 授权（ADR-012）。

**来源 marker**：
- `[SPEC-DEFER:phase-future.embedding-provider-remote]`（ADR-027 母 ADR 残留 live 端点联调 + 真实召回维度——本 phase 兑现，add-only Phase-37 Amendment 标 fulfilled、不溯改 D-body）。
- `core/src/embedding/factory.rs:52` 注释 "config plumbing is a follow-up"（remote endpoint/model/provider 仅 env 读取、config.toml 未接通——本 phase task-37.2 兑现）。

**候选 task 拆分**：
- **task-37.1** remote-embedding-live-recall-harness：新增 `core/tests/remote_embedding_recall.rs`（`#![cfg(feature = "embedding-remote")]`，env-gated `CONTEXTFORGE_REMOTE_API_KEY` honest-defer skip）；作者手工标注语义集（15 case / 16 doc，含故意近义干扰 `config_save`/`config_load`、`bm25`/`hybrid`、`cjk_index`/`cjk_vector`）同一 `BruteForceVectorBackend` 精确余弦路径上 real 模型 vs deterministic 基线 recall@1/@3，floor `r3>=0.70` 且 remote>deterministic；非网络 well-formed 守护无 key 也跑。🟢 守护 / 🔴 live 端点。主 agent 本机真实 run（SiliconFlow Qwen3-Embedding-8B，dim=1024）实测 **remote recall@1=0.8667 / recall@3=1.0000 vs deterministic 0.0000 / 0.0667**（真实非预填，详 task-37.1 §10 / ADR-042 D2，ADR-013）。
- **task-37.2** remote-embedding-config-bridge：Go `RemoteProviderConfig` add-only `Model` 字段 + 新 `setRemoteEnv` 跨进程 env-bridge（镜像 Phase 34 `setVectorEnv`：`[remote]` 段 → 导出 `CONTEXTFORGE_REMOTE_ENDPOINT/_MODEL/_PROVIDER`，env-wins，无段不导出）接线 doServe/doMCP；API key env-only 永不进 config；Rust 0 toml dep。🟢。
- **task-37.3** v0.30.0 closeout：smoke v27[46/46]（staging `cf-v29-cfg` offset +2）+ release docs（真实 recall 数 + 诚实记 CI honest-defer：remote 付费外部 API 无免费 service container，召回由本机已认证 run 实测——与 qdrant 不同）+ ADR-042 ratify + ADR-027 add-only Phase-37 Amendment + roadmap §3.19/§4 + adapter。🟢。

**ADR**：**ADR-042 embedding-provider-remote-live**（Proposed，D1 live 语义 recall harness 方法学（real vs deterministic 基线对照 + 作者标注集诚实范围 + env-gated honest-defer + 小集 caveat）/ D2 真实实测召回数（本机真实 SiliconFlow run；CI honest-defer 因 remote 付费 API 无免费 service container）/ D3 remote-embedding-config-bridge（Go `[remote]` Model + setRemoteEnv env-bridge，API key env-only，Rust 0-dep）/ D4 默认 0-network + 0 新 dep + 既有契约不变（ADR-004/008））；ADR-027 add-only Phase-37 Amendment（兑现 `embedding-provider-remote`，不溯改 D-body D5）。ADR-014 第二十八次激活。

**v0.30.0 推进记录（已落地 2026-06-06，add-only）**：§3.19 全 3 task 合入 master，ADR-042 据 D1-D4 真实 ratify（Proposed → Accepted）：task-37.1（#242，harness `core/tests/remote_embedding_recall.rs`，本机真实 SiliconFlow `Qwen/Qwen3-Embedding-8B` dim=1024 实测 **remote recall@1=0.8667–0.9333 跨 run 波动 / recall@3=1.0000 稳定** vs deterministic 0.0000/0.0667）+ task-37.2（#243，Go `RemoteProviderConfig` add-only `Model` + `setRemoteEnv` env-bridge，env-wins，API key env-only 永不进 config，Rust 0 toml dep）+ task-37.3 closeout（smoke v27[46/46] + release docs + ADR-042 ratify + ADR-027 add-only Phase-37 Amendment 标 `embedding-provider-remote` fulfilled + roadmap/adapter）。诚实：recall@3=1.0 = real 模型把明显语义对排在近义干扰之上的小集正确性证明（非大基准质量断言，大语料续 `[SPEC-DEFER:phase-future.embedding-large-corpus-recall]`）；recall@1 跨 run 波动据实记录（remote 模型/服务非完全确定）；CI honest-defer 因 remote 付费 API 无免费 service container（与 qdrant 诚实差异），召回由本机已认证 run 实测。

### 3.20 v0.31.0 / Phase 38 — embedding-remote-reranker-live（承 Phase 37 remote provider live 血脉 + ADR-026 reranker 维度，post-v0.30.0 add-only 排期）

**目标**：兑现 `[SPEC-DEFER:phase-future.embedding-remote-reranker-live]`（由 ADR-042 / phase-37 spec §不在范围 / roadmap §3.19 follow-up 记录）——把「reranker 维度（ADR-026 已确立 `Reranker` trait + `IdentityReranker` + feature-gated `CrossEncoderReranker`）已有本地实现、但 `RemoteRerankerProvider` 不存在、`select_reranker` 工厂不存在、reranker 至今从未由 config 在生产数据面路径接线」推进到「构建 remote reranker provider + 工厂 + 对真实 cross-encoder over HTTP 端点跑出真实 rerank 质量（MRR/recall@1）+ 首次把 reranker 从 config 在生产路径 opt-in 接通」。与 Phase 37（复用既有 provider）的**核心差异**：本 phase 要**构建** provider（`RemoteEmbeddingProvider` 自 Phase 22 已全实现，但 `RemoteRerankerProvider` / `select_reranker` 工厂从无），且**数据面首次 opt-in 接线**（`server.rs` hybrid `:334` + semantic `:376` 与 `data_plane/search.rs` semantic `:282` 三处只调 `.with_embedder().with_vector_searcher()`、从不调 `.with_reranker()`，reranker 仅 Phase 21 的 opt-in builder seam 仅测试用）。**关键 de-risk 已由主 agent 本机真实证明**：SiliconFlow `https://api.siliconflow.cn/v1/rerank` + `Qwen/Qwen3-VL-Reranker-8B`（与 embedding 同 URL 同 key、仅 model 不同）端到端 round-trip 跑通——查询「如何保存配置到文件」对 4 文档 rerank，相关文档 `config_save` `relevance_score=0.7356` 排 #1、近义干扰 `config_load=0.0158` 排 #2、无关项 ~0.0006/0.0003，HTTP 200，约 46x 区分度，证明端点可用 + 排序语义正确。默认构建不变（`reranker-remote` opt-in、0 网络 / 0 新 dep——`ureq` 自 task-22.3 已 optional，新增 `reranker-remote = ["dep:ureq"]` 复用既有 ureq；0 proto / 0 migration；数据面默认 `CONTEXTFORGE_RERANKER_PROVIDER` unset → 字节等价无 rerank，向后兼容，ADR-004/008）；API key env-only 永不进 config。

**来源 marker**：
- `[SPEC-DEFER:phase-future.embedding-remote-reranker-live]`（ADR-042 / phase-37 spec §不在范围 / roadmap §3.19 follow-up 记录 remote reranker cross-encoder over HTTP live 联调——本 phase 兑现，ADR-026 母 ADR add-only Phase-38 Amendment 标 fulfilled、不溯改 D-body，亦 ADR-042 add-only Amendment 标 follow-up fulfilled）。
- ADR-026 reranker-provider（v0.14.0，确立 `Reranker` trait + `IdentityReranker` + `CrossEncoderReranker`）的 remote reranker 维度——本 phase 兑现，构建 `RemoteRerankerProvider` + `select_reranker` 工厂镜像 `embedding/factory.rs:27-96` 的 "remote" 分支。

**候选 task 拆分**：
- **task-38.1** remote-reranker-provider-and-live-recall：新增 `core/src/rerank/remote_provider.rs`（`RemoteRerankerProvider`：`build_rerank_request_body`/`parse_rerank_response` 纯函数 + ureq POST，镜像 `RemoteEmbeddingProvider` 与 `CrossEncoderReranker` 的 by-index 映射，Debug 不打印 api_key）+ `core/src/rerank/factory.rs`（`select_reranker(name)` 工厂，镜像 `embedding/factory.rs:27-96`，"remote" 分支从 env 读 `CONTEXTFORGE_RERANKER_ENDPOINT/_MODEL/_PROVIDER/_API_KEY`、feature-off 显式 Err 不静默）+ 新 feature `reranker-remote = ["dep:ureq"]`（复用既有 ureq，0 新 dep）+ 新增 `core/tests/remote_rerank_recall.rs`（`#![cfg(feature = "reranker-remote")]`，env-gated `CONTEXTFORGE_RERANKER_API_KEY` honest-defer skip 不 fail）；作者手工标注 query×candidate 集（每 query 一个已知相关文档 + 故意近义干扰，镜像 embedding harness 的 `docs()`/`cases()` 风格）；候选喂入统一 / 无相关性先验 score（`IdentityReranker` no-semantic-signal 基线 ≈ chance）；real remote cross-encoder vs `IdentityReranker` MRR/recall@1，先 eprintln 再 assert（floor MRR_remote >= 0.70 且 MRR_remote > MRR_identity）；非网络契约 + well-formed 守护（`build/parse` fixture + `select_reranker` 路由 + 标注集 well-formed）无 key 也跑。🟢 守护 / 🔴 live 端点（真实 MRR/recall 真实跑出后回填，绝不预填，ADR-013；de-risk 探针 `config_save relevance_score=0.7356` 排 #1 可引用为可行性证据）。
- **task-38.2** reranker-config-bridge-and-data-plane-wiring：Go `internal/config/config.go` 新增 `RerankerConfig`（`Enabled`/`Provider`/`Endpoint`/`Model`，toml round-trip，无 api-key 字段）+ 新 `setRerankerEnv` 跨进程 env-bridge（镜像 Phase 37 `setRemoteEnv` / Phase 34 `setVectorEnv`：`[reranker]` 段 → 导出 `CONTEXTFORGE_RERANKER_ENDPOINT/_MODEL/_PROVIDER`，env-wins，无段不导出）接线 doServe/doMCP；API key env-only 永不进 config；Rust 数据面新增 `reranker_from_env()`（读 `CONTEXTFORGE_RERANKER_PROVIDER` → 非空 / 非 none 时 `select_reranker` → `with_reranker`）在 `server.rs` hybrid `:334` / semantic `:376` + `data_plane/search.rs` semantic `:282` 三处生产路径 opt-in 接线；默认 unset → 无 reranker provenance marker、向后兼容字节等价无 rerank（这正是为何「只加 Go `[reranker]` config 桥而数据面不消费」不诚实，故 task-38.2 端到端：Go 桥 + Rust 数据面消费）；Rust core 0 toml dep。🟢。
- **task-38.3** v0.31.0 closeout：smoke v28[47/47]（banner v27→v28，staging `cf-v30-cfg` offset +2，TestTask383 镜像 TestTask373，无 [37/37]..[46/46] 回归，`bash -n` 校验）+ release docs（真实 MRR/recall 数 + 诚实记 CI honest-defer：remote 付费外部 API 无免费 service container、rerank 质量由本机已认证 run 实测，复用 `[SPEC-DEFER:phase-future.embedding-remote-ci-credential]`）+ README/RELEASE_NOTES v0.31 段 + ADR-043 据 D1-D4 ratify + ADR-026 add-only Phase-38 Amendment（标 remote reranker 兑现，不溯改 D-body D5）+ ADR-042 add-only Phase-38 Amendment（标 `embedding-remote-reranker-live` follow-up fulfilled）+ roadmap §3.20/§4 + adapter + feature。🟢。

**ADR**：**ADR-043 embedding-remote-reranker-live**（Proposed，D1 live rerank harness 方法学（real cross-encoder vs `IdentityReranker` no-semantic-signal 基线对照 + 作者标注 query×candidate 集诚实范围 + env-gated honest-defer + 小集 caveat）/ D2 真实实测 MRR/recall@1（本机真实 SiliconFlow run；CI honest-defer 因 remote 付费 API 无免费 service container 复用 `embedding-remote-ci-credential`）/ D3 reranker-config-bridge + data-plane-wiring（Go `[reranker]` round-trip + `setRerankerEnv` env-bridge env-wins、API key env-only、Rust 数据面 `reranker_from_env()` 三处 opt-in 接线、Rust 0 toml dep）/ D4 默认 0-network + 0 新 dep + `CONTEXTFORGE_RERANKER_PROVIDER` unset 字节等价无 rerank + 既有契约不变（ADR-004/008））；ADR-026 add-only Phase-38 Amendment（兑现 remote reranker 维度，不溯改 D-body D5）+ ADR-042 add-only Phase-38 Amendment（标 `embedding-remote-reranker-live` follow-up fulfilled）。ADR-014 第二十九次激活。

**v0.31.0 推进记录（已落地 2026-06-06，add-only）**：§3.20 全 3 task 合入 master，ADR-043 据 D1-D4 真实 ratify（Proposed → Accepted）：task-38.1（#247，构建 `RemoteRerankerProvider`（`core/src/rerank/remote_reranker.rs`，镜像 `CrossEncoderReranker` by-index 映射 + `RemoteEmbeddingProvider` 纯函数 + ureq POST + Debug 不打印 api_key）+ `select_reranker` 工厂（`core/src/rerank/factory.rs`）+ `reranker-remote = ["dep:ureq"]`（复用 ureq，0 新 dep）+ harness `core/tests/remote_rerank_recall.rs`；本机真实 SiliconFlow `Qwen/Qwen3-VL-Reranker-8B` 3 次 run 实测 **remote MRR=1.0000 recall@1=1.0000（全稳定）vs identity no-semantic 基线 MRR=0.4762 recall@1=0.0000，delta_MRR=+0.5238**）+ task-38.2（#248，Go `RerankerConfig` add-only + `setRerankerEnv` env-bridge env-wins + Rust `reranker_from_env()` 在 `server.rs` hybrid/semantic + `data_plane/search.rs` semantic 三处生产路径首次 opt-in `with_reranker`，默认 unset 字节等价无 rerank，Rust 0 toml dep）+ task-38.3 closeout（smoke v28[47/47] + release docs + ADR-043 ratify + ADR-026/042 add-only Phase-38 Amendment + roadmap/adapter）。诚实：MRR=1.0/recall@1=1.0 = real cross-encoder 把明显相关文档排在近义干扰之上的小集（14 case）正确性证明（非大基准断言，大语料续 `[SPEC-DEFER:phase-future.reranker-large-corpus-quality]`）；与 Phase 37 embedding recall@1 跨 run 波动不同，cross-encoder rerank 3 次 run 零波动；CI honest-defer 因 remote 付费 API 无免费 service container（与 qdrant 诚实差异，复用 `[SPEC-DEFER:phase-future.embedding-remote-ci-credential]`），rerank 质量由本机已认证 run 实测；仓库转 public 后 Actions 免费跑 CI（#247/#248 各 14/14 绿）。

### 3.21 v0.32.0 / Phase 39 — console-api-retrieval-signal-forward（承 Phase 20 console-api 语义贯通范式 + Phase 21 hybrid 融合 + Phase 38 reranker 数据面接线，post-v0.31.0 add-only 排期）

**目标**：兑现 ADR-025 一路 honest-defer 的 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]`（对外 console-api `?hybrid=true` REST 转发）并据实**重界定**其姊妹延后项 `[SPEC-DEFER:phase-future.console-api-rerank-forward]`——把已存在于检索内核但**对外 REST 不可达 / 不可见**的两个信号（hybrid BM25+vector RRF 融合 + rerank `reason` provenance）经 console_data_plane proto + 数据面 dispatch + Go console-api 转发**首次贯通到对外 `POST /v1/search`**，使 Console / REST 用户能像 `?semantic=true`（Phase 20）那样请求 `?hybrid=true` 并看到 `hybrid_score` 融合分 + rerank `reason` provenance。**核心特征：0 backend 算法改动——「贯通而非重写」**：core hybrid 融合（`server.rs` hybrid 路径 + `search_hybrid` + `req.hybrid` + `hybrid_score`，Phase 21）+ reranker 数据面 opt-in（`reranker_from_env` → `with_reranker`，Phase 38）+ `?semantic` 转发范式（Phase 20）+ `vector_score=16` add-only provenance 范式（Phase 32）均已存在，本 phase 复用全部既有范式只加 console_data_plane proto 两个 add-only 字段 + 数据面 hybrid dispatch 分支 + Go 三处转发 + smoke 端到端断言。**诚实校正（ADR-013）**：历史 `console-api-rerank-forward` 设想 `?rerank=true` per-request 转发，但 Phase 38（ADR-043 D3）已确立 reranker 为服务端 env 驱动（非 per-request）——per-request 转发与 env 驱动模型**冲突**，故本 phase **兑现** hybrid per-request 转发（hybrid 是算法路由 flag，与 `?semantic` 同类），**重界定** rerank-forward 为 rerank provenance（`reason`）在对外 REST 可见性（链路已通、缺端到端断言），`?rerank=true` per-request 据实记为被 ADR-043 D3 取代（superseded）、不实现。默认构建不变（`hybrid` 默认 false → 既有 semantic / BM25 路径字节等价；reranker 默认 unset 字节等价无 rerank；proto add-only 既有字段号冻结，ADR-004/008/015；0 新 dep / 0 migration）。

**来源 marker**：
- `[SPEC-DEFER:phase-future.console-api-hybrid-forward]`（ADR-025 §Follow-ups / task-21.1 §下游 / task-21.3 / README:350 / `console_smoke.sh:49-50` / `smoke_syntax_test.go:705-706` 记录——本 phase 经 console_data_plane proto + 数据面 dispatch + Go 转发兑现，ADR-025 母 ADR add-only Phase-39 Amendment 标 fulfilled、不溯改 D-body）。
- `[SPEC-DEFER:phase-future.console-api-rerank-forward]`（task-21.2 / task-21.3 / README:350 记录——本 phase 据 ADR-043 D3（reranker env 驱动）**重界定**为 rerank provenance 可见性 fulfilled + `?rerank` per-request superseded，ADR-043 add-only Phase-39 Amendment 标记、不溯改 D-body）。

**候选 task 拆分**：
- **task-39.1** console-dataplane-hybrid-proto-and-dispatch：`proto/contextforge/console_data_plane/v1/console_data_plane.proto` add-only `SearchRequest.hybrid=8`（镜像 `v1/search.proto:28`）+ `SearchResultItem.hybrid_score=17`（镜像 `v1 RetrievalResult.hybrid_score=15`，既有字段号 1-7 / 1-16 冻结，ADR-015 D1）+ `buf generate` 重生 Go/Rust stub + `core/src/data_plane/search.rs` `query()` 加 hybrid dispatch 分支（`if req.hybrid {..} else if req.semantic {..} else {BM25}`，hybrid 分支镜像 `server.rs` hybrid 路径 + 数据面 semantic 分支结构：`search_hybrid` + `retrieval_method="hybrid"` + 复用 `reranker_from_env` opt-in）+ 结果映射 `hybrid_score` 填充（镜像 `vector_score` 条件）；默认 `hybrid=false` 字节等价；0 新 dep / 0 migration。🟢。
- **task-39.2** console-api-hybrid-forward-and-rerank-visibility：`internal/contractv1/contractv1.go` add-only `SearchRequest.Hybrid bool`（镜像 `Semantic`）+ `SearchResult.HybridScore float32`（镜像 `VectorScore`）+ `internal/consoleapi/handlers.go` `handleSearch` `?hybrid` OR-merge（镜像 `?semantic`）+ `internal/consoleapi/grpcclient/grpcclient.go` `Search` 转发 `Hybrid` + `protoToSearchResult` 映射 `HybridScore`；对外 `POST /v1/search`（body `{"hybrid":true}` 或 `?hybrid=true`）贯通 hybrid；rerank `reason` provenance 在对外 REST 可见（reranker 保持 env 驱动、不做 per-request，`?rerank` 据 ADR-044 D3 superseded）；默认 hybrid=false 字节等价；0 新 dep / 0 proto 再改。🟢。
- **task-39.3** v0.32.0 closeout：smoke v28→v29[48/48]（staging 顺位 offset，端到端断言 `?hybrid=true` → `retrieval_method="hybrid"` / `hybrid_score` + `CONTEXTFORGE_RERANKER_PROVIDER=identity` → rerank `reason` 对外 REST 可见，TestTask393 镜像 TestTask383，无 [37/37]..[47/47] 回归）+ release docs（hybrid 贯通 + rerank provenance 可见证据 + README:350 措辞替换，tag/run/digest `<backfill>` marker）+ ADR-044 据 D1-D4 ratify + ADR-025 add-only Phase-39 Amendment（标 console-api-hybrid-forward fulfilled）+ ADR-043 add-only Phase-39 Amendment（标 console-api-rerank-forward 重界定 fulfilled + `?rerank` per-request superseded）+ roadmap §3.21/§4 + adapter + defer marker 更新。🟢。

**ADR**：**ADR-044 console-api-retrieval-signal-forward**（Proposed，D1 console_data_plane proto add-only + 数据面 hybrid dispatch（既有字段号冻结、镜像 `server.rs` hybrid 路径、默认 hybrid=false 字节等价）/ D2 Go console-api hybrid 转发 + rerank provenance 可见性（镜像 `?semantic` 范式、对外 `POST /v1/search` 贯通）/ D3 rerank-forward 重界定（reranker 保持 env 驱动、`?rerank` per-request superseded by ADR-043 D3、交付 provenance 可见性）/ D4 默认 hybrid=false / reranker unset 字节等价 + 0 新 dep + proto add-only 既有契约不变（ADR-004/008/015））；ADR-025 add-only Phase-39 Amendment（兑现 `console-api-hybrid-forward`，不溯改 D-body D5）+ ADR-043 add-only Phase-39 Amendment（重界定 `console-api-rerank-forward`，不溯改 D-body D5）。ADR-014 第三十次激活。Phase 39 实现 + 发版须另行 ADR-012 授权（本批为规划稿，Draft/Proposed）。

**v0.32.0 推进记录（已落地 2026-06-06，add-only）**：§3.21 全 3 task 合入 master，ADR-044 据 D1-D4 真实 ratify（Proposed → Accepted）：task-39.1（#252，console_data_plane proto add-only `SearchRequest.hybrid=8` + `SearchResultItem.hybrid_score=17`（既有字段号 1-7 / 1-16 冻结，`buf generate proto` rawDesc 重编码无 message/service 重排）+ `core/src/data_plane/search.rs` `query()` 三分支 hybrid dispatch（复用 `search_hybrid` + `reranker_from_env`，hardcoded `BruteForceVectorBackend`）+ `hybrid_score` 填充镜像 `vector_score`；TEST-39.1.1 数据面 hybrid dispatch + TEST-39.1.2 proto 字段号 8/17 via prost wire tag）+ task-39.2（#253，Go `contractv1.SearchRequest.Hybrid` / `SearchResult.HybridScore` add-only + `handleSearch` `?hybrid` OR-merge + `grpcclient` 转发/映射；对外 `POST /v1/search` 贯通 hybrid + rerank `reason` provenance 可见；reranker 保持 env 驱动不加 `?rerank`；TEST-39.2.1/39.2.2）+ task-39.3 closeout（smoke v29[48/48] REAL 模式 `?hybrid=true` → `retrieval_method="hybrid"` + TestTask393 无回归 + release docs + ADR-044 ratify + ADR-025/043 add-only Phase-39 Amendment + roadmap/adapter + defer marker 更新）。**0 backend 算法改动「贯通而非重写」**：`search_hybrid` / RRF / `reranker_from_env` 自 Phase 21/38 起 0 改动；默认 `hybrid=false` 字节等价 + reranker unset 字节等价 + proto add-only 既有字段号冻结 + 0 新 dep（ADR-004/008/015）。诚实校正（ADR-013）：`?rerank=true` per-request 与 ADR-043 D3 env 驱动冲突 → 记为 superseded、不实现，改交付 rerank provenance 可见性（兑现 `console-api-hybrid-forward` + 重界定 `console-api-rerank-forward`，ADR-025/043 add-only Phase-39 Amendment 标 fulfilled、不溯改 D-body D5）。**与 remote embedding/reranker 不同**：hybrid 无外部付费 API 依赖（model-free + 0-dep backend，0 网络），故由每次 CI run 全程守护（无 honest-defer skip）。#252/#253 各 CI 14/14 绿。真实 v0.32.0 tag/release 经用户授权 push（ADR-012），tag SHA/digest/tlog post-tag-push 回填（ADR-013 不预填）。

### 3.22 v0.33.0 / Phase 40 — governance-debt-cleanup-3（承 Phase 31/33 治理债清扫血脉，post-v0.32.0 add-only 排期）

**目标**：第三轮治理债清扫（镜像 Phase 31 / ADR-036 + Phase 33 / ADR-038 的「核实-诚实化-补全」打法），清理两组在 grounding 中确认为**真实且 code-local 可单测**的跨 Phase 治理 marker：**memory pin actor 透传**（`core/src/data_plane/memory.rs` `pin()` 把调用 actor 硬编码 `"console-api"`——因 `PinMemoryRequest` 无 actor field、Go `MemoryStore.Pin` 无 actor 参数、`handleMemoryPin` 不读调用方标识；`set_pinned_with_actor` store 层本就接受 actor（task-27.1 / ADR-032 D1），仅入口透传链缺）+ **L2 embedding 缓存访问序 LRU**（Phase 33 D1 给 L2 加了 rowid-FIFO 插入序驱逐但 `sqlite_get` 命中不重排 → 是插入序 FIFO 而非访问序 LRU）。**核心特征：0 新依赖、复用既有范式**：actor 透传复用既有 `set_pinned_with_actor` 显式 actor 参数 + `r.Header.Get` header 读取范式；L2 访问序 LRU 复用既有隐式 rowid + Go memstore 命中 move-to-front 技法。**诚实校正（ADR-013）**：(1) pin actor 本轮交付**调用方透传**（header → proto → store），**认证身份**（把 header 值校验为已认证 auth subject）须 console-api 鉴权层 → honest-defer；(2) L2 真-LRU 据实**更正** Phase 33「须加 created_at 列 + ALTER」假设——命中 bump 隐式 rowid 即得访问序 LRU（0 schema migration，与 Go memstore move-to-front 同技法），命中 bump 写放大 + `with_sqlite` 无生产调用点现网零影响据实记；(3) 其余治理 marker（`vector-dim-feature-enforce` 须 feature build / `tracestore-multi-workspace-strict` 余下读路径 / `chunk-source-type-filter` 须 import-path migration）据实保持延后不强行扩面（焦点小版本，honest over padding）。默认构建不变（pin actor proto field + Go 参数 add-only、空 actor 回落 `"console-api"` byte-equiv；L2 命中 bump 仅有限 cap 生效、cap==0 byte+perf-equiv；ADR-004/008/015；0 新 dep / 0 schema migration）。

**来源 marker**：
- `[SPEC-DEFER:phase-future.memory-actor-propagation]`（`core/src/data_plane/memory.rs:227` / ADR-032 §D1 Trade-offs 记录——本 phase 经 proto add-only `actor=3` + Go 参数链 + REST `X-Actor` header + Rust 空回落兑现 actor 入口透传维度，ADR-032 add-only Phase-40 Amendment 标 fulfilled、不溯改 D-body；认证身份续 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`）。
- `[SPEC-DEFER:phase-future.l2-cache-true-lru]`（ADR-038 §A2/D4 / `[SPEC-DEFER]` cache.rs 记录——本 phase 经命中 bump 隐式 rowid（0 schema migration）兑现真-LRU 维度 + 据实更正 Phase 33「须加时间列」假设，ADR-038 + ADR-027 add-only Phase-40 Amendment 标记、不溯改 D-body）。

**候选 task 拆分**：
- **task-40.1** memory-actor-propagation：`PinMemoryRequest` add-only `string actor = 3`（既有 memory_id=1 / pin=2 字段号冻结，ADR-015 D1）+ buf generate + Go `MemoryStore.Pin(id,pin)` → `Pin(id,pin,actor)`（interface + `memoryClient.Pin` / `MemMemoryStore.Pin` 两实现）+ `grpcclient` 填 `pb.PinMemoryRequest.Actor` + `handleMemoryPin` 读 `r.Header.Get("X-Actor")`（缺省空串）+ Rust `pin()` `set_pinned_with_actor(.., if req.actor.is_empty() { "console-api" } else { req.actor.as_str() })`（空回落 byte-equiv）；认证身份 honest-defer；ADR-022 D2 宽松 body 契约不改；0 新 dep / proto add-only。🟢。
- **task-40.2** l2-embedding-cache-true-lru：`core/src/embedding/cache.rs` `sqlite_get` 命中时（仅 `l2_cap > 0`）`INSERT OR REPLACE` 原样回写命中行 bump 隐式 rowid 到表尾，使既有 `sqlite_put` rowid 序驱逐由插入序 FIFO 升访问序 LRU；cap==0 不 bump（保插入序、零额外写）；复用既有隐式 rowid、0 新 dep / 0 schema migration；据实更正 Phase 33 真-LRU 假设；命中 bump 写放大 + opt-in-path 现网零影响据实记。🟢。
- **task-40.3** v0.33.0 closeout：smoke v29→v30[49/49]（staging 顺位 offset，pin actor 透传 + L2 访问序 LRU，TestTask403 镜像 TestTask393 无 [37/37]..[48/48] 回归）+ release docs（tag/run/digest `<backfill>` marker）+ ADR-045 据 D1-D3 ratify + ADR-032 add-only Phase-40 Amendment（标 memory-actor-propagation fulfilled + 认证身份续延后）+ ADR-038 + ADR-027 add-only Phase-40 Amendment（标 l2-cache-true-lru fulfilled + 真-LRU 假设据实更正）+ ADR-015 add-only Amendment（proto add-only field）+ roadmap §3.22/§4 + adapter + defer marker 更新。🟢。

**ADR**：**ADR-045 governance-debt-cleanup-3**（Proposed，D1 memory pin actor add-only 透传（proto field 字段号冻结 + Go 参数链 + REST header + 空回落 byte-equiv；认证身份 honest-defer）/ D2 L2 embedding 缓存访问序 LRU（命中 bump 隐式 rowid，0 migration，更正 Phase 33 真-LRU 假设；写放大 + opt-in-path 现网零影响据实）/ D3 默认行为 + proto add-only field + 0-dep / 0-network 不变 + honest-defer 边界）；ADR-032 add-only Phase-40 Amendment（兑现 `memory-actor-propagation`，不溯改 D-body D5）+ ADR-038 + ADR-027 add-only Phase-40 Amendment（兑现 `l2-cache-true-lru` + 据实更正真-LRU 假设，不溯改 D-body D5）+ ADR-015 add-only Amendment（proto add-only field）。ADR-014 第三十一次激活。Phase 40 实现 + 发版经用户 ADR-012 授权（本轮规划 + 实现 + 发版无人值守）。

**v0.33.0 推进记录（已落地 2026-06-07，add-only）**：§3.22 全 3 task 合入 master，ADR-045 据 D1-D3 真实 ratify（Proposed → Accepted）：task-40.1（#257，68046c3）memory-actor-propagation → ✅（`PinMemoryRequest` add-only `actor=3` 既有字段号冻结 + `buf generate` 4 不相关 pb.go 还原 + Go `Pin(id,pin,actor)` interface + 三实现 + `grpcclient` 填 Actor + `handleMemoryPin` 读 `X-Actor` + Rust `pin()` 空回落 byte-equiv；TEST-40.1.1 prost wire-tag actor=3 = `[0x1A,0x01,0x78]` + TEST-40.1.2 Rust 透传/空回落 + TEST-40.1.3 Go X-Actor + TEST-40.1.4 grpcclient Actor；认证身份 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`，ADR-022 D2 宽松 body 契约不改）。task-40.2（#258，08e8db6）l2-embedding-cache-true-lru → ✅（`cache.rs` `sqlite_get` 命中仅 `l2_cap > 0` `INSERT OR REPLACE` 原样回写 bump 隐式 rowid → 既有 `sqlite_put` rowid 序驱逐由插入序 FIFO 升访问序 LRU，cap==0 不 bump；TEST-40.2.1 LRU 驱逐最久未用 vs FIFO + TEST-40.2.2 cap 门控 bump + 结果不变；据实更正 Phase 33（ADR-038 A2/D4）真-LRU 假设，命中 bump 0 schema migration 与 Go memstore move-to-front task-33.2 同技法；写放大 + `with_sqlite` 无生产调用点现网零影响据实记）。task-40.3 closeout → ✅（smoke v30[49/49] TestTask403 REAL 模式 `X-Actor`→`pinned_by` 端到端断言 + release docs + ADR-045 per-D ratify + ADR-032 add-only Phase-40 Amendment（memory-actor-propagation 入口透传维度兑现 + 认证身份续延后）+ ADR-038 + ADR-027 add-only Phase-40 Amendment（l2-cache-true-lru 兑现 + 真-LRU 假设据实更正）+ ADR-015 add-only Amendment + roadmap/adapter add-only）。**0 新依赖、复用既有范式**（`set_pinned_with_actor` / `sqlite_put` / `r.Header.Get` / 隐式 rowid，自 task-27.1 / task-33.1 起 backend 0 改动）；默认 pin actor 空回落 byte-equiv + L2 命中 bump 仅有限 cap 生效结果不变 + proto add-only 既有字段号冻结 + 0 schema migration（ADR-004/008/015）。**与 remote embedding/reranker 不同**：两项均无外部付费 API 依赖（proto/Go/Rust wire + 本地 SQLite rowid bump，0 网络），故 TEST-40.1.* / 40.2.* 由每次 CI run 全程守护（无 honest-defer skip）。诚实校正（ADR-013）：pin actor 调用方透传 vs 认证身份 honest-defer；L2 真-LRU 据实更正 Phase 33 假设；其余 marker（vector-dim-feature-enforce / tracestore-multi-workspace-strict / chunk-source-type-filter）据实保持延后不强行扩面（焦点小版本）。**多 agent 对抗审查（4 维度 review × 每 finding 3 独立 skeptic）核实 0 真实缺陷**（2 findings 均关于 L2 命中 bump 读路径写、据真实代码驳回：输出 byte-equivalent + L2 生产不可达 + 写放大已据实记录）。#257/#258 各 CI 14/14 绿。真实 v0.33.0 tag/release 经用户授权 push（ADR-012），tag SHA/digest/tlog post-tag-push 回填（ADR-013 不预填）。

### 3.23 v0.34.0 / Phase 41 — tokenizer-default-on（承 Phase 24/30 检索 tokenizer 血脉，post-v0.33.0 add-only 排期）

**目标**：做出 Phase 30（cjk-true-segmenter, Done / v0.23.0）经 ADR-035 §D3 据「翻默认是产品决策」诚实延后的那个产品决策——把 code/CJK 感知 tokenizer `code_cjk`（task-24.1 / ADR-029：camelCase/snake_case/dotted.path/kebab-case 拆子词 + 保留原 token + CJK bigram，**纯 std、0-dep**）从 **opt-in** 翻为**新建 collection 的生产默认**，使全体用户**默认**获 Phase 24 实测的 recall 增益（+0.0909 over 默认 `TEXT`），而既有 collection 不受影响、并提供 opt-out。grounding 真实状态：(a) 生产索引全走 `IndexSession::open(..)`（`server.rs:141` `CoreService::index` RPC + `jobs/index_session_backend.rs:151`）→ `open_with_tokenizer(.., DEFAULT_TOKENIZER="default")` → 新建 collection 绑 Tantivy 默认 `TEXT`；今天 0 tokenizer env/config 接线。(b) tokenizer 绑定真相源是 `meta.json`——`open_with_tokenizer` 仅 create 时绑传入 tokenizer，**open 既有 collection 走 `Index::open_in_dir` 读回持久化 schema、忽略传入值**；query 侧 schema-driven 对称 → **翻默认对既有 collection 自动安全**（既有 `TEXT` collection 不被静默失效），仅新建 collection 绑 `code_cjk`。(c) 迁移工具 `IndexSession::reindex_with_tokenizer` + schema-driven 对称口径自 Phase 30 已备。**关键诚实定性（ADR-013，本 phase 核心）**：这是项目**首次刻意改默认行为**（新建 collection 倒排词项 `TEXT`→`code_cjk`，**非 byte-equivalent**）——由新 ADR-046 显式承接，三重安全 + 一处实测收益为据：① 既有 collection 不受影响；② `CONTEXTFORGE_TOKENIZER=default` env / `[retrieval] tokenizer` config opt-out 回 legacy `TEXT`（byte-equiv）；③ 既有 collection 升级由用户经 `reindex_with_tokenizer` 主动触发（不自动迁移用户数据）；④ Phase 24 实测 +0.0909 justify。`code_cjk` 纯 std → **0 新依赖**；jieba 真分词 `cjk_segmenter` 仍 feature-gated opt-in（Phase 30 实测 jieba vs bigram delta=+0.0000 → 默认不取重词典 dep）。

**来源 marker**：
- `[SPEC-DEFER:phase-future.tokenizer-default-on]`（ADR-029:54 §Negative/Follow-ups + ADR-035 D3 + phase-24 spec :41/:42 记录——本 phase 兑现默认开启维度，ADR-029/035 母 ADR add-only Phase-41 Amendment 标 fulfilled、不溯改 D-body）。

**候选 task 拆分**：
- **task-41.1** tokenizer-default-on：`core/src/server.rs` add `resolve_tokenizer()` env-resolution（镜像 `resolve_data_dir`/`resolve_vector_backend`：unset/"" → `code_cjk` 翻默认 / `"default"` → opt-out 回 `TEXT` / `"code_cjk"`/`"cjk_segmenter"`(feature) passthrough / unknown/feature-off → stderr WARN + `code_cjk`）+ 生产索引两调用点（`server.rs:141` + `jobs/index_session_backend.rs:151`）改 `open_with_tokenizer(.., &resolve_tokenizer())`；`IndexSession::open`/`DEFAULT_TOKENIZER` 库 API+常量不动（向后兼容库调用方+既有单测）；既有 collection 经 `open_in_dir` 保持持久化 `TEXT`；Phase 24 harness 复测真实 recall delta +0.0909（首次刻意默认变更非 byte-equiv，由 ADR-046 承接，真实数回填不预填）。🟢 / 🟡 本地 real delta。
- **task-41.2** tokenizer-config-bridge：Go `internal/config/config.go` add-only `RetrievalConfig{Tokenizer}` + `[retrieval]` 段 round-trip（镜像 `VectorConfig`/`[vector]`）+ `cmd/contextforge/main.go` `setTokenizerEnv`（镜像 `setVectorEnv`：`[retrieval] tokenizer` 非空且 env 未设 → 导出 `CONTEXTFORGE_TOKENIZER`，env-wins，无段不导出 → Rust 默认 `code_cjk`）接线 doServe/doMCP；tokenizer 非密钥；Rust core 0 toml dep。🟢。
- **task-41.3** v0.34.0 closeout：smoke v30→v31[50/50]（production 默认 `code_cjk` + `CONTEXTFORGE_TOKENIZER=default` opt-out 端到端断言；TestTask413 镜像 TestTask403 无 [37/37]..[49/49] 回归，`bash -n`）+ release docs（tag/run/digest `<backfill>` marker；Upgrade 段记翻默认 + opt-out + 既有 collection 不受影响 + reindex 升级）+ ADR-046 据 D1-D4 ratify + ADR-029 add-only Phase-41 Amendment（默认开启维度兑现）+ ADR-035 add-only Phase-41 Amendment（D3 产品决策兑现）+ ADR-004/008 守线引用 + roadmap §3.23/§4 + adapter + defer marker 更新。🟢。

**ADR**：**ADR-046 tokenizer-default-on**（Proposed，D1 production 默认翻 `code_cjk`（resolve_tokenizer env-resolution + 既有 collection schema-driven 安全 + Phase 24 实测 +0.0909 justify）/ D2 `CONTEXTFORGE_TOKENIZER` env opt-out + Go `[retrieval] tokenizer` config 桥（env-wins，无段默认 code_cjk，Rust 0 toml dep）/ D3 recall delta 复测 + honest-defer 边界（jieba 默认不取 0-dep + Phase 30 delta=0 / 既有 collection 不自动迁移 / 大语料续 SPEC-DEFER）/ D4 首次刻意默认变更由本 ADR 承接 + 0-dep / 0-network + opt-out 保 legacy byte-equiv（ADR-004/008））；ADR-029 add-only Phase-41 Amendment（兑现默认开启维度，不溯改 D-body D5）+ ADR-035 add-only Phase-41 Amendment（兑现 D3 产品决策，不溯改 D-body D5）。ADR-014 第三十二次激活。Phase 41 实现 + 发版须另行 ADR-012 授权（规划稿 Draft/Proposed → 用户「继续实现」授权实现，已落地见下）。

**v0.34.0 推进记录（已落地 2026-06-07，add-only）**：§3.23 全 3 task 合入 master，ADR-046 据 D1-D4 真实 ratify（Proposed → Accepted）：task-41.1（#262，`35bb421`）tokenizer-default-on → ✅（`core/src/server.rs` add `resolve_tokenizer()`（pub fn）+ `parse_tokenizer()`（pub(crate) 纯函数，镜像 `resolve_data_dir`/`resolve_vector_backend`）：unset/""→`code_cjk` 翻默认 / `"default"`→`DEFAULT_TOKENIZER` opt-out 回 legacy `TEXT` / `"code_cjk"`/`"cjk_segmenter"`(feature) passthrough / unknown·feature-off→stderr WARN+`code_cjk` 不静默落 TEXT + 生产索引两调用点 `server.rs:141` `CoreService::index` + `jobs/index_session_backend.rs:151` 改 `open_with_tokenizer(.., &resolve_tokenizer())`；`IndexSession::open`/`DEFAULT_TOKENIZER` 库 API+常量不动；既有 collection 经 `open_in_dir` 读回持久化 schema 自动安全；TEST-41.1.1 env 矩阵 + TEST-41.1.2 生产路径绑定 code_cjk·opt-out TEXT·既有 collection 安全；`cargo test --lib` 222 passed + clippy clean）。task-41.2（#263，`2cead8b`）tokenizer-config-bridge → ✅（Go `internal/config/config.go` add-only `RetrievalConfig{Tokenizer}` + `[retrieval]` 段 round-trip 镜像 `VectorConfig` + `cmd/contextforge/main.go` `setTokenizerEnv` 镜像 `setVectorEnv`（env-wins、无段不导出→core 默认 code_cjk、tokenizer 非密钥）接线 doServe/doMCP；Rust core 0 toml dep；TEST-41.2.1 round-trip + TEST-41.2.2 env-bridge；`go test ./...` 全绿 + vet + gofmt 0 diff）。task-41.3 closeout → ✅（smoke v31[50/50] REAL camel 子词 `runner`(of JobRunner) 经 code_cjk 默认命中（legacy TEXT 会 miss，distinguishing）+ TestTask413 无 [37/37]..[49/49] 回归 + release docs + ADR-046 per-D ratify + ADR-029 add-only Phase-41 Amendment（默认开启维度兑现）+ ADR-035 add-only Phase-41 Amendment（D3 产品决策兑现）+ ADR-015 无关 + roadmap/adapter）。**首次刻意默认行为变更**（新建 collection 倒排词项 `TEXT`→`code_cjk` 非 byte-equiv）由 ADR-046 承接 + 三重安全（既有 collection 经 `open_in_dir` 不受影响 / `CONTEXTFORGE_TOKENIZER=default`·`[retrieval]` opt-out 回 legacy byte-equiv / 不自动迁移用户经 `reindex_with_tokenizer` 主动）；**实测 recall delta +0.1250 recall@5/@10**（default 0.8750 → code_cjk 1.0000 over 当前 16-题 golden，与 ADR-035 Amendment D4 测量 delta(seg−default)=+0.1250 一致；Phase 24 原始 11-题 golden 为 +0.0909——据实记当前数不沿用旧数，ADR-013）。jieba `cjk_segmenter` 默认不取（0-dep baseline + Phase 30 实测 vs bigram delta=+0.0000）。**0 新依赖**（`code_cjk` 纯 std；jieba feature-gated）+ 0 network（ADR-004/008）。#262/#263 各 CI 14/14 绿。真实 v0.34.0 tag/release 经用户授权 push（ADR-012），tag SHA/digest/tlog post-tag-push 回填（ADR-013 不预填）。

### 3.24 v0.35.0 / Phase 42 — chunk-source-type-filter（承 Phase 32 据实延后的 chunk filter 血脉，post-v0.34.0 add-only 排期）

**目标**：把 chunk 检索的 `source_type` 过滤从 Phase 32（task-32.3 / ADR-037）经 grounding 据实记的「documented no-op」落地为**真实过滤**，并据 grounding 诚实校正 `agent_scope` 的归属。背景：`SearchFilters.source_type`/`agent_scope`（`core/src/retriever/mod.rs:137/139`）+ v1 proto `SearchFilters.source_type=1`（`search.proto:13`）自 task-4.2 起就有契约，但 chunks 表无该列、`SearchResult.source_type` 恒 `DEFAULT_SOURCE_TYPE=""`、`agent_scope` 恒空 → Phase 32 据实定为 documented no-op + 开 `[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`。grounding 真实状态（决定方案）：(a) **source_type 可由 file_path 确定性派生**——`indexer/mod.rs:483 lang_hint_from_path` 已有「扩展名 → 语言」纯函数范式，source_type 是其**粗粒度桶**（code/doc/config/other）→ 无须存储、**无须 schema migration**（chunks/files/provenance §5.3 **保持 FROZEN**）；确定性派生 == 存储值。(b) **读路径已就绪**——v1 `server.rs:440-453` 已把 proto `filters.source_type` → `RetrieverFilters.source_type`（只是 retriever no-op）；console `data_plane/search.rs:378` 已把 `source_file_type: h.source_type` 写入响应（只是 `h.source_type` 恒空）→ retriever 真实派生 + 过滤后 v1 gRPC/REST body 立即生效、console 响应立即显示真实值。(c) **agent_scope 是 memory 层概念**——`agent_scope` 真实归属 memory（`memory_items` 0013 / `MemoryListFilter` / `ListMemory` scope / `memstore.go:629-635`），chunks 无 agent 关联、无可派生维度。**关键诚实校正（ADR-013，本 phase 核心）**：`chunk-source-type-filter` 与 `chunk-agent-scope-filter` 经 grounding **不对称**——前者可派生、可真实落地（0 migration）；后者须 ingest-path schema 工程（为 chunks 引入 agent 维度）且价值不明，**本 phase 不伪造**，agent_scope 续 documented no-op、`[SPEC-DEFER:phase-future.chunk-agent-scope-filter]` 据实保持（镜像 Phase 32/34/35 的 grounding 校正手法）。

**来源 marker**：
- `[SPEC-DEFER:phase-future.chunk-source-type-filter]`（task-32.3 / ADR-037 §3.14 排期时新开，本 phase 兑现 — `core/src/retriever/mod.rs:329` + ADR-037）。
- `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（task-32.3 / ADR-037 §3.14，本 phase 据 grounding 据实保持延后，非兑现）。

**候选 task 拆分**：
- **task-42.1** chunk-source-type-derivation-and-filter：`core/src/retriever/mod.rs` add `classify_source_type(file_path) -> &'static str`（扩展名确定性桶 code/doc/config/other，镜像 `indexer::lang_hint_from_path`，纯 std 0-dep）+ 三构造点（`search()` BM25 :466 / `get_chunk` :558 / `search_semantic` :806）`source_type` 由 `DEFAULT_SOURCE_TYPE` 改真实派生 + `search()` BM25 加 source_type post-filter（镜像 `:386` language post-filter，空 filter byte-equiv）+ `agent_scope` 续 documented no-op（窄化 `:321-336` no-op 块仅 agent_scope）；0 schema migration（§5.3 FROZEN）；v1 `server.rs:440-453` 已映射 → 立即生效；据真契约改写 `TEST-32.3.2`。🟢。
- **task-42.2** console-api-source-type-forward：`console_data_plane.proto` `SearchRequest` add-only `repeated string source_type = 9`（既有字段 1-8 号冻结，ADR-015 add-only，buf generate）+ `data_plane/search.rs` 按 `req.source_type` 对 populate 后 hit post-filter（覆盖 BM25/semantic/hybrid 一致，空 → 不过滤 byte-equiv）+ Go `internal/contractv1.SearchRequest` add-only `SourceType []string` + `handleSearch` `?source_type=`（query param + body 并集，镜像 `?semantic`/`?hybrid`）+ grpcclient 映射；console 响应 `source_file_type` 响应侧已就绪（populate 后显示真实值）。🟢。
- **task-42.3** closeout：smoke v31→v32[51/51]（REAL source_type 真实过滤端到端：复用既有 `index-job-real` 全 markdown fixture，`runner`/JobRunner 文档于 .md → source_type=doc → `?source_type=doc` 保留 doc hit / `?source_type=code` 过滤掉它 / 空 filter 返全部，distinguishing）+ TestTask423（no [37/37]..[50/50] 回归）+ release docs + ADR-047 ratify + ADR-037 add-only Phase-42 Amendment（source_type no-op supersede / agent_scope no-op 保持）+ roadmap §3.24/§4 + adapter。🟢。

**ADR**：**ADR-047 chunk-source-type-filter**（Proposed，D1 source_type 由 file_path 确定性派生 0 migration §5.3 FROZEN / D2 v1 retriever 真实过滤 + 三路径 populate 空 filter byte-equiv / D3 console proto add-only source_type=9 + data_plane post-filter + Go `?source_type=` forward / D4 agent_scope 据实 honest-defer memory 层概念续 no-op）；ADR-037 add-only Phase-42 Amendment（source_type no-op 被真实过滤 supersede / agent_scope no-op 据实保持，不溯改 D-body D5）；ADR-015（proto add-only）/ ADR-024 / ADR-044（console 请求侧 forward 范式）/ ADR-004（空 filter byte-equiv + source_type value 填补 v0.1 schema gap）/ ADR-008（0 新依赖）守线。ADR-014 第三十三次激活。Phase 42 实现 + 发版须另行 ADR-012 授权（规划稿 Draft/Proposed → 用户授权后实现 + 发版）。

**v0.35.0 推进记录（已落地 2026-06-07，add-only）**：§3.24 全 3 task 合入 master，ADR-047 据 D1-D4 真实 ratify（Proposed → Accepted）：task-42.1（#267，`e290649`）chunk-source-type-derivation-and-filter → ✅（`core/src/retriever/mod.rs` add `classify_source_type(file_path) -> &'static str` 扩展名确定性桶 code/doc/config/other 镜像 `indexer::lang_hint_from_path` 纯 std + 三构造点（`search()` BM25 / `get_chunk` / `assemble_vector_result`）source_type 真实派生 + `search()` BM25 source_type post-filter（SQLite JOIN 后，镜像 language post-filter，空 → byte-equiv）+ 窄化 no-op 块仅 agent_scope + **删除孤儿 `DEFAULT_SOURCE_TYPE`**（grounding 校正：三构造点改派生后成孤儿 `-D warnings` 卡红，规划稿「不删常量」plan 假设被编译期 grounding 覆盖）+ populate 连带更新 `test_4_2_1`/`test_6_2_e1`/`server.rs` wire/`phase4_smoke`/`phase6_smoke` 旧 schema-gap "" 断言→「有效桶」；TEST-42.1.1 classify 矩阵 + TEST-42.1.2 真实过滤 + populate + agent_scope no-op；0 schema migration（§5.3 FROZEN））。task-42.2（#268，`5f88604`）console-api-source-type-forward → ✅（`console_data_plane.proto` `SearchRequest` add-only `repeated string source_type = 9`（既有字段 1-8 号冻结，`buf generate proto`）+ `data_plane/search.rs` hits 装配后按 `req.source_type` post-filter（覆盖 BM25/semantic/hybrid 一致）+ Go `contractv1.SearchRequest.SourceType` + `handleSearch` `?source_type=` query/body 并集 + grpcclient → pb；grounding 校正：`buf generate proto`（module 根 proto/buf.yaml）非裸 buf generate / buf 4 不相关 pb.go 据实还原仅留 console_data_plane.pb.go / 既有 PbSearchRequest 字面量补 source_type；TEST-42.2.1 prost wire-tag field 9（0x4A）+ TEST-42.2.2 handleSearch 并集 + grpcclient 转发 + data_plane post-filter）。task-42.3 closeout → ✅（smoke v32[51/51] REAL：复用 `index-job-real` 全 markdown fixture（`runner`/JobRunner 文档于 .md → source_type=doc）→ `?source_type=doc` 保留 JobRunner doc hit / `?source_type=code` 过滤掉它 distinguishing（pre-tag 多 agent 对抗审查校正了原 code/doc 方向写反，no-op 会两者皆返回）+ TestTask423 无 [37/37]..[50/50] 回归 + release docs + ADR-047 per-D ratify + ADR-037 add-only Phase-42 Amendment（source_type no-op superseded / agent_scope no-op reaffirmed）+ ADR-015/024/044/004/008 守线 + roadmap/adapter）。**关键诚实校正（ADR-013）**：`chunk-source-type-filter` 与 `chunk-agent-scope-filter` 经 grounding 不对称——source_type 可由 file_path 确定性派生（0 migration）真实落地、ADR-037 source_type no-op superseded；`agent_scope` 是 memory 层概念 chunks 无 agent 维度 → 续 documented no-op + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]` 据实保持（不伪造）。**新增 backlog 条目（add-only）**：`chunk-importer-source-type-tagging`（importer 显式 source_type 打标，超 file_path 派生粗粒度桶，须 §5.3 解冻加 chunks 列）/ `semantic-path-source-type-filter`（v1 semantic 路径 retriever-内过滤，本 phase v1 BM25 内过滤镜像 language scope，console 经 data_plane post-filter 覆盖 semantic/hybrid）。**0 新 dep（`classify_source_type` 纯 std）+ 0 network + 0 schema migration（§5.3 FROZEN）+ 空 filter byte-equiv**；core lib 225 全绿 + go test + clippy + go vet + gofmt clean。真实 v0.35.0 tag/run/digest/tlog 经用户授权 push（ADR-012），post-tag-push 回填（ADR-013 不预填）。

### 3.25 v0.36.0 / Phase 43 — governance-debt-cleanup-4（承 Phase 33 task-33.3 indexing-replay-e2e 血脉，post-v0.35.0 add-only 排期）

**目标**：第四轮治理债清扫，**单聚焦 `indexing-replay-e2e` 拼接缺口**——承 Phase 33 task-33.3（ADR-038 D3，indexing event 持久化 + replay mapper）血脉的"最后一公里"。背景：Phase 33 交付了 indexing event **持久化**（add-only migration 0019 + `SqliteIndexingEventStore` + 4 emit 点）+ **replay mapper**（`indexing_rows_to_pb_events`，`events.rs:438`，`test_33_3_2` 守护），但 mapper **从未在 live subscribe 路径被调用**——4 个拼接缺口（grounding 已亲自核实）：(1) `list(limit)` 缺 since_ts 参数；(2) `DataPlaneStores`（`mod.rs:43-74`）无 `indexing_event_store` 字段；(3) `serve_full`（`server.rs:788-798`）`DataPlaneStores::full()` 未传入已构造 store（store 在 `:756` 局部构造传给了 `IndexSessionBackend` 写路径，但没 clone 进 DataPlaneStores 读路径）；(4) `EventsServer::subscribe`（`events.rs:241-250`）replay splice 只接了 memory audit replay，漏了 indexing。结果：`since_ts > 0` 的订阅者能收到 missed 的 memory 事件，但**收不到** missed 的 indexing 事件。**关键诚实定性（ADR-013，本 phase 核心）**：本 phase 交付 splice **拼接**（接进 live subscribe 路径 + since_ts 时序对齐 audit + 默认 byte-equiv），🟢 纯本地单测守护时序 + 拼接 + 退化 byte-equiv；live daemon restart-then-replay 端到端 e2e（真起进程 + 跨 restart 双窗口断言）须 running daemon（须 console 跨进程）→ 🟡 honest-defer `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]` 不预填；memory-actor-all-rpc（grounding 显示 Deprecate/SoftDelete 需 7 层改动 + 新 schema migration / HardDelete 须 audit 层重设计——非小债）据实 honest-defer 留独立 phase，不在本 phase 强行扩面（roadmap §3.17/§3.22 "据实排小不凑数"）。

**来源 marker**：
- `[SPEC-DEFER:phase-future.indexing-replay-e2e]`（ADR-038 D3，Phase 33 task-33.3 标 mapper 🟢 已达 / e2e 🟡 未跑——本 phase 兑现 splice 维度，mapper 接进 live subscribe + since_ts 时序）。

**候选 task 拆分**：
- **task-43.1** indexing-replay-splice：`core/src/data_plane/indexing_events.rs` add `list_since(limit, since_ts)`（since_ts>0 时 `WHERE ts_unix >= ?` 镜像 `replay_events_from_audit`，since_ts<=0 不过滤；既有 `list(limit)` 不动）+ `DataPlaneStores`（`mod.rs`）add `indexing_event_store: Option<Arc<SqliteIndexingEventStore>>` 字段 + `full()` 加第 10 参数（既有 constructor 补 None byte-equiv）+ `server.rs` serve_full 传入 `Some(indexing_event_store.clone())`（store 已在 `:756` 构造）+ `events.rs` `subscribe` replay 段 splice indexing replay（since_ts>0 时 list_since + `indexing_rows_to_pb_events`，audit replay 后、live forward 前；store None / lock 失败 `unwrap_or_default` 空）；TEST-43.1.1 list_since 时序过滤 + TEST-43.1.2 subscribe splice 时序 indexing→audit→live + since_ts<=0 byte-equiv + store=None 退化。🟢。0 新 dep / 0 schema migration（复用 0019）/ 0 proto 改动。
- **task-43.3** closeout：smoke v32→v33[52/52]（indexing replay splice 可达则断言 since_ts>0 订阅者收到 evt-idx-* 事件序列、否则 doc/status 归因 unit TEST-43.1.2）+ TestTask433（no [37/37]..[51/51] 回归）+ release docs + ADR-048 ratify + ADR-038 add-only Phase-43 Amendment（indexing-replay-e2e splice 维度兑现，live daemon e2e 续延后）+ roadmap §3.25/§4 + adapter。🟢。

**ADR**：**ADR-048 indexing-replay-splice**（Proposed，D1 list_since 时序过滤镜像 audit / D2 DataPlaneStores 字段 + full() 加参 + serve_full 接线（store 已在 clone 读路径）/ D3 subscribe splice indexing replay（audit 后、live 前，event_id 命名空间独立 dedup）/ D4 默认 byte-equiv（since_ts<=0 / store=None）+ live daemon e2e honest-defer + 0 dep/0 migration/0 proto）；ADR-038 add-only Phase-43 Amendment（indexing-replay-e2e splice 维度兑现，live daemon e2e 续延后，不溯改 D-body D5）；ADR-031（replay 范式源 task-26.2 引用）/ ADR-021（audit replay splice 镜像源引用）/ ADR-004（默认 byte-equiv + 既有契约不变）/ ADR-008（0 新依赖）守线。ADR-014 第三十四次激活。Phase 43 实现 + 发版经用户授权 ADR-012（规划稿 Draft/Proposed → 用户授权后实现 + 发版）。

**v0.36.0 推进记录（已落地 2026-07-01，add-only）**：§3.25 全 task 合入 master，ADR-048 据 D1-D4 真实 ratify（Proposed → Accepted；D4 live daemon e2e 🟡 honest-defer）：task-43.1（PR #276，`2c98cc2`）indexing-replay-splice + task-43.3 closeout。🟢 纯本地 + 0 dep/0 migration（复用 0019）/0 proto + 默认 byte-equiv；live daemon e2e 🟡 defer；memory-actor-all-rpc 据实延后留独立 phase。真实 v0.36.0 tag/run/digest/tlog 经用户授权 push（ADR-012），post-tag-push 回填（ADR-013 不预填）。

### 3.26 v0.37.0 / Phase 44 — memory-unpin-actor-propagation（承 Phase 40 task-40.1 pin actor 血脉，post-v0.36.0 add-only 排期）

**目标**：闭环 pin/unpin actor 透传不对称。Phase 40 task-40.1（ADR-045 D1）给 `pin` 加了 actor 透传（X-Actor → PinMemoryRequest.actor → store pinned_by），但 `unpin` 漏了（unpin handler `memory.rs:298` 硬编码 "console-api"）。**grounding 发现真实价值在 audit/event**（ADR-013，改变范围设计）：`set_pinned_with_actor(id, false, actor)` 在 pinned=false 时丢弃 actor（store.rs:192-196 清 pinned_by）→ 单纯透传到 store 是"空透传"（违 ADR-013）；真实落点是 `emit_audit_and_event`（memory.rs:52 不携 actor / :59 硬编码 source）——让 actor 进入 audit log + event stream，console 部署在 auth 代理后时 unpin 可归因。本 phase **完整闭环**：unpin handler 透传 + `emit_audit_and_event` 加 actor 参数（audit/event source 归因）+ pin 顺带闭环（add-only byte-equiv）+ Go 透传链。

**来源 marker**：`[SPEC-DEFER:phase-future.memory-actor-all-rpc]`（roadmap 行 556 Phase 40 closeout 新增 backlog "其它 memory RPC 的 actor 透传"——本 phase 兑现 Unpin 子项 + emit_audit_and_event 共用基础，Deprecate/SoftDelete/HardDelete 续延后）。

**候选 task 拆分**：
- **task-44.1** unpin-actor-propagation：proto `UnpinMemoryRequest` add-only `actor=2`（既有 memory_id=1 冻结，ADR-015）+ buf generate + Rust unpin handler 透传 actor（镜像 pin :231-235）+ `emit_audit_and_event` 加 `actor: &str` 参数（audit source / build_memory_event source 用 actor）+ pin handler 顺带传 actor 闭环 + deprecate/softdelete/harddelete 传固定值 byte-equiv + Go 4 处透传（types/grpcclient/handlers X-Actor/memstore）+ degraded stub + TEST-44.1.1（unpin actor 进 audit source）/.2（pin 顺带闭环）/.3（空 actor byte-equiv）/.4（Go X-Actor）。🟢。0 dep/0 migration/proto add-only。
- **task-44.3** closeout：smoke v33→v34[53/53]（unpin X-Actor 端到端 / 不可达诚实归因 unit）+ TestTask443 + release docs + ADR-049 ratify + ADR-032/045 add-only Phase-44 Amendment + roadmap/adapter。🟢。

**ADR**：**ADR-049 memory-unpin-actor-propagation**（Proposed，D1 proto add-only + Rust unpin 透传 / D2 emit_audit_and_event actor 参数 pin/unpin 闭环 / D3 Go 透传链 / D4 默认 byte-equiv + 认证身份 + 其余 3 RPC honest-defer）；ADR-032+045 add-only Phase-44 Amendment（unpin actor 透传维度兑现，不溯改 D-body D5）；ADR-021（emit_audit_and_event 镜像源）/ ADR-022 D2（lenient body 保持）/ ADR-015（proto add-only）/ ADR-004（空 actor byte-equiv）/ ADR-008（0 新依赖）守线。ADR-014 第三十五次激活。Phase 44 实现 + 发版经用户全权授权 ADR-012（规划稿 Draft/Proposed → 实现 + 发版一条龙）。

**v0.37.0 推进记录（已落地 2026-07-01，add-only）**：§3.26 全 task 合入 master，ADR-049 据 D1-D4 真实 ratify（Proposed → Accepted；D4 认证身份/其余 3 RPC 🔴 honest-defer）：
- **task-44.1**（PR #280，master @ `8f6e94f`）unpin-actor-propagation → ✅：`proto/.../console_data_plane.proto` `UnpinMemoryRequest` add-only `actor=2`（既有 memory_id=1 冻结）+ buf generate；`core/src/data_plane/memory.rs` `emit_audit_and_event` 加 `actor: &str` 参数（audit source 非空用 actor / 空回落 "console-api"）+ `build_memory_event` 加 actor（event source 非空用 actor / 空回落 "contextforge-core"）+ unpin handler 透传（`let actor = if req.actor.is_empty() { "console-api" } else { req.actor.as_str() };` 镜像 pin）+ `set_pinned_with_actor(id, false, actor)`（store 丢弃，接口对称）+ `emit_audit_and_event(MemoryUnpin, id, &req.actor)` + pin handler 顺带传 `&req.actor`（消除残余不对称）+ deprecate/softdelete/harddelete 传 `""` byte-equiv + TEST-44.1.1（unpin actor 进 event source "bob"）+ TEST-44.1.2（pin 顺带闭环 "alice"）+ TEST-44.1.3（空 actor byte-equiv "contextforge-core"）；Go 4 处透传（types `Unpin(id, actor)` + grpcclient `pb.UnpinMemoryRequest{MemoryId, Actor}` + handlers `handleMemoryUnpin` 读 `X-Actor`（:559 范式）+ memstore `Unpin(id, _actor)` + degraded fallback 签名同步 + gofmt 对齐 + memstore_test）+ `unpin_actor_test.go` TEST-44.1.4（源码 grep）。lib 229→232 全绿 + clippy 0 warning + go test 全绿 + workspace 全绿。
- **task-44.3** closeout → ✅：smoke v33→v34 `[53/53]`（unpin X-Actor 端到端 / 不可达诚实归因 unit TEST-44.1.1/.2）+ `TestTask443`（no-regression [37/37]..[52/52]）+ v0.37.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ ADR-049 per-D ratify（D1-D3 unit 🟢 / D4 默认 byte-equiv 🟢 + 认证身份/其余 3 RPC 🔴 honest-defer）+ ADR-032/045 add-only Phase-44 Amendment（unpin actor 透传 + audit/event source 归因维度兑现，不溯改 D-body D5）+ ADR-021/022/015/004/008/013/014 守线引用 + roadmap/adapter。

**关键诚实定性（ADR-013）**：本 phase 交付**调用方透传**（audit/event source 归因真实调用方），🟢 纯本地单测守护（lib 229→232）；认证身份（X-Actor → 已认证 auth subject）须 console-api 鉴权层 → 🔴 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；deprecate/softdelete/harddelete actor 透传须 7 层+新 migration / audit 重设计 → 🔴 honest-defer `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`（本 phase 仅做 emit_audit_and_event actor 参数共用基础，这 3 RPC 未来顺带受益）。**0 新 dep + 0 network + 0 schema migration + proto add-only（actor=2）+ 默认 byte-equiv**；core lib 232 全绿 + go test（含 TestTask441 + TestTask443）+ clippy + go vet clean + CI spec-lint 全绿。真实 v0.37.0 tag/run/digest/tlog 经用户全权授权 push（ADR-012），post-tag-push 回填（ADR-013 不预填）。

---

## 4. 长尾 backlog（尚未归入上述版本，留 vNext）

下列 `[SPEC-DEFER]` 标记承诺度低 / 范围小 / 依赖未明，暂不排入 v0.13–v0.16，待对应版本启动时据数据决定纳入或继续延后：

- **向量 backend 细化**：`multi-backend-production`、`qdrant-server-lifecycle`（**§3.18 Phase 36 已兑现关闭**——真实 live KNN 召回 recall@10=1.0000 + CI service-container 永久守护，CI run 26961084355）、`qdrant-deployment-topology`、`qdrant-semantic-golden-recall`（vs golden 语义标签需真实 embedding model）、`vector-large-corpus-perf`（百万级 qdrant 性能基准）、`lancedb-index-tuning`、`lancedb-schema-compaction`、`lancedb-build-prereq-ci`、`vector-dim-feature-enforce`（add-only，承 §3.16 Phase 34——声明 dim 的 feature backend qdrant/lancedb/sqlite-vec 真实 dim 强制，须 feature build）。
- **eval**：`rust-native-eval-runner`（现 Go runner，承 `task-14.1`）、`eval-dataset-validation`、`case-results-subtable`、`semantic-golden-dataset`（语义近邻标注扩充）。
- **检索 tokenizer**：`cjk-and-code-tokenizer`（CJK + 代码符号分词，`phase-19` §2）。
- **trace / events**：`tracestore-sqlite-vacuum`、`tracestore-fts`、`tracestore-multi-workspace-strict`、`events-sse-push`、`events-replay-from-audit`、`events-drain-timeout-config`、`event-bus-partition`、`event-bus-capacity`、`memstore-event-emit`、`observability-metrics-facility`（结构化计数器/metrics facility，core 现无、stderr surfacing 是忠实 scope，承 §3.17 Phase 35，🟡）、`memstore-degraded-observability-warn`（MemMemoryStore nil-sink 一次性降级告警，若 sink optional-by-design 则 honest non-issue，承 §3.17 Phase 35，🟡）。
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
>
> **v0.26.0 / Phase 33 推进记录（已落地 2026-06-03，add-only）**：§3.15 全 4 task 合入 master，ADR-038 据 D1-D5 真实 ratify（Proposed → Accepted；D3 e2e + D4 dropped-nits/datadir 🟡 honest-defer）：
> - **task-33.1**（PR #218，squash 3ce290a）L2 cache rowid-FIFO 有界 → ✅：`core/src/embedding/cache.rs` `sqlite_put` 写后 row-count cap + `DELETE ... WHERE rowid NOT IN (... ORDER BY rowid DESC LIMIT cap)` rowid-FIFO；`DEFAULT_L2_EMBEDDING_CACHE_CAP=50_000` + add-only `with_sqlite_capacity`；0 新 dep / 0 migration。`test_33_1_1`/`_33_1_2` pass。opt-in-path caveat（`with_sqlite` 无生产调用点，纵深防御非 live leak）+ true-LRU `[SPEC-DEFER:phase-future.l2-cache-true-lru]`。
> - **task-33.2**（PR #219，squash 7b92f22）memstore access-order LRU + hard-delete invariant → ✅：`memstore.go` chunk/trace 缓存读命中 + 覆写均 move-to-front（`moveToMRU`），`TestMemStore_CacheEviction_FIFO`→`_LRU` + 新 `_LRU_Trace`；`test_33_2_3` schema 内省断言 `memory_id` 仅 `memory_items` + hard_delete 后 get=None。cascade `[SPEC-DEFER:phase-future.memory-harddelete-cascade]`（非问题）；handleMemoryPin lenient（ADR-022 D2）据实不改。
> - **task-33.3**（PR #220，squash 823beca）observability indexing replay + trace isolation + drain verify-only → ✅/🟡：add-only migration 0019 + `SqliteIndexingEventStore` + 4 emit 点持久写 + `indexing_rows_to_pb_events` mapper（真实 job_id/processed/total，`evt-idx-{id}`）；`GetSearchTraceRequest`/`ListQueriesRequest` add-only `workspace_id=2` + search_persist/TraceStore/handler `WHERE workspace_id`（空=aggregate-all byte-equiv）；drain-timeout verify-only 引证 `TestDrainTimeoutFromEnv`。`test_33_3_1`..`_4` pass；indexing-replay-e2e `[SPEC-DEFER:phase-future.indexing-replay-e2e]` + tracestore isolation e2e `[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]` 🟡。
> - **task-33.4** closeout → ✅：export `--timeout` add-only flag（默认 60s byte-equiv，`TestParseExportOpts_Timeout`）+ smoke v23 step [42/42]（TestTask334）+ release docs + ADR-038 per-D ratify + ADR-031/027 add-only Amendment（Phase 33）+ roadmap/adapter add-only。dropped-nits 诚实：`%v→%w` non-bug / tracestore-fts already-fixed / datadir env→Options `[SPEC-DEFER:phase-future.daemon-options-datadir]`。
>
> 全 phase 真实验证：`cargo test --workspace` lib 207 + 全 integration pass；`go test ./...` 全过（含 TestTask334）；`cargo clippy --workspace --all-targets -D warnings` 0 warning；`bash -n scripts/console_smoke.sh` exit 0；`spec_drift_lint --touched origin/master` 0 unannotated hits。默认构建 0 新 dep + 0 network + 既有契约（proto add-only `workspace_id` + migration add-only 0019 + export add-only `--timeout`）不变（ADR-004/008）。真实 v0.26.0 tag/release 经用户授权（ADR-012）。

> **v0.27.0 / Phase 34 排期更新（规划中 2026-06-03，add-only，不删上方历史条目）**：§3.16 把 §3.14 排期时新开的两条 grounded backlog `[SPEC-DEFER:phase-future.vector-dim-auto-negotiation]` + `[SPEC-DEFER:phase-future.vector-backend-config-file]` 排入 **v0.27.0 / Phase 34 — vector-config-completeness**（task-34.1 vector-dim-auto-negotiation：`factory.rs` `negotiate_vector_dim` + `VectorBackend::expected_dim` DEFAULT None，默认 BruteForce dim-agnostic no-op honest-caveat、feature backend 真实强制续 `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]` / task-34.2 vector-backend-config-file：Go `config.toml` `[vector]` 段 → `setVectorEnv` 跨进程 env-bridge（仿 `CONTEXTFORGE_DATA_DIR`，env-wins、无段=byte-equiv，Rust 0-dep 保留）/ task-34.3 closeout）。这是一个**刻意小**版本——Phase 31/33 两轮治理债清扫后绿区 backlog 已薄，据实排小版本不凑数（ADR-013）。**grounding 诚实校正（add-only，不删上方条目）**：`get_source_chunk` workspace 隔离经核**已实存**（`core/src/data_plane/search.rs:421-423` 自 task-12.2 起按 `req.workspace_id` scope candidates，空=aggregate-all 兼容）——survey 高估为 gap，Phase 34 仅 verify-only 不变式测试记录已存在隔离，不新增代码（task-34.3，ADR-039 D3 记此校正）。**新增 backlog（add-only）**：`vector-dim-feature-enforce`（声明 dim 的 feature backend 真实 dim 强制，须 feature build，🟡）。真实数值 / 受阻维度真实跑出后回填（ADR-013，不预填）。ADR-039 Proposed。
>
> **v0.27.0 / Phase 34 推进记录（已落地 2026-06-03，add-only）**：§3.16 全 3 task 合入 master，ADR-039 据 D1-D4 真实 ratify（Proposed → Accepted；feature dim-enforce 🟡 honest-defer）：
> - **task-34.1**（PR #224，squash `fed7a90`）vector-dim-auto-negotiation → ✅：`factory.rs` `let _ = dim;` 替为 `negotiate_vector_dim(dim, backend.expected_dim())?`（纯函数镜像 `embedding::factory::negotiate_dim`，复用既有 `VectorError::DimMismatch`）+ `VectorBackend::expected_dim()` add-only 默认 `None`；默认 BruteForce dim-agnostic 任意 dim byte-equiv；TEST-34.1.1/.2 绿（lib 207→209）。feature backend live dim-enforce 续 `[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`。
> - **task-34.2**（PR #225，squash `a4ae446`）vector-backend-config-file → ✅：Go `config.go` add-only `[vector]` 段 + `setVectorEnv` 跨进程 env-bridge（仿 `setDataDirEnv`，env-wins、无段=byte-equiv），接线 doServe + doMCP；Rust core 0 toml dep 保留；TEST-34.2.1/.2 绿。
> - **task-34.3** closeout → ✅：`get_source_chunk` workspace 隔离 verify-only 守护测试（TEST-34.3.1，grounding 校正已实存自 task-12.2）+ smoke v24 [43/43]（TestTask343）+ release docs + ADR-039 ratify + ADR-037 add-only Phase 34 Amendment + roadmap/adapter。
>
> 全 phase 真实验证：`cargo test -p contextforge-core --lib` 209 + `go test ./...` 全过（含 TestTask343）+ `cargo clippy --workspace --all-targets -D warnings` 0 warning + `bash -n scripts/console_smoke.sh` exit 0 + `spec_drift_lint --touched origin/master` 0 unannotated。默认构建 0 新 dep + 0 network + 既有契约（Rust 0 toml dep / `expected_dim` add-only 默认方法 / `[vector]` add-only 段）不变（ADR-004/008）。真实 v0.27.0 tag/release 经用户授权（ADR-012）。

> **v0.28.0 / Phase 35 排期更新（规划中 2026-06-04，add-only，不删上方历史条目）**：§3.17 把「热路径静默错误显式化」排入 **v0.28.0 / Phase 35 — observability-hardening**（task-35.1 rust-silent-failure-surfacing：`index_session_backend.rs:201` store.append + `retriever/mod.rs:415` desync-skip 经 `eprintln!` WARN 镜像 `search.rs:109`、best-effort 保持 / task-35.2 go-silent-failure-surfacing：`setVectorEnv` config.Load/Setenv 经 `fmt.Fprintf(os.Stderr)` 镜像 `daemon/rest.go:110`、stderr-capture RED→GREEN、`memstore.go:579` nil-sink 🟡 impl-grounding / task-35.3 closeout）。这是一个**刻意小**版本——承 Phase 31/33 治理债血脉、第三轮债清理边际递减，据实排小不凑数（ADR-013，honest over padding）；backlog grounding 实测 v0.27.0 后 GitHub 0 issue/0 PR、高价值项全 🔴 外部受阻 → 绿区焦点小版本。**grounding 诚实校正（add-only，不删上方条目）**：survey 7 候选 → 3-4 真静默，DROP/LEAVE 4 处（`search.rs:109` 已显式化+core 无 metrics facility / `mcpadapter/server.go:298` task-31.3 已显式化 / `mcpadapter/allowlist.go:31` 有意 POSIX-only 平台 caveat / `index_session_backend.rs:193` eb.send 有意 no-subscribers）记 grounding 校正不改代码（ADR-040 D3）。**新增 backlog（add-only）**：`observability-metrics-facility`（结构化计数器，core 现无 metrics facility，🟡）/ `memstore-degraded-observability-warn`（若 grounding 显 MemMemoryStore sink optional-by-design 则 honest non-issue，🟡）。真实数值 / 受阻维度真实跑出后回填（ADR-013，不预填）。ADR-040 Proposed。

> **v0.28.0 / Phase 35 推进记录（已落地 2026-06-04，add-only）**：§3.17 全 3 task 合入 master，ADR-040 据 D1-D4 真实 ratify（Proposed → Accepted；memstore nil-sink honest non-issue grounding 校正）：
> - **task-35.1**（PR #229，squash `9a57647`）rust-silent-failure-surfacing → ✅：`index_session_backend.rs` **4 处** `store.append`（progress/index-error/commit-error/cancelled，grounding 发现 4 处非 1）`let _ =` → `if let Err(persist_err) { eprintln! }`（best-effort 不阻断 indexing）+ `retriever/mod.rs:415` `Err(_)=>continue` → `Err(e) => { eprintln!; continue }`（skip 保留）；`eb.send` 保留 as-is（no-subscribers intentional）；store 具体类型无 trait → TEST-35.1.1 改 behavior-lock+inspection（不引 trait scope creep），retriever `:373` surgical 留 as-is；TEST-35.1.1/.2 绿（lib 209→212）。
> - **task-35.2**（PR #230，squash `69fc367`）go-silent-failure-surfacing → ✅：`setVectorEnv` `config.Load`（`errors.Is(os.ErrNotExist)` 守护 missing 静默/malformed 报警）+ `os.Setenv` 失败 → `fmt.Fprintf(os.Stderr)` 镜像 `daemon/rest.go:110`，best-effort 保留；**memstore nil-sink grounding 校正=honest non-issue DROP**（`NewMemMemoryStore` 唯一生产调用 `console_api_serve.go:109` 紧随无条件 `SetEventSink`:112，`memstore.go` 0 改动）；TEST-35.2.1 stderr-capture 真 RED→GREEN（malformed→WARN/missing→no/valid→no）。
> - **task-35.3** closeout → ✅：7→3-4 grounding 校正如实记录（4 处 DROP/LEAVE 不改代码，不引新 metrics facility）+ smoke v25 [44/44]（TestTask353）+ release docs + ADR-040 ratify + ADR-031 add-only Phase 35 Amendment + roadmap §3.17/§4 + adapter。
>
> 全 phase 真实验证：`cargo test -p contextforge-core --lib` 212 + `go test ./...` 全过（含 TestSetVectorEnv_LoadErrorSurfacing + TestTask353）+ `cargo clippy --workspace --all-targets -D warnings` 0 warning + `bash -n scripts/console_smoke.sh` exit 0 + `spec_drift_lint --touched origin/master` 0 unannotated。默认构建 0 新 dep + 0 network + 既有契约（observability-only，best-effort 不转 fail-fast，不引 log/tracing/metrics facility）不变（ADR-004/008）。真实 v0.28.0 tag/release 经用户授权（AskUserQuestion 2026-06-04，ADR-012）。

> **v0.29.0 后 defer-marker 复扫刷新（2026-06-05，add-only，不删上方历史条目）**：应行 361 约定对 `rg 'SPEC-(DEFER|OWNER):phase-future'` 复扫，与本文件 §3.x 各 phase **已验证推进记录**交叉比对，确认 §4 上方清单（行 354-359）中下列 backlog 条目**实已在后续 phase 兑现关闭**（marker 文本当时未同步更新 → 本次据实刷新）。源码 / 已闭 spec 内对应 `[SPEC-DEFER]` 注释按 ADR-014 D5 保留为历史快照不溯改；唯一例外是 `core/src/retriever/vector/qdrant.rs::open()` 一处现在时声明「CI 无 live server」已为假，随本次据实更新。
>
> | marker | 兑现处（in-doc 证据） |
> |---|---|
> | `eval-dataset-validation` / `semantic-golden-dataset` | v0.17.0 / Phase 24 task-24.2（见行 365） |
> | `cjk-and-code-tokenizer` | v0.17.0 / Phase 24 task-24.1（见行 365；**非** §3.12 的 `cjk-true-segmenter`，后者另列） |
> | `tracestore-fts` / `tracestore-sqlite-vacuum` | v0.19.0 / Phase 26 task-26.1（`search_fts` + VACUUM + migration 0016，见行 186） |
> | `events-sse-push` / `events-replay-from-audit` | v0.19.0 / Phase 26 task-26.2（见行 187） |
> | `events-drain-timeout-config` / `event-bus-partition` / `event-bus-capacity` | v0.19.0 / Phase 26（见行 371 诚实校正） |
> | `memory-pin-actor` / `memory-pinned-at-timestamp` / `is-pinned-backfill-from-audit` | v0.20.0 / Phase 27 task-27.1（见行 201） |
> | `memory-pin-unpin-split` / `hard-delete-policy` | v0.20.0 / Phase 27 task-27.2（见行 202） |
> | `memstore-event-emit` | v0.24.0 / Phase 31 task-31.1（见行 265） |
> | `case-results-subtable` | v0.24.0 / Phase 31 task-31.3（migration 0018，见行 267） |
> | `cache-lru` / `cache-cap-configurable` | v0.24.0 / Phase 31 task-31.2（见行 266） |
> | `compose-resource-limits` / `compose-tls-termination` | v0.24.0 / Phase 31 task-31.2（compose mem/cpu + 可选 TLS proxy；真实 cert 自动签发续延后，见行 266） |
> | `qdrant-server-lifecycle` | v0.29.0 / Phase 36 task-36.1/36.2（已见行 354 inline 标注；本次补 `qdrant.rs::open()` 源码注释据实更新） |
>
> **非问题（grounding 校正，非兑现亦非债，据实不实现）**：`handle-memory-pin-strict-body`（ADR-022 D2 蓄意 lenient，见行 295/387）、`memstore-degraded-observability-warn`（Phase 35 校正为 honest non-issue DROP，见行 406）、`memory-harddelete-cascade`（无可级联表，仅不变式守护，见行 387）。
>
> **仍真延后（承诺度低 / 需外部前置，本次不动其承诺度）**：`multi-backend-production`、`qdrant-deployment-topology`、`vector-large-corpus-perf`、`lancedb-index-tuning` / `lancedb-schema-compaction` / `lancedb-build-prereq-ci`、`vector-dim-feature-enforce`、`rust-native-eval-runner`（无 consumer）、`tracestore-multi-workspace-strict`（SQL 级隔离已交付 Phase 33 task-33.3，console e2e 续延后 🟡）、`indexing-replay-e2e`、`observability-metrics-facility`、`daemon-options-datadir`、`l2-cache-true-lru`、`sqlite-vec-inprocess-matrix`、`embed-remote-probe` / `embedding-provider-remote`（骨架已 Phase 22 落地，真实联调须 API key）、`hybrid-scoring` / `reranker-real-quality`（管道已落地，真实大语料质量曲线须语料）、`vector-incremental-index`、`hnsw-graph-persistence`（语义检索按需内存索引路径）、`chunk-source-type-filter` / `chunk-agent-scope-filter`、`multi-arch-image`。

> **v0.30.0 / Phase 37 排期更新（规划中 2026-06-06，add-only，不删上方历史条目）**：§3.19 把 ADR-027 母 ADR 的 `[SPEC-DEFER:phase-future.embedding-provider-remote]`（真实远程 embedding 端点联调 + 实测语义召回）排入 **v0.30.0 / Phase 37 — embedding-provider-remote-live**（task-37.1 env-gated live recall harness：real 模型 vs deterministic 基线同标注集 recall@1/@3 / task-37.2 Go `[remote]` Model add-only + `setRemoteEnv` env-bridge 兑现 `factory.rs:52` config plumbing follow-up / task-37.3 closeout）。de-risk 已由主 agent 本机真实证明（SiliconFlow + `Qwen/Qwen3-Embedding-8B` round-trip + Windows MSVC `--features embedding-remote` 编译跑通；本机实测 remote recall@3=1.0000 vs deterministic 0.0667）。上方「仍真延后」段所列 `embedding-provider-remote` 经本 phase **真实联调 + 实测召回兑现**（task-37.3 closeout 经 ADR-027 add-only Phase-37 Amendment 标 fulfilled，不溯改 D-body）。**新增 backlog 条目（add-only）**：`embedding-remote-ci-credential`（CI 跑 live remote 召回——remote 是付费外部 API、无免费 service container，与 qdrant 诚实差异，召回由本机已认证 run 实测）/ `embedding-large-corpus-recall`（大语料 / 大基准语义质量，超小型手工标注集）/ `embedding-multi-provider-live`（多 remote provider live 矩阵：Cohere / 其它 OpenAI-compatible）/ `embedding-remote-reranker-live`（remote reranker cross-encoder over HTTP live 联调）/ `embedding-remote-health-probe`（远程探针命中 / 健康度 live 守护）。真实数值 / 受阻维度真实跑出后回填（ADR-013，不预填）。ADR-042 Proposed。
>
> **v0.31.0 / Phase 38 排期更新（规划中 2026-06-06，add-only，不删上方历史条目）**：§3.20 把 §3.19 排期时新开的 backlog `[SPEC-DEFER:phase-future.embedding-remote-reranker-live]`（remote reranker cross-encoder over HTTP live 联调）排入 **v0.31.0 / Phase 38 — embedding-remote-reranker-live**（task-38.1 remote-reranker-provider-and-live-recall：构建 `RemoteRerankerProvider` + `select_reranker` 工厂（镜像 `embedding/factory.rs:27-96`）+ 契约测试 + env-gated live harness（real cross-encoder vs `IdentityReranker` no-semantic-signal 基线同标注 query×candidate 集 MRR/recall@1）/ task-38.2 reranker-config-bridge-and-data-plane-wiring：Go `[reranker]` 段 + `setRerankerEnv` env-bridge（镜像 `setRemoteEnv`）+ Rust 数据面 `reranker_from_env()` 在 `server.rs` hybrid:334/semantic:376 + `data_plane/search.rs` semantic:282 三处 opt-in 接线（reranker 首次从 config 在生产路径接通，默认 unset 字节等价无 rerank）/ task-38.3 closeout）。承 Phase 37 remote provider live 血脉 + ADR-026 reranker 维度——与 Phase 37 核心差异：本 phase 要**构建** provider（`RemoteEmbeddingProvider` 自 Phase 22 已全实现，但 `RemoteRerankerProvider` / `select_reranker` 工厂从无）且**数据面首次 opt-in 接线**。de-risk 已由主 agent 本机真实证明（SiliconFlow `/v1/rerank` + `Qwen/Qwen3-VL-Reranker-8B` round-trip，`config_save relevance_score=0.7356` 排 #1 约 46x 区分度）。上方 §3.19 新开「仍真延后」段所列 `embedding-remote-reranker-live` 经本 phase **真实构建 + 联调 + 实测 rerank 质量兑现**（task-38.3 closeout 经 ADR-026 add-only Phase-38 Amendment 标 fulfilled、ADR-042 add-only Phase-38 Amendment 标 follow-up fulfilled，均不溯改 D-body）。**新增 backlog 条目（add-only）**：`reranker-large-corpus-quality`（大语料 rerank 质量 NDCG/MRR 大基准，超作者小型手工标注 query×candidate 集）。**复用既有 backlog（不另造碎片）**：CI 跑 live remote rerank 复用 `embedding-remote-ci-credential`（remote 付费 API 无免费 service container、reranker 同理，**不另造 reranker-ci-credential**）；多 provider rerank live 续用 `embedding-multi-provider-live`。默认构建 0 新 dep（`reranker-remote = ["dep:ureq"]` 复用 `ureq` 自 task-22.3 已 optional）+ 0 network + 0 proto + 0 migration + `CONTEXTFORGE_RERANKER_PROVIDER` unset 字节等价无 rerank（ADR-004/008）；API key env-only 永不进 config。真实数值 / 受阻维度真实跑出后回填（ADR-013，不预填）。ADR-043 Proposed。
>
> **v0.33.0 / Phase 40 排期更新（规划中 2026-06-07，add-only，不删上方历史条目）**：§3.22 把两组 code-local 治理 marker 排入 **v0.33.0 / Phase 40 — governance-debt-cleanup-3**（task-40.1 memory pin actor 透传 / task-40.2 L2 embedding 缓存访问序 LRU / task-40.3 closeout）。上方「memory」段所列 `memory-pin-actor`（v0.20.0 Phase 27 已落 store first-class 字段，但入口透传链缺）经本 phase **入口透传兑现**（proto add-only `actor=3` + Go 参数链 + REST `X-Actor` header + Rust 空回落 byte-equiv，ADR-032 add-only Phase-40 Amendment 标 fulfilled）；上方「仍真延后」段所列 `l2-cache-true-lru` 经本 phase **命中 bump 隐式 rowid 兑现**（0 schema migration，据实更正 Phase 33「须加时间列」假设，ADR-038 + ADR-027 add-only Phase-40 Amendment 标 fulfilled）。**诚实校正（ADR-013）**：pin actor 本轮交付调用方透传，认证身份续 `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；L2 命中 bump 写放大 + `with_sqlite` 无生产调用点现网零影响据实记。**新增 backlog 条目（add-only）**：`memory-actor-authenticated-identity`（把 `X-Actor` header 值校验映射为已认证 auth subject，须 console-api 鉴权层）/ `memory-actor-all-rpc`（其它 memory RPC 的 actor 透传）/ `l2-cache-production-wire`（`with_sqlite` 接入生产 daemon）/ `l2-lru-bump-batching`（L2 命中 bump 写放大优化）。**仍真延后不强行扩面**：`vector-dim-feature-enforce`（须 feature build）/ `tracestore-multi-workspace-strict`（余下读路径）/ `chunk-source-type-filter` / `chunk-agent-scope-filter`（须 import-path schema migration）据实续延后。默认构建 0 新 dep + 0 network + proto add-only 既有契约不变（ADR-004/008/015）。真实数值 / 受阻维度真实跑出后回填（ADR-013，不预填）。ADR-045 Proposed。
>
> **v0.34.0 / Phase 41 排期更新（规划中 2026-06-07，add-only，不删上方历史条目）**：§3.23 把 `[SPEC-DEFER:phase-future.tokenizer-default-on]`（ADR-029:54 + ADR-035 D3 据「翻默认是产品决策」诚实延后）排入 **v0.34.0 / Phase 41 — tokenizer-default-on**（task-41.1 production 默认翻 `code_cjk`：`server.rs` add `resolve_tokenizer()` env-resolution + 生产索引两调用点 `open_with_tokenizer(.., &resolve_tokenizer())`，`IndexSession::open`/`DEFAULT_TOKENIZER` 不动，既有 collection schema-driven 安全，Phase 24 harness 复测 +0.0909 / task-41.2 Go `[retrieval] tokenizer` config 桥：`setTokenizerEnv` 镜像 `setVectorEnv`、env-wins、无段默认 code_cjk、Rust 0 toml dep / task-41.3 closeout）。上方 §3.12 / 历史「检索 tokenizer」血脉所列 `tokenizer-default-on`（ADR-029/035 两度据「产品决策」延后 full default flip）经本 phase **兑现默认开启维度**（生产默认翻 `code_cjk` + opt-out + 既有 collection 安全，ADR-029/035 add-only Phase-41 Amendment 标 fulfilled、不溯改 D-body）。**关键诚实定性（ADR-013）**：本 phase 是项目**首次刻意改默认行为**（新建 collection 倒排词项 `TEXT`→`code_cjk`，**非 byte-equivalent**）——由 ADR-046 显式承接 + 三重安全（既有 collection 不受影响 / `CONTEXTFORGE_TOKENIZER=default` / `[retrieval]` opt-out 回 legacy byte-equiv / 既有 collection 不自动迁移）+ Phase 24 实测 +0.0909 justify；jieba `cjk_segmenter` 默认不取（0-dep baseline + Phase 30 实测 jieba vs bigram delta=+0.0000）。**新增 backlog 条目（add-only）**：`cjk-segmenter-default-on`（jieba 真分词默认开启——重词典 dep + Phase 30 无增益）/ `tokenizer-auto-reindex-on-upgrade`（既有 collection 升级时自动 reindex 到 code_cjk——不自动改用户数据）/ `tokenizer-large-corpus-recall`（大语料 tokenizer recall 基准，超小 golden）/ `retriever-config-tokenizer-routing`（`RetrieverConfig.tokenizer` vestigial 字段真路由——ADR-035 D3 已定性 schema-driven 对称）。默认构建 0 新 dep（`code_cjk` 纯 std）+ 0 network。真实 recall delta / 受阻维度真实跑出后回填（ADR-013，不预填）。ADR-046 Proposed。
>
> **v0.35.0 / Phase 42 排期更新（规划中 2026-06-07，add-only，不删上方历史条目）**：§3.24 把上方「向量 backend 细化」/§3.14 排期时新开的 `[SPEC-DEFER:phase-future.chunk-source-type-filter]`（task-32.3 / ADR-037，real chunk source_type filter）排入 **v0.35.0 / Phase 42 — chunk-source-type-filter**（task-42.1 chunk-source-type-derivation-and-filter：`core/src/retriever/mod.rs` `classify_source_type(file_path)` 扩展名确定性桶 code/doc/config/other 镜像 `lang_hint_from_path` + 三构造点 source_type 真实派生 + `search()` BM25 source_type post-filter 镜像 language + agent_scope 续 no-op，**0 schema migration**（§5.3 FROZEN）/ task-42.2 console-api-source-type-forward：`console_data_plane.proto` `SearchRequest` add-only `source_type=9` + `data_plane` post-filter + Go `?source_type=` forward / task-42.3 closeout）。上方 §3.14 新开「real chunk filter」段所列 `chunk-source-type-filter` 经本 phase **真实过滤兑现**（source_type 由 file_path 确定性派生、ADR-037 source_type no-op 被 supersede，ADR-037 add-only Phase-42 Amendment 标 fulfilled）。**关键诚实校正（ADR-013）**：`chunk-source-type-filter` 与 `chunk-agent-scope-filter` 经 grounding **不对称**——source_type 可由 file_path 确定性派生（0 migration）真实落地；`agent_scope` 是 memory 层概念（`memory_items` 0013 / `ListMemory` scope / `memstore.go:629-635`）、chunks 无 agent 关联、无可派生维度 → 本 phase **不伪造**，`[SPEC-DEFER:phase-future.chunk-agent-scope-filter]` 据实保持延后（须 ingest-path schema 工程 + 价值不明，镜像 Phase 32/34/35 grounding 校正手法）。**新增 backlog 条目（add-only）**：`chunk-importer-source-type-tagging`（importer 显式 source_type 打标，超 file_path 派生粗粒度桶，须 §5.3 解冻加 chunks 列）/ `semantic-path-source-type-filter`（v1 semantic 路径 retriever-内 source_type 过滤，本 phase v1 `search()` BM25 内过滤镜像 language scope，console 经 data_plane post-filter 覆盖 semantic/hybrid）。**仍真延后不强行扩面**：`chunk-agent-scope-filter`（memory 层概念 + 须 ingest-path schema）/ `vector-dim-feature-enforce`（须 feature build）/ `tracestore-multi-workspace-strict`（余下读路径）据实续延后。默认构建 0 新 dep（`classify_source_type` 纯 std）+ 0 network + 0 schema migration（§5.3 FROZEN）+ proto add-only 既有契约不变（ADR-004/008/015）。source_type value 由空串变真实派生值系填补 task-4.2 §2A v0.1 schema gap（契约本意）由 ADR-047 据实记。真实过滤行为 / 受阻维度真实跑出后回填（ADR-013，不预填）。ADR-047 Proposed。

---

## 5. 执行协议（承项目契约）

每个候选版本的实现遵循既有规则，无例外：

1. **S2V 四步**逐 task：spec（已在规划阶段起草，Draft）→ RED → GREEN → REFACTOR。
2. **一 task 一 PR**；CI 三门（cargo-test / go-test / spec-lint）全绿后自主 merge + 删分支；**红灯绝不合**（已知 phase9 Tantivy `LockBusy` flake → `gh run rerun --failed` 复跑至绿）。
3. **ADR-014 D1-D5** 逐 task：D1 mapping、D2 lint 0 未标注命中、D3 verified-by、D4 自治、**D5 不溯改已闭合 Phase 1-19 spec（ADR 改动 add-only amendment）**。
4. **ADR-013 禁伪造**：🟡/🔴 项的真实数值 / 联调 / 跨平台只在真实证据下记录；未达不标 `[x]`，provisional ADR 不在缺真实数据时翻 Accepted。
5. 版本全部 task 合入后起 release docs（README/RELEASE_NOTES/evidence/artifacts）+ phase §6 ACs `[x]` + Status Done + adapter 行 Done；**tag push 前停下等用户明确授权**；授权后 push → release.yml → post-tag-push backfill PR 填实 tag SHA / run ID / 镜像 digest。
6. 治理承**单驱动 + 内部 Agent subagent**（ADR-011 / ADR-012），不外派 worker。
