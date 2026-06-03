# Task `30.3`: `closeout-v0.23.0 — smoke v20 step 39 + v0.23.0 release docs + ADR-035 据 D1-D5 真实 ratify（D1 真分词器 feature-gated / D2 双站点注册对称 / D3 tokenizer-default-on 评估·迁移工具 / D4 扩 CJK golden recall delta 真实回填 / D5 默认构建 baseline 不变）+ ADR-029 add-only Amendment + phase-30 §6 闭合`

**Status**: Done

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 30 (cjk-true-segmenter)
**Dependencies**: task-30.1（真分词器 analyzer feature-gated + 双站点注册对称）/ task-30.2（tokenizer-default-on 评估 + 既有索引迁移工具 + 扩 CJK golden recall delta）全 Done / ADR-035（cjk-true-segmenter-and-tokenizer-default，本 task ratify）/ ADR-029（code-and-cjk-tokenizer-and-eval-hardening，本 task add-only Amendment）/ ADR-008（optional dep add-only，若实施时新增 jieba-rs/lindera 经主 agent R7 chore）/ ADR-004（默认构建 0-dep / 默认 analyzer baseline 不变）/ ADR-012（tag/release outward-facing 须用户显式授权）/ ADR-013（禁伪造凭据 / recall 数值红线）/ ADR-014 D1-D5（第二十一次激活）

## 1. Background

Phase 30 两个实现 task 全 Done：30.1（真 CJK 词分词器 analyzer 落在新 `cjk-segmenter` feature 后、默认 off → 0 新 dep；PARALLEL analyzer 名 + 在 `IndexSession::open_with_tokenizer` 与 `Retriever::open_with_config` 双站点注册保对称；沿用 Phase 24 bigram 作 0-dep fallback）/ 30.2（评估 tokenizer 从 opt-in 翻为 default-on、配套既有索引 reindex/migration 工具 + `RetrieverConfig.tokenizer` 路由接线或 schema-driven 对称文档化；扩 `golden-semantic.jsonl` CJK case 后跑真实 before/after/segmenter recall delta）。本 task 收口 v0.23.0：smoke v20 step 39 + release docs + ADR-035 据真实结果逐 D 项 ratify + ADR-029 add-only Amendment + phase §6 闭合 + adapter + feature。

## 2. Goal

据 30.1/30.2 **真实分词单测 / 真实扩 CJK golden recall delta** 收口 v0.23.0：ADR-035 `Proposed → Accepted`（逐 D 项如实——D1 真分词器 feature-gated 落地、D2 双站点注册对称、D3 tokenizer-default-on 评估结论 + 迁移工具如实，full default flip 太重则诚实延后、D4 扩 CJK golden 真实 recall delta 回填、D5 默认构建 baseline 不变）；ADR-029 add-only Amendment（记录真分词替/补 bigram + tokenizer-default-on 评估结果，不溯改正文 D1-D5，ADR-014 D5）；phase-30 §6 AC1-4 置 `[x]` + Status Done；smoke v20 step 39（默认构建 init baseline + 默认 tokenization 不变；`cjk-segmenter` 为 feature-gated 无 console-api 运行时面）；release docs（evidence/artifacts/README/RELEASE_NOTES，tag/run/digest 用 backfill 待回填）；adapter（Phase 30 Done + Tasks 3 + ADR-035 Accepted + feature 行）。**真实 v0.23.0 tag/release 须用户显式授权**（不自行 tag，ADR-012）。

## 3. Scope

### In Scope（计划交付）

