# Phase 43 · governance-debt-cleanup-4
# 第四轮治理债清扫，单聚焦 indexing-replay-e2e 拼接缺口——承 Phase 33 task-33.3（ADR-038 D3）血脉的"最后一公里"：
# mapper indexing_rows_to_pb_events（events.rs:438）已写好 + test_33_3_2 守护但从未在 live subscribe 路径调用。
# 4 拼接缺口（grounding 已亲自核实）：list 缺 since_ts / DataPlaneStores 无 indexing_event_store 字段 /
# serve_full full() 未传 store / subscribe replay 只 splice audit 漏 indexing。
# 本 phase 补 4 缺口：list_since + DataPlaneStores 字段 + serve_full 接线 + subscribe splice indexing replay。
# 🟢 纯本地单测 + 0 dep/0 migration（复用 0019）/0 proto + 默认 byte-equiv（since_ts<=0/store=None）。
# live daemon restart-then-replay e2e 🟡 honest-defer（ADR-013 不预填）。
# memory-actor-all-rpc（非小债）据实延后留独立 phase（roadmap §3.17/§3.22 "据实排小不凑数"）。
# ADR-048（Proposed→Accepted @ task-43.3）/ ADR-038 add-only Phase-43 Amendment（ADR-004/008/013/031/038）。

