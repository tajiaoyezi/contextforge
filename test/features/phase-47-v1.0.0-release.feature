# Phase 47 · v1.0.0-release
# v1.0 收口冲刺终点：ADR-050 完整 ratify Proposed→Accepted + maturity label flip Pre-1.0→v1.0.0 + known limitations。
# Phase 45/46 已交付 D1-D4 全维度，本 phase 完整 ratify + maturity 里程碑声明 + v1.0.0 tag。
# 🟢 纯文档 + tag + 0 代码逻辑 / 0 dep / 0 migration / 0 proto / 0 schema。
# ADR-050 完整 ratify Accepted / ADR-013 known limitations honest-defer / ADR-014 第三十八次激活。

Feature: v1.0.0-release — v1.0 收口终点（完整 ratify + maturity label flip + known limitations）
  作为 ContextForge 用户/维护者
  我希望项目正式声明 v1.0.0 成熟度里程碑（ADR-050 完整 ratify Accepted）
  以便有一个明确的稳定承诺（recall@5/@10=1.0 超 PRD 北极星 + API/CLI 冻结 + 文档对齐 + GitHub Release 流程）
  且 v1.0 不含的能力（multi-user/认证/自动更新/arm64 native + large-corpus benchmarks）诚实列为已知限制推 v2.0
  而 v1.0.0 是成熟度声明不是功能声明（ADR-013 honest-defer）

  # ---- task-47.1: maturity label flip + ADR-050 完整 ratify + known limitations ----

  Scenario: README maturity label flip Pre-1.0 → v1.0.0（成熟度里程碑声明）
    Given README maturity label 是 "Pre-1.0，v1.0 收口中"（Phase 46 诚实未虚标）
    And ADR-050 D1-D4 全维度已 Phase 45/46 交付 + 验证
    When flip maturity label 为 "v1.0.0" + pin v0.39.0→v1.0.0
    Then README maturity label = v1.0.0（无 Pre-1.0）+ pin = v1.0.0
    And 这是诚实里程碑（D1-D4 全验证，非虚标）

  Scenario: ADR-050 完整 ratify Proposed → Accepted（D1-D4 全真实交付验证）
    Given ADR-050 处于 "部分 ratify D1/D2/D3/D4"（Proposed）
    When Phase 47 closeout 据 Phase 45/46 真实 CI 验证 D1-D4 全交付
    Then ADR-050 Status = Accepted（D1 能力 + D2 API/CLI 冻结 + D3 文档对齐 + D4 GitHub Release 全 ✅）

  Scenario: v1.0.0 Release notes 列已知限制（active SPEC-DEFER 按 category 归类）
    Given 项目有 ~180 个 active SPEC-DEFER markers（phase-future.*）
    When 按 category 归类为 v1.0 known limitations（Retrieval quality / Memory / Observability / Release-CI / Interfaces / Platform）
    Then RELEASE_NOTES v1.0.0 段 + CHANGELOG [v1.0.0] 列 6 category known limitations
    And honest-defer（不伪造完成，显式列 + 指向 roadmap §4 backlog）

  Scenario: v1.0.0 closeout + v1.0.0 tag（major version 里程碑）
    Given task-47.1 全交付（maturity flip + ADR-050 Accepted + known limitations）
    When smoke v37[56/56] + release docs + v1.0.0 tag push
    Then v1.0.0 GitHub Release 对象自动创建（D4 Phase 46 流程）
    And ADR-014 第三十八次激活 D1-D5 通过
