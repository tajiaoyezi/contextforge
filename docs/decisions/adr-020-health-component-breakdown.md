# ADR `020`: `health-component-breakdown`

**Status**: Proposed (2026-05-26；将于 Phase 15 closeout / v0.8.0 ship 时 promote 到 Accepted)
**Category**: 协议接口 / 可观测性 / 健康检查
**Date**: 2026-05-26
**Decided By**: tajiaoyezi objective + main agent execution + ContextForge-Console PR #91/#93 backlog 反馈
**Related**: ADR-015 (console-contract-v1-compatibility) / ADR-016 (cross-process-rust-go-via-grpc-bridge) / ADR-017 (console-contract-completion-22-endpoint) / Phase 15 (console-functional-gap-closure)

## Context

ContextForge v0.7.0 ship 后 `/v1/health` endpoint 返 binary `{status: "healthy"|"degraded"|"unreachable"}`（`internal/contractv1/contractv1.go::CoreHealth`），无 5 链路（db / index / embed / retriever / eval）细分。

ContextForge-Console 团队 PR #91/#93 backlog 列项 #7（P2 优先级）：

> Console UI `CoreHealthCard` 期望 5 链路细分 — 用户视图把 ContextForge backend 当成 5 个子系统：SQLite (`db`) / Tantivy (`index`) / Embedding provider (`embed`) / Retriever (`retriever`) / Eval (`eval`)。当前 binary status 让 UI 只能显示 "整体 healthy/degraded"，无法定位哪条链路坏了。

**为什么不是 v0.7 / 不是 v0.9**：

- v0.7.0 ship 时 22-endpoint conformance 是最高优先级，5 链路 health 是 nice-to-have
- v0.8 收口 6/11 Console backlog（P0+P1+P2#7）是中等跨度窗口；5 链路 health 是 P2 中唯一具备 self-contained scope（其它 P2/P3/P4 需要跨仓 Console UI 同步开发）
- v0.9 计划 Phase 16 P2#6 `is_pinned` 字段 amendment（ADR-015 D5 BREAKING window）—— 与本 ADR 无强耦合，独立推进更清晰

## Decision

ContextForge v0.8.0 minor release（Phase 15 task-15.6）通过 **5 个 Decision** 给 `/v1/health` 加 5 链路 component breakdown：opt-in 行为 + add-only schema + 跨仓零冲突。

### D1 — 5 健康探针定义

5 子系统探针 1:1 映射 ContextForge backend 关键依赖：

| Component | 探针实现 | 失败语义 |
|---|---|---|
| `db` | SQLite ping (`SELECT 1` on `workspaces.db`) | `degraded` if connection error / `unreachable` if file missing |
| `index` | Tantivy `Index::open_in_dir` + reader load 验证 segment meta 可读 | `degraded` if open fails or meta corrupt |
| `embed` | Embedding provider 配置读取 (`config.toml` 段 + env `CONTEXTFORGE_EMBED_PROVIDER`) | `healthy` if config valid（不实际调远程 provider 避免 rate limit + secret 暴露）；`degraded` if config missing |
| `retriever` | 简单 `top_k=1` query exercise (`retriever.search(SearchOptions{ query: "health", top_k: 1, explain: false })`) | `degraded` if search returns Err；`healthy` if Ok |
| `eval` | `SqliteEvalStore.open(data_dir)` 验证 `eval_runs` 表 schema 可读 | `degraded` if migration not applied or DB error |

**理由**：5 探针覆盖 v0.4-v0.7 ship 的 5 个 backend 子系统；Console UI `CoreHealthCard` 5 链路面板的 1:1 字段填充；不引入新依赖。

### D2 — Schema add-only：`ComponentHealth` message + `CoreHealth.components` map

proto `console_data_plane.proto` 加 `ComponentHealth` message：

```proto
message ComponentHealth {
  string name = 1;            // "db" | "index" | "embed" | "retriever" | "eval"
  string status = 2;          // "healthy" | "degraded" | "unreachable"
  optional int64 latency_ms = 3;   // 探针执行耗时（毫秒）
  optional string error_reason = 4; // 失败时简短原因，成功时省略
}
```

既有 `CoreHealth` proto message（或 Go-side `contractv1.CoreHealth`）加新字段：

```proto
// Go-side: type CoreHealth struct { ... + Components map[string]ComponentHealth `json:"components,omitempty"` }
map<string, ComponentHealth> components = 6;  // key = component name; 仅 ?detailed=true 时填
```

**add-only 约束（ADR-015 D1）**：

- 不删 / 不改既有 `CoreHealth.Status` / `ContractVersion` / `LastConnectedAt` / `ErrorReason` / `MissingMustHaveFields` 字段
- `Components` map 在 `?detailed=false`（默认）时**省略**（JSON `omitempty`）— Console v0.7.x 客户端解析时跳过未知字段（contract v1 forward-compat）
- 不动既有 5 字段 JSON tag / 序号

### D3 — Query 参数 opt-in：`?detailed=true`

- `GET /v1/health` （default）→ 返既有 5 字段 binary CoreHealth；不跑 5 探针；不动既有 v0.4-v0.7 行为
- `GET /v1/health?detailed=true` → 跑 5 探针 + 返 `CoreHealth.Components` map；总耗时上限 200ms（5 探针 P95 各 < 40ms；超时 → 该探针 `status=degraded` + `error_reason="probe timeout"`）

**理由**：默认 binary 健康检查覆盖 99% 用例（docker healthcheck / k8s readinessProbe / 简单 curl）；5 探针 expensive（含 retriever query exercise + index segment load），不应每次 health hit 都跑；opt-in 让 Console UI / debug session 显式触发。

