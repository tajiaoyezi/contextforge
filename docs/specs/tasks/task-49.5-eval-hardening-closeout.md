# Task `49.5`: `eval-hardening-closeout — README/RELEASE_NOTES recall 声明更新 + defer marker 清理 + phase closeout`

**Status**: Ready
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 49 (eval-hardening)
**Dependencies**: task-49.1-49.4 全部完成 / ADR-013 / ADR-014（第四十一次激活，phase closeout）
**Required Reading**: phase-49-eval-hardening.md / task-49.4 spike doc（实测 recall 数字）/ README.md:28（现状 recall 声明）/ RELEASE_NOTES.md / docs/roadmap.md §4（SPEC-DEFER backlog）

## 1. Background
task-49.4 大语料实测后，README/RELEASE_NOTES 的 recall 声明（"16-question golden recall@5/@10=1.0"）可能不再准确。需据实更新（ADR-013 禁伪造）。同时 phase 49 closeout 需 redeem/继续 defer 相关 SPEC-DEFER。

## 2. Goal
(1) README:28 recall 声明据 task-49.4 实测更新（如 recall 仍 1.0 则强化声明；如退化则改保守措辞 + 标注大语料 caveat）。
(2) RELEASE_NOTES.md 加 v1.1.0 段（含实测数字 + caveat + golden 扩展说明）。
(3) redeem/继续 defer SPEC-DEFER：`embedding-large-corpus-recall` / `cjk-golden-corpus-expansion`（据实测结果）；不 redeem `cross-lingual-golden`（日韩）/ `reranker-large-corpus-quality`（NDCG 标准基准）。
(4) phase closeout：smoke gate + roadmap/adapter 索引 + CHANGELOG。

## 3. Scope
- 改 `README.md:28`：recall 声明据实测更新（具体措辞取决于 task-49.4 结果）
- 改 `RELEASE_NOTES.md`：加 v1.1.0 段
- 改 `docs/roadmap.md`：§4 SPEC-DEFER 状态更新（redeem 的标 ✅，继续 defer 的保留）
- 改 `docs/s2v-adapter.md`：Phase 49 行 + Task 总索引 49.1-49.5 行 Status → Done
- 改 `CHANGELOG.md`：[v1.1.0] 段
- 改 `docs/specs/phases/phase-49-eval-hardening.md`：Status Ready → Done
- redeem marker（据实测）：在相关 ADR / 源码注释标 "fulfilled by task-49.4"
- 新增 `docs/releases/v1.1.0-evidence.md`（如需）+ `test/features/phase-49-eval-hardening.feature`

## 4.1 声明更新决策树（据 task-49.4 结果）
- **若 recall 仍 ≥ gate 阈值（Top5≥0.75/Top10≥0.85）**：README 强化（"~120-question / ~500-1000-chunk 大语料实测仍达标"），redeem `embedding-large-corpus-recall`
- **若 recall 退化但 >0.7**：README 改保守（"大语料实测 recall@5=X.XX（小语料 1.0 是过拟合上界）"），部分 redeem + 标 caveat，不 redeem 完整 `embedding-large-corpus-recall`
- **若 recall 退化到 <0.7**：README 大改（诚实暴露天花板），不 redeem，标 `[SPEC-DEFER]` 继续等优化
- **CJK delta**：若仍=0 → bigram 默认确认（redeem `cjk-golden-corpus-expansion` 但标注 delta=0 诚实记录）；若≠0 → 据 delta 决定是否建议重评估 ADR-046 默认

## 6. AC
- [ ] **AC1**: README recall 声明与 task-49.4 实测一致（ADR-013 禁伪造）— verified by **TEST-49.5.1**
- [ ] **AC2**: RELEASE_NOTES v1.1.0 段完整（含实测数字 + caveat + golden 扩展）— verified by **TEST-49.5.2**
- [ ] **AC3**: spec_drift_lint 过；phase closeout（roadmap/adapter/CHANGELOG）— verified by **TEST-49.5.3**
- [ ] **AC4**: ADR-014 D1-D5（第四十一次激活）phase closeout mapping 表 — verified by PR body

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-49.5.1 | README recall 声明与实测一致 | grep + 人工核对 | Not Started |
| TEST-49.5.2 | RELEASE_NOTES v1.1.0 段完整 | grep | Not Started |
| TEST-49.5.3 | spec_drift_lint + phase closeout 索引 | lint + grep | Not Started |

## 9. Verification
```bash
# README recall 声明（据实测，具体 grep 据 task-49.4 结果定）
# RELEASE_NOTES v1.1.0 段
grep -q 'v1.1.0' RELEASE_NOTES.md
# spec_drift_lint
bash scripts/spec_drift_lint.sh --strict
# phase closeout
grep -q 'Done' docs/specs/phases/phase-49-eval-hardening.md  # Status 行
# ADR-014 D2
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes
**Status**: Ready

1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：无（phase closeout；v2.0 路线独立）
