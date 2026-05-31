# Task `24.3`: `closeout-v0.17.0 — task-24.1 tokenizer over task-24.2 扩充 golden 的真实 before/after recall delta（ADR-013 real 数据，受阻诚实延后）+ core/src/eval/runner.rs rust-native-eval-runner 评估（promote 最小 runner 或诚实延后 [SPEC-DEFER:phase-future.rust-native-eval-runner]）+ scripts/console_smoke.sh v14 step + v0.17.0 release docs（README/RELEASE_NOTES/evidence/artifacts）+ ADR-029 据真实结果 ratify + ADR-006/008 add-only Amendment + phase-24 §6 闭合 + adapter`

**Status**: Draft

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 24 (retrieval-tokenizer-and-eval-hardening)
**Dependencies**: task-24.1（code/CJK tokenizer + opt-in + 分词单测）/ task-24.2（eval 数据集校验器 + 扩充 golden 含代码/CJK case）/ task-23.3（closeout 模板 + smoke v13 step 32 基线 + tag/backfill pattern）/ task-19.7（closeout 模板 + tag/backfill pattern）/ ADR-029（code-and-cjk-tokenizer-and-eval-hardening，本 phase 新 Proposed）/ ADR-006（recall gate 阈值不变）/ ADR-013（禁伪造 recall / runner 凭据）/ ADR-014 D1-D5（第十五次激活）

## 1. Background

task-24.1 已让 `core/src/indexer/mod.rs` 在 opt-in 时对 `content` 字段用自定义 code/CJK `TextAnalyzer`（代码符号拆分 + 保留原 token + CJK bigram，默认 tokenization 不变）；task-24.2 已加 eval golden 数据集独立校验器 + 扩充 `test/fixtures/eval/golden-semantic.jsonl`（含代码符号 + CJK annotated query，exercise task-24.1 tokenizer）。本 task 收口 Phase 24：(1) 实测 **task-24.1 tokenizer over task-24.2 扩充 golden 的真实 before/after recall delta**（ADR-013 真实非合成；小语料 delta 不显著则如实记录，受阻则诚实延后）；(2) 评估 **rust-native-eval-runner**——`core/src/eval/runner.rs`（placeholder，`[SPEC-DEFER:phase-future.rust-native-eval-runner]`）promote 为最小 Rust-native runner（+ 单测）或诚实延后 + 文档化评估口径；(3) 把 console_smoke 升 v14，加 tokenizer / eval 加固相关断言；(4) 产 v0.17.0 release docs；(5) 据真实非合成结果 ratify ADR-029 + ADR-006/008 add-only Amendment（若需，不溯改正文 D5）；(6) 闭合 phase-24 §6 AC；(7) 更新 s2v-adapter。

承 v0.12.0 / v0.13.0 / v0.16.0 收口模式：closeout = smoke final + release evidence/artifacts + README/RELEASE_NOTES + ADR 状态 + adapter；tag push 由主 agent 据 goal 授权自主决断（ADR-012）后由 release.yml 触发 + post-tag-push backfill。

## 2. Goal

