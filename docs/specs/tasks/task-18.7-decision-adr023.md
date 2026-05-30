# Task `18.7`: `decision-adr023 — 4 路 backend 5 维实测横向对比 + ADR-023 默认 backend 选型（Proposed）+ comparison 文档`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 18 (vector-backend-selection)
**Dependencies**: task-18.3（sqlite-vec）/ task-18.4（qdrant）/ task-18.5（lancedb）/ task-18.6（hnsw）4 路 spike evidence 齐备 / task-18.2（harness 产数）/ ADR-002（sqlite+tantivy 持久化）/ ADR-014 D1-D5 第十三次激活 / task-18.8（real-embedding recall，下游 ratify）

## 1. Background

task-18.3–18.6 在同一 Linux x86_64 host 上用 task-18.2 harness 跑出 4 路 backend 的 5 维实测（n=5000 + n=100000）。本 task 横向对比这 4 路数据，产出 `docs/spikes/phase-18-comparison.md` 与 `docs/decisions/adr-023-vector-backend-default.md`（默认 backend 选型，Proposed）。

关键事实：合成语料上 4 路 recall@5/10 均 = 1.0（不可区分），故选型由 ContextForge 架构约束（local-first 单二进制 / SQLite-based ADR-002 / 跨平台含 Windows MSVC dev）驱动，而非 recall；真实 recall 排序留 task-18.8（dogfood embedding）。

## 2. Goal

横向对比 4 路 5 维数据，给出数据驱动的默认 backend 选型（分层 + feature-gated），落 ADR-023（Proposed，pending task-18.8 ratify）+ comparison 文档；补齐 hnsw evidence 的 Linux RSS + 100k 数据；清理 task-18.4 引入的 `known_backends` unused_mut warning。默认 `cargo test --workspace` + `go test ./...` 不退化；D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新建 `docs/spikes/phase-18-comparison.md`** — 4 路 backend n=5000 + n=100000 5 维对比表 + 规模分析 + 各 backend 优劣 + 对 ContextForge 的 bearing。
- **新建 `docs/decisions/adr-023-vector-backend-default.md`** — 分层选型决策（D1 sqlite-vec 推荐嵌入式默认 provisional / D2 hnsw 跨平台 fallback / D3 qdrant scale-out / D4 lancedb 嵌入式列式 / D5 默认 BM25-only feature-gated / D6 ratify + 运行时 wiring 后置），Status Proposed。
- **重写 `docs/spikes/phase-18-hnsw.md`** — 补 Linux RSS（5k）+ n=100000 数据（原仅 Windows，RSS n/a）。
- **修改 `bench/src/backends.rs`** — `known_backends` `let mut v` 加 `#[allow(unused_mut)]`（默认无 feature 时 mut 未用 warning）。
- **修改 `docs/s2v-adapter.md`** — Phase 18 表 18.7 行 Deferred/新增 → Done；ADR 索引加 ADR-023。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **ADR-023 Accepted ratify** [SPEC-OWNER:task-18.8-eval]：默认选型 ratify 须在 task-18.8 真实 embedding recall 后 + Phase 18 closeout。
- **生产 embedding pipeline + Retriever 运行时 wiring** [SPEC-OWNER:phase-future.vector-retrieval-integration]：需 embedding 模型，本 task 不接入热路径；task-18.1 已留 `with_vector_searcher` seam。
- **hnsw 图持久化** [SPEC-DEFER:phase-future.hnsw-graph-persistence] / **sqlite-vec Windows MSVC** [SPEC-DEFER:phase-future.sqlite-vec-cross-platform]：选型 follow-up，后置。
- **dogfood 真实语料 recall 复跑** [SPEC-OWNER:task-18.8-eval]：本 task 用合成 100k 数据（recall 不可区分已记录）。

## 4. Actors

