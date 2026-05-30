# Task `18.2`: `spike-harness — bench/ crate + 确定性合成语料 + dogfood 语料 + 5 维测量 runner + evidence 模板`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 18 (vector-backend-selection)
**Dependencies**: task-18.1 (`VectorBackend` / `VectorIndexer` / `VectorSearcher` 三 trait + `NoopVectorBackend` 已冻结并合入 master `cfe8098`) / PRD §Open Questions O2 (向量后端最终选型 — 本 phase 解) / PRD §Constraints Performance (P95 < 500ms / idle RSS < 300MB) / PRD §Success Metrics (Top-5 ≥75% / Top-10 ≥85%) / ADR-002 (sqlite-tantivy 既有存储，本 task 不改) / ADR-006 (recall-eval-acceptance-gate — 本 harness 是其 spike 前身) / ADR-014 D1-D5 第九次激活

## 1. Background

Phase 18 §2A 决策 4 锁 **trait-first**：task-18.1 已冻结 `Vector{Backend,Indexer,Searcher}` 三 trait。本 task 在该 trait 之上建一个**统一测量台**（`bench/` crate），让 task-18.3-18.6 的四个真实 backend（[SPEC-OWNER:task-18.3-spike-sqlite-vec] / [SPEC-OWNER:task-18.4-spike-qdrant-embedded] / [SPEC-OWNER:task-18.5-spike-lancedb] / [SPEC-OWNER:task-18.6-spike-hnsw]）按**同一口径**产出可比数据；task-18.7 的选型决策 [SPEC-OWNER:task-18.7-decision-adr023] 依赖这批可比数据。

测量台对任意实现了 trait 的 backend 跑 Phase 18 §2A 决策 2 的 5 维：`recall@5` + `recall@10` + `P95 latency` + `单机 RSS`（must）+ `cold-start` + `索引重建耗时`（nice-to-have）。

本 task **不引入任何真 backend dep**，唯一可用 backend 是 task-18.1 的 `NoopVectorBackend`（search 返空）——因此本 task 的验收是**测量台机器本身可跑通**（语料生成确定性、recall/percentile 数学正确、对 Noop 端到端 smoke 产出 recall=0 的报告且不 panic），真实召回数据由下游四个 backend task 接入后产出。

## 2. Goal

