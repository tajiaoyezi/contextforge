# Task `46.2`: `changelog-and-adr-index — 建 CHANGELOG.md（Keep a Changelog）+ docs/decisions/README.md（49 ADR 访客分类导航）`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 46 (v1.0-docs-and-release-flow)
**Dependencies**: ADR-050（D3 文档对齐）/ ADR-013（CHANGELOG 提炼真实不伪造）/ RELEASE_NOTES.md（提炼源）/ docs/decisions/ 50 ADR 文件（索引源）

## 1. Background
v1.0 需对外标准 changelog（CHANGELOG.md），但项目当前只有 `RELEASE_NOTES.md`（1734 行内部详档，每个 phase 的 What shipped / ADR / Upgrade path / Rollback path / Contract / 凭据 backfill）。RELEASE_NOTES.md 是面向维护者的详档，CHANGELOG.md 是面向用户的简表（Keep a Changelog 1.1.0 格式：Added / Changed / Deprecated / Removed / Fixed / Security）。50 个 ADR 散在 `docs/decisions/adr-NNN-*.md` 文件名里，无面向访客的分类导航（adapter 内部有表格但那是 s2v 治理表，非访客入口）。

## 2. Goal
(1) 建 `CHANGELOG.md`（Keep a Changelog 1.1.0 格式，从 RELEASE_NOTES.md + git tag 历史提炼 v0.1→v0.38.0 关键里程碑——非全文搬运，是对外简表；每版本 Added/Changed/Removed 三类足矣）。
(2) 建 `docs/decisions/README.md`（50 ADR 按 category 分组导航：Architecture / Storage & Retrieval / Interfaces / Release & Distribution / Governance & Process；每条 = # + title + status + 一句话摘要 + 链接）。

## 3. Scope
- 新增 `CHANGELOG.md`：
  - 头部：Keep a Changelog 1.1.0 banner + 项目说明（指向 RELEASE_NOTES.md 详档）
  - 主体：按版本倒序（v0.38.0 → v0.1.0），每版本列 Added / Changed / Removed / Fixed（有则列，无则省）
  - 提炼源：RELEASE_NOTES.md（每 phase 的 What shipped）+ git tag 列表交叉验证
  - 诚实边界（ADR-013）：版本里程碑无法从 RELEASE_NOTES.md 确认 → 该版本行省略或 honest note，非伪造
- 新增 `docs/decisions/README.md`：
  - 头部：ADR 说明（什么是 ADR + 如何新增 + Status 取值）
  - 主体：50 ADR 按 category 分组表格（每行：# + title + status + 一句话摘要）
  - category 分组：Architecture（001/002/008/016）/ Storage & Retrieval（002/023/026/027/028/029/030/034/035/037/039/041/042/043/046/047）/ Interfaces（003/013/015/017/024/025/044）/ Release & Distribution（007/033/050）/ Governance & Process（004/005/006/009/010/011/012/014/018/020/021/022/031/032/036/038/040/045/048/049）
  - status 标注：Accepted / Proposed（与 adapter ADR 索引表一致）

## 6. AC
- [x] **AC1**（CHANGELOG.md）: `CHANGELOG.md` 在场 + Keep a Changelog 1.1.0 头 + 版本倒序 + 至少覆盖 v0.30.0→v0.38.0（近期版本完整，早期版本可粗粒度） — verified by **TEST-46.2.1**
- [x] **AC2**（ADR 索引）: `docs/decisions/README.md` 在场 + 49 ADR 全列（adr-019 跳号）+ category 分组 + status 标注 — verified by **TEST-46.2.2**（grep 计数 = 49 table entries + 1 prose mention of 019）

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-46.2.1 | CHANGELOG.md 在场 + Keep a Changelog 头 + 版本倒序 + 近期版本覆盖 | docs grep | Done |
| TEST-46.2.2 | docs/decisions/README.md 在场 + 49 ADR 全列 + category 分组 | docs grep + 计数 | Done |

## 9. Verification
```bash
# CHANGELOG.md 在场 + Keep a Changelog 头
test -f CHANGELOG.md && head -5 CHANGELOG.md | grep -q "Keep a Changelog"
# 版本倒序 + 近期版本覆盖（v0.38.0 在 v0.30.0 之前出现）
awk '/^## \[v0\.38\.0\]/{a=NR} /^## \[v0\.30\.0\]/{b=NR} END{exit !(a>0 && b>0 && a<b)}' CHANGELOG.md
# docs/decisions/README.md 在场 + 50 ADR 计数
test -f docs/decisions/README.md
# 50 ADR 链接计数（adr-001..adr-050，注意 019 跳号 —— 核实际文件数）
grep -c 'adr-[0-9]' docs/decisions/README.md   # 应 >= 49（019 跳号，实际 49 文件但索引列 50 编号含 050）
# category 分组在场
grep -q "Architecture" docs/decisions/README.md && grep -q "Governance" docs/decisions/README.md
```

## 10. Completion Notes
**Status**: Done

1. **完成日期**：2026-07-03
2. **改动文件**：
   - CHANGELOG.md（新增，Keep a Changelog 1.1.0 格式，v0.1.0→v0.38.0 + Unreleased）
   - docs/decisions/README.md（新增，49 ADR 按 5 category 分组导航 + 一句话摘要）
3. **commit 列表**：- `06d2a19` docs(v1.0-docs): task-46.1 README 重构 + task-46.2 CHANGELOG + ADR 索引（D3 文档对齐）
4. **§9 Verification 结果**：
   - lint: N/A（纯 markdown）
   - typecheck: N/A
   - unit-test: N/A（纯文档 task）
   - docs grep: ✅ CHANGELOG Keep a Changelog banner + 版本倒序（v0.38 before v0.30）+ v0.30-v0.38 全覆盖；docs/decisions/README.md 49 table entries（adr-019 跳号在 prose 注明）+ 5 category 分组（Architecture / Storage & Retrieval / Interfaces / Release & Distribution / Governance & Process）
5. **剩余风险 / 未做项**：无（CHANGELOG 提炼自 RELEASE_NOTES.md + git tag 历史，早期版本 v0.5-v0.9 粗粒度一行，近期版本 v0.28+ 详 Added/Changed/Removed；ADR 摘要一句精准，详档指向各 ADR 文件）
6. **下游 task 影响**：task-46.3（CHANGELOG 首版就绪供 GitHub Release body 引用 + README Releases 段已指向 CHANGELOG.md）
