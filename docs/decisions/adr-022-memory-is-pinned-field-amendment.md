# ADR `022`: `memory-is-pinned-field-amendment`

**Status**: Proposed
**Category**: 协议接口 / 契约演进 / Memory schema
**Date**: 2026-05-28
**Decided By**: tajiaoyezi objective + main agent execution + ContextForge-Console PR #91/#93 backlog 反馈
**Related**: ADR-015 (console-contract-v1-compatibility) / ADR-017 (console-contract-completion-22-endpoint) / ADR-021 (memory-event-bus-bridge) / Phase 13 (memory-rest-surface) / Phase 15 / Phase 16 / Phase 17 (is-pinned-amendment)

## Context

ContextForge-Console PR #91/#93 backlog 11 项中**最后一项剩余**（v0.9.0 ship 后 10/11 closed）：

> **P2 #6 — `MemoryItem.is_pinned` 字段缺失**：Console UI Memory 列表 / 详情面板期望按 `is_pinned` 排序（pinned 项排前）+ 显示 pin 状态图标；当前 `MemoryItem`（既 ContextForge 侧 `internal/contractv1/contractv1.go` 又 Console 侧 `internal/contractv1/contractv1.go`）**没有** `is_pinned` 字段；Console UI 只能通过查 `MemoryOperation.op_type=pin` 历史**推断**当前状态，逻辑脆弱（unpin → 历史中仍有 pin 记录）。

历史背景：

- **Phase 13 (v0.6 ship)**：5 Memory RPC 落地（List / Get / Pin / Deprecate / SoftDelete）；当时 `MemoryItem` schema 锁定 9 字段（memory_id / agent_scope / content_preview / source_type / source_ref / created_at / updated_at / hit_count / status），**不含** `is_pinned`
- **Phase 15 (v0.8 ship)**：通过 ADR-021 memory.* → EventBus bridge 让 `memory.pin` / `memory.unpin` event 实时桥接到 events stream — 但**仍未**让 Console UI 知道**当前** pin 状态（events 是动作流，不是状态快照）
- **Phase 16 (v0.9 ship)**：剩余 backlog 4 项（P3 + P4）closure 收口；P2 #6 因需 **cross-repo coordination**（ContextForge 与 Console 双方 contractv1.go 同步加字段）独立留 Phase 17

**为什么不是 v0.6 / v0.7 / v0.8 / v0.9 ship**：

- **v0.6**：Phase 13 ship 时 Console UI Memory 面板尚未实施，pin 字段未被显式要求
- **v0.7**：22-endpoint conformance 是最高优先级，`MemoryItem` 字段集合属于 ADR-015 D5 "schema 冻结"范围，需 amendment ADR 才能解锁；v0.7 不引 amendment
- **v0.8**：Phase 15 收口 6/11 backlog（P0 + P1 + P2#7），`is_pinned` 作为 P2 #6 留 ADR-015 D5 BREAKING window；事实上 Phase 15 添加 ADR-020/021 时同样面对"add-only 字段"问题，已用 add-only schema 演进先例打通
- **v0.9**：Phase 16 关注 P3 + P4（CI / deploy / persist / long-poll），无 schema 字段变更；`is_pinned` 留 Phase 17 单独 amendment ADR 更清晰

**为什么需要独立 ADR**：

- ADR-015 D5 明确声明"contract v1 字段集合锁定"，任何字段增量需经 **amendment ADR** 走签批路径
- 跨仓 schema 演进（ContextForge `internal/contractv1` + Console `internal/contractv1`）需要双向锚点；ADR 是双方都能引用的 source of truth
- 后续若有更多字段需要 add-only amendment（如 `tags`、`pinned_at`），可沿用本 ADR pattern

## Decision

ContextForge v0.10+（Phase 17）通过 **5 个 Decision** 给 `MemoryItem` 添加 `is_pinned bool` 字段：跨仓 add-only + Console UI 直接渲染 + 持久化由 Pin RPC 写穿 + 不破坏既有 v0.6-v0.9 client。

### D1 — Add-only 字段：`MemoryItem.is_pinned bool`

ContextForge 侧 `internal/contractv1/contractv1.go::MemoryItem` 加新字段：

```go
type MemoryItem struct {
    MemoryID       string            `json:"memory_id"`
    AgentScope     string            `json:"agent_scope"`
    ContentPreview string            `json:"content_preview"`
    SourceType     string            `json:"source_type"`
    SourceRef      string            `json:"source_ref"`
    CreatedAt      time.Time         `json:"created_at"`
    UpdatedAt      time.Time         `json:"updated_at"`
    HitCount       int               `json:"hit_count"`
    Status         string            `json:"status"`
    IsPinned       bool              `json:"is_pinned"`           // ADR-022 D1 新增
    Availability   FieldAvailability `json:"field_availability"`
}
```

proto 侧 `core/proto/console_data_plane.proto::MemoryItem` 同步加 `bool is_pinned = N`（add-only 字段，序号在既有字段后追加；具体序号在 task-17.1 实施时按 proto 文件当前最大序号 + 1 确定）。