落地 `bench/` workspace 成员 crate：给定任意 `&dyn VectorIndexer + &dyn VectorSearcher`，在**确定性合成 100k 语料** + **ContextForge dogfood 语料**上测量 5 维并写出每 backend 一份 evidence md。语料用**确定性种子**生成（[SPEC-DEFER:phase-future.embedding-provider-full] 真实 ONNX/transformer embedding provider 不在本 task；本 harness 用种子向量规避外部模型依赖）；100k 语料**按需生成不入 git**（[SPEC-DEFER:phase-future.spike-corpus-fixture-commit] 大 fixture 落盘策略后置）。本 task ship 后对 `NoopVectorBackend` 端到端 smoke 产出 recall=0 报告即验收；`cargo test --workspace` + `go test ./...` 不退化；ADR-014 D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **新建 `bench/Cargo.toml`** — workspace 新成员，package `contextforge-bench`，`[[bin]] name = "spike"`；依赖 `contextforge-core`（消费已冻结 trait + `NoopVectorBackend`）+ `serde`/`serde_json`（既有 workspace dep）；**不引入任何真 backend dep**（sqlite-vec / qdrant / lancedb / hnsw 由 [SPEC-OWNER:task-18.3-spike-sqlite-vec] 等各自 PR 接入）。
- **修改根 `Cargo.toml`** — `members = ["core", "bench"]`（add-only）。
- **新建 `bench/src/corpus.rs`** — 确定性语料生成：内置 splitmix64/xorshift 种子 PRNG（无 `rand` dep），`gen_synthetic(seed, n, dim) -> Vec<VectorChunk>`（确定性 embedding）+ `gen_queries(seed, &corpus, m, dim) -> Vec<Query>`（每条 query 携带 brute-force 真值 chunk_id）+ `load_dogfood(path) -> Vec<VectorChunk>`（读 `test/fixtures/spike/dogfood-contextforge.jsonl`）。
- **新建 `bench/src/measure.rs`** — 纯函数测量数学：`recall_at_k(hits: &[ChunkId], truth: &ChunkId, k) -> bool`、`recall_rate(results, truths, k) -> f64`、`percentile(durations: &mut [Duration], p) -> Duration`（P95）、`brute_force_topk(query, &corpus, k) -> Vec<ChunkId>`（cosine 精确近邻，作 recall 真值）。
- **新建 `bench/src/runner.rs`** — `run(backend, corpus, queries, cfg) -> MeasureReport`：对 `dyn VectorIndexer` 跑 index（计 cold-start + reindex 耗时）→ 对 `dyn VectorSearcher` 逐 query search（计每次 latency → P95）→ 算 recall@5/10 → 采样 RSS。`MeasureReport { backend_name, n, dim, recall_at_5, recall_at_10, p95_latency_ms, idle_rss_mb, index_rss_mb, cold_start_ms, reindex_ms }`（serde Serialize）。
- **新建 `bench/src/rss.rs`** — 跨平台 RSS 采样：Linux 读 `/proc/self/statm`；其他平台返 `None`（[SPEC-DEFER:phase-future.rss-sampling-macos-windows] 非 Linux RSS 采样后置，Phase 18 §7 R1 锚 P0=Linux）。
- **新建 `bench/src/backends.rs`** — backend 注册表：当前仅 `"noop" -> NoopVectorBackend`；下游 backend 由各自 task 用 `#[cfg(feature = "vector-<backend>")]` 追加（[SPEC-OWNER:task-18.3-spike-sqlite-vec] 等）。
- **新建 `bench/src/main.rs`** — CLI：`spike --backend <name> --n <N> --dim <D> --seed <S> [--dogfood <path>] [--out <md>]`；跑 runner → 打印 `MeasureReport` JSON + 可选写 evidence md。
- **新建 `bench/src/lib.rs`** — 模块导出 + `#[cfg(test)] mod tests`（≥6 unit test 覆盖 TEST-18.2.x）。
- **新建 `scripts/spike_vector_backends.sh`** — wrapper：对每个已接入 backend 跑 `spike` → 写 `docs/spikes/phase-18-<backend>.md`；本 task ship 时仅 `noop` 可跑（真 backend 由下游接入后纳入）。
- **新建 `docs/spikes/_template.md`** — 5 维测量结果表 schema + trade-off 讨论段模板。
- **新建 `test/fixtures/spike/dogfood-contextforge.jsonl`** — 小型 dogfood 语料样本（< 200 行，确定性手工/脚本生成，直接 commit）。
- **新建 `bench/README.md`** — 用法 + 5 维口径说明。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **真 backend 实现 + dep** [SPEC-OWNER:task-18.3-spike-sqlite-vec] / [SPEC-OWNER:task-18.4-spike-qdrant-embedded] / [SPEC-OWNER:task-18.5-spike-lancedb] / [SPEC-OWNER:task-18.6-spike-hnsw]：四个真实 backend 与各自 Cargo dep 不在本 task；本 task 仅提供测量台 + Noop smoke。
- **默认 backend 选型 + ADR-023** [SPEC-OWNER:task-18.7-decision-adr023]：5 维数据出齐后的决策由 18.7 负责。
- **eval semantic recall 接入** [SPEC-OWNER:task-18.8-eval-semantic-recall]：`internal/eval/eval.go` 的 SemanticRecall@K 不在本 task。
- **真实 embedding provider** [SPEC-DEFER:phase-future.embedding-provider-full]：fastembed-rs / candle / ONNX 本地 embedding 不在本 task；harness 用确定性种子向量。
- **100k fixture 落盘/git-lfs** [SPEC-DEFER:phase-future.spike-corpus-fixture-commit]：合成 100k 语料按需生成不 commit；落盘策略后置。
- **非 Linux RSS 采样** [SPEC-DEFER:phase-future.rss-sampling-macos-windows]：macOS/Windows RSS 采样后置（Phase 18 §7 R1 锚 P0=Linux）。
- **CI 接入 spike 跑批** [SPEC-DEFER:phase-future.spike-ci-job]：spike 跑批是 dev-time 行为，不入 ci.yml（编译时间 + 真 backend dep 顾虑）；CI 只需 `cargo build -p contextforge-bench` 通过。

