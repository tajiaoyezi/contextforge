# ADR `017`: `console-contract-completion-22-endpoint`

**Status**: Proposed
**Category**: 协议接口 / 兼容性
**Date**: 2026-05-24
**Decided By**: tajiaoyezi objective + main agent execution
**Related**: ADR-001 (go-rust-dual-binary-architecture) / ADR-003 (cli-rest-mcp-grpc-interfaces) / ADR-013 (cli-data-plane-grpc-bridge) / ADR-014 (cross-phase-exit-criteria-validation) / ADR-015 (console-contract-v1-compatibility) / ADR-016 (cross-process-rust-go-via-grpc-bridge) / PRD §Open Questions O15 / O16 / O17

## Context

ContextForge v0.4 (Phase 11, HEAD `5781d35`，tag `v0.4.0` @ `a4c9a542`) 通过 [ADR-016](./adr-016-cross-process-rust-go-via-grpc-bridge.md) 把 Console Contract v1 真业务面接通 —— Workspace / IndexJob / Search / Events 4 个 gRPC service + JobRunner 真触发 IndexSession + Retriever 真返回 chunks + EventBus 真接 progress。但 ContextForge-Console 的 HTTPAdapter 实际期望的是 **22 个 REST endpoint**（Console PRD §Technical Approach「Contract v1 must-have 字段」+ Console `console-api/internal/coreadapter/http_adapter.go` 22 endpoint × 17 type schema），ContextForge 当前只 ship 了 9 个。

**9/22 → 22/22 缺口（13 endpoint）按 backend 工作量分四档：**

| Wave | Endpoint | 现状 backend | 需新增 |
|---|---|---|---|
| **Wave 1（quick win）** | PATCH `/v1/workspaces/{id}/config`、GET `/v1/index-jobs?status=active`、cancel 改 204、X-Confirm 服务端兜底 412 | gRPC WorkspaceService.Update + JobService.List 已存在（ADR-016 D2 Service v1 已 ship）—— 仅缺 REST 路由 + grpcclient wrapper | 无新 backend |
| **Wave 2（mid scope）** | GET `/v1/source-chunks/{id}`、GET `/v1/search/{query_id}/trace` | Retriever 已索引 chunks 但无 by-id 查询 API；Search 当前 trace 只 inline 在 POST 响应，未持久化 by query_id | SearchService 加 `GetSourceChunk` RPC + `GetSearchTrace` RPC + Rust 端 trace 持久化 by query_id |
| **Wave 3（new phase）** | GET `/v1/memory`、GET `/v1/memory/{id}`、POST `/v1/memory/{id}/{pin\|deprecate\|soft-delete}` | Phase 5 memoryops dedup/lifecycle/audit 已 ship（`internal/memoryops/` Go + `core/src/memoryops/audit.rs` Rust）但**未暴露 gRPC service + REST 表面** | 新 MemoryService gRPC（5 RPC）+ Rust impl wrapping memoryops + Go grpcclient.MemoryClient + 5 REST handler |
| **Wave 4（new phase）** | POST `/v1/eval-runs`、GET `/v1/eval-runs/{id}` | Phase 8 eval harness 已 ship（`internal/eval/eval.go` + CLI `contextforge eval run`）但 proto/v1/eval.proto 仅 recall-only 二参 schema，Console 期望完整 EvalRun（status lifecycle + case_results + metrics + config_snapshot） | EvalService gRPC proto upgrade + Rust impl wrapping eval harness + Go grpcclient.EvalClient + 2 REST handler |

**为什么 1 个 ADR 而不是 3 个？**
- Wave 1/2/3/4 都围绕「让 Console 22 endpoint conformance 全 PASS」单一业务目标 —— 决策同源
- Wave 1+2 的 trade-off（X-Confirm 兜底语义 / 204 vs 200 / RFC3339Nano / long-poll v1.0 lock）必须在 Phase 12 之前拍板，否则 endpoint 行为漂移
- Wave 3 (Memory) + Wave 4 (Eval) 的 gRPC service 设计**完全沿用 ADR-016 D2/D3 pattern**（Rust 持 SoT + Go thin proxy + 复用 :50552 端口），不引入新架构决策
- 每个 Phase 自有 phase spec 含详细 AC；ADR-017 只锁顶层 6 个决策；细化在 Phase 12/13/14 spec