实测 task-24.1 tokenizer over task-24.2 扩充 golden 的真实 before/after recall delta（default analyzer vs opt-in code/CJK analyzer 在代码符号 + CJK query case 上的 recall@5/10 对比），落 `docs/spikes/phase-24-tokenizer-recall.md`（数据源 / 语料规模 / case 数 / per-case 分解 / before vs after 全标注，ADR-013 真实非合成；小语料 delta 不显著或受阻则如实记录 / 诚实延后）。评估 `core/src/eval/runner.rs` rust-native-eval-runner 并据可行性 promote 最小 runner（Rust 侧对一组 question + 检索结果算召回，复用既有口径 + deterministic 单测）或诚实延后 `[SPEC-DEFER:phase-future.rust-native-eval-runner]` + 文档化评估口径。`scripts/console_smoke.sh` 升 v14：既有 step 不退化 + 新增 tokenizer / eval 加固 smoke 断言（opt-in tokenizer 分词 smoke 或如实标 feature/Rust 层验证）。新增 `docs/releases/v0.17.0-{evidence,artifacts}.md` + `README.md` v0.17 段 + `RELEASE_NOTES.md` v0.17.0 段。`docs/decisions/adr-029-*.md` 据真实结果 Status `Proposed → Accepted`（或受阻维度记录维持）+ ADR-006/008 add-only Amendment（若需）。`docs/specs/phases/phase-24-*.md` §6 AC1-6 全 `[x]` + Status `Draft → Done`。`docs/s2v-adapter.md` Phase 24 `Draft → Done` + Tasks `0 → 3` + ADR-029 索引 + roadmap §4 四 marker 推进记录。ADR-014 D1-D5 第十五次激活 closeout PR body。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **实测 tokenizer 真实 before/after recall delta + 新增 `docs/spikes/phase-24-tokenizer-recall.md`**：用 task-24.2 扩充 golden（代码符号 + CJK query case）跑 task-24.1 tokenizer——default analyzer（before）vs opt-in code/CJK analyzer（after）的 recall@5/10 对比；evidence 记数据源 / 语料 chunk 数 / case 数 / per-case（per-query）分解 / before vs after delta / 与既有 gate 阈值（ADR-006）对照；ADR-013 真实非合成标注。小语料 delta 不显著则如实记录（不夸大 / 不篡改）；tokenizer 实测受阻则诚实延后口径文档化。
- **评估 + 修改 `core/src/eval/runner.rs`（rust-native-eval-runner promote 或诚实延后）**：评估 placeholder promote 为最小 Rust-native runner 的可行性 + 收益；可行且收益清晰则落最小 runner（Rust 侧算召回 + deterministic 单测）+ 文档化；否则诚实延后 `[SPEC-DEFER:phase-future.rust-native-eval-runner]` + 在 spike / spec §10 文档化评估口径（不在 placeholder 伪造已实现，ADR-013）。
- **修改 `scripts/console_smoke.sh`**：v14 注释段 + 新增 tokenizer / eval 加固 smoke 断言（opt-in tokenizer 分词 smoke step，或据实标注 Rust feature/config 层验证 + 默认构建 intact 断言）；既有 step 标号 / 断言不动语义；终态 marker 保留（v13 现为 step 32，v14 升 step 33）。
- **新增 `docs/releases/v0.17.0-evidence.md` + `docs/releases/v0.17.0-artifacts.md`**：承 v0.12.0/v0.13.0/v0.16.0 模板（合入记录 / S2V 状态 / 验证证据 / CI / AC 达成 / 平台矩阵 / upgrade-rollback / ADR-014 record / tag+镜像 SHA backfill 段）；含 tokenizer opt-in re-index 含义 + recall delta 结论 + runner 评估结论。
- **修改 `README.md`**：v0.17 段——opt-in code/CJK tokenizer（含 re-index 含义）+ eval 数据集校验器 + golden 代码/CJK 扩充。
- **修改 `RELEASE_NOTES.md`**：v0.17.0 段（task 表 + tokenizer 改进 + recall delta 结论 + eval 加固 + runner 评估结论 + upgrade/rollback 含 re-index 提示）。
- **修改 `docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md`**：据 task-24.1/24.2 + 本 task recall delta + runner 评估真实结果 Status `Proposed → Accepted`（或受阻维度记录维持）；ADR-006（gate 阈值不变，若 recall 度量推进需）/ ADR-008（若 tokenizer 引入分词依赖）以 add-only Amendment 记录（不溯改正文，D5）。
- **修改 `docs/specs/phases/phase-24-retrieval-tokenizer-and-eval-hardening.md`**：§6 AC1-6 全 `[x]` + Status `Draft → Done` + §8 DoD 勾选。
- **修改 `docs/s2v-adapter.md`**：Phase 24 行 `Draft → Done` + `Tasks 0 → 3` + Task 索引 24.1-24.3 Done + ADR-029 索引行 + BDD phase-24 feature 行 + roadmap §4 四 marker 推进注。
- **新增 `test/features/phase-24-retrieval-tokenizer-and-eval-hardening.feature`**（≥3 scenario）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **code/CJK tokenizer 实现** [SPEC-OWNER:task-24.1-code-and-cjk-tokenizer]：本 task 在 smoke / recall delta / release docs 引用它，不实现。
- **eval 数据集校验器 + golden 扩充实现** [SPEC-OWNER:task-24.2-eval-dataset-hardening]：本 task 用其扩充 golden 跑 recall delta，不重做校验器/数据集。
- **CJK 真正分词器（替 bigram）** [SPEC-DEFER:phase-future.cjk-true-segmenter]：本 task 据 bigram 实测；真正分词器属后续版本。
- **tokenizer 从 opt-in 转默认开启 + 既有索引迁移工具** [SPEC-DEFER:phase-future.tokenizer-default-on]：本 task release docs 文档化 re-index 含义；默认开启 + 迁移属后续。
- **rust-native-eval-runner 的完整远程召回实现**（若本 task 评估延后）[SPEC-DEFER:phase-future.rust-native-eval-runner]：本 task 据评估 promote 最小 runner 或诚实延后，完整远程召回属后续。
- **v0.17.0 tag push 实际执行**：closeout PR 合入后，主 agent 据本次 goal 授权自主 push `v0.17.0` annotated tag 触发 release.yml（沿用历史 release 流 + ADR-012 自治）。post-tag-push backfill 填实 tag SHA / run ID / 镜像 digest 由独立 backfill PR 承接（仿 v0.13.0/v0.16.0 pattern）。
- **golden case_results 子表** [SPEC-DEFER:phase-future.case-results-subtable]：`docs/roadmap.md` §4 长尾。

