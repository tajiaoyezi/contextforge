# Phase 48 · v1.0.1-patch
# v1.0 收口后审查发现的 4 个残留的 patch 修复。
# P0: CLI version 字符串过时（cli.go 0.38.0-dev + Dockerfile 无 ldflags → 镜像报错版本）。
# P1-P3: docs/decisions/README.md ADR-050 漏 Accepted + README Latest 段描述过时 + example.toml header 过时。
# 🟢 代码 + CI + 文档 / 0 dep / 0 migration / 0 proto / 0 schema。
# ADR-014 第三十九次激活。

Feature: v1.0.1-patch — v1.0 收口审查残留修复（P0 CLI version ldflags + P1-P3 文档）
  作为 ContextForge 用户
  我希望 v1.0.1 镜像的 `contextforge version` 正确报版本号（1.0.1，非 0.38.0-dev）
  且 v1.0 文档无过时残留（ADR-050 状态 / README 描述 / example.toml header 一致）
  以便 D2 API/CLI 冻结承诺兑现（version 子命令输出正确版本）

  # ---- task-48.1: P0 CLI version ldflags 注入 ----

  Scenario: CLI version 字符串修复（D2 API/CLI 冻结缺陷）
    Given internal/cli/cli.go Version 默认值是 "0.38.0-dev"（Phase 45 task-45.3 加）
    And Dockerfile go build 无 ldflags 注入 + release.yml 无 build-args
    When cli.go Version 默认值 → "1.0.1-dev" + Dockerfile ARG VERSION + ldflags -X cli.Version + release.yml build-args
    Then v1.0.1 镜像 `contextforge version` 打印 tag 版本（1.0.1）非 0.38.0-dev
    And cli.go 默认值兜底（本地 go build 也报 1.0.1-dev 非 0.38.0-dev）

  # ---- task-48.1: P1-P3 文档残留清理 ----

  Scenario: docs/decisions/README.md ADR-050 状态 flip Accepted
    Given Phase 46 建 ADR 访客索引时 ADR-050 是 "Proposed (partial D1/D2)"
    And Phase 47 完整 ratify Accepted 但漏更新此访客索引
    When flip ADR-050 行为 "Accepted (full D1/D2/D3/D4)"
    Then docs/decisions/README.md ADR-050 与 adapter ADR 索引 + ADR-050 文件 Status 一致（全 Accepted）

  Scenario: README Latest 段描述 + example.toml header 版本刷新
    Given README Latest 段描述残留 v0.39.0"第二步"措辞
    And contextforge.example.toml header 仍写 v0.38.0
    When Latest 段描述 → v1.0 收口终点 + example.toml header → v1.0.1
    Then README + example.toml 无 v0.x 残留版本引用

  Scenario: v1.0.1 closeout + v1.0.1 tag（patch release）
    Given task-48.1 全交付（P0 ldflags + P1-P3 文档）
    When smoke v38[57/57] + release docs + v1.0.1 tag push
    Then v1.0.1 GitHub Release 对象自动创建（D4 流程第三次实践）
    And ADR-014 第三十九次激活 D1-D5 通过
