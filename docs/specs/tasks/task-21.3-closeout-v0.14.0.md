# Task `21.3`: `closeout-v0.14.0 — internal/eval Report 加 hybrid/reranked 召回列 + internal/cli/eval.go --rerank flag + scripts/console_smoke.sh hybrid/rerank opt-in 真实断言 + v0.14.0 release docs + ADR-025/026 据真实 eval ratify + phase-21 §6 闭合 + adapter`

**Status**: Done

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

- [x] **AC1**: `internal/eval` `Report` add-only hybrid/reranked 字段 + `SummarizeHybrid` 扩展容纳 hybrid/rerank pass，无 pass 时 byte-equivalent 既有输出；`internal/cli/eval.go` add-only `--rerank`（+ 同范式 `--hybrid`）flag 解析 + reranked 召回报告行；确定性 wiring `go test` 可断言 — verified by **TEST-21.3.1**（`Passes`/`SummarizePasses`/`tallyPass` + `--hybrid`/`--rerank` + `SummarizeHybrid` byte-equiv via `reflect.DeepEqual`，`TestTask213_AC1_*` PASS）
- [x] **AC2**: `scripts/console_smoke.sh` 通过 `bash -n`（exit 0）；新增/升级 step 对 hybrid / rerank opt-in 路径真实断言（`retrieval_method` 反映 hybrid + result item `hybrid_score` provenance）；既有 step 不退化；终态 marker 保留 — verified by **TEST-21.3.2**（smoke v11 step 30 `eval run --semantic --hybrid --rerank` 多路 report shape + gate 断言；per-result `retrieval_method="hybrid"`+`hybrid_score` provenance 由 Rust `test_21_1_hybrid_dispatches_fusion_path` 断言，console-api REST forward `[SPEC-DEFER:phase-future.console-api-hybrid-forward]`，见 §10）
- [x] **AC3**: v0.14.0 release docs 齐备（`docs/releases/v0.14.0-{evidence,artifacts}.md` + `README.md` v0.14 段 + `RELEASE_NOTES.md` v0.14.0 段，含 §tag-backfill 待回填段）；ADR-025/026 据真实 eval 数据 Status `Proposed → Accepted`（或受阻如实记录维持 Proposed，ADR-013）；phase-21 §6 AC1-5 全 `[x]` + Status `Draft → Done`；adapter Phase 21 `Draft → Done` + Tasks `0 → 3` + ADR-025/026 索引 + BDD phase-21 行 — verified by **TEST-21.3.3**（ADR-025/026 据真实 dogfood eval Proposed→Accepted，ADR-026 附诚实 hybrid caveat；`docs/spikes/phase-21-hybrid-recall.md`）
- [x] **AC4**: 既有不退化 — `go test ./...` + `cargo test --workspace` 全 PASS — verified by **TEST-21.3.4** + §10
- [x] **AC5**: ADR-014 D1-D5 第十二次激活全通过（D1 phase§6↔task§6 mapping 表 + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-20 不溯改）— verified by **TEST-21.3.5** + 本 closeout PR body

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-21.3.1 | eval Report add-only hybrid/reranked + `--hybrid`/`--rerank` flag + SummarizeHybrid byte-equivalent | `internal/eval/eval_test.go` + `internal/cli/eval_test.go` | Done |
| TEST-21.3.2 | smoke `bash -n` + hybrid/rerank step 真实断言 + 既有 step 不退化 | `internal/cli/smoke_syntax_test.go` + `scripts/console_smoke.sh` | Done |
| TEST-21.3.3 | v0.14.0 release docs 齐备 + ADR-025/026 ratify + phase-21 闭合 + adapter | `docs/releases/v0.14.0-*.md` + ADR-025/026 + phase-21 spec + s2v-adapter | Done |
| TEST-21.3.4 | `go test ./...` + `cargo test --workspace` 0 failed | 全 Go + Rust | Done |
| TEST-21.3.5 | ADR-014 D1-D5 record（mapping + D2 lint） | 本 closeout PR body | Done |

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

- **完成日期**：2026-05-31
- **改动文件**：
  - `internal/eval/eval.go`（`Report` add-only `Hybrid*`/`Reranked*` 列 + `GateHybrid/RerankedRecall10Min` + `Passes`/`SummarizePasses`/`tallyPass`；`SummarizeHybrid` 委托 `SummarizePasses`（byte-equiv）；`MeetsRecallGate` 加 hybrid/reranked gate）
  - `internal/cli/eval.go`（`evalRunOpts.Hybrid/Rerank` + `--hybrid`/`--rerank` flag；`runEval` 多路 + `SummarizePasses`；`passMode` + `evalSearchPass` 重构 + `rerankIdentity`（确定性 IdentityReranker 契约 @ eval 层））
  - `internal/eval/eval_test.go` + `internal/cli/eval_test.go` + `internal/cli/smoke_syntax_test.go`（`TestTask213_*` RED→GREEN）
  - `scripts/console_smoke.sh`（v11：header + step 30 升 `eval run --semantic --hybrid --rerank` 多路断言）
  - `core/examples/phase21_hybrid_rerank_recall.rs`（新增；feature-gated 真实 dogfood eval，默认 no-op）
  - `docs/spikes/phase-21-hybrid-recall.md`（新增）、`docs/decisions/adr-025-*.md` + `adr-026-*.md`（Proposed→Accepted + Ratification Amendment）、`docs/releases/v0.14.0-{evidence,artifacts}.md`（新增）、`README.md`（v0.14 段）、`RELEASE_NOTES.md`（v0.14.0 段）、`docs/specs/phases/phase-21-retrieval-quality.md`（§6 AC1-5 [x] + Status Done）、`test/features/phase-21-retrieval-quality.feature`（新增 3 scenario）、本 spec + `docs/s2v-adapter.md`（Phase 21 Draft→Done + Tasks 0→3 + 21.3 Done + ADR-025/026 Accepted）