**v0.4.0 ship 后 Console 端的实际 conformance 状态（我方 audit）：**
- 9/22 endpoint 走 happy path 应 PASS（Workspace 3 + IndexJob 3 + Search 1 + Events 1 + Health 1）
- 13/22 endpoint 当前返 404（路由未注册）
- 17 type 字段全 1:1 对齐 Console contractv1.go（Phase 10 落入；本 ADR 不动字段）
- 4 处行为 trade-off 未锁定（cancel 200/204、RFC3339Nano、X-Confirm 兜底、long-poll vs SSE）—— 本 ADR D2/D3/D4/D5 拍板

**Console 端 v1.0 deliverable 状态**：Console repo `console-api/internal/coreadapter/contractv1/contractv1.go` 17 type 已 freeze（Console PRD §Implementation Phases Phase 6 ship）；Console HTTPAdapter 全 22 endpoint 实现 + 22-endpoint conformance suite。本 ADR 只在 ContextForge 单仓内补完，不改 Console。

**v0.5 ↔ v0.7 release 节奏（本 ADR 推导）**：
- v0.5.0 = Phase 12 closeout（Wave 1+2 共 +6 endpoint = 15/22 ≈ 68%）
- v0.6.0 = Phase 13 closeout（Wave 3 +5 endpoint = 20/22 ≈ 91%）
- v0.7.0 = Phase 14 closeout（Wave 4 +2 endpoint = 22/22 = 100% Console conformance 全 PASS）

## Decision

ContextForge v0.5 → v0.7 跨三个 minor release 共 3 个 phase（Phase 12/13/14）完成 Console Contract v1 22 endpoint 完整覆盖，由 6 个 Decision 段组成。所有 Decision 围绕"沿用 ADR-016 D1-D6 + 锁定 4 处行为 trade-off + 不动 contractv1 字段"。

### D1 — 22 endpoint 闭环路线图

Phase 12 → Phase 13 → Phase 14 串行实施，每 phase 自含 v0.X.0 release：

| Phase | Endpoint 增量 | 累计 | Release | 工作量 |
|---|---|---|---|---|
| Phase 12 console-contract-completion | PATCH workspace config + GET index-jobs?status=active + GET source-chunks/{id} + GET search/{query_id}/trace + cancel 204 + X-Confirm 兜底 | 9 → 15 (+6) | v0.5.0 | ~1.5-2 周（3 task） |
| Phase 13 memory-rest-surface | GET memory + GET memory/{id} + POST memory/{id}/{pin,deprecate,soft-delete} | 15 → 20 (+5) | v0.6.0 | ~1.5-2 周（2 task） |
| Phase 14 eval-rest-surface | POST eval-runs + GET eval-runs/{id} | 20 → 22 (+2) | v0.7.0 | ~1-1.5 周（2 task） |

- **不修改 `internal/contractv1/contractv1.go`**：17 type 字段集合 Phase 10 已 freeze（ADR-015 D1）；本 ADR 不改字段 + 不引入 contractv2（add-only 演进留 ADR-018+）
- **不修改 Console repo**：本 ADR 范围 = ContextForge 单仓；Console 端 22-endpoint conformance suite 直接反向跑

**理由**：节奏对齐 PR 复杂度（每 phase 7-10 PR 量级，与 Phase 10/11 一致）；release 节奏让 Console 端可以增量验证；v0.5/v0.6/v0.7 minor bumps 不破坏 contract v1 anchor。

### D2 — 服务端 X-Confirm 双因子兜底语义

Console BFF 端对破坏性操作（PATCH workspace config / POST memory/{id}/deprecate / POST memory/{id}/soft-delete）会注入 `X-Confirm: yes` header **+** `?confirm=true` query。ContextForge 服务端**必须**校验**任一**标识：