**理由**：

- `bool` 类型 — 与 `Pin RPC` 既有"pin / unpin"二态对齐；不需要 `pinned_at` timestamp（如需历史溯源，查 `MemoryOperation.op_type=pin/unpin` audit log）
- JSON tag `is_pinned`（snake_case）— 与既有 `memory_id` / `hit_count` / `content_preview` 命名风格一致
- **不**用 `*bool`（指针型 omitempty）— Memory state 永远有值（默认 `false`），缺省语义会让 Console UI 渲染歧义

### D2 — Pin RPC 写穿语义不变 + 新增 IsPinned 持久化

既有 `Pin RPC`（task-13.1 实施）行为：

- `POST /v1/memory/{id}/pin` body `{"pin": true|false}` → Rust `MemoryService.Pin(req: {memory_id, pin: bool})` → 调 `SqliteMemoryStore.set_pinned(memory_id, pin)`（task-17.1 新增方法）
- 写穿 SQLite `memory_items` 表新增 `is_pinned INTEGER NOT NULL DEFAULT 0` 列（migration `0017_memory_items_add_is_pinned.sql`）
- emit `audit_log` (既有) + `EventBus.send(memory.pin / memory.unpin)` (ADR-021 D1 既有)

**理由**：

- 复用 `Pin RPC` 既有调用点；不引入新 endpoint
- SQLite migration add-only column with default — 既有数据 backfill 为 `false`（与 `hit_count` 列相同 pattern）
- 不动 ADR-021 D2 event_type 字符串集合（`memory.pin` / `memory.unpin` 复用既有）

### D3 — List/Get 返 `is_pinned` 字段约定

- `GET /v1/memory[?...filter]` → `MemoryItem[]` 每项含 `is_pinned`
- `GET /v1/memory/{id}` → 单项 `MemoryItem` 含 `is_pinned`
- `MemMemoryStore` fallback 模式（ADR-018 deny 模式下不触发；`CONSOLE_API_FALLBACK_INMEM=1` 显式 opt-in 时）seed fixtures 默认 `is_pinned: false`，调 `MemMemoryStore.Pin(id, pin)` 同步更新内存状态

**理由**：

- 客户端从 List/Get 单次拿到 pin 状态 — 不需多请求合并（unlike 当前 query MemoryOperation history 推断）
- Fallback 模式行为对齐 — `CONSOLE_API_FALLBACK_INMEM=1` 不破坏 Console UI 测试体验

### D4 — Cross-repo 同步：ContextForge 与 Console contractv1.go 双向 amend

**实施顺序约定**（cross-repo coordination 关键）：

1. **Console 侧先 ship**: Console 主仓 `internal/contractv1/contractv1.go::MemoryItem` 加 `IsPinned bool` 字段（add-only；不消费 ContextForge 任何响应中的 `is_pinned`；旧 ContextForge v0.9 client JSON 解析缺省 `is_pinned` → Go `bool` 默认值 `false`，Console UI 渲染"未 pin"是合理 fallback）
2. **ContextForge 侧后 ship**: 见 Phase 17 task-17.1 — `internal/contractv1/contractv1.go::MemoryItem.IsPinned` + proto MemoryItem + Rust SqliteMemoryStore + Go REST 序列化全链路

**理由**：

- Console UI 先 ship 后，向 v0.9 ContextForge daemon 拉取 Memory 列表时 JSON 缺省 `is_pinned` → 解析为 `false` → UI 显示"未 pin"（与"无 pin 数据"行为一致）— **零破坏**
- ContextForge ship 后再次 hit，Console UI 收到真实 `is_pinned` — **逐步加强**
- 反序（ContextForge 先 ship）也可行但 Console UI 无法立刻消费；先 Console 节奏更顺

### D5 — Phase 17 Pending → Ready trigger

**Phase 17 spec / task-17.1 spec Status = `Pending`** 在 ADR-022 与 Phase 17 scaffold ship 后：

- 触发条件 = **Console 主仓 PR ship `internal/contractv1/contractv1.go::MemoryItem.IsPinned` add-only field**（pre-condition D4 第 1 步）
- 触发后由 ContextForge 主 agent 把 `Pending → Ready`（状态机正常推进；不需要额外 ADR amendment）
- 触发信号 = 用户人工转发 Console PR merge SHA 给 ContextForge 主 agent；ContextForge 主 agent 验证 Console master HEAD 含 IsPinned 字段后启动 task-17.1 实施

**理由**：

- "Pending" 显式标识"等 cross-repo 信号"，区别 "Ready"（可启动）/ "Draft"（写 spec）/ "Blocked"（被阻塞）；信号性强
- 由用户做 cross-repo bridge（与 v0.8 / v0.9 ship 后通知 Console 团队是同一 pattern）— 与 ADR-011 单驱动 + ADR-012 主 agent 自治一致

## Trade-offs / Conscious limitations

