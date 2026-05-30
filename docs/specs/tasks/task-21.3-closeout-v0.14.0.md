# Task `21.3`: `closeout-v0.14.0 — internal/eval Report 加 hybrid/reranked 召回列 + internal/cli/eval.go --rerank flag + scripts/console_smoke.sh hybrid/rerank opt-in 真实断言 + v0.14.0 release docs + ADR-025/026 据真实 eval ratify + phase-21 §6 闭合 + adapter`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 21 (retrieval-quality)
**Dependencies**: task-21.1（hybrid scoring 融合 + `Retriever::search_hybrid` + `SearchResult.hybrid_score` + proto `hybrid_score=15`/`hybrid=8`）/ task-21.2（`Reranker` trait + `IdentityReranker` + `CrossEncoderReranker` feature-gated + `Retriever::with_reranker`）/ task-19.4（smoke 30-step + `internal/cli/eval.go --semantic` flag 范式）/ task-18.8（`internal/eval` `SummarizeHybrid` + `MeetsRecallGate` + `SemanticRecall@K` add-only 范式）/ ADR-025（hybrid-scoring-fusion，本 phase 新 Proposed）/ ADR-026（reranker-provider，本 phase 新 Proposed）/ ADR-006（recall gate）/ ADR-013（禁伪造）/ ADR-014 D1-D5（第十二次激活）

## 1. Background

task-21.1 已落地 hybrid scoring 融合（`Retriever::search_hybrid` + `retrieval_method = "hybrid"` + `hybrid_score`）；task-21.2 已落地 `Reranker` trait + 确定性 `IdentityReranker` + feature-gated `CrossEncoderReranker`。本 task 收口 Phase 21：把 hybrid / reranked 召回接进 eval 报告与 CLI（仿 task-18.8 `SummarizeHybrid` + task-19.4 `--semantic` flag 范式），升级 smoke 对 hybrid / rerank opt-in 路径做真实断言，产出 v0.14.0 release docs，据真实 eval 数据 ratify ADR-025 / ADR-026（或如实记录维持 Proposed），闭合 phase-21 §6 AC，更新 s2v-adapter。

承 v0.12.0 / v0.13.0 收口模式（task-19.7 / task-20.3）：closeout = eval/smoke final + release evidence/artifacts + README/RELEASE_NOTES + ADR 状态 + adapter；tag push 经用户授权后由 release.yml 触发 + post-tag-push backfill。

## 2. Goal