- **主 agent**：对比分析 + ADR 主笔 + PR 主理。
- **ADR-023**：默认 backend 选型 source of truth（Proposed）。
- **下游 task-18.8**：产真实 embedding recall → ratify ADR-023 D1。
- **下游 task-18.9**：Phase 18 closeout 引 ADR-023 Accepted（若 18.8 ratify）。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/spikes/phase-18-{sqlite-vec,qdrant,lancedb,hnsw}.md`（4 路 evidence）
- `docs/specs/tasks/task-18.1-vector-trait.md`（`with_vector_searcher` seam）
- `docs/decisions/adr-002-*.md`（sqlite 持久化对齐论据）+ `adr-014-*.md`（D1-D5）

### 5.2 选型逻辑（数据驱动）

- recall 合成不可区分 → 由架构约束驱动：单二进制（排除 qdrant 作默认）/ ADR-002 SQLite 对齐（sqlite-vec 嵌入式默认）/ 跨平台 dev 含 Windows MSVC（hnsw fallback，纯 Rust 0 native）。
- 分层：D1 sqlite-vec（Linux prod，provisional）/ D2 hnsw（跨平台 dev/小语料）/ D3 qdrant（scale-out）/ D4 lancedb（嵌入式列式）/ D5 默认 BM25-only feature-gated。
- D1 provisional：ratify 须经 task-18.8 真实 embedding recall。

## 6. Acceptance Criteria

- [x] **AC1**: 4 路 backend 5 维数据（n=5000 + n=100000）横向对比表落 `docs/spikes/phase-18-comparison.md`，数据取自各 backend spike 实测（非伪造）— verified by **TEST-18.7.1**（comparison 文档 + 各 spike evidence 数据一致）
- [x] **AC2**: ADR-023 落 `docs/decisions/adr-023-vector-backend-default.md`，Status Proposed，含分层选型 D1-D6 + provisional pending task-18.8 标注 — verified by **TEST-18.7.2**（ADR 文件存在 + Status/Decision 段完整）
- [x] **AC3**: hnsw evidence 补 Linux RSS（5k idle/index 4.4/11.0）+ n=100000 数据 — verified by **TEST-18.7.3**（`docs/spikes/phase-18-hnsw.md` 含 Linux RSS + 100k 行）
- [x] **AC4**: `known_backends` unused_mut warning 清除 — verified by **TEST-18.7.4**（`cargo build` 默认无 unused_mut warning）
- [x] **AC5**: 既有不退化 — 默认 `cargo test --workspace` 全 PASS；`go test ./...` 全 PASS — verified by **TEST-18.7.5** + §10 实测
- [x] **AC6**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中 — verified by §10 记录的 D2 lint 实跑输出

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-18.7.1 | 4 路 5 维对比表 + 数据一致 | `docs/spikes/phase-18-comparison.md` | Done |
| TEST-18.7.2 | ADR-023 Proposed 分层选型 | `docs/decisions/adr-023-vector-backend-default.md` | Done |
| TEST-18.7.3 | hnsw evidence 补 Linux RSS + 100k | `docs/spikes/phase-18-hnsw.md` | Done |
| TEST-18.7.4 | known_backends unused_mut 清除 | `bench/src/backends.rs` | Done |
| TEST-18.7.5 | 默认 cargo test --workspace 0 failed | 全 workspace | Done |

## 8. Risks

- **R1（高）recall 合成不可区分**：4 路均 recall 1.0，选型不能基于 recall。
  - **缓解**：ADR-023 D1 标 provisional；真实 recall 排序留 task-18.8（dogfood embedding）；ratify 后置。
- **R2（中）推荐默认 sqlite-vec 不可在 Windows dev 构建**：dev/prod backend parity 不完美。
  - **缓解**：vector 搜索 opt-in（默认 BM25-only）；hnsw 作跨平台 dev fallback；ADR-023 D2 + Consequences 明确记录。
- **R3（低）选型为 architectural commitment**：默认 backend 长期影响。
  - **缓解**：ADR Status Proposed（非 Accepted），closeout + tajiaoyezi ratify；trait 抽象使任一 tier 可换。

## 9. Verification Plan

```bash
cargo build --workspace            # 默认无 unused_mut warning
cargo test --workspace
go test ./...
bash scripts/spec_drift_lint.sh --touched master
# 数据溯源：comparison 表数据对照 docs/spikes/phase-18-*.md 各 backend evidence
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30
- **改动文件**：`docs/spikes/phase-18-comparison.md`（新增）、`docs/decisions/adr-023-vector-backend-default.md`（新增）、`docs/spikes/phase-18-hnsw.md`（补 Linux RSS + 100k）、`bench/src/backends.rs`（unused_mut allow）、`docs/s2v-adapter.md`（18.7 行 Done + ADR 索引）、`docs/specs/tasks/task-18.7-decision-adr023.md`（本 spec）
- **commit 列表**：见本 task PR（分支 `feat/task-18.7-decision-adr023`）；合入后以 merge commit 为准
- **§9 Verification 结果**：见 PR 描述（默认 cargo test / go test 全绿 + D2 lint 0 命中）
- **剩余风险 / 未做项**：ADR-023 D1 provisional → task-18.8 ratify；运行时 wiring 见 [SPEC-OWNER:phase-future.vector-retrieval-integration]
- **下游 task 影响**：task-18.8（真实 recall → ratify ADR-023）/ task-18.9（closeout 引 ADR-023）
