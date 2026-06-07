# Phase 40 · governance-debt-cleanup-3
# 第三轮治理债清扫（镜像 Phase 31 / ADR-036 + Phase 33 / ADR-038）：清两组 code-local 真实治理 marker
# —— memory pin actor 入口透传 + L2 embedding 缓存访问序 LRU。
# ADR-045（Proposed→Accepted @ task-40.3）。0 新 dep / proto add-only / 0 schema migration / 默认 byte-equiv（ADR-004/008/015）。

Feature: governance-debt-cleanup-3 — memory pin actor 透传 + L2 缓存访问序 LRU
  作为 ContextForge 维护者
  我希望补齐两组真实治理债 marker（pin actor 入口透传 + L2 访问序 LRU）
  以便 console 部署能把 pin 操作归因到真实调用方、L2 缓存按访问序而非插入序驱逐
  且对受阻 / 另一层的 marker 据 ADR-013 据实保持延后、不伪造、不夸大、不强行扩面

  # ---- task-40.1: memory pin actor add-only 透传（ADR-045 D1）----

  Scenario: PinMemoryRequest add-only actor=3 字段号冻结
    Given PinMemoryRequest 既有 memory_id=1 / pin=2
    When 我加 add-only "string actor = 3" 并 buf generate 重生 Go/Rust binding
    Then 既有 memory_id=1 / pin=2 字段号不动（ADR-015 D1）
    And in-crate prost wire-tag 断言 PinMemoryRequest{actor} 编码字段号 3
    And 0 新 dep

  Scenario: Rust pin() 空 actor 回落 console-api（默认 byte-equiv）
    Given Rust pin() RPC 收到 PinMemoryRequest
    When req.actor 为空（既有 client / 无 X-Actor header）
    Then set_pinned_with_actor 第三参回落 "console-api"（与改前硬编码值 byte-equiv，ADR-004）
    And pinned_by 写入 "console-api"

  Scenario: Rust pin() 非空 actor 透传写入 pinned_by
    Given Rust pin() RPC 收到 PinMemoryRequest
    When req.actor 为 "alice"
    Then set_pinned_with_actor 第三参透传 "alice"
    And pinned_by 写入 "alice"

  Scenario: Go handleMemoryPin 读 X-Actor header 透传到 Pin(actor)
    Given console-api POST /v1/memory/{id}/pin
    When 请求带 header "X-Actor: alice"
    Then handleMemoryPin 读 r.Header.Get("X-Actor") 并调 Pin(id, pin, "alice")
    And grpcclient 填 pb.PinMemoryRequest.Actor="alice"
    And 无 X-Actor header 时 Pin(id, pin, "")（缺省空串）
    And ADR-022 D2 宽松 body 契约不改

  Scenario: pin actor 认证身份据实延后（ADR-013 不夸大）
    Given 本 phase 交付调用方透传（actor 取自 header，未做认证校验）
    When 我据实记录边界
    Then actor 是调用方声明的标识、非已认证身份
    And 认证身份标 [SPEC-DEFER:phase-future.memory-actor-authenticated-identity]
    And 其它 memory RPC 的 actor 透传标 [SPEC-DEFER:phase-future.memory-actor-all-rpc]

  # ---- task-40.2: L2 embedding 缓存访问序 LRU（ADR-045 D2）----

  Scenario: L2 命中 bump 隐式 rowid → 访问序 LRU 驱逐最久未用
    Given L2 SQLite embedding_cache 有限 cap=2，put a 再 put b
    When get a 命中（仅 l2_cap>0 时 INSERT OR REPLACE 原样回写命中行 bump 隐式 rowid 到表尾）
    And put c 触发驱逐
    Then 驱逐 b（最久未用 = 最小 rowid），保留 a
    And 对比 Phase 33 插入序 FIFO（命中不 bump）会驱逐 a
    And 复用既有隐式 rowid、0 新 dep / 0 schema migration

  Scenario: cap==0 不 bump 保插入序（零额外写）
    Given L2 cap==0（不限）
    When get 命中
    Then 不 bump（不限容量下无驱逐 → LRU 序无意义）
    And 行为同 Phase 33 插入序基线、无写放大

  Scenario: 据实更正 Phase 33 真-LRU 假设（ADR-013）
    Given Phase 33（ADR-038 A2/D4）把真 LRU 据「须加 created_at 列 + ALTER」延后
    When grounding 更正该假设
    Then 命中 bump 隐式 rowid 即得访问序 LRU、不须时间列、0 schema migration
    And 与 Go memstore 命中 move-to-front（task-33.2）同技法
    And Phase 33 D1 的 rowid-FIFO（row-count cap）是正确且必要前序、不溯改其正文（ADR-014 D5）

  Scenario: L2 命中 bump 写放大 + opt-in-path 现网零影响据实记
    Given 命中 bump 给 L2 读路径加一次行重写（写放大）
    And with_sqlite 无生产调用点（Phase 33 D1 已标 opt-in-path，出厂 daemon 走 memory-only L1）
    When 我据实记录边界
    Then 写放大是访问序 LRU 固有代价（同 Go memstore move-to-front）
    And 本项是 opt-in 路径语义补全、非已确认线上问题（不夸大）

  # ---- task-40.3: v0.33.0 收口 + 默认零依赖守线 + honest-defer 边界 ----

  Scenario: 其余治理 marker 据实保持延后不强行扩面（ADR-013，honest over padding）
    Given vector-dim-feature-enforce 须 feature build / tracestore-multi-workspace-strict 余下读路径 / chunk-source-type-filter 须 import-path schema migration
    When 本焦点小版本据实分级
    Then 三者均据实保持延后、不强行扩面
    And vector-dim-feature-enforce 续 [SPEC-DEFER:phase-future.vector-dim-feature-enforce]
    And tracestore 余下读路径续 [SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]
    And chunk filter 续 [SPEC-DEFER:phase-future.chunk-source-type-filter] / [SPEC-DEFER:phase-future.chunk-agent-scope-filter]

  Scenario: v0.33.0 收口 + 默认零依赖守线
    Given task-40.1 + task-40.2 全 Done
    When task-40.3 收口
    Then scripts/console_smoke.sh v30[49/49]（pin actor 透传 + L2 访问序 LRU）+ TestTask403 无 [37/37]..[48/48] 回归
    And ADR-045 据 D1-D3 真实测试 ratify Proposed→Accepted
    And ADR-032 add-only Phase-40 Amendment（pin actor 透传维度兑现）
    And ADR-038 + ADR-027 add-only Phase-40 Amendment（L2 true-LRU 维度兑现 + 真-LRU 假设据实更正）
    And ADR-015 add-only Amendment（proto add-only field）
    And 默认行为 / proto（add-only field）/ 既有契约不变 + 0 新 dep + 0 网络（ADR-004/008）
    And 真实 v0.33.0 tag/run/digest/tlog post-tag-push 回填（ADR-013 不预填）