- 缺失两者 → 返 `412 Precondition Failed` + ErrorBody `{code:"PRECONDITION_FAILED", message:"X-Confirm:yes header or ?confirm=true query required for destructive op"}`
- 只携任一即放行（Console 实现注入两者，但服务端校验 OR 语义，向后兼容运维工具 curl 单一标识场景）
- pin 操作非破坏性，**不**走 412 校验（即便 Console 注入 X-Confirm 也直接放行）

**实施位置**：在 `internal/consoleapi/router.go` 加 `confirmMiddleware(handler)` wrapper，路由声明时显式标注哪些 endpoint 需要 confirm。Memory 4 个、Workspace PATCH 1 个共 5 个 endpoint 走 wrapper。

**理由**：deep defense 原则 —— Console BFF 端 ConfirmDialog 是第一道防线；服务端 412 是第二道；缺失即失败防御链断（重要业务 endpoint 误操作风险）。412 而非 400 因为是 precondition not met 语义（RFC 7232）。

### D3 — cancel 改 204 No Content（v0.5.0 起 breaking-compatible）

v0.4.0 `POST /v1/index-jobs/{id}/cancel` 当前返 `200 OK` + 空 body。Console HTTPAdapter spec 写 `204`。Phase 12 task-12.1 改为：
- 成功 → `204 No Content`（不返 body；Console HTTPAdapter `if statusCode != 204 && != 200 { error }` 两 code 都接受 → 向后兼容）
- 已终态 → `409 Conflict`（不变）
- 未找到 → `404 Not Found`（不变）

**理由**：与 Console spec 严格对齐 + 节省 1 字节 body + 与 HTTP REST 惯例一致。Console HTTPAdapter v1.0 实现已经做 `200 || 204` 双 check（见 Console repo `http_adapter.go::CancelIndexJob`），切到 204 不破坏 v0.4 client。

### D4 — Observability events long-poll v1.0 lock（SSE defer to v2.x）

`GET /v1/observability/events?wait=Ns&limit=N` long-poll 模式在 v0.4 (Phase 11 task-11.4) ship；v1.0 不引入 SSE。

- 默认 `wait=30s` clamped to [1s, 60s]
- 默认 `limit=100` clamped to [1, 500]
- 返 `200` + JSON array（可能为空，等到 wait 超时即返）；**不**返 `204`（Console v1.0 HTTPAdapter 预期 200+array）
- v1.x 引入 SSE `text/event-stream` content type 留 `[SPEC-DEFER:console-events-sse]`

**理由**：Console v1.0 HTTPAdapter `GetObservabilityEvents` 实现是 simple GET + JSON poll；改 SSE 需 Console 端 fetch 切 EventSource；v1.0 不引入跨仓改动。long-poll 在 Console UI 端用 5-10s 间隔轮询模拟 push 已足够 demo / single-user。

### D5 — JSON RFC3339Nano output kept; Console Zod schema relax to accept

Go `encoding/json` 对 `time.Time` 默认输出 RFC3339Nano（含纳秒 `2026-05-24T03:04:05.123456789Z`）。Console v1.0 contractv1.go `time.Time` 同款 Go marshal 也是 Nano；但 Console 前端 Zod schema 如果用 `z.string().regex(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/)` strict RFC3339 会 reject 我方输出。

- ContextForge 服务端**不**改 `time.Time.MarshalJSON`（避免自写 marshal + 引入字符串 truncate bug）
- Console 端建议改 Zod schema 为 `z.string().datetime({ offset: true, precision: 9 })` 接受 Nano（或 `z.string().datetime({ offset: true })` 允许任意小数位）
- 本决议通过 cross-repo follow-up 通知 Console 团队（不强制 cross-repo PR；属 Console 端可选优化）

**理由**：服务端字段集合在 ADR-015 D1 锁定 + Go `time.Time` 默认行为 + Console 前端 Zod 校验 layer 收紧成本 = 让 Console 端放宽 strict (合 v1 contract 既已锁字段集合不锁 time precision)。