## 4. Actors

- **主 agent**：本 task 实施 + PR 主理（§2A 自审 + RED→GREEN→REFACTOR + verify + commit + 合入）。
- **bench crate**：测量台 SoT；持有语料生成 / 测量数学 / runner。
- **下游 task-18.3-18.6**：各自接入真 backend 到 `bench/src/backends.rs` 注册表并跑 `spike` 产出 evidence。
- **下游 task-18.7**：消费四份 evidence 的 5 维数据做选型。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-18-vector-backend-selection.md`（§2A 5 决策 + §3.18.2 模块 + §6 AC2 owner + §7 R5/R6/R9）
- `docs/specs/tasks/task-18.1-vector-trait.md`（已冻结 trait API §5.3 §A/§B — 本 task 消费方）
- `core/src/retriever/vector/{traits,types,noop}.rs`（trait 真实签名 — 实施 RED 阶段对照）
- `docs/decisions/adr-006-recall-eval-acceptance-gate.md`（recall gate 口径 — 本 harness 是其 spike 前身）
- `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5；本 task D2 lint + D3 verified-by + D5 历史不溯改）

### 5.2 Imports（全部已在 workspace 既有 dep — 0 新 dep）

```rust
// bench/src/corpus.rs / runner.rs
use contextforge_core::retriever::vector::{
    ChunkId, NoopVectorBackend, VectorChunk, VectorFilter, VectorHit, VectorIndexConfig,
    VectorIndexer, VectorMetric, VectorScore, VectorSearcher,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
```

**确定性 RNG**：内置 splitmix64（`fn next_u64(state: &mut u64) -> u64`）+ 由其派生 `next_f32_unit`，不引入 `rand` crate（确定性 + 0 dep）。

### 5.3 关键签名

```rust
// bench/src/corpus.rs
pub struct Query { pub vec: Vec<f32>, pub truth: ChunkId }
pub fn gen_synthetic(seed: u64, n: usize, dim: usize) -> Vec<VectorChunk>;
pub fn gen_queries(seed: u64, corpus: &[VectorChunk], m: usize, dim: usize) -> Vec<Query>;
pub fn load_dogfood(path: &std::path::Path) -> std::io::Result<Vec<VectorChunk>>;

// bench/src/measure.rs
pub fn brute_force_topk(query: &[f32], corpus: &[VectorChunk], k: usize) -> Vec<ChunkId>;
pub fn recall_rate(got: &[Vec<ChunkId>], truths: &[ChunkId], k: usize) -> f64;
pub fn percentile_ms(durations: &mut Vec<Duration>, p: f64) -> f64;

// bench/src/runner.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasureReport {
    pub backend_name: String, pub n: usize, pub dim: usize,
    pub recall_at_5: f64, pub recall_at_10: f64,
    pub p95_latency_ms: f64, pub idle_rss_mb: Option<f64>, pub index_rss_mb: Option<f64>,
    pub cold_start_ms: f64, pub reindex_ms: f64,
}
pub fn run<B: VectorIndexer + VectorSearcher>(backend: &B, corpus: &[VectorChunk], queries: &[Query], dim: usize) -> Result<MeasureReport, contextforge_core::retriever::vector::VectorError>;
```

> **确定性契约**：同 `(seed, n, dim)` → `gen_synthetic` 逐字节相同输出（embedding + chunk_id）；同 `(seed, corpus, m, dim)` → `gen_queries` 相同 query 集与真值。这是四个 backend 数据可比的前提。
> **recall 真值**：`brute_force_topk`（cosine 精确近邻）是 ground truth；recall@k = backend top-k 命中真值 top-1 的比例。`NoopVectorBackend` 返空 → recall=0（预期）。

## 6. Acceptance Criteria

> ADR-014 D3：每条 AC 末尾显式 `verified by <test-id> 或 <smoke-step>`。