- `scripts/console_smoke.sh`——banner v19→v20 + v20 changelog 块 + step 39（`[39/39]`，文档/状态步：断言默认构建 init baseline + 默认 tokenization 不变；`cjk-segmenter` feature-gated 无 console-api 运行时面；既有 step 不退化 + denominator 不溯改 ADR-014 D5）。当前线上脚本为 `[37/37]`；Phase 29 closeout（task-29.4，尚未实施）规划 `[38/38]`；故 Phase 30 顺位规划 `[39/39]`。
- `internal/cli/smoke_syntax_test.go`——新增 `Test` 断言 `[39/39]` + 标记 + 无回归既有 `[37/37]` / `[38/38]`（denominator 不溯改 ADR-014 D5）。
- 新增 `docs/releases/v0.23.0-evidence.md` + `docs/releases/v0.23.0-artifacts.md`（tag SHA / run id / digest 用 angle-bracket backfill 待回填）+ `README.md` v0.23 段 + `RELEASE_NOTES.md` v0.23.0 段。
- `docs/decisions/adr-035-cjk-true-segmenter-and-tokenizer-default.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification（v0.23.0 / task-30.3）` 节（逐 D 真实依据；重词典 dep / 小语料受阻维度据已达维度 ratify 部分，不强 ratify）。
- `docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md`——append `## Amendment (Phase 30 / v0.23.0)`（记录真分词器升级 + tokenizer-default-on 评估结果；若实施新增 dep 附 ADR-008 add-only 备注；不溯改正文 ADR-014 D5）。
- `docs/specs/phases/phase-30-cjk-true-segmenter.md`——Status Draft→Done + §6 AC1-4 `[x]`（逐维如实，受阻维度标注）。
- `docs/s2v-adapter.md`——§Phase 30 行 + §Task +30.1/30.2/30.3 行 + §ADR 035 Accepted 行 + §BDD +phase-30 行。
- `test/features/phase-30-cjk-true-segmenter.feature`——已于 Phase 30 规划阶段创建（本 task 不重复创建）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 真实 v0.23.0 tag push + release run（cosign 真签 + GHCR 推送）[SPEC-OWNER:user-authorized-release]——outward-facing 不可逆须用户授权（ADR-012）；post-tag-push backfill PR 填实 tag SHA / release run id / 镜像 digest。
- tokenizer 全量 default-on flip（若 30.2 评估认定迁移太重）`[SPEC-DEFER:phase-future.tokenizer-default-on]`——保 opt-in + 迁移工具，default flip 诚实延后。
- 重词典分词器（lindera 嵌入 IPADIC/ko-dic）`[SPEC-DEFER:phase-future.heavy-dict-segmenter]` / 多语种分词（日/韩独立词典）`[SPEC-DEFER:phase-future.multilang-segmenter]` / 大规模 CJK 语料 recall 基准 `[SPEC-DEFER:phase-future.large-cjk-corpus-eval]`。

## 4. Actors

