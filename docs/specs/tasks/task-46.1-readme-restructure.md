# Task `46.1`: `readme-restructure — README 从 "38 段 changelog 污染" 重构为 "Features 汇总 + maturity label + current pin" 访客友好结构（ADR-050 D3）`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 46 (v1.0-docs-and-release-flow)
**Dependencies**: ADR-050（D3 文档对齐）/ ADR-013（maturity label 不虚标 v1.0）/ ADR-007（分发定义）/ RELEASE_NOTES.md（38 changelog 段的目标搬迁地）

## 1. Background
README.md 776 行，其中 **38 个 `## What's new` 段**（v0.3.0→v0.38.0）占 ~85%——访客打开 README 第一眼是 changelog 墙，看不到产品 Features。这些 changelog 段**已在** RELEASE_NOTES.md（1734 行内部详档），README 重复搬运。末尾 `## v0.2 limitations` 段含过时声明（"does not publish a GitHub Release object" 正是 task-46.3 要改的）。"Run the released image" 写死 `v0.28.0`（当前 v0.38.0）。"Where to go next" 引用 v0.1 未实装。Phase 45 / ADR-050 D3 要求 v1.0 前文档对齐。

## 2. Goal
README 重构为访客友好结构：(1) 删 38 个 `What's new` 段（已在 RELEASE_NOTES.md，README 只留指向链接）；(2) 删 `## v0.2 limitations` 过时段（内容分散融入新结构或删）；(3) 新增 **Features 汇总段**（local-first / Go+Rust 双二进制 / 三模式检索 BM25+semantic+hybrid / reranker / tokenizer / memory ops / console-api REST / MCP）；(4) 加 **maturity label**（"Pre-1.0，v1.0 收口中"，诚实不虚标 v1.0，ADR-013）；(5) 刷新版本 pin（写死 `v0.28.0`→`v0.38.0` current）。

## 3. Scope
- 改 `README.md`（重构主体）：
  - 保留：标题 + 一句话定位 + 双二进制说明 + Run the released image（刷新 pin）+ Quick Start + Expected output + Where to go next（刷新链接，去 v0.1 未实装引用）
  - 删除：38 个 `## What's new` 段（v0.3.0→v0.38.0）— 这些已在 RELEASE_NOTES.md
  - 删除：`## v0.2 limitations` 段 — 过时（platform target / license / release artifact 声明分散融入新结构）
  - 新增：**Features 段**（产品能力清单，基于 PRD 北极星 + v1.0 能力锚点）
  - 新增：**maturity label**（标题下方一行："**Status:** Pre-1.0，v1.0 收口中（v0.38.0）" — 诚实，ADR-013）
  - 新增：**Releases 段**（指向 GitHub Releases / GHCR + 指向 RELEASE_NOTES.md + CHANGELOG.md）— task-46.3 落地 Release 对象后此段指向真实 Release
  - 刷新：所有写死版本号 `v0.28.0`→`v0.38.0`

## 6. AC
- [x] **AC1**（README 重构）: README 删 38 changelog 段 + 删 v0.2 limitations + 新增 Features 汇总段 + maturity label（Pre-1.0 收口中）+ 刷新版本 pin（v0.28.0→v0.38.0）+ Quick Start 保留可用 — verified by **TEST-46.1.1**（grep 守护：Features 段在场 / maturity label 在场 / 无 `## What's new in v0.3`..v0.37 段 / pin = v0.38.0 / Quick Start 保留）

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-46.1.1 | README Features 段 + maturity label + 无 38 changelog 段 + pin = v0.38.0 + Quick Start 保留 | docs grep + 行数核 | Done |

## 9. Verification
```bash
# Features 段在场
grep -q "^## Features" README.md
# maturity label 在场（Pre-1.0，不虚标 v1.0）
grep -q "Pre-1.0" README.md && ! grep -qi "v1\.0 ready\|production ready\|stable release" README.md
# 38 changelog 段已删（只允许留 v0.38.0 作为"最新版"指向，不允许成段）
test "$(grep -c '^## What.s new in v0\.' README.md)" -le 2   # 最多留最近 1-2 版简述
# v0.2 limitations 段已删
! grep -q "^## v0\.2 limitations" README.md
# pin 刷新（不再写死 v0.28.0；v0.38.0 在场）
! grep -q "v0\.28\.0" README.md && grep -q "v0\.38\.0" README.md
# Quick Start 保留（核心命令序列不丢）
grep -q "contextforge init" README.md && grep -q "contextforge search" README.md
```

## 10. Completion Notes
**Status**: Done

1. **完成日期**：2026-07-03
2. **改动文件**：- README.md（重构主体：776→153 行）
3. **commit 列表**：- `06d2a19` docs(v1.0-docs): task-46.1 README 重构 + task-46.2 CHANGELOG + ADR 索引（D3 文档对齐）
4. **§9 Verification 结果**：
   - lint: N/A（纯 markdown，无 gofmt/clippy）
   - typecheck: N/A
   - unit-test: N/A（纯文档 task，无代码）
   - docs grep: ✅ Features 段在场 / maturity label Pre-1.0 在场 / 0 个 What's new 段（38→0）/ 无 v0.2 limitations / 无 v0.28.0 pin（→v0.38.0）/ Quick Start init+search 保留 / 无 "does not publish a GitHub Release" 过时声明
5. **剩余风险 / 未做项**：无（纯文档重构；maturity label 诚实标 Pre-1.0，v1.0.0 flip 在 Phase 47）
6. **下游 task 影响**：task-46.3（README Releases 段已建指向 GitHub Releases，待 release.yml Release 对象落地后链接成立）
