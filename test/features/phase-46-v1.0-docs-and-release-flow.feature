# Phase 46 · v1.0-docs-and-release-flow
# v1.0 收口冲刺第二步：D3 文档对齐 + D4 GitHub Release 流程（承 ADR-050）。
# README 776 行中 38 段 changelog 占 ~85% + v0.2 limitations 过时 + 写死 v0.28.0 → 重构访客友好结构。
# CHANGELOG.md 不存在 + docs/decisions 无访客索引 → 建 Keep a Changelog + ADR 分类导航。
# release.yml 无 GitHub Release 对象自动创建 → 加 softprops/action-gh-release@v2 step。
# 🟢 纯文档 + 1 CI step + 0 代码逻辑改动 / 0 dep / 0 migration / 0 proto / 0 schema。
# ADR-050 D3/D4 ratify（完整 ratify 待 Phase 47 v1.0.0）/ ADR-007 add-only Amendment（ADR-013/014）。

Feature: v1.0-docs-and-release-flow — D3 文档对齐 + D4 GitHub Release 流程
  作为 ContextForge 访客/用户
  我希望 README 展示产品 Features（而非 38 段 changelog 墙）+ 有标准 CHANGELOG + ADR 导航
  且每次发版自动创建 GitHub Release 对象（含签名镜像 + SBOM provenance 链接）
  以便快速理解项目能力与决策脉络（D3 文档对齐）
  且发版产物可发现可验证（D4 发布流程，承 ADR-050）
  而 README 的 maturity label 诚实标注 "Pre-1.0 收口中"（不虚标 v1.0，ADR-013）

  # ---- task-46.1: README 重构（Features 汇总 + maturity label + pin 刷新）----

  Scenario: README 删除 38 段 changelog 污染（已在 RELEASE_NOTES.md）
    Given README.md 776 行中 38 个 "What's new" 段占 ~85%（v0.3.0→v0.38.0）
    And 这些 changelog 段已在 RELEASE_NOTES.md（1734 行内部详档）
    When 删除 38 个 changelog 段（README 只留指向 RELEASE_NOTES.md 的链接）
    Then 访客打开 README 第一眼是产品定位 + Features（非 changelog 墙）
    And Quick Start 核心命令序列保留可用

  Scenario: README 新增 Features 汇总段 + maturity label
    Given README 无 Features 汇总段 + 无 maturity label（v1.0 锚点缺失的体现）
    When 新增 Features 段（local-first / Go+Rust 双二进制 / 三模式检索 BM25+semantic+hybrid / reranker / tokenizer / memory ops / console-api REST / MCP）
    And 加 maturity label（"Pre-1.0，v1.0 收口中（v0.38.0）"）
    Then Features 段在场 + maturity label 诚实（Pre-1.0，不虚标 v1.0，ADR-013）

  Scenario: README 刷新版本 pin（v0.28.0 → v0.38.0）+ 删 v0.2 limitations 过时段
    Given "Run the released image" 写死 v0.28.0（当前 v0.38.0）
    And "v0.2 limitations" 段含过时声明（does not publish a GitHub Release object）
    When 刷新所有写死版本号 v0.28.0→v0.38.0
    And 删除 v0.2 limitations 段（内容分散融入新结构）
    Then pin = v0.38.0（current）+ 无 v0.2 limitations 过时段

  # ---- task-46.2: CHANGELOG.md + ADR 访客索引 ----

  Scenario: 建 CHANGELOG.md（Keep a Changelog 1.1.0 格式）
    Given 项目无 CHANGELOG.md（只有 RELEASE_NOTES.md 1734 行内部详档）
    When 从 RELEASE_NOTES.md + git tag 历史提炼 v0.1→v0.38.0 关键里程碑
    Then CHANGELOG.md 在场（Keep a Changelog 1.1.0 banner + 版本倒序 + Added/Changed/Removed）
    And 是对外简表（非全文搬运 RELEASE_NOTES.md）+ 指向 RELEASE_NOTES.md 详档

  Scenario: 建 docs/decisions/README.md（50 ADR 分类导航）
    Given 49 个 ADR 文件散在 docs/decisions/（adr-019 跳号）无访客导航
    When 建 docs/decisions/README.md 按 category 分组（Architecture / Storage & Retrieval / Interfaces / Release & Distribution / Governance & Process）
    Then 49 ADR 全列 + 每条一句话摘要 + status 标注 + 链接
    And 与 adapter 内部 ADR 表互补（adapter 是 s2v 治理表，此 README 是访客入口）

  # ---- task-46.3: D4 GitHub Release 流程 + closeout ----

  Scenario: release.yml 加 GitHub Release 对象自动创建（D4）
    Given release.yml 只推 image 到 GHCR + cosign sign/attest，无 Release 对象
    And README 自承 "does not publish a GitHub Release object"
    When 加 softprops/action-gh-release@v2 step（tag push 触发，在 sign + attest 之后）
    And body 从 RELEASE_NOTES.md 对应版本段拼接 + 标注 GHCR + cosign verify + SBOM provenance
    And permissions contents: read → contents: write（Release 创建需要）
    Then tag push 自动创建 GitHub Release 对象（v0.39.0 首次实践）

  Scenario: v0.39.0 closeout + ADR-050 D3/D4 ratify
    Given task-46.1（D3 README）+ task-46.2（D3 CHANGELOG/ADR）+ task-46.3（D4 Release）全交付
    When smoke v36[55/55] + release docs + ADR-050 D3/D4 ratify + ADR-007 Amendment
    Then ADR-050 D3/D4 Accepted（完整 ratify 待 Phase 47 v1.0.0）
    And ADR-014 第三十七次激活 D1-D5 通过