- 主 agent（ADR-012 自治；真实 release 须用户授权）
- closeout 文档集（smoke / release docs / ADR / ADR-029 Amendment / phase spec / adapter / feature）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-30-cjk-true-segmenter.md §6/§8`（AC + DoD）
- `docs/decisions/adr-035-cjk-true-segmenter-and-tokenizer-default.md`（§D1-D5 + Consequences Ratification 条款）
- task-30.1 §10 + task-30.2 §10（真实分词单测 + 真实扩 CJK golden recall delta + tokenizer-default-on 评估结论）
- `core/src/indexer/mod.rs:364-377`（`build_code_cjk_analyzer` seam + `register_code_cjk` 双站点注册）+ `core/src/retriever/mod.rs:99` / `:250`（vestigial `RetrieverConfig.tokenizer` + query 站点注册）
- `docs/releases/v0.21.0-evidence.md` / `docs/releases/v0.21.0-artifacts.md`（模板）

### 5.2 关键设计 — 诚实 per-D ratify + backfill 待回填

- ADR-035 ratify **逐 D 项据真实结果**：D1 真分词器 feature-gated 落地（`cjk-segmenter` off → 0 新 dep 默认构建据 TEST-30.1.3 真实验证）/ D2 双站点注册对称（`:442` + `:250`，据 TEST-30.1.2 round-trip 真实命中）/ D3 tokenizer-default-on 评估 + 迁移工具（据 TEST-30.2.2；full default flip 若太重则诚实延后 `[SPEC-DEFER:phase-future.tokenizer-default-on]`，不伪造已 flip）/ D4 扩 CJK golden 真实 recall delta（据 TEST-30.2.1，数值真实跑出后回填、小语料不外推）/ D5 默认构建 baseline 不变（据 TEST-30.1.3）。重词典 dep / 大语料受阻维度据已达维度 ratify **部分**，不为「全 Accepted」伪造重词典分词或大语料 recall 已验证（ADR-013）。
- tag SHA / release run id / 镜像 digest 在 release docs 用 angle-bracket backfill 待回填——真实 v0.23.0 tag/release 是 closeout 合入后的**用户授权步**，post-tag-push backfill PR 填实（承 v0.8–v0.21 pattern）。
- smoke step 39 是文档/状态步（`cjk-segmenter` feature-gated 无 console-api 运行时面）；只验默认构建 init baseline + 默认 tokenization 不变（ADR-004）+ 文档化两 task 状态。

### 5.3 不变量

- 0 行为变更 / 默认构建 0 新依赖（closeout 纯文档 + smoke step；真分词器在 `cjk-segmenter` feature 默认不编译；smoke 既有 step + denominator 不溯改 D5）。
- ADR-014 D5：历史 Phase 1-29 spec 不溯改；ADR-029 add-only Amendment 不改正文；smoke denominator `[37/37]` / `[38/38]` 不溯改。
- 真实 tag/release 不自行触发（ADR-012）；recall 数值真实跑出后回填、不预填、不外推（ADR-013）。

## 6. Acceptance Criteria

- [x] AC1（smoke v20 step 39）: smoke banner v19→v20 + step 39（`[39/39]` 默认构建 init baseline + 默认 tokenization 不变 + `cjk-segmenter` feature-gated 状态）+ `TestTask303_SmokeV20CjkTrueSegmenterStep`（含无回归既有 `[35/35]`..`[38/38]`，denominator 不溯改）— verified by TEST-30.3.1（`bash -n` exit 0 + `go test -run TestTask303` PASS）
- [x] AC2（v0.23.0 closeout bundle）: v0.23.0 release docs（`v0.23.0-{evidence,artifacts}.md` backfill 待回填 + README v0.23 段 + RELEASE_NOTES v0.23.0 段）+ ADR-035 per-D ratify `Proposed→Accepted`（D1/D2/D4/D5 Accepted；D3 default flip honest-defer PARTIAL）+ ADR-029 add-only Amendment（Phase 30）+ phase-30 §6 AC1-4 `[x]` + Status Done + adapter 闭合（Phase 30 Done/Tasks 3/ADR-035 Accepted）+ feature 已存在 — verified by TEST-30.3.2
- [x] AC3（ADR-014 D2 lint）: bash scripts/spec_drift_lint.sh --touched origin/master PR 触及行 0 未标注命中 — verified by TEST-30.3.3

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-30.3.1 | smoke v20 step 39（`[39/39]` + 默认 tokenization baseline 标记 + 无回归既有 denominator）+ `bash -n` 过 + `go test -run TestTask303` 过 | `scripts/console_smoke.sh` + `internal/cli/smoke_syntax_test.go` | Done (PASS) |
| TEST-30.3.2 | release docs + ADR-035 per-D ratify Accepted（D3 default flip honest-defer 部分）+ ADR-029 add-only Amendment + phase-30 §6 闭合 + adapter + feature bundle | release/ADR/phase/adapter/feature | Done |
| TEST-30.3.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Done (PASS) |

## 8. Risks

- **R1（低）closeout 误报重词典分词 / 大语料 recall 为已达成**：诚实风险。
  - **缓解**：ADR-035 ratify + release docs + smoke + phase §6 全逐维如实——重词典 dep / 大语料维度据已达维度部分 ratify，受阻维度 `[SPEC-DEFER:phase-future.*]` 如实标注；不伪造（ADR-013）。stop-condition：任何「真分词器召回提升」表述须有真实扩 CJK golden recall delta 凭据，否则标 待实测回填 / DEFERRED。
- **R2（低）recall delta 预填 / 外推**：扩 CJK golden 后 delta 须真实跑出。
  - **缓解**：release docs / ADR-035 §D4 数值真实跑出后回填、小语料 caveat、不外推（ADR-013）；TEST-30.2.1 守护。
- **R3（低）smoke denominator 误溯改**：新 step 39 须 `[39/39]`，既有 `[37/37]` / `[38/38]` 不动。
  - **缓解**：新 `Test` 无回归断言守护；ADR-014 D5。

## 9. Verification Plan

```bash
# AC1 — smoke 语法 + syntax test
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run Task303