### D6 — Memory/Eval 新 gRPC service 沿用 ADR-016 D2/D3 pattern

Phase 13 MemoryService + Phase 14 EvalService 完全复用 ADR-016 已建立的 cross-process gRPC bridge 模式：

- 新 proto 文件 `proto/contextforge/console_data_plane/v1/memory.proto` + `eval.proto`（与 ADR-016 D2 `console_data_plane.proto` 同包 `contextforge.console_data_plane.v1` 同 lib，不另起 package）
- Rust 实现在 `core/src/data_plane/memory.rs` + `eval.rs`（与现有 workspace.rs/job.rs 同 module 结构）；底层包既有 `core/src/memoryops/` + `internal/eval/`
- Go grpcclient.MemoryClient + EvalClient 与现有 WorkspaceClient/JobClient 同款 wrapper pattern
- 复用端口 `:50552`（Rust DEFAULT_LISTEN；ADR-016 D2）；不引入新端口
- bearer auth 仍在 Go middleware 层（D3 thin proxy）
- 错误映射沿用 v0.4 sentinel（NotFound → 404 / FailedPrecondition → 412 / Unavailable → 503）

**理由**：ADR-016 已建立 4 service × 14 RPC pattern + tonic/prost 工具链 + DataPlaneStores 共享 stores 链；Phase 13/14 只需扩 service trait + new RPC，无需重新设计。

### D7 — ADR-014 cross-validation gate 第三/四/五次激活

Phase 12/13/14 全程沿用 ADR-014 D1-D5：
- D1 closeout PR body mapping 表（Phase §6 AC × 4 字段）—— v0.3 首次 / v0.4 第二次 / v0.5/v0.6/v0.7 第三/四/五次
- D2 `bash scripts/spec_drift_lint.sh --touched origin/master` 0 violation in PR-touched lines
- D3 phase §6 + task §6 每条 AC 含 `verified by` 显式 owner
- D4 main-agent self-merge 在 ADR-012 governance 下
- D5 历史不溯改

不引入新 governance ADR（与 ADR-016 D6 同款理由）。

**理由**：v0.3/v0.4 跑通的 cross-validation gate 制度稳定，v0.5/v0.6/v0.7 增量验证；governance 不在 ship-first phase 内扩展（留 v1.0 retrospective）。

## Rationale

- **不动 contractv1 字段**：ADR-015 D1 anchor freeze；Console 端 contractv2 升级走 Console 主导 cross-repo PR（add-only 字段加入则双仓镜像即可；breaking change 留 v2.0）
- **不引入新端口**：ADR-013 + ADR-016 `:50552` 单端口 multi-service 模式已建；新 service 直接 `add_service` 到现有 tonic Router
- **不引入新 R7 deps**：tonic + prost + tonic-build 已在 ADR-013 引入；本 ADR 仅扩 `.proto` 文件
- **不修改 Console repo**：Console v1.0 HTTPAdapter 22 endpoint 实现 + conformance suite 已 ship；ContextForge 单仓内补 endpoint = 双仓 cross-check 在 Console side 自动跑
- **3 phase 串行而非并行**：每 phase 内多个 task 可并行（如 Phase 13 task-13.1 + 13.2），但 phase 间串行因为：(a) Wave 顺序对应 Console UI 价值优先级（Workspace/IndexJob 比 Memory/Eval 更紧）；(b) 主 agent 单驱动 governance 不适合多 phase 并发
- **release 节奏 v0.5/v0.6/v0.7**：minor bumps，不破坏 v1 contract anchor；patch release v0.5.x 留小修；major v1.0 留 22/22 全 PASS + Console UI 公开 hit + multi-user scenario unlock
- **X-Confirm OR 语义而非 AND**：Console 注入两者但 OR 检查向后兼容 curl 单标识场景 + 运维脚本简洁

## Alternatives