Feature: indexing-replay-splice — indexing replay mapper 接进 live subscribe 路径
  作为 ContextForge 维护者
  我希望把 Phase 33 投入的 indexing replay mapper 接进 EventsServer::subscribe live 路径
  以便 since_ts>0 的订阅者能收到 missed 的 indexing.progress/.cancelled/.error 事件（与 memory audit replay 对称）
  且默认 byte-equiv（since_ts<=0 / store=None 两条退化路径）、0 schema migration（复用 0019）、0 proto 改动
  且 live daemon e2e + memory-actor-all-rpc 据实 honest-defer 不强行扩面（ADR-013）

  # ---- task-43.1: indexing replay splice（ADR-048 D1/D2/D3）----

  Scenario: list_since since_ts 时序过滤（镜像 replay_events_from_audit）
    Given SqliteIndexingEventStore list(limit) 缺 since_ts 参数（replay 无法只取"自 since_ts 起 missed"事件）
    When add list_since(limit, since_ts)（since_ts>0 时 WHERE ts_unix >= ? ORDER BY id ASC LIMIT，since_ts<=0 不过滤返全量）
    Then since_ts=150 时返 ts>=150 的行（镜像 replay_events_from_audit 的 ts < since_ts → skip 语义，含等号边界）
    And since_ts<=0 时不过滤返全量 limit（与既有 list() 行为一致）
    And 既有 list(limit) 保留不动（test_33_3_2 等既有调用方不破）

  Scenario: DataPlaneStores 加 indexing_event_store 字段 + serve_full 接线
    Given DataPlaneStores（mod.rs:43-74）字段列表无 indexing_event_store（events subscribe 经 self.stores 读不可达）
    And serve_full（server.rs:756）局部已构造 indexing_event_store 传 IndexSessionBackend（写路径 OK）但 full() 9 参数未传入（读路径不可达）
    When DataPlaneStores add indexing_event_store: Option<Arc<SqliteIndexingEventStore>> 字段 + full() 加第 10 参数
    And 既有 constructor（new/with_eval/with_memory/with_runner/with_runner_and_bus）补 None byte-equiv
    And serve_full full() 传入 Some(indexing_event_store.clone())（store 已在，clone 读路径）
    Then 写路径 IndexSessionBackend 持原 Arc，读路径 DataPlaneStores 持 clone Arc（共享同一 Mutex<Connection>）
    And 既有 constructor 补 None byte-equiv（单测、非 serve_full 路径不接 indexing replay 退化现状）

  Scenario: subscribe splice indexing replay（audit 后、live 前；event_id 命名空间独立 dedup）
    Given events.rs subscribe replay 段（:241-250）只 splice memory audit replay，漏 indexing
    And indexing_rows_to_pb_events mapper（:438）已写好但从未在 live 路径调用
    When subscribe replay 段 after audit replay 加 indexing replay（since_ts>0 时 list_since + mapper，合并进 replay Vec）
    Then splice 严格 audit replay 后、live forward（spawn）前（subscribe-first 保证不丢 live 事件，镜像 task-26.2）
    And 两类 replay 各 id ASC / ts ASC 内部有序，event_id 命名空间独立（evt-idx-{id} vs evt-audit-{id}）客户端 dedup
    And store None / lock 失败 / 空 → unwrap_or_default() 空切片（best-effort 镜像 audit :245-247）

  Scenario: 默认 byte-equiv（since_ts<=0 / store=None 两条退化路径，ADR-004）
    Given subscribe indexing replay splice 仅 req.since_ts > 0 时生效（与既有 audit replay :241 同守护）
    When since_ts<=0（订阅首连无 since_ts）
    Then 无 indexing replay（行为与现状 byte-identical）
    When indexing_event_store == None（旧 constructor / 单测不设）
    Then 无 indexing replay（退化到现状）
    And 仅 serve_full 生产路径（since_ts>0 + store=Some）时 indexing replay 生效

  # ---- task-43.3: v0.36.0 收口 + honest-defer 边界 + 0-dep/0-migration 守线 ----

  Scenario: live daemon restart-then-replay e2e 据实 honest-defer（ADR-013）
    Given 本 phase 交付 splice 拼接 + unit 级时序单测（TEST-43.1.1 list_since / TEST-43.1.2 subscribe splice）
    When 评估 live daemon restart-then-replay 端到端 e2e（真起进程 + 跨 restart 双窗口断言）
    Then 须 running daemon（须 console 跨进程）→ 🟡 honest-defer [SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]
    And 不预填 e2e、不夸大 unit splice 为 live e2e 已验（ADR-013）
    And ADR-048 据 unit 级已达维度 ratify（D1-D3 🟢）+ 如实记录 D4 live daemon e2e 受阻（不强 ratify e2e）

  Scenario: memory-actor-all-rpc 据实延后留独立 phase（非小债）
    Given grounding 显示 memory-actor-all-rpc 真实范围：Unpin 单 RPC store 层已有 slot（3 层无 migration）
    But Deprecate/SoftDelete 需 7 层改动 + 新 schema migration（set_status 无 actor 参数 + 无列记录 actor）
    And HardDelete 物理 DELETE 行后无法在行上存 actor（须 audit 层重设计 emit_audit_and_event 硬编码 source）
    When 据实分级
    Then 超"治理债小 phase 刻意小"定位（roadmap §3.17/§3.22 "据实排小不凑数"）
    And 本 phase 单聚焦 indexing-replay 不扩面，memory-actor-all-rpc honest-defer 留独立 phase [SPEC-DEFER:phase-future.memory-actor-all-rpc]

  Scenario: v0.36.0 收口 + 默认零依赖零迁移守线
    Given task-43.1 Done
    When task-43.3 收口
    Then scripts/console_smoke.sh v32→v33[52/52] indexing replay splice 可达断言（不可达诚实归因 unit TEST-43.1.2）+ TestTask433 无 [37/37]..[51/51] 回归
    And ADR-048 据 D1-D4 真实测试 ratify Proposed→Accepted（D1-D3 unit 🟢 / D4 live daemon e2e 🟡 honest-defer）
    And ADR-038 add-only Phase-43 Amendment（indexing-replay-e2e splice 维度兑现，live daemon e2e 续延后，不溯改 D5）
    And ADR-031（replay 范式源 task-26.2 引用）/ ADR-021（audit replay splice 镜像源）/ ADR-004（默认 byte-equiv）守线
    And 0 新 dep + 0 网络 + 0 schema migration（复用 0019）+ 0 proto 改动
    And 真实 v0.36.0 tag/run/digest/tlog post-tag-push 回填（ADR-013 不预填）
