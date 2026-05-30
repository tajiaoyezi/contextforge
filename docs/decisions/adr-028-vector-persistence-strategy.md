# ADR `028`: `vector-persistence-strategy`

**Status**: Proposed (2026-05-30; Phase 23 task-23.1/23.2 起草。落地 + hnsw 真实持久化往返 + sqlite-vec 真实跨平台构建结果后于 task-23.3 据真实非合成验证 ratify Proposed→Accepted，ADR-013。)
**Category**: 数据平面 / 向量检索 / 持久化 + 跨平台
**Date**: 2026-05-30
**Decided By**: 主 agent (ADR-012 自治)；tajiaoyezi ratification at v0.16.0 closeout
**Related**: ADR-023 (vector-backend-default — D1 sqlite-vec 默认 / D2 hnsw 跨平台 fallback「rebuild-on-restart」前提 + Follow-ups 三 marker) / ADR-002 (sqlite+tantivy persistence) / ADR-008 (core-library-selection) / ADR-004 (local-first-privacy-baseline) / ADR-014 (D1-D5) / Phase 18 (vector-backend-selection — task-18.3 sqlite-vec / task-18.6 hnsw spike) / Phase 19 (vector-retrieval-integration — Retriever 语义热路径) / Phase 23 (vector-persistence-and-cross-platform)

## Context

Phase 18/19 把向量检索落地为 feature-gated backend（ADR-023 分层选型）+ Phase 19 语义热路径（`Retriever::search_semantic` + 默认 0-dep `BruteForceVectorBackend`）。两块向量持久化 / 跨平台技术债被 ADR-023 显式列为 Follow-ups：

1. **hnsw 图无持久化**：`core/src/retriever/vector/hnsw.rs::HnswBackend`（`instant-distance` 纯 Rust HNSW）用 `Builder::default().build(points, values)` 一次性全量建图存进内存（`map: Mutex<Option<HnswMap<...>>>`），无序列化。ADR-023 D2（`adr-023:55-60`）把 hnsw 定为跨平台 / dev / 小语料 fallback，但明记其 disqualifying 项含「in-memory-only model（rebuild on restart）」+ 100k 图构建实测 28.4s（`docs/spikes/phase-18-hnsw.md`）。`core/src/server.rs:293-296` 语义热路径据此对每请求「按需从 SQLite 枚举 + 重 embed + 重建索引」（`no persistence yet — [SPEC-DEFER:phase-future.hnsw-graph-persistence]`）。`core/src/retriever/vector/types.rs::VectorIndexConfig::persistence_path: Option<PathBuf>` 已预留持久化 seam（task-18.1），当前恒 `None`。

2. **sqlite-vec Windows MSVC 受阻**：`core/src/retriever/vector/sqlite_vec.rs::SqliteVecBackend`（`sqlite-vec` 0.1.9 `vec0` 虚表）在 Linux x86_64 gcc 可构建并跑出真实数据（task-18.3，`docs/spikes/phase-18-sqlite-vec.md`），但 C amalgamation 在 Windows MSVC 工具链受阻（`docs/releases/v0.11.0-evidence.md` 凭据保留）。ADR-023 D1 把 sqlite-vec 定为生产嵌入式推荐默认，但 Consequences 记「dev/prod backend parity is imperfect」（`[SPEC-DEFER:phase-future.sqlite-vec-cross-platform]`）。

3. **向量增量索引未定**：所有 backend 默认全量 reindex（各 `delete` 为 clear+rebuild 语义），无单 chunk 增量路径（`[SPEC-DEFER:phase-future.vector-incremental-index]`，承 `phase-19` §2）。

本 ADR 记录三者的处理策略：hnsw 图持久化格式、sqlite-vec 跨平台调查结论、向量增量索引评估口径。

## Decision

向量持久化与跨平台采用 **feature-gated、诚实定论、不破坏默认 0-dep baseline** 的策略：

### D1 — hnsw 图持久化：序列化往返 + rebuild-on-load fallback（`vector-hnsw` feature）

`HnswBackend` 在 `vector-hnsw` feature 下加图持久化（task-23.1），路径据 `instant-distance` 0.6 序列化面二选一：