- **A. 1 个 phase 同时做 13 endpoint**：拒，单 PR 不可 review（>3 kloc 改动 / 12+ test fixture / 多个底层 module wiring）+ violates v0.3/v0.4 1 phase ~5-10 PR 节奏
- **B. 不做 22 endpoint 直接跳 v1.0**：拒，Console v1.0 已 ship 22-endpoint conformance test；ContextForge 端 9/22 即 v1.0 release = 41% endpoint 兼容 = Console UI 无法用业务级 demo（memory / eval 是 Console UI 「下一阶段」核心 view）
- **C. Console 端改 conformance suite 只测 9 endpoint**：拒，破坏 Console 单一事实源 + cross-repo 字段镜像约定（ADR-015 D1 不允许 ContextForge 单方面缩减）
- **D. ADR-017 拆 3 个独立 ADR**：拒，6 D-clauses 高度耦合（D2 X-Confirm + D3 204 + D4 long-poll + D5 RFC3339Nano 都是 v0.5.0 ship 前必须锁定的 4 trade-off）；拆分会让每 ADR 都引用其它两个 → 阅读成本反而高
- **E. 引入 SSE 替代 long-poll**：拒（D4 已 decline），Console v1.0 HTTPAdapter 用 GET poll；改 SSE 跨仓改动 + Console 端 EventSource refactor 不在 v1 范围
- **F. 改 Go `time.Time` marshal 输出 strict RFC3339**：拒（D5 已 decline），自定义 marshal 引入字符串 truncate / nano 数据丢失 / Go 内部 timestamp 比较精度漂移 风险；Console 端 Zod schema relax 成本远低

## Consequences

**正面**：
- Console 22-endpoint conformance suite 在 v0.7.0 ship 后全 PASS → Console UI 端可以无 Mock 切到生产模式
- contractv1 字段集合保持不动 → v1.0 contract anchor 稳定
- ADR-016 D1-D6 pattern 第二/三次复用验证（Phase 13/14 各自一次）→ cross-process gRPC bridge 制度成熟
- ADR-014 D1-D5 cross-validation gate 第三/四/五次激活 → governance 稳定性进一步验证
- 服务端 X-Confirm 412 兜底引入 deep defense → 误操作风险降低

**负面 / 成本**：
- 3 phase × ~1.5-2 周 = ~5-6 周连续主 agent 自治 ship 时间（与 Phase 10/11 跨度同量级）
- gRPC service 数量 4 → 6（+ MemoryService + EvalService），proto 文件数量 +2；tonic Server::builder 链增长（每 service 一行 .add_service）
- Rust `core/src/data_plane/` module size 增长 ~2-3 kloc
- Console BFF 端 GetEvent 等 endpoint 在 v0.5/v0.6/v0.7 之间从 404 渐变到 200 → Console UI 端需要 graceful degrade 显示（FieldAvailability 机制天然支持，但 Console 端实际 UI 需要呈现 "feature coming"）

**中性**：
- proto field add-only 演进规则（ADR-001/003 边界不动）
- Memory/Eval gRPC method 与 REST endpoint 1:1 匹配（与 ADR-016 D2 同款）
- v0.5 / v0.6 / v0.7 各自 release tag + RELEASE_NOTES + evidence/artifacts 落盘（与 v0.3.0/v0.4.0 同款 release docs 节奏）

**对 v0.5+ 的影响**：
- v0.5.0 ship 后 ContextForge ↔ Console 端到端 demo 含 Workspace config 修改 + 长任务 list filter + chunk 详情下钻 + search trace 复盘
- v0.6.0 ship 后 Memory 治理（pin/deprecate/soft-delete）端到端可用 = 真正实现 PRD §Core Capabilities #3「MemoryOps 治理」的 UI 闭环
- v0.7.0 ship 后 Eval 端到端可用 = PRD §Core Capabilities #4「召回评测」的 UI 闭环；ContextForge v1 contract 全表达完
- v1.0.0 release gate（留 ADR-018+ retrospective）评估：multi-user / cluster / cross-repo cross-validation / SSE 引入

## Rollback Or Migration Plan

如 Phase 12/13/14 实施中发现：