`internal/eval/eval.go` add-only `Report` 字段（hybrid / reranked 召回列，仿既有 `SemanticRecall@K` add-only 字段）+ `SummarizeHybrid` 扩展容纳 hybrid/reranked pass（add-only，BM25/semantic-only 时 byte-equivalent）。`internal/cli/eval.go` add-only `--rerank` flag（仿 `--semantic`：再跑一趟重排 pass 并报告 reranked 召回 + gate 行；off → 不变）。`scripts/console_smoke.sh` 升级：既有 step 不退化 + 新增/升级 step 对 hybrid（`?hybrid=true` 或等价入口）/ rerank opt-in 路径做真实语义断言（响应 `retrieval_method` 反映 hybrid 路径 / result item 含 `hybrid_score` provenance）。新增 `docs/releases/v0.14.0-{evidence,artifacts}.md` + `README.md` v0.14 段 + `RELEASE_NOTES.md` v0.14.0 段。`docs/decisions/adr-025-hybrid-scoring-fusion.md` + `docs/decisions/adr-026-reranker-provider.md` 据真实 eval 数据 Status `Proposed → Accepted`（或受阻如实记录维持 Proposed，ADR-013）。`docs/specs/phases/phase-21-retrieval-quality.md` §6 AC1-5 全 `[x]` + Status `Draft → Done`。`docs/s2v-adapter.md` Phase 21 `Draft → Done` + Tasks `0 → 3` + ADR-025/026 索引 + BDD phase-21 feature 行。ADR-014 D1-D5 第十二次激活 closeout PR body。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `internal/eval/eval.go`**：`Report` add-only hybrid / reranked 召回字段（仿既有 `SemanticEvaluated` / `SemanticRecallAt5/10`：如 `HybridEvaluated` / `HybridRecallAt5/10` / `RerankedEvaluated` / `RerankedRecallAt5/10`，json tag add-only）+ `SummarizeHybrid` 扩展（或新增 summarize 入口）容纳 hybrid/reranked pass，无 pass 时 byte-equivalent 既有输出。
- **修改 `internal/cli/eval.go`**：add-only `--rerank` flag（仿 `--semantic` @ 134：`fs.Bool("rerank", false, ...)`）+ rerank pass（`evalSearchPass` 复用，喂 rerank 请求字段）+ 报告 reranked 召回行（仿 semantic 输出 @ 75-79）。
- **修改 `internal/eval/eval_test.go` + `internal/cli/eval_test.go`**：Go 测试断言 hybrid/reranked Report 字段 + `--rerank` flag 解析 + SummarizeHybrid add-only 不退化（off 时 byte-equivalent）。
- **修改 `scripts/console_smoke.sh`**：升级注释段 + 新增/升级 step 对 hybrid / rerank opt-in 路径真实断言（`retrieval_method` 反映 hybrid / result item 含 `hybrid_score`）；既有 step 标号 / 断言不动语义；终态 marker 保留。
- **新增 `docs/releases/v0.14.0-evidence.md` + `docs/releases/v0.14.0-artifacts.md`**：承 v0.13.0 模板（合入记录 / S2V 状态 / 验证证据 / CI / AC 达成 / 平台矩阵 / upgrade-rollback / ADR-014 record / tag+镜像 SHA backfill 段）。
- **修改 `README.md`**：v0.14 段——hybrid scoring + reranker Quick start（opt-in 用法 + 默认仍 BM25 baseline 说明）。
- **修改 `RELEASE_NOTES.md`**：v0.14.0 段（task 表 + add-only contract 说明 + upgrade/rollback + reranker 真实质量诚实口径）。
- **修改 `docs/decisions/adr-025-hybrid-scoring-fusion.md` + `docs/decisions/adr-026-reranker-provider.md`**：据真实 eval 数据 Status `Proposed → Accepted`（add-only ratification 段，仿 ADR-023 Amendment）或受阻如实记录维持 Proposed（ADR-013）。
- **修改 `docs/specs/phases/phase-21-retrieval-quality.md`**：§6 AC1-5 全 `[x]` + Status `Draft → Done` + §8 DoD 勾选。
- **修改 `docs/s2v-adapter.md`**：Phase 21 行 `Draft → Done` + `Tasks 0 → 3` + Task 索引 21.1-21.3 Done + ADR-025/026 索引行 + BDD phase-21 feature 行。
- **新增 `test/features/phase-21-retrieval-quality.feature`**（≥3 scenario）。
- **新增 `docs/spikes/phase-21-hybrid-recall.md`**：记 hybrid / reranked 真实召回对比（real run / deterministic / 受阻三态如实标，喂 ADR-025/026 ratify）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **hybrid scoring 融合实现** [SPEC-OWNER:task-21.1-hybrid-scoring]：本 task 在 eval/smoke 验证它，不实现。
- **reranker trait + 实现** [SPEC-OWNER:task-21.2-reranker-pipeline]：本 task 引用其交付。
- **real cross-encoder 真实质量数值在受阻平台复跑** [SPEC-DEFER:phase-future.reranker-real-quality]：本 task 如实记录受阻态，不伪造。
- **v0.14.0 tag push 实际执行**：closeout PR 合入后，据用户明确授权 push `v0.14.0` annotated tag 触发 release.yml（沿用历史 release 流；用户授权前不 push）。post-tag-push backfill 填实 tag SHA / run ID / 镜像 digest 由独立 backfill PR 承接（仿 v0.10/v0.11/v0.12/v0.13 pattern）。
- **console-api `?hybrid=true` / `?rerank=true` 转发** [SPEC-DEFER:phase-future.console-api-hybrid-forward] / [SPEC-DEFER:phase-future.console-api-rerank-forward]：core 数据面 hybrid/rerank 已落地，console-api Go 转发承 Phase 20 范式，后续版本贯通。
- **Console UI 重排 / 融合 explain** [SPEC-OWNER:phase-future.console-semantic-explain]：跨仓库，本 task 仅在 release docs 记数据通路就绪 + 通知项。
- **remote embedding provider / embedding 缓存** [SPEC-DEFER:phase-future.embedding-provider-remote] / [SPEC-DEFER:phase-future.embedding-cache]：v0.15.0 / Phase 22。

## 4. Actors