# AC2 — 文档闭合人工核（ADR-035 Accepted + per-D Ratification / ADR-029 Amendment / phase §6 [x] / adapter Done / feature 存在）
# AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# 既有不退化（closeout 文档 + smoke 不影响 workspace；默认构建 0 新 dep）
cargo test --workspace && go test ./...
```

> **outward-facing 红线**：真实 v0.23.0 tag push + release run（GHCR 推送 + cosign 签名）是 closeout 合入后的**用户授权步**（ADR-012）；本 task 不自行 tag，release docs 的 tag/run/digest 用 angle-bracket backfill 待回填，待 post-tag-push backfill PR 填实。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification 实测证据**：`bash -n scripts/console_smoke.sh` exit 0；`go test ./internal/cli/ -run TestTask303` PASS（`[39/39]` + cjk-true-segmenter + TEST-30.1/30.2 + reindex + jieba 标记 + 无回归 `[35/35]`..`[38/38]`）；`cargo test --workspace` 0 failed + `go test ./...` 不退化；spec-lint `--touched origin/master` 0 未标注命中。ADR-035 据 30.1/30.2 真实产物 per-D ratify Accepted（D1 真分词 feature / D2 双站点对称 / D3 reindex 工具达成·default flip honest-defer / D4 真实 recall delta seg−bigram=+0.0000 诚实零 / D5 baseline 不变）；ADR-029 add-only Amendment（Phase 30）兑现 cjk-true-segmenter + tokenizer-default-on 部分；release docs tag/run/digest 待用户授权 tag 后 post-tag-push backfill 填实。

**计划改动文件**：

- `scripts/console_smoke.sh`（banner v19→v20 + v20 changelog 块 + step 39 `[39/39]`）
- `internal/cli/smoke_syntax_test.go`（新 `Test` 断言 `[39/39]` + 无回归既有 denominator）
- `docs/releases/v0.23.0-evidence.md` + `docs/releases/v0.23.0-artifacts.md`（新，tag/run/digest angle-bracket backfill 待回填）
- `README.md`（v0.23 段）+ `RELEASE_NOTES.md`（v0.23.0 段）
- `docs/decisions/adr-035-cjk-true-segmenter-and-tokenizer-default.md`（Status Proposed→Accepted + `## Ratification（v0.23.0 / task-30.3）` 节，per-D 真实依据）
- `docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md`（append `## Amendment (Phase 30 / v0.23.0)`，不溯改正文）
- `docs/specs/phases/phase-30-cjk-true-segmenter.md`（Status Draft→Done + §6 AC1-4 `[x]`）
- `docs/s2v-adapter.md`（Phase 30 行 + Task 30.1/30.2/30.3 行 + ADR-035 行 + BDD 行）

**§9 Verification 计划** (will record real evidence at impl)：

- AC1：`bash -n scripts/console_smoke.sh` + `go test ./internal/cli/ -run Task303` 真实跑出后回填结果。
- AC2：文档闭合人工核（ADR-035 per-D Ratification 据 task-30.1/30.2 §10 真实分词单测 + 真实扩 CJK golden recall delta；重词典 / 大语料受阻维度据已达维度部分 ratify，不伪造 ADR-013）；recall delta 数值待实测回填。
- AC3：`bash scripts/spec_drift_lint.sh --touched origin/master`（CI spec-lint 权威）真实跑出后回填。
- 既有不退化：`cargo test --workspace` + `go test ./...` 待实测回填（默认构建 0 新 dep，`cjk-segmenter` feature 默认不编译）。
- **outward-facing**：真实 v0.23.0 tag/release（GHCR 推送 + cosign 签名）待用户授权 → post-tag-push backfill PR 填实 evidence/artifacts tag SHA / run id / digest 待回填。
