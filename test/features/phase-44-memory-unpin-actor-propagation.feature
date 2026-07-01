# Phase 44 · memory-unpin-actor-propagation
# 闭环 pin/unpin actor 透传不对称——Phase 40 task-40.1 给 pin 加了 actor 透传，unpin 漏了。
# grounding 发现真实价值在 audit/event（store pinned=false 丢弃 actor）：emit_audit_and_event
# 加 actor 参数让 audit/event source 归因真实调用方（pin 顺带闭环）。
# 🟢 纯本地单测 + 0 dep / 0 migration / proto add-only (actor=2) + 默认 byte-equiv（空 actor 回落）。
# 认证身份 🔴 honest-defer；deprecate/softdelete/harddelete actor 透传 🔴 honest-defer（本 phase 仅做共用基础）。
# ADR-049（Proposed→Accepted @ task-44.3）/ ADR-032/045 add-only Amendment（ADR-004/008/013/015/021/022）。

Feature: memory-unpin-actor-propagation — unpin actor 透传 + audit/event source 归因
  作为 ContextForge 维护者
  我希望闭环 pin/unpin actor 透传不对称，让 unpin 的 audit log + event stream 归因到真实调用方
  且 pin 顺带闭环（audit/event 也归因），默认 byte-equiv（空 actor 回落）
  且认证身份 + deprecate/softdelete/harddelete 据实 honest-defer（ADR-013）

  # ---- task-44.1: unpin actor 透传 + audit/event source 归因（ADR-049 D1/D2/D3）----

  Scenario: unpin actor 进 audit/event source（真实归因，非空透传）
    Given Phase 40 task-40.1 给 pin 加了 actor 透传，unpin 漏了（memory.rs:298 硬编码 "console-api"）
    And store set_pinned_with_actor(pinned=false) 丢弃 actor（store.rs:192-196，故透传到 store 是空透传）
    And emit_audit_and_event 不携 actor + source 硬编码（audit "console-api" / event "contextforge-core"）
    When unpin handler 透传 actor + emit_audit_and_event 加 actor 参数（audit/event source 用 actor）
    Then unpin(actor="bob") → audit source "bob" / event source "bob"（真实归因到调用方）
    And 这是 unpin actor 透传的真实落点（非 store pinned_by，因 unpin 清 pin 快照）

  Scenario: pin 顺带闭环（消除 audit/event 不归因残余不对称）
    Given pin handler 虽透传 actor 到 store pinned_by，但其 emit_audit_and_event 不携 actor
    When pin handler 传 actor 给升级后的 emit_audit_and_event
    Then pin(actor="alice") → audit source "alice" / event source "alice"（顺带闭环）
    And add-only byte-equiv（既有 pin 行为不变，仅 audit/event source 由 actor 取代硬编码）

  Scenario: 默认 byte-equiv（空 actor 各自回落原值，ADR-004）
    Given emit_audit_and_event 空 actor 守护
    When 空 actor unpin/pin
    Then audit source "console-api"（既有值 byte-equiv）+ event source "contextforge-core"（既有值 byte-equiv）
    And deprecate/softdelete/harddelete 传固定值 byte-equiv（这 3 RPC 真实透传统续延后）

  Scenario: Go handleMemoryUnpin 读 X-Actor 透传（镜像 pin handlers.go:559）
    Given handleMemoryUnpin 不读 X-Actor（与 handleMemoryPin 不对称）
    When handleMemoryUnpin 读 X-Actor + Unpin(id, actor) + grpcclient pb.UnpinMemoryRequest.Actor
    Then Go 透传链 4 处对称 pin（types/grpcclient/handlers/memstore）+ proto add-only actor=2

  # ---- task-44.3: v0.37.0 收口 + honest-defer + 0-dep 守线 ----

  Scenario: 认证身份 + deprecate/softdelete/harddelete 据实 honest-defer（ADR-013）
    Given 本 phase 交付调用方透传（audit/event source 归因）
    When 评估认证身份（X-Actor → 已认证 auth subject）
    Then 须 console-api 鉴权层 → 🔴 honest-defer [SPEC-DEFER:phase-future.memory-actor-authenticated-identity]
    When 评估 deprecate/softdelete/harddelete actor 透传
    Then Deprecate/SoftDelete 需 7 层+新 migration / HardDelete 须 audit 重设计 → 🔴 honest-defer [SPEC-DEFER:phase-future.memory-actor-all-rpc]
    And 本 phase 仅做 emit_audit_and_event actor 参数共用基础，这 3 RPC 未来顺带受益

  Scenario: v0.37.0 收口 + 默认零依赖守线
    Given task-44.1 Done
    When task-44.3 收口
    Then smoke v33→v34 [53/53]（unpin X-Actor 端到端 / 不可达诚实归因 unit）+ TestTask443 无 [37/37]..[52/52] 回归
    And ADR-049 据 D1-D4 ratify Proposed→Accepted + ADR-032/045 add-only Phase-44 Amendment
    And 0 新 dep + 0 network + 0 migration + proto add-only actor=2 + 默认 byte-equiv
    And 真实 v0.37.0 tag/run/digest/tlog post-tag-push 回填（用户全权授权 ADR-012，ADR-013 不预填）
