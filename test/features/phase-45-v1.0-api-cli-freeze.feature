# Phase 45 · v1.0-api-cli-freeze
# v1.0 收口冲刺第一步：立 v1.0 锚点（ADR-050）+ API/CLI 冻结（D2）。
# 项目从未立过 v1.0 锚点（PRD/roadmap/README 四处查证）——本 phase 立 ADR-050 正式定义。
# 交付 daemon REST 移除 2 个 501 未实装 + 实装 chunk_count + CLI --version/--help + example.toml 补全。
# 🟢 纯本地 + 0 dep/0 migration + daemon REST 移除是 v1.0 前 breaking（major 边界）。
# v1.0 不含 multi-user/认证/自动更新/arm64（推 v2.0）。
# ADR-050 Proposed（Phase 47 v1.0.0 完整 ratify）/ ADR-007 add-only Amendment（ADR-004/008/013/015/017）。

Feature: v1.0-api-cli-freeze — 立 v1.0 锚点 + API/CLI 冻结准备
  作为 ContextForge 维护者
  我希望正式定义 v1.0（ADR-050）并清理 API/CLI 冻结的 P0 阻塞项
  以便为 v1.0.0 正式发版奠定锚点（D2 API/CLI 冻结；D3 文档 Phase 46；D4 发布 Phase 46-47）
  且 v1.0 不含 multi-user/认证/自动更新/arm64（honest-defer 推 v2.0，ADR-013）

  # ---- task-45.1: ADR-050 v1.0 定义（承 ADR-017 悬空 v1.0 gate）----

  Scenario: ADR-050 正式定义 v1.0（4 维度 + 不含清单）
    Given 项目从未立过 v1.0 锚点（PRD P0 是 v0.1 的 / roadmap 零 v1.0 / README 无成熟度标签）
    And ADR-017 出现过悬空的 "v1.0 release gate" 但从未被承接
    When 立ADR-050 Proposed
    Then D1 能力锚点（v0.1 P0 已满足且远超，recall@5/@10=1.0 超北极星）
    And D2 API/CLI 冻结锚点（proto 已 FROZEN + daemon REST 清 501 + CLI --version）
    And D3 文档锚点（README 重构 Phase 46）+ D4 发布锚点（GitHub Release Phase 46-47）
    And 不含清单：multi-user/认证身份/自动更新/arm64 native（推 v2.0）

  # ---- task-45.2: daemon REST 冻结（移除 501 + 实装 chunk_count）----

  Scenario: daemon REST 移除 2 个 501 未实装（v1.0 前 breaking，major 边界）
    Given daemon REST 有 5 endpoint，其中 POST /v1/import + POST /v1/eval/run 是 501 未实装（§2A 决策 B）
    And console-api /v1/index-jobs + /v1/eval-runs 已完整覆盖 import/eval
    When 移除 handleImport/handleEval + 路由注册
    Then daemon REST 留 search/chunks/collections 3 个真实端点
    And 这是 v1.0 前允许的 breaking change（major 版本边界）；release notes 显式记

  Scenario: chunk_count 实装（真实 COUNT 非 placeholder 0）
    Given handleCollections chunk_count 恒 0（v0.1 placeholder）
    When 实装打开 collection metadata.sqlite COUNT 查询
    Then fixture 索引后 chunk_count > 0（真实值）+ best-effort（单 collection 失败不阻断列表）

  # ---- task-45.3: CLI 冻结（--version + --help + example.toml）----

  Scenario: CLI version 子命令（v1.0 产品必须有版本可查）
    Given CLI 无 --version/version 子命令
    When 加 version 子命令（从 main.go 版本常量注入）
    Then contextforge version 打印版本（v1.0 产品硬伤修复）

  Scenario: 顶层 --help（修复 -h 落 unknown subcommand exit 2）
    Given cli.go:119-127 -h/--help 落 unknown subcommand exit 2
    When Execute 入口检测 -h/--help/help → 打印子命令清单
    Then contextforge --help 不 exit 2 + 打印用法（新手可上手）

  Scenario: example.toml 补全 4 个检索 section
    Given example.toml 仅 16 行缺 [embedding]/[vector]/[reranker]/[retrieval]
    When 补全 4 section（镜像 config.go + env var 注释）+ 头部 v0.1→v0.38
    Then 用户凭 example.toml 可配置全部检索功能

  # ---- task-45.4: v0.38.0 收口 + ADR-050 部分 ratify ----

  Scenario: v0.38.0 收口 + ADR-050 部分 ratify（D1/D2）
    Given task-45.1/45.2/45.3 全 Done
    When task-45.4 收口
    Then smoke v34→v35 [54/54]（v1.0 API/CLI 冻结端到端）+ TestTask454 无 [37/37]..[53/53] 回归
    And ADR-050 部分 ratify（D1 能力已满足 / D2 API/CLI 冻结 Phase 45 交付）
    And D3 文档对齐续 Phase 46 / D4 GitHub Release 续 Phase 46-47 / v1.0.0 正式发版 Phase 47
    And ADR-007 add-only Amendment（v1.0 分发定义收窄为务实收口）
    And 0 新 dep + 0 migration + daemon REST 移除是 v1.0 前 breaking