- **不引入 `pinned_at` timestamp**：当前 ADR 仅 `bool`；如未来 UI 需"按 pin 时间排序"，留 `[SPEC-DEFER:phase-future.memory-pinned-at-timestamp]` amendment ADR-023+
- **不引入 `pin_actor` 字段**：谁 pin 的留 `MemoryOperation.actor` audit log 查（不污染 `MemoryItem` schema）
- **不重写历史 audit log 推 is_pinned 当前态**：Console UI 首次升级到 amend 后 client + 拉 v0.9 ContextForge daemon 看 `is_pinned=false`（缺省）— 用户 next pin/unpin 操作后才更新；接受作为 "字段 backfill 不溯源"trade-off [SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]
- **Console UI 渲染依赖 ContextForge daemon ≥ v0.10**：如 Console UI 启用"按 pin 排序"feature flag 但用户连 v0.9 daemon → `is_pinned` 全 `false` → UI 应有 fallback 文案 "Backend ≥ v0.10 才显示 pin 状态" 或自动降级隐藏 pin 列；不在本 ADR scope
- **不动 `Status` 字段语义**：`MemoryItem.Status` 仍是 `active / deprecated / soft_deleted` 三态；`IsPinned` 与 `Status` 正交（pinned + deprecated 合法状态：用户 pin 过但已废弃）
- **MemMemoryStore SeedFixtures 默认 `false`**：5 个 fixture item 全部 `is_pinned: false`；如 UI 自动化测试需要 pinned fixture，由 task-17.1 spec §3 内决定（推荐保留至少 1 个 fixture `is_pinned: true` 作 UI 渲染验证）

## Verification (Phase 17 task-17.1 ship 时)

```bash
# 1. proto add-only 验证
git diff master..HEAD -- proto/contextforge/console_data_plane/v1/console_data_plane.proto | grep -E '^(-|\+)\s*(string|bool|int|repeated|message)'
# expect: 仅 + 行；MemoryItem message 新增 bool is_pinned = N 字段

# 2. Rust unit test
cargo test -p contextforge-core --lib memory_is_pinned::tests
# expect: ≥3 PASS (Pin true 后 get 返 is_pinned=true / Pin false 后 false / List 返字段)

# 3. SQLite migration 验证
sqlite3 ~/.contextforge/data/workspaces.db ".schema memory_items" | grep is_pinned
# expect: is_pinned INTEGER NOT NULL DEFAULT 0

# 4. Go REST 序列化验证
curl http://localhost:48181/v1/memory | jq '.[0].is_pinned'
# expect: false (or true after Pin RPC)

curl -X POST -H "X-Confirm: yes" http://localhost:48181/v1/memory/mem-1/pin -d '{"pin": true}'
curl http://localhost:48181/v1/memory/mem-1 | jq .is_pinned
# expect: true

# 5. cross-repo client compatibility 验证
# Console v0.7 client (pre-amend) 解析 v0.10 response:
echo '{"memory_id":"x","is_pinned":true,...}' | go run cmd/legacy-client-test/main.go
# expect: success; is_pinned 字段 忽略 不破坏

# Console v0.10+ client (post-amend) 解析 v0.9 response:
echo '{"memory_id":"x",...no_is_pinned...}' | go run cmd/new-client-test/main.go
# expect: success; is_pinned 字段 = false 默认值
```

## Rollback path

如 Phase 17 task-17.1 ship 后发现：

- `is_pinned` SQLite migration 失败（生产 DB 不可应用）→ revert task-17.1 migration commit；ContextForge v0.10.0.1 patch ship 仅含 proto + Go 字段（运行时 `is_pinned` 永远 `false`），不破坏 Console UI（字段存在但全 false）
- Console UI 端"按 pin 排序"feature 引入回归 → Console 端 patch（不影响 ContextForge）
- 极端：proto 字段序号冲突（被并行 PR 占用）→ revert + 重新分配序号；ADR-022 不撤回（设计意图无误）

**ADR-022 不撤回 default**（D1-D5 都是 add-only + cross-repo coord；rollback 路径是 patch fix 而非 ADR superseded）。

## Upgrade path

### Console UI 用户

- Console v0.7-v0.9 (pre-amend) → v0.10+ (post-amend): UI 端实施"按 pin 排序"+ pin 状态图标 visual closure
- ContextForge v0.6-v0.9 (pre-amend daemon) + Console v0.10 client: `is_pinned` 缺省 `false` — UI 显示"全部未 pin" fallback；用户升级 ContextForge ≥ v0.10 后 UI 自动获真状态

### contractv1.go 客户端用户（CLI / 第三方集成）

- 升级 contractv1.go v0.9.x → v0.10.x: `MemoryItem` 含 `IsPinned bool` 字段
- 旧代码继续编译（add-only 不破坏）；新代码可读 `item.IsPinned` 字段

### Memory data 用户（自建 ContextForge stack）

- 升级 ContextForge v0.9 → v0.10: SQLite migration `0017_memory_items_add_is_pinned.sql` 自动应用；既有 memory items 全部 backfill 为 `is_pinned = 0`（false）
- 后续 Pin RPC 调用正常写穿 `is_pinned` 列 + emit event + audit log