1. **ADR-016 D1 SoT 反向（Go 写 SQLite memory/eval 表）出现**：立刻 STOP（沿用 ADR-016 §自决规则 R8）；revert 该 commit；Memory/Eval schema 单 owner = Rust 不动
2. **ADR-016 D3 thin proxy 被违反**：立刻 STOP；handler 内字段映射代码 / 业务逻辑下推到 Rust gRPC method
3. **D2 X-Confirm 412 兜底误伤合法操作**：先扩 OR 语义到 X-Confirm header / ?confirm=true / Console-injected User-Agent 三选一；若仍误伤 → STOP + ADR-017 amendment
4. **D3 cancel 改 204 破坏 v0.4 老 client**：保留 200（D3 amendment 修订），不引入 breaking；Console HTTPAdapter v1.0 已实现 200/204 双 check，不应出现此问题
5. **D5 RFC3339Nano 在 Console 端实际 reject**：双方决定（a）服务端自定义 marshal 输出 strict RFC3339（trade-off：truncate nano → 接受精度损失）OR（b）Console Zod schema relax
6. **D6 Memory/Eval gRPC service 设计与 ADR-016 D2/D3 pattern 不符**：立刻 STOP；按 ADR-016 D2/D3 重设计
7. **task-13.1 Memory gRPC service 暴露 dedup/lifecycle/audit 时发现 internal/memoryops 接口不够 stable**：保 5 endpoint 但接口降级到「最小可调用集」（如 list/get/pin 三个先 ship）；deprecate/soft-delete 留 task-13.x amendment
8. **task-14.1 Eval proto 升级破坏现有 `contextforge eval run` CLI（task-8.1 ship）**：proto add-only 演进 + 旧字段保留；CLI 调用路径不变 + 新字段 optional

Rollback 通过新 ADR superseding 完成；Phase 12/13/14 已 ship 的 gRPC service 保持向后兼容（add-only 演进，与 task-1.1 proto 同款规则）。

## History

- 2026-05-24 Proposed via Phase 12 spec PR draft. 6 D-clauses（D1 22-endpoint roadmap + D2 X-Confirm OR 412 + D3 cancel 204 + D4 long-poll v1.0 lock + D5 RFC3339Nano kept + D6 Memory/Eval 沿用 ADR-016 + D7 ADR-014 第三/四/五次激活）。Status Proposed → Accepted 将在 Phase 14 closeout PR 内（v0.7.0 release 前）回填。

## Follow-ups

- **本 ADR Proposed in Phase 12 spec PR**：E1 spec PR 落入即 Proposed；Status → Accepted 在 **Phase 14 closeout PR** 内（v0.7.0 release tag 前）—— 因为 6 D-clauses 完整覆盖 3 phase，全程 Accepted 需 Phase 14 ship 收口
- **Phase 12 实施后**：D2/D3/D4/D5 4 trade-off 行为锁定；Console BFF 端可以确认 X-Confirm 兜底语义到位
- **Phase 13 实施后**：Memory 5 endpoint ship → Console UI 端 Memory 治理 view unblocked
- **Phase 14 实施后**：Eval 2 endpoint ship → Console 22-endpoint conformance suite 全 PASS（双方握手成功）
- **Cross-repo follow-up**：D5 RFC3339Nano 通知 Console 团队评估 Zod schema relax；不强制 cross-repo PR
- **关联 PRD §Open Questions**：
  - O15 新提（RFC3339Nano vs strict RFC3339）—— 本 ADR D5 resolved
  - O16 新提（Memory v0.6 REST gating）—— 本 ADR D1 Wave 3 partial resolved；完整 v1.0 留 multi-user
  - O17 新提（Eval EvalRun schema strict alignment）—— 本 ADR D1 Wave 4 partial resolved
- **关联 ADR-014 D2 lint**：Phase 12/13/14 全程 spec PR / task PR / closeout PR 跑 `bash scripts/spec_drift_lint.sh --touched origin/master`；强制 0 violation
- **v1.0 retrospective（ADR-018+）**：multi-user / cluster / SSE / cross-repo cross-validation 制度化