- **主 agent**：实施 + PR 主理 + closeout 决策。
- **`internal/eval/eval.go`（add-only hybrid/reranked Report 字段）**：eval 报告面，仿 task-18.8 `SemanticRecall@K` add-only。
- **`internal/cli/eval.go`（add-only `--rerank` flag）**：CLI eval 入口，仿 `--semantic`。
- **`scripts/console_smoke.sh`**：端到端 C1 兜底 smoke，本 task 升级 hybrid/rerank 真实断言。
- **`docs/releases/` + `README.md` + `RELEASE_NOTES.md`**：v0.14.0 release 文档面。
- **`docs/decisions/adr-025-*.md` + `adr-026-*.md`**：本 phase 新 ADR，本 task 据真实 eval ratify。
- **`docs/s2v-adapter.md`**：Phase/task/ADR/BDD 索引。
- **用户**：v0.14.0 tag push 授权（stop-condition）。

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/eval/eval.go`（`Report` struct @ 55 + `SemanticEvaluated`/`SemanticRecallAt5/10` add-only @ 66-77 + `SummarizeHybrid` @ 310 + `MeetsRecallGate` @ 341 + `SemanticRecallAtK` @ 291）
- `internal/cli/eval.go`（`--semantic` flag @ 134 + `evalSearchPass` @ 104 + semantic 报告输出 @ 75-79 + `evalRunOpts.Semantic` @ 19）
- `scripts/console_smoke.sh`（既有 step + 终态 marker）+ `docs/specs/tasks/task-19.4-smoke-v9.md` §10（smoke 诚实口径 + WSL step-26 quirk）
- `docs/specs/tasks/task-19.7-closeout-v0.12.0.md` + `task-20.3-closeout-v0.13.0.md`（closeout 模板 + tag/backfill pattern）
- `docs/releases/v0.12.0-{evidence,artifacts}.md`（release 文档结构 + §backfill 段）
- `docs/specs/tasks/task-21.1-hybrid-scoring.md` + `task-21.2-reranker-pipeline.md`（本 phase 上游交付）
- `docs/decisions/adr-025-hybrid-scoring-fusion.md` + `adr-026-reranker-provider.md`（本 phase ADR）+ `docs/decisions/adr-023-vector-backend-default.md`（数据驱动 ratify Amendment 范式）+ `adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）
- `docs/s2v-adapter.md` §Phase / §Task / §ADR / §BDD 索引

### 5.2 关键设计 — eval/smoke hybrid/rerank 真实断言 + 诚实口径

- eval `Report` add-only hybrid/reranked 字段：仿 task-18.8 `SemanticRecall@K` add-only 范式——无 hybrid/rerank pass 时 `*Evaluated=false`，输出 byte-equivalent 既有 BM25/semantic-only Report（向后兼容）。
- CLI `--rerank`：仿 `--semantic` 再跑一趟 rerank pass（喂 rerank 请求字段，经 task-21.2 `with_reranker` 路径）+ 报告 reranked 召回 + gate 行。确定性 `IdentityReranker` 缺省下 rerank 路径产稳定序（管道成形）；real `CrossEncoderReranker` 质量数值口径属 spike doc。
- smoke 升级 step：hybrid（`?hybrid=true` 或等价 CLI/REST 入口）断言响应 `retrieval_method` 反映 hybrid 路径 + result item 含 `hybrid_score` provenance；ADR-013：smoke 断言**hybrid/rerank 通路生效 + provenance 成形**，不预判具体召回/质量数值（数值口径属 spike）。
- 既有 step + 终态 marker 不动；release docs 诚实口径（承 task-19.7/20.3 §10）：deterministic 默认 / real 本地 / 受阻三态如实标；WSL step-26 daemon restart quirk 如实记录（非 Phase 21 回归）。
- ADR-025/026 ratify 仅在真实 eval 数据下（ADR-013：据真实非合成）；hybrid 融合策略真实召回对比驱动 ADR-025；real cross-encoder 真实质量驱动 ADR-026，受阻则维持 Proposed 如实记录。

### 5.3 不变量

- eval `Report` add-only：`--semantic`/无 flag 时输出 byte-equivalent 既有（hybrid/reranked 字段 add-only，off 时不出现/为零值）。
- `--rerank` 缺省 false：既有 `eval run` 调用不变。
- smoke 既有 step 不退化（仅新增/升级 hybrid/rerank step + 注释）。
- ADR-025/026 ratify 据真实非合成数据（ADR-013）；受阻则诚实记录维持 Proposed，不强 ratify。

## 6. Acceptance Criteria