## 4. Actors

- **主 agent**：实施 + PR 主理 + closeout 决策（recall delta 记录 vs 延后 + runner promote vs 延后 + ADR-029 ratify vs 维持 + tag push 授权）。
- **`core/src/indexer/mod.rs` tokenizer（task-24.1）+ `test/fixtures/eval/golden-semantic.jsonl`（task-24.2）**：recall delta 实测的被测项 + 数据集。
- **`core/src/eval/runner.rs`**：rust-native-eval-runner placeholder，本 task 评估 promote 或延后。
- **`scripts/console_smoke.sh`**：端到端 C1 兜底 smoke，本 task 升 v14。
- **`docs/releases/` + `README.md` + `RELEASE_NOTES.md`**：v0.17.0 release 文档面。
- **`docs/decisions/adr-029-*.md`**：本 phase 新 ADR，本 task ratify；ADR-006/008 add-only Amendment。
- **`docs/s2v-adapter.md`**：Phase/task/ADR/BDD 索引。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/tasks/task-23.3-closeout-v0.16.0.md`（closeout 模板 + smoke v13 step 32 基线 + tag/backfill + ADR ratify pattern）+ `docs/specs/tasks/task-19.7-closeout-v0.12.0.md`（closeout 模板 + tag/backfill pattern）
- `docs/releases/v0.16.0-{evidence,artifacts}.md` + `docs/releases/v0.12.0-{evidence,artifacts}.md`（release 文档结构 + 平台矩阵 + backfill 段）
- `scripts/console_smoke.sh:55-84`（v13 注释段 + step 32 + 终态 marker `CONSOLE_REAL_SMOKE_EXIT=0`）
- `docs/specs/tasks/task-24.1-code-and-cjk-tokenizer.md` + `docs/specs/tasks/task-24.2-eval-dataset-hardening.md`（本 phase 上游交付）
- `docs/decisions/adr-029-code-and-cjk-tokenizer-and-eval-hardening.md`（本 phase ADR + Ratification 待回填段）+ `docs/decisions/adr-006-recall-eval-acceptance-gate.md`（gate 阈值不变）+ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）
- `core/src/eval/runner.rs`（`EvalRunner` placeholder + `[SPEC-DEFER:phase-future.rust-native-eval-runner]` marker）+ `core/src/eval/mod.rs` + `core/src/eval/store.rs`（rust-native runner promote 评估基线）+ `internal/eval/eval.go`（Go harness 召回口径 — Rust runner 若 promote 复用之）
- `docs/spikes/phase-19-real-recall.md`（real recall 度量口径 + 小语料 caveat — 本 task recall delta 同口径）+ `docs/s2v-adapter.md` §Phase / §Task / §ADR / §BDD 索引

### 5.2 关键设计 — recall delta + runner 评估 + smoke v14 + ADR ratify

- **tokenizer 真实 before/after recall delta（ADR-013）**：用 task-24.2 扩充 golden（代码符号 + CJK query case）对真实 ContextForge 源码语料跑两次检索——default analyzer（before）vs opt-in code/CJK analyzer（after），用既有 recall@K 口径（`internal/eval` `SemanticRecallAtK` 等价 / file-level Strong-hit@K，承 task-19.5 §10 口径）算 before vs after delta。evidence 记数据源 / 语料规模 / case 数 / per-query 分解 / before-after 表 / gate 对照。**小语料 delta 不显著**（承 phase-24 §7 R2，扩充 golden 仍小语料）则如实记录「opt-in tokenizer 落地 + 分词正确性单测背书 + recall delta 在本小语料 <X」，不为正 delta 改语料/口径（ADR-013）；tokenizer 实测受阻则诚实延后口径文档化。
- **rust-native-eval-runner 评估（ADR-013 / ADR-029 D4）**：评估 placeholder promote 可行性——Rust 侧复刻 Go harness 召回口径的成本 / 收益（task-14.1 已选 Go-side runner）。可行且收益清晰则落最小 runner（Rust 对 question + 检索结果算召回 + deterministic 单测）+ 文档化；否则诚实延后 `[SPEC-DEFER:phase-future.rust-native-eval-runner]` + spike / spec §10 文档化评估口径（不在 placeholder 伪造已实现）。
- **smoke v14**：新增 tokenizer / eval 加固 smoke——opt-in tokenizer 分词 smoke（feature/config 下代码符号 + CJK 拆分），或据实标注 Rust feature/config 层 + Go eval 校验器层验证（非 console 热路径）+ 默认构建 intact 断言（不伪造 console tokenizer 路径，ADR-013）；既有 step 断言不动；终态 marker 保留；step 32 → step 33 标号同步（仿 v13 step 升级 pattern）。
- **ADR-029 ratify（ADR-013）**：据 task-24.1 真实分词单测 + task-24.2 真实校验器单测 + 本 task 真实 before/after recall delta + runner 评估结论 Proposed→Accepted；若某维度受阻（如 recall delta 小语料不显著 / runner 评估延后）则 ADR-029 据「已达维度 ratify + 受阻维度如实记录」处理，不据合成 / 伪造 ratify。
- **ADR-006/008 add-only Amendment**：recall 度量推进结果（若需）以 add-only Amendment 记录，不改 gate 阈值 / 不溯改 ADR-006 正文（D5）；tokenizer 若引入分词依赖则 ADR-008 add-only 记依赖变更（task-24.1 若 std-only 则无 ADR-008 变更）。

### 5.3 不变量

- smoke 既有 step 不退化（仅新增 tokenizer / eval 加固 step + v14 注释 + step 标号同步）。
- release docs 诚实口径（承 task-23.3 / task-19.7 §10）：default-vs-opt-in delta 真实 / 小语料 caveat 如实标 / runner 评估结论（promote 或延后）如实记；不伪造 recall / runner 凭据。
- ADR-029 ratify 仅在 task-24.1/24.2 + 本 task recall delta + runner 评估真实落地后（ADR-013：据真实非合成）；受阻维度不强 ratify。
- 默认构建 0 新 dep + 默认 tokenization 不变 + eval gate 阈值不变（ADR-004 / ADR-006）。

## 6. Acceptance Criteria

- [ ] **AC1**: tokenizer 真实 before/after recall delta 实测 + runner 评估 — `docs/spikes/phase-24-tokenizer-recall.md` 记 task-24.1 tokenizer（default vs opt-in）over task-24.2 扩充 golden 的真实 before/after recall@5/10 delta（数据源 / 语料规模 / per-query 分解 / gate 对照，ADR-013 真实非合成；小语料 delta 不显著则如实记录）；`core/src/eval/runner.rs` rust-native-eval-runner promote 最小 runner（+ deterministic 单测）或诚实延后 `[SPEC-DEFER:phase-future.rust-native-eval-runner]` + 文档化评估口径；`scripts/console_smoke.sh` v14 通过 `bash -n`（exit 0）+ tokenizer/eval 加固 smoke 断言 + 既有 step 不退化 — verified by **TEST-24.3.1**
- [ ] **AC2**: v0.17.0 release docs 齐备（`docs/releases/v0.17.0-{evidence,artifacts}.md` + `README.md` v0.17 段 + `RELEASE_NOTES.md` v0.17.0 段）；evidence 含 task 表 / CI / AC 达成 / 平台矩阵 / upgrade-rollback（含 tokenizer opt-in re-index 含义）/ §tag-backfill 待回填段 — verified by **TEST-24.3.2**
- [ ] **AC3**: ADR-029 据 task-24.1/24.2 + recall delta + runner 评估真实结果 Status `Proposed → Accepted`（或受阻维度记录维持）；ADR-006/008 add-only Amendment 记推进结果（不溯改正文，gate 阈值不变）；phase-24 §6 AC1-6 全 `[x]` + Status `Draft → Done`；adapter Phase 24 `Draft → Done` + Tasks `0 → 3` + ADR-029 索引 + roadmap §4 四 marker 推进注 — verified by **TEST-24.3.3**
- [ ] **AC4**: 既有不退化 — 默认 `cargo test --workspace` + `go test ./...` 全 PASS；opt-in tokenizer（task-24.1）+ eval 校验器（task-24.2）+ runner（若 promote）单测不退化；默认 tokenization + eval gate 阈值不变 — verified by **TEST-24.3.4** + §10
- [ ] **AC5**: ADR-014 D1-D5 第十五次激活全通过（D1 phase§6↔task§6 mapping 表 + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-23 不溯改）— verified by **TEST-24.3.5** + 本 closeout PR body

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-24.3.1 | tokenizer before/after recall delta + runner 评估（promote/延后）+ smoke v14 `bash -n` + tokenizer/eval 断言 | `docs/spikes/phase-24-tokenizer-recall.md` + `core/src/eval/runner.rs` + `internal/cli/smoke_syntax_test.go` + `scripts/console_smoke.sh` | Planned |
| TEST-24.3.2 | v0.17.0 release docs 齐备 + 结构校验（含 re-index 含义） | `docs/releases/v0.17.0-*.md` + README + RELEASE_NOTES | Planned |
| TEST-24.3.3 | ADR-029 ratify + ADR-006/008 Amendment + phase-24 闭合 + adapter | `docs/decisions/adr-029-*.md` + phase-24 spec + s2v-adapter | Planned |
| TEST-24.3.4 | 默认 `cargo test --workspace` + `go test ./...` + 上游单测不退化 0 failed | 全 Rust + Go | Planned |
| TEST-24.3.5 | ADR-014 D1-D5 record（mapping + D2 lint） | 本 closeout PR body | Planned |

## 8. Risks

- **R1（中）tokenizer recall delta 小语料不显著**（承 phase-24 §7 R2）：扩充 golden 仍小语料，代码/CJK case 数有限，before/after delta 可能偏弱甚至打平。
  - **缓解**：如实记真实 before/after delta（不篡改 / 不夸大），evidence 标注语料规模 + case 数 + per-query 分解；delta 不显著时如实记录「opt-in tokenizer 落地 + 分词正确性单测背书 + recall delta 在本小语料 <X」，AC1 以「真实 delta 实测 + 诚实记录」满足，不为正 delta 改语料/口径（ADR-013）。
- **R2（中）rust-native-eval-runner promote 收益不足 → 评估延后**（承 phase-24 §7 R3）：Rust runner 复刻 Go harness 召回口径成本高、收益不足（task-14.1 已选 Go-side）。
  - **缓解**：真实评估 promote 可行性 + 收益；可行且收益清晰则落最小 runner + deterministic 单测，否则诚实延后 `[SPEC-DEFER:phase-future.rust-native-eval-runner]` + 文档化评估口径，AC1 以「真实评估 + promote 或诚实延后」满足，不在 placeholder 伪造已实现（ADR-013）。
- **R3（低）ADR-029 某维度受阻 → ratify 须真实结果**：recall delta 不显著 / runner 评估延后属受阻维度。
  - **缓解**：ADR-029 据「tokenizer 分词 + 校验器 + 数据集已达维度 ratify + recall delta / runner 受阻维度如实记录」处理——已达维度 ratify，受阻维度如实记（ADR-013），不据合成 ratify。
- **R4（低）smoke v14 tokenizer 分词在 CI 默认构建不可跑**（opt-in / feature-gated）：默认 CI 无 opt-in tokenizer。
  - **缓解**：tokenizer 分词 smoke 在 opt-in/feature 下本地 / 合规环境跑（🟡），默认 CI 跑既有 step 不退化 + `bash -n` 语法门 + 默认构建 intact 断言；如实标 opt-in 依赖（ADR-013）。

## 9. Verification Plan

```bash
# smoke v14 语法 + step 标号
bash -n scripts/console_smoke.sh
go test ./internal/cli/... -run 'TestTask24|TestTask233' -v

# 既有不退化
go test ./...
cargo test --workspace

# opt-in tokenizer（task-24.1）+ eval 校验器（task-24.2）+ runner（若 promote）单测
cargo test -p contextforge-core indexer
cargo test -p contextforge-core eval
go test ./internal/eval/...

# 端到端 smoke（合规环境；opt-in tokenizer 分词）
bash scripts/console_smoke.sh        # 期望末行 CONSOLE_REAL_SMOKE_EXIT=0

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按 6 项回填：完成日期 / 改动文件 / commit 列表 / §9 Verification 实测结果（ADR-013 真实非合成；含 tokenizer before/after recall delta 数 + runner 评估结论 + smoke v14 step 33）/ 设计取舍（recall delta 记录 vs 延后 + runner promote vs 延后 + ADR-029 ratify 维度 + ADR-006/008 Amendment + tag push 自治授权口径）/ 剩余风险 + 下游影响（CJK 真正分词器 / tokenizer 默认开启 + 索引迁移 / rust-native-eval-runner 若延后续 backlog / tag+release backfill）。