### D4 — Status 聚合规则

`CoreHealth.Status`（顶层）聚合规则：

| 5 探针状态 | 顶层 Status |
|---|---|
| 5/5 `healthy` | `healthy` |
| ≥1 `degraded` | `degraded` |
| ≥1 `unreachable` | `unreachable`（HTTP 503） |
| `?detailed=false` 或不带 query | 沿用 v0.7 behavior（gRPC ping 决定） |

**理由**：兼容 docker / k8s 现有 healthcheck 期望；`?detailed=true` 不改变 HTTP status code 与既有 5 字段 — 仅追加 `components` 字段。

### D5 — Cross-repo coord：Console UI standby PR

ContextForge v0.8.0 ship 后：

- ContextForge 侧 `contractv1.go` 含 `ComponentHealth` struct + `CoreHealth.Components` 字段（add-only）
- Console 侧 standby PR 由 Console 主 Agent 启动：`internal/contractv1/contractv1.go` 同步加 `ComponentHealth` + `Components` 字段（cross-repo schema 对齐）
- Console UI `CoreHealthCard` 改造为 5 链路细分面板（独立 PR；不阻塞 v0.8.0 ContextForge ship）

**zero-conflict 约束**：

- ContextForge 侧 add-only（不删 / 不改既有字段）→ Console 旧客户端继续工作（忽略未知 `components` 字段）
- Console 侧加字段后回放 ContextForge v0.8.0 不带 `?detailed=true` 的 health 调用 → `components` 字段缺省 → Console UI 渲染回退到 binary 视图（feature flag）

## Trade-offs / Conscious limitations

- **5 探针耗时上限 200ms**：retriever query exercise 在大 workspace（>100k chunks）可能超 40ms P95；触发 → 该 component status 降级 → 顶层 status 降级 — 接受作为 health 行为的"严格信号"（Console UI 可以 expose 给用户 "retriever P95 > 40ms"）
- **`embed` 探针不实际调远程**：仅校验 config 存在；远程 provider 实际可达性需要 hit endpoint（成本：rate limit / secret 暴露 / latency P99 > 1s）—— 留 [SPEC-DEFER:phase-future.embed-remote-probe] v1.x
- **`?detailed=true` 不缓存**：每次 hit 重新跑 5 探针；如 Console UI 高频 poll（每 5s）可能压 retriever exercise — 缓解 Console UI 自身节流（建议 ≥30s interval；UI 内通过 SWR cache）
- **不暴露 5 探针个体阈值 / 历史趋势**：当前快照式 health；不存历史；Grafana / 时序库集成留 v1.x
- **Default `GET /v1/health` 不动**：v0.7 既有 client（docker / k8s）零迁移；显式 `?detailed=true` 才触发新行为

## Verification (Phase 15 task-15.6 ship 时)

```bash
# 1. proto add-only 验证
git diff master..HEAD -- proto/contextforge/console_data_plane/v1/console_data_plane.proto | grep -E '^(-|\+)\s*(string|int|repeated|message|map)'
# expect: 仅 + 行；无 - 字段（add-only）

# 2. Rust 5 探针 unit test
cargo test -p contextforge-core --lib health::tests --no-fail-fast
# expect: ≥5 测试 PASS（db / index / embed / retriever / eval）

# 3. Go contractv1 编译 + 序列化
go test ./internal/contractv1/...
# expect: PASS

# 4. REST endpoint 实测
contextforge-daemon &  # background
curl 'http://localhost:48181/v1/health'                     # 默认 binary
curl 'http://localhost:48181/v1/health?detailed=true' | jq .components
# expect: 5 keys (db/index/embed/retriever/eval) each with name/status/latency_ms

# 5. console_smoke.sh v6 加 health-detail step
bash scripts/console_smoke.sh
# expect: CONSOLE_REAL_SMOKE_EXIT=0 含 health-detail step
```

## Rollback path

如 Phase 15 task-15.6 ship 后发现：

- 5 探针耗时 > 1s 导致 Console UI 卡顿 → 单独 ship patch 把 `?detailed=true` 改为 cached（30s TTL）
- proto `ComponentHealth` 字段不够（如 `version` 字段 missing 让 UI 显示不全）→ ADR-020 amendment（add-only），不撤回本 ADR
- 极端：5 探针实现 bug 让默认 `/v1/health` 也降级 → revert task-15.6 commit；v0.8.0.1 patch ship 仅含 task-15.1-15.5

ADR-020 不撤回 default（D1-D5 是 add-only design；rollback 仅是行为微调，不破坏契约）。

## Upgrade path (v0.7.x → v0.8.0)

### Docker / k8s healthcheck 用户

- v0.7 → v0.8 行为零变化（不带 `?detailed=true` → 沿用 binary CoreHealth）
- 想用 5 链路细分：healthcheck 改为 `/v1/health?detailed=true` + 解析 `components` map

### Console UI 用户

- v0.8.0 ship 后 Console UI 端 standby PR 同步（ContextForge contractv1.go 字段 + UI 5 链路面板）
- Console v1.x ship 时 `CoreHealthCard` 切换到 5 链路视图（feature flag）

### contractv1.go 客户端用户（CLI / 第三方集成）

- 升级 contractv1.go v0.8.x → `CoreHealth` 含 `Components` 字段（`map[string]ComponentHealth`）
- 旧代码继续工作（`Components` 为 nil 时 JSON 缺省）
