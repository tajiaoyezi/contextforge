# language: en
# Maps to:
#   - docs/specs/tasks/task-6.2-rest-api.md
#
# 轻量 BDD（s2v §9.2）；占位场景由 task agent 实施时填 Given/When/Then。

Feature: daemon
  In order to <TBD-by-user>
  As <TBD-by-user>
  I want 本地 REST API server (/v1/*) + 长任务调度 + 本地监听/token 安全基线

  # ---
  # Maps to: docs/specs/tasks/task-6.2-rest-api.md
  Scenario: SCEN-6.2.1 — 对应 AC1（/v1/search 契约一致）
    Given `contextforge serve` 已启动 + token 已生成 + 索引 fixture collection
    When 客户端发起 `POST /v1/search` 带 `Authorization: Bearer <token>` + JSON body
    Then daemon 返 200 + JSON SearchResponse（含 results 数组 + 12 字段 RetrievalResult）
    And gRPC Status 错误自动映射到 HTTP code（InvalidArgument→400 / FailedPrecondition→412 / NotFound→404 / Internal→500）

  Scenario: SCEN-6.2.2 — 对应 AC2（其余 /v1/* 可用）
    Given REST server 已启动 + 有效 token
    When 客户端发起 `GET /v1/chunks/{id}` / `GET /v1/collections` / `POST /v1/import` / `POST /v1/eval/run`
    Then chunks/{id} 命中返 200 + RetrievalResult JSON；未命中返 404
    And collections 返 200 + JSON `{collections: [{id, chunk_count, last_indexed_at}]}`
    And import 与 eval/run 返 501 + JSON body `{"error":"deferred to phase 8"}`（§2A 决策 B）

  Scenario: SCEN-6.2.3 — 对应 AC3（默认本地监听禁 0.0.0.0）
    Given 用户执行 `contextforge serve --addr=0.0.0.0:8080`
    When CLI 解析 flag + 校验监听地址
    Then 启动失败，stderr 报 "refusing wildcard bind 0.0.0.0 (loopback only)"
    And 默认（无 --addr 无 --unix）走自动选 loopback 端口（127.0.0.1:N）

  Scenario: SCEN-6.2.4 — 对应 AC4（token 0600）
    Given `<data_dir>/token` 不存在
    When `contextforge serve` 启动调用 loadOrGenerateToken(data_dir)
    Then 生成 32-byte 随机 + hex 编码（64 字节十六进制）+ 写入文件 `<data_dir>/token`
    And 文件权限 `0600`（POSIX 平台校验；Windows ACL 留 Phase 8）
    And 重启时同 token 复用（不重新生成）

  Scenario: SCEN-6.2.5 — 对应 AC5（无 token 拒绝 + 审计）
    Given REST server 已启动 + 已生成 token
    When 客户端发起 `POST /v1/search` **不带** Authorization header
    Then 返 401 + JSON body `{"error":"missing or invalid token"}`
    And `<data_dir>/audit-rest.log` 追加 1 条 JSON 事件（endpoint / status=401 / timestamp，不记 token 值 / 不记请求 body）