- [x] **AC1**: `bench/` 作为 workspace 成员存在，根 `Cargo.toml` `members = ["core", "bench"]`；`cargo build -p contextforge-bench` exit 0；`core` 不引入新 dep（`git show` core/Cargo.toml + Cargo.lock 仅 bench 相关增量，无真 backend dep）— verified by **TEST-18.2.1**（`cargo build -p contextforge-bench` PASS）+ closeout PR diff 检查（本 task 新增，refs phase-18 §7 R7）
- [x] **AC2**: 语料生成确定性 — 同 `(seed,n,dim)` 两次 `gen_synthetic` 输出逐字节相同（chunk_id + embedding）；同参 `gen_queries` 相同 query 集 — verified by **TEST-18.2.2**（`tests::test_corpus_deterministic`）（本 task 新增，refs §5.3 确定性契约）
- [x] **AC3**: `recall_rate` 数学正确 — 已知 got/truth fixture 上算出预期比例（全命中=1.0 / 全不命中=0.0 / 半命中=0.5）— verified by **TEST-18.2.3**（`tests::test_recall_rate_math`）（本 task 新增，refs ADR-006 recall 口径）
- [x] **AC4**: `percentile_ms(P95)` 数学正确 — 已知 100 样本上 P95 落在第 95 百分位 — verified by **TEST-18.2.4**（`tests::test_p95_percentile`）（本 task 新增，refs PRD §Constraints P95<500ms）
- [x] **AC5**: `brute_force_topk` 真值正确 — 构造已知最近邻语料，top-1 命中构造的近邻 — verified by **TEST-18.2.5**（`tests::test_brute_force_topk`）（本 task 新增）
- [x] **AC6**: runner 对 `NoopVectorBackend` 端到端跑通 — `run(&NoopVectorBackend, corpus, queries, dim)` 返 `MeasureReport`，`recall_at_5 == 0.0 && recall_at_10 == 0.0`（Noop 返空），`p95_latency_ms >= 0.0`，无 panic — verified by **TEST-18.2.6**（`tests::test_runner_noop_end_to_end`）+ **smoke**（`cargo run -p contextforge-bench -- --backend noop --n 500 --dim 32 --seed 1` exit 0 + 打印 JSON）（本 task 新增，refs phase-18 §6 AC2）
- [x] **AC7**: evidence 模板 + 写出 — `docs/spikes/_template.md` 含 5 维结果表 schema；runner 的 `--out <md>` 写出含真实 `MeasureReport` 字段的 md — verified by **TEST-18.2.7**（`tests::test_render_evidence_md`）+ `_template.md` 存在（本 task 新增）
- [x] **AC8**: 既有不退化 — `cargo test --workspace` 全 PASS（含新 bench 测试）；`go test ./...` 全 PASS（Go 未触及）— verified by **TEST-18.2.8**（`cargo test --workspace` 0 failed）+ §10 实测计数（本 task 新增，refs PRD §Anti-metrics）
- [x] **AC9**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中（本 spec 所有延后行为已 [SPEC-DEFER]/[SPEC-OWNER] 标注）— verified by §10 记录的 D2 lint 实跑输出（refs ADR-014 D2 第九次激活）

## 7. 追踪表

> Status 取值（standard.md §12.2）：Not Started / Spec Ready / Scenario Ready / Test Red / In Progress / Verified / Waived / Blocked / Done

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-18.2.1 | bench crate workspace 成员 build PASS + core 0 新 dep | `cargo build -p contextforge-bench`（本地实测 + CI build） | Done |
| TEST-18.2.2 | 语料生成确定性（同 seed 逐字节相同） | `bench/src/lib.rs::tests::test_corpus_deterministic` | Done |
| TEST-18.2.3 | recall_rate 数学（1.0/0.0/0.5） | `bench/src/lib.rs::tests::test_recall_rate_math` | Done |
| TEST-18.2.4 | percentile P95 数学 | `bench/src/lib.rs::tests::test_p95_percentile` | Done |
| TEST-18.2.5 | brute_force_topk 真值正确 | `bench/src/lib.rs::tests::test_brute_force_topk` | Done |
| TEST-18.2.6 | runner 对 Noop 端到端 recall=0 不 panic | `bench/src/lib.rs::tests::test_runner_noop_end_to_end` | Done |
| TEST-18.2.7 | evidence md 渲染含 MeasureReport 字段 | `bench/src/lib.rs::tests::test_render_evidence_md` | Done |
| TEST-18.2.8 | cargo test --workspace 0 failed（regression） | 全 workspace | Done |

## 8. Risks