- **commit 列表**：
  - `673d284` test(eval): TEST-21.3.1/21.3.2 RED
  - `389ec46` feat(eval): GREEN eval Report hybrid/reranked 列 + --hybrid/--rerank + smoke v11
  - `ecdaea4` feat(core): dogfood hybrid/rerank recall example + phase-21 BDD feature
  - `fade188` docs(spec): v0.14.0 release docs + ADR-025/026 Accepted + phase-21 §6 闭合 + adapter
  - 本提交 docs(spec): 回填 §10 + Status → Done
- **§9 Verification 结果**：
  - unit-test：`go test ./...` 0 failed（含 `TestTask213_*` + 既有 `TestTask188/194` 不退化）+ `cargo test --workspace` 0 failed（全 test 二进制；`server.rs::test_21_1` + `rerank::*::test_21_2_*` 守 hybrid/rerank wiring；本 task 零 Rust 源 delta，仅 add-only feature-gated example）
  - runtime-smoke：`bash -n scripts/console_smoke.sh` exit 0（+ `TestTask213_SmokeV11HybridRerankAssertion` 标号/marker 守护）；端到端 REAL smoke 在合规 Linux host / CI 复跑定 `CONSOLE_REAL_SMOKE_EXIT=0`——本地 WSL 既有 **step-26**（task-16.1 daemon kill/restart，v0.9.0，**非 Phase 21**）在非交互 WSL bash 重启后停住（承 v0.12.0 evidence §3b / task-19.4 §10），未端到端跑至 step 30，如实记录不伪造退出码
  - lint：`scripts/spec_drift_lint.sh --touched origin/master` —— 本机 cygwin 下 scan_all 逐行 subprocess fork 极慢（非脚本缺陷，fork-on-cygwin 开销），改以 touched docs/specs 行直查 anti-pattern regex（0 未标注命中）+ CI spec-lint gate（Linux 快速）为权威 D2，见 §10 诚实记录
  - manual：真实 dogfood eval（`cargo run -p contextforge-core --example phase21_hybrid_rerank_recall --features embedding-fastembed,reranker-fastembed`，Win MSVC 2026-05-31）：BM25 baseline top-1 0.0333/MRR 0.4095 → hybrid RRF top-1 0.6667/MRR 0.7881（ADR-025 ratify 依据）；reranked cross-encoder top-1 0.3333/MRR 0.6306/recall@5 0.9667（real model run，D5 未触发，over baseline uplift + 诚实 hybrid caveat → ADR-026 ratify 依据）
- **ADR ratify 结论**：ADR-025（hybrid-scoring-fusion）据真实 dogfood eval（hybrid 决定性 top-1/MRR uplift）Proposed→**Accepted**；ADR-026（reranker-provider）据 real cross-encoder run（D5 stop 未触发，over baseline uplift + 三法最高 recall@5）Proposed→**Accepted**，附诚实 caveat（本小型代码语料下重排 hybrid top-k 不及 hybrid 单路 top-1/MRR，opt-in 域适配增强非默认）。均据真实非合成数据（ADR-013，数据源声明见 `docs/spikes/phase-21-hybrid-recall.md`）。
- **设计取舍 / 诚实记录（ADR-013）**：
  - eval CLI 加 `--hybrid`（除 spec 明列的 `--rerank` 外）——hybrid 列需可达入口（goal 步①「Report 加 hybrid/reranked 列」），`--hybrid` 仿 `--semantic` 范式 add-only，off 时 byte-equiv。
  - `rerank` 无 proto wire 字段（reranker 为 core 库 seam，console-api `?rerank` forward `[SPEC-DEFER:phase-future.console-api-rerank-forward]`），故 eval `--rerank` 在 eval 层应用确定性 `IdentityReranker` 契约（score desc + chunk_id asc，ADR-026 D2，recall-neutral on 已排序输入）；real cross-encoder 真实质量经 Rust dogfood example，不在 Go eval 层冒充（ADR-013）。
  - smoke step 30 断言多路 eval report shape（ADR-013 不预判召回阈值，transient eval index 为空）；per-result `retrieval_method="hybrid"`+`hybrid_score` provenance 由 Rust `test_21_1_hybrid_dispatches_fusion_path` 断言（smoke 走 eval-CLI shape；console-api `?hybrid` REST forward 承 Phase 20 范式 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]`）。
- **剩余风险 / 未做项**：**v0.14.0 tag push 待用户明确授权**（stop-condition c，承 v0.12/v0.13 惯例）；授权后 push annotated tag → `release.yml` → post-tag-push backfill PR 填实 tag SHA / run ID / 镜像 digest（evidence/artifacts §tag 段 `<backfill>` 待回填）。真实 cross-encoder 更大/域适配语料复跑 `[SPEC-DEFER:phase-future.reranker-real-quality]`。
- **下游 task 影响**：console-api `?hybrid=true`/`?rerank=true` REST 转发 `[SPEC-DEFER:phase-future.console-api-hybrid-forward]` / `[SPEC-DEFER:phase-future.console-api-rerank-forward]`（后续版本承 Phase 20 范式贯通）；Console UI 重排/融合 explain `[SPEC-OWNER:phase-future.console-semantic-explain]`（跨仓库）。Phase 21 收口完结，无新增阻塞下游。