- **路径 A（原生序列化）**：若 `HnswMap` 暴露 serde / 可编码，把已建图序列化到 `VectorIndexConfig.persistence_path` 磁盘文件，重载后 `search` 无需重建。
- **路径 B（重建输入持久化）**：若 crate 不暴露原生序列化，持久化 `(unit-normalized embedding, chunk_id)` 输入集，load 时复用 `flush` 全量建图——仍消除「从 SQLite 重新枚举 + 重 embed」的昂贵步骤。

`load(path)` 在文件缺失 / 格式版本不兼容 / 反序列化失败时返回可识别状态，调用方走全量重建（rebuild-on-load fallback，不 panic、不静默吞错）。`persistence_path: None` 维持纯内存现状（行为逐字节不变）。格式带版本头，跨版本不兼容归入 rebuild-on-load。

### D2 — sqlite-vec 跨平台：三路径真实调查 + 诚实定论（`vector-sqlite` feature）

sqlite-vec Windows MSVC 经 task-23.2 真实调查三路径：(a) bundled C amalgamation 的 MSVC 编译选项调整；(b) 预编译 `vec0` 扩展 + 运行时 `load_extension`；(c) 同等 KNN 能力的替代 Rust 绑定。任一路径在 Windows MSVC `cargo build --features vector-sqlite` 真实通过则落地 + 记录真实凭据；三路径全部确证受阻则诚实文档化 stop-condition（承 Phase 18 gcc-only 既有结论 + 本次失败凭据），推荐 dev 用 hnsw 跨平台 fallback（ADR-023 D2）。**ADR-013 红线：不在源码 / 文档伪造 Windows MSVC 构建通过**；受阻态以真实凭据如实记录。

### D3 — 向量增量索引：据 backend 能力最小实现或如实延后

向量增量索引（task-23.3 评估）据各 backend 行级能力分层：sqlite-vec `vec0` 支持行级 INSERT/DELETE、brute-force `rows` 可追加 → 优先落最小增量实现（单 chunk 追加 / 删除不全量 reindex）+ deterministic 单测；hnsw `instant-distance` 全量建图无增量插入 → 如实延后 `[SPEC-DEFER:phase-future.vector-incremental-index]` + 文档化评估口径。

### D4 — 默认构建不变：0 vector 依赖 + BM25-only baseline

持久化 / 跨平台 / 增量能力**全部在各自 feature（`vector-hnsw` / `vector-sqlite`）下**，默认构建 0 新 vector 依赖、`HnswBackend` / `SqliteVecBackend` 不编译、语义路径仍经默认 0-dep `BruteForceVectorBackend`（ADR-023 D5）。序列化依赖（若需）仅在 optional feature 下引入，经主 agent R7 chore + ADR-008 add-only 记录。本 ADR 不改 task-18.1 三 trait（`VectorBackend`/`VectorIndexer`/`VectorSearcher`）签名。

## Consequences

- **Positive**: hnsw fallback 在中等语料下可用于持久部署（消除重启重建）；sqlite-vec 跨平台缺口据真实调查缩小或诚实定论；向量增量索引按 backend 能力务实推进；默认构建保持 0 vector 依赖 + 跨平台（ADR-023 D5 不破坏）；持久化 seam（`VectorIndexConfig.persistence_path`）首次接通。
- **Negative / open**: hnsw 持久化路径取决于 `instant-distance` 序列化面（路径 A vs B 取舍待 task-23.1 核实）；sqlite-vec Windows MSVC 可能经调查仍受阻（D2 受阻态如实记录，dev 用 hnsw fallback）；向量增量索引在建图类 backend 受 crate 限制可能延后。
- **Ratification**: 本 ADR **Proposed**。task-23.1 真实持久化往返（index→save→重载→search 命中等价）+ task-23.2 真实跨平台构建结果（落地或 stop-condition）通过后，于 v0.16.0 closeout（task-23.3）据真实非合成验证 ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）；某维度受阻则据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify。
- **Follow-ups**: ADR-023 D2「rebuild-on-restart」前提经 hnsw 持久化解除（add-only Amendment 记录，不溯改 ADR-023 正文 D1-D6，D5）；sqlite-vec on-disk 编码细化 `[SPEC-DEFER:phase-future.sqlite-vec-on-disk]`；向量增量索引完整化 `[SPEC-DEFER:phase-future.vector-incremental-index]`；若 D2 落地替代绑定则 ADR-008 add-only 记依赖变更。