- **R1（中）合成种子向量 recall 代表性弱**：种子随机向量分布与真实 embedding 不同，recall 绝对值仅用于**四 backend 横向可比**，不代表生产召回。
  - **缓解**：dogfood 语料补真实分布信号；evidence 明确标注「合成数据测 P95/RSS 量级，dogfood 测 recall 相对排序」（phase-18 §7 R5 同根）。
- **R2（中）`brute_force_topk` 在 100k×dim 上 O(n·m·dim) 偏慢**：作 ground truth 全量算可能数秒级。
  - **缓解**：smoke/CI 用小 n（≤1000）；真 100k 跑批是 dev-time 离线行为，不入 CI。
- **R3（低）非 Linux RSS 返 None**：本机若 Windows，`idle_rss_mb`/`index_rss_mb` 为 `None`。
  - **缓解**：[SPEC-DEFER:phase-future.rss-sampling-macos-windows]；report schema 用 `Option<f64>` 容忍缺失；Linux CI/dev 跑批取真值。
- **R4（低）确定性 RNG 质量**：splitmix64 非加密级，但确定性 + 均匀性足够 spike。
  - **缓解**：仅用于可复现语料，不用于安全场景。

## 9. Verification Plan

```bash
# typecheck + build
cargo check --workspace
cargo build -p contextforge-bench

# unit-test（按 TEST-ID）
cargo test -p contextforge-bench

# regression — 既有不退化
cargo test --workspace
go test ./...

# end-to-end smoke（AC6）
cargo run -p contextforge-bench -- --backend noop --n 500 --dim 32 --seed 1
# expect: exit 0 + 打印 MeasureReport JSON，recall_at_5=0.0 recall_at_10=0.0

# evidence 渲染 smoke（AC7）
cargo run -p contextforge-bench -- --backend noop --n 200 --dim 16 --seed 1 --out /tmp/noop-spike.md
# expect: /tmp/noop-spike.md 含 5 维表

# ADR-014 D2 lint
bash scripts/spec_drift_lint.sh --touched master
# expect: 0 unannotated hits
```

## 10. Completion Notes (s2v 6 项标准)

> 实施完成后按 standard.md §8.3 回填（替换 `<TBD-after-impl>`）；Status: Ready → In Progress → Done。

- **完成日期**：2026-05-30
- **改动文件**：
  - `Cargo.toml`（workspace `members = ["core", "bench"]`）
  - `bench/Cargo.toml` + `bench/src/{lib,corpus,measure,rss,runner,backends,main,tests}.rs`（新增）
  - `bench/README.md` + `scripts/spike_vector_backends.sh` + `docs/spikes/_template.md` + `test/fixtures/spike/dogfood-contextforge.jsonl`（新增）
- **commit 列表**：见本 task PR（分支 `feat/task-18.2-spike-harness`）；合入后以 master merge commit 为准（不在此钉 pre-merge SHA，避免漂移）
- **§9 Verification 结果**：
  - build: ✅ `cargo build -p contextforge-bench` exit 0
  - unit-test: ✅ `cargo test -p contextforge-bench` 6 passed / 0 failed
  - regression: ✅ `cargo test --workspace` 202 passed / 0 failed（196 既有 + 6 bench）；`go test ./...` 全 PASS（Go 未触及）
  - smoke (AC6): ✅ `spike --backend noop --n 500 --dim 32` → recall@5=0 / recall@10=0、无 panic、exit 0
  - evidence (AC7): ✅ `--out` 写出含 5 维表的 md
  - D2 lint (AC9): ✅ `bash scripts/spec_drift_lint.sh --touched master` 0 未标注命中（延后行为均 [SPEC-DEFER]/[SPEC-OWNER]）
  - 平台注：本机 Windows，idle/index RSS 为 `None`（Linux-only 采样，R3 [SPEC-DEFER:phase-future.rss-sampling-macos-windows]）
- **剩余风险 / 未做项**：真 backend（18.3-18.6）/ ADR-023 选型（18.7）/ eval（18.8）按序后续；合成 100k 语料按需生成不入 git；非 Linux RSS 采样后置
- **下游 task 影响**：task-18.3-18.6（接入真 backend 到 `bench/src/backends.rs` 跑 evidence）、task-18.7（消费 5 维数据选型）