- [ ] **AC1**: `internal/eval` `Report` add-only hybrid/reranked 字段 + `SummarizeHybrid` 扩展容纳 hybrid/rerank pass，无 pass 时 byte-equivalent 既有输出；`internal/cli/eval.go` add-only `--rerank` flag 解析 + reranked 召回报告行；确定性 wiring `go test` 可断言 — verified by **TEST-21.3.1**
- [ ] **AC2**: `scripts/console_smoke.sh` 通过 `bash -n`（exit 0）；新增/升级 step 对 hybrid / rerank opt-in 路径真实断言（`retrieval_method` 反映 hybrid + result item `hybrid_score` provenance）；既有 step 不退化；终态 marker 保留 — verified by **TEST-21.3.2**
- [ ] **AC3**: v0.14.0 release docs 齐备（`docs/releases/v0.14.0-{evidence,artifacts}.md` + `README.md` v0.14 段 + `RELEASE_NOTES.md` v0.14.0 段，含 §tag-backfill 待回填段）；ADR-025/026 据真实 eval 数据 Status `Proposed → Accepted`（或受阻如实记录维持 Proposed，ADR-013）；phase-21 §6 AC1-5 全 `[x]` + Status `Draft → Done`；adapter Phase 21 `Draft → Done` + Tasks `0 → 3` + ADR-025/026 索引 + BDD phase-21 行 — verified by **TEST-21.3.3**
- [ ] **AC4**: 既有不退化 — `go test ./...` + `cargo test --workspace` 全 PASS — verified by **TEST-21.3.4** + §10
- [ ] **AC5**: ADR-014 D1-D5 第十二次激活全通过（D1 phase§6↔task§6 mapping 表 + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-20 不溯改）— verified by **TEST-21.3.5** + 本 closeout PR body

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-21.3.1 | eval Report add-only hybrid/reranked + `--rerank` flag + SummarizeHybrid byte-equivalent | `internal/eval/eval_test.go` + `internal/cli/eval_test.go` | Planned |
| TEST-21.3.2 | smoke `bash -n` + hybrid/rerank step 真实断言 + 既有 step 不退化 | `internal/cli/smoke_syntax_test.go` + `scripts/console_smoke.sh` | Planned |
| TEST-21.3.3 | v0.14.0 release docs 齐备 + ADR-025/026 ratify + phase-21 闭合 + adapter | `docs/releases/v0.14.0-*.md` + ADR-025/026 + phase-21 spec + s2v-adapter | Planned |
| TEST-21.3.4 | `go test ./...` + `cargo test --workspace` 0 failed | 全 Go + Rust | Planned |
| TEST-21.3.5 | ADR-014 D1-D5 record（mapping + D2 lint） | 本 closeout PR body | Planned |

## 8. Risks

- **R1（高）ADR-026 ratify 依赖 real cross-encoder 真实质量，受阻则不可 ratify**（承 phase-21 §7 R2 / task-21.2 §8 R1）：cross-encoder 模型 / 平台受阻 → 真实质量数值不可得。
  - **缓解**：ADR-025（hybrid 融合）据 hybrid 真实召回对比可在确定性+真实 dogfood eval 下 ratify；ADR-026（reranker）若 real 质量受阻则诚实记录维持 Proposed，不强 ratify（ADR-013）；spike doc 三态如实标。
- **R2（中）smoke hybrid/rerank 真实断言在 WSL 受 step-26 quirk 阻**（承 task-19.4 §10 / v0.12.0 evidence §3b）：既有 step-26 daemon restart 在非交互 WSL bash 停住。
  - **缓解**：hybrid/rerank step 真实断言以合规 Linux host / CI / release smoke 复跑定稿；本地 WSL 受阻如实记录（非 Phase 21 回归），不伪造终态 marker。
- **R3（低）v0.14.0 tag 误在用户授权前 push**：release stop-condition。
  - **缓解**：closeout PR 仅备齐 release docs；tag push 经用户明确授权后单独执行（沿用历史 release 流）。

## 9. Verification Plan

```bash
# eval/CLI：hybrid/reranked Report 字段 + --rerank flag + byte-equivalent
go test ./internal/eval/... ./internal/cli/... -run 'TestTask213|TestTask188|TestTask194' -v

# smoke 语法 + step 标号
bash -n scripts/console_smoke.sh

# 既有不退化
go test ./...
cargo test --workspace

# 端到端 REAL smoke（合规 Linux / WSL）
bash scripts/console_smoke.sh        # 期望末行 CONSOLE_REAL_SMOKE_EXIT=0

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按 6 项回填，含 hybrid 真实召回对比（ADR-025 ratify 依据）+ real cross-encoder 真实质量数值或受阻 defer 记录（ADR-026 ratify 或维持 Proposed，ADR-013 数据源声明）+ smoke 实跑结论（合规环境 `CONSOLE_REAL_SMOKE_EXIT=0` / WSL 受阻如实记录）+ v0.14.0 tag/backfill 状态（用户授权后）。
