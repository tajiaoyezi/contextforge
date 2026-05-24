# Phase 10 · console-contract-v1

**Status**: Ready

> Phase Spec（s2v full-standard §8.2）。本 phase 是 v0.3.0 minor release 收口 phase — 实现 ContextForge ↔ ContextForge-Console (后文 "Console") Contract v1 兼容层，落地 9 个 REST endpoint + Workspace/IndexJob 资源模型 + cross-repo conformance test + docker compose 集成 smoke。本 phase 最后一个 task 完工/合并前必须执行 §6 端到端 smoke（`s2v_preflight_phase` C1）。
>
> 治理基线：本 phase 按 ADR-011 单驱动 + ADR-012 主 agent 自治 + **ADR-014 cross-validation gate（D1/D2/D3/D4/D5 首次完整激活）**；§2A Ready review 由主 agent 自审（带用户复核选项 — 本 phase 涉及 PRD §Implementation Phases 修改 + cross-repo 依赖，建议保留用户审）。详见 [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) + [ADR-014](../../decisions/adr-014-cross-phase-exit-criteria-validation.md)。

## 1. 阶段目标

实现 ContextForge ↔ Console v1.0 (已 ship) Contract v1 兼容层：`internal/contractv1/` Go types 1:1 镜像 Console `console-api/internal/coreadapter/contractv1/contractv1.go`；Workspace / IndexJob 资源模型 + SQLite migration (`0010_workspaces.sql` + `0011_index_jobs.sql`)；9 个 REST endpoint (`/v1/health` + `/v1/workspaces*` + `/v1/index-jobs*` + `/v1/search` + `/v1/observability/events`) 严格对齐 Console HTTPAdapter 期望 + OpenAPI yaml；`test/conformance/console_contractv1_test.go` 反向跑 Console fakehttpserver oracle；`scripts/console_smoke.sh` docker compose 端到端 (Console UI 真返回 ContextForge workspace 列表，非 Mock)。来源：[ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) D1-D6 / Console PRD §Technical Approach「Contract v1 must-have 字段」/ Console `internal/coreadapter/contractv1/contractv1.go` / PRD §Open Questions O13 / PRD §Implementation Phases v0.3 新增（见 PRD §Implementation Phases Phase 10 行）。

## 2. 业务价值

闭环 Console v1.0 已 ship 但无真 Core 可对接的 cross-repo gap (PRD §Open Questions O13)。直接支撑：
- Console UI demo 调真 ContextForge（不仅 Mock Adapter）— Console PRD §Implementation Phases Phase 7 integration 验收前提
- ContextForge 对外 REST 面从 v0.2 单 endpoint (`/v1/search` 仅 search 子集，由 task-6.2 实现) 扩到 9，覆盖 Console HTTPAdapter 调真 ContextForge 的最小集 happy path
- Workspace / IndexJob 资源模型为 ContextForge v0.4+ 多 collection / 多 worker / RBAC 演进留扩展点
- 跨仓库 Contract v1 字段对齐 verifiable（conformance test + D2 lint）— 降低 cross-repo drift 风险

## 3. 涉及模块

- `internal/contractv1/`（新增：17 Contract v1 类型 Go 镜像 + types_test.go）
- `core/src/workspace/`（新增：Rust workspace module + WorkspaceStore trait + SqliteWorkspaceStore）
- `core/migrations/0010_workspaces.sql`（新增：workspaces 表 schema）
- `core/src/jobs/`（新增：IndexJob 状态机 + JobStore trait + SqliteJobStore + JobRunner async executor）
- `core/migrations/0011_index_jobs.sql`（新增：index_jobs 表 schema）
- `internal/consoleapi/`（新增：9 REST handler + router + bearer auth middleware + error mapping）
- `internal/daemon/`（修改：注册 consoleapi router 到现有 daemon REST listener，复用 task-6.2 net/http + chi 基础设施）
- `docs/consoleapi/openapi.yaml`（新增：9 endpoint OpenAPI 3.0 描述）
- `test/conformance/console_contractv1_test.go`（新增：反向跑 Console fakehttpserver oracle 验证 ContextForge REST 输出能被 Console HTTPAdapter 解析）
- `scripts/console_smoke.sh`（新增：docker compose 启动 Console v1.0 + ContextForge daemon + Postgres + Redis + curl Console UI 真验证）
- `deploy/console-stack.yml`（新增：docker compose 描述）
- `README.md` / `RELEASE_NOTES.md` / `docs/releases/v0.3.0-*.md`（v0.3.0 发布文档）
- `docs/s2v-adapter.md` / `docs/prds/context-forge.prd.md`（adapter Phase 10 + ADR-015 索引；PRD §Implementation Phases Phase 10 行；§Open Questions O13 resolved by ADR-015）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 10.1 | internal/contractv1 | `../tasks/task-10.1-contractv1-types.md` |
| 10.2 | core/workspace | `../tasks/task-10.2-workspace-resource.md` |
| 10.3 | core/jobs | `../tasks/task-10.3-indexjob-resource.md` |
| 10.4 | internal/consoleapi | `../tasks/task-10.4-rest-endpoints.md` |
| 10.5 | test/conformance | `../tasks/task-10.5-conformance-test.md` |
| 10.6 | scripts/console_smoke | `../tasks/task-10.6-console-integration-smoke.md` |

## 5. 依赖关系

- **依赖**：Phase 9（cli-pipeline）— 复用 task-9.2 IndexSession::index_path_with_progress + task-9.3 daemon spawn 模式；Phase 6（cli-api-export）— 复用 task-6.2 daemon REST net/http + chi router 基础设施 + local random token auth 思路。
- **可并行**：否（v0.3 收口 phase）。Phase 内顺序：10.1 → {10.2 ∥ 10.3} → 10.4 → 10.5 → 10.6。
- **Phase 内并行机会**：task-10.2 (workspace) ∥ task-10.3 (jobs) 在 task-10.1 完成后可并行 — 两者各自独立 SQLite migration + 独立 Rust module，无源文件写冲突；REST handler (task-10.4) 同时消费两者，必须等两者都 merge 后再启动。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（任务 10.1-10.6 全 Done，实测验证；每条 AC 含 ADR-014 D3 verified by 显式 owner）**：

- [ ] AC1：`internal/contractv1/` 含 17 Contract v1 类型 Go 镜像 + `ContractVersion = "v1"` + FieldAvailability helper + types_test.go JSON roundtrip 全过 — **verified by task-10.1 §6 AC1-5**
- [ ] AC2：`core/src/workspace/` + `core/migrations/0010_workspaces.sql` 实现 workspace 资源 CRUD + workspace_id ↔ collection_id 1:1 映射 + `cargo test workspace` 全过 — **verified by task-10.2 §6 AC1-5**
- [ ] AC3：`core/src/jobs/` + `core/migrations/0011_index_jobs.sql` 实现 IndexJob 状态机 (queued/running/succeeded/failed/cancelled) + heartbeat + 异步 lifecycle + `cargo test jobs_lifecycle` 全过 — **verified by task-10.3 §6 AC1-5**
- [ ] AC4：`internal/consoleapi/` 9 endpoint REST handler 全实现 + 路径 / shape / 错误码严格对齐 Console HTTPAdapter 期望 + `docs/consoleapi/openapi.yaml` 落 + `go test ./internal/consoleapi/... -run TestRESTEndpoints_E2E` 真启 daemon + 真 HTTP 调用全过 — **verified by task-10.4 §6 AC1-5**
- [ ] AC5：`test/conformance/console_contractv1_test.go` 反向取 Console fakehttpserver oracle 跑过 ContextForge REST 端到端 — **verified by task-10.5 §6 AC1-5**
- [ ] AC6：`scripts/console_smoke.sh` 启动 docker compose (Console v1.0 + ContextForge daemon + Postgres + Redis) + curl Console UI `/api/workspaces` 真返回 ContextForge workspace 列表（非 Mock）+ `CONSOLE_SMOKE_EXIT=0` — **verified by task-10.6 §6 AC1-5 + phase-smoke step 1 (cmd: `bash scripts/console_smoke.sh`)**
- [ ] AC7：ADR-014 cross-validation gate 全套通过：D2 lint (`bash scripts/spec_drift_lint.sh --touched origin/master` 0 violation) + D3 phase §6 每条 AC 含 verified by + D1 closeout PR body 含 mapping 表 — **verified by phase-smoke step 2 (cmd: `bash scripts/spec_drift_lint.sh --touched origin/master`)**

**端到端 smoke**：

```bash
# step 1 — Phase 10 主集成 smoke (Console docker compose + ContextForge daemon)
bash scripts/console_smoke.sh

# step 2 — ADR-014 cross-validation gate (D2 lint)
bash scripts/spec_drift_lint.sh --touched origin/master
```

step 1 是 task-10.6 的 Gate 3 入口：在 docker compose 网络中启动 Console v1.0 image + ContextForge daemon + Postgres + Redis；curl Console UI `:3000/api/workspaces` 验证 Console BFF 调真 ContextForge → 真返回 workspace 列表。`CONSOLE_SMOKE_EXIT=0` 是 final marker。

step 2 是 ADR-014 D2 lint gate：phase closeout PR 触及行无未标注 anti-pattern；强制 0 violation。

**Scope 注**：本 phase smoke 与 task-8.3 release_smoke.sh + task-9.6 quickstart_smoke.sh 互补 — task-8.3 (v0.1) gate tarball + Rust gRPC search smoke；task-9.6 (v0.2) 新增 CLI binary 7-step；task-10.6 (v0.3) 新增 "Console UI 真调真 ContextForge" 段。三条 smoke 均跑通才允许 v0.3.0 tag。

## 7. 阶段级风险

- **关联 ADR-015 §Rollback 5 条风险**：
  - D2 workspace 1:1 collection 映射撞用户期望 → 概率低 (v0.2 collection 单一概念已稳定)；缓解 add `default_collection_id` 字段不改 schema 接口
  - D3 IndexJob 异步 lifecycle race condition → 中等概率（tokio async + heartbeat 周期需调）；缓解 task-10.3 §6 AC4 显式跑 lifecycle 状态机集成测试
  - D4 9 endpoint scope 不足 → 中低概率（Console Mock Adapter 覆盖 Phase 5/6 不阻塞 v0.3）；缓解 v0.4 增量 endpoint task
  - D5 Console fakehttpserver 与实际 HTTPAdapter 不一致 → cross-repo 反向依赖触发 → ADR-014 D4 + playbook §自决规则 #8 转 §8 STOP
  - D6 docker compose 跨容器网络 / Console image 拉取失败 → 中等概率 (Linux/WSL2 优先验证，Windows skip)
- **关联 PRD §Technical Risks R1**（Go↔Rust gRPC 边界）：本 phase 不动 gRPC 契约（task-9.1 add-only freeze 维持）；新增 REST 面在 Go daemon 内部消费现有 gRPC。
- **关联 PRD §Technical Risks R6**（大仓库性能）：task-10.3 IndexJob async lifecycle 加 heartbeat 写 SQLite，引入额外 IO；缓解 heartbeat 周期 5s + batch update 不每文件写。
- **关联 ADR-014 cross-validation 治理风险**：本 phase 是 ADR-014 D1/D2/D3/D4 制度首次完整激活；如 lint 词表误报 / D1 mapping 表撰写成本超预期 → ADR-014 §Rollback 路径调整。

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done（10.1/10.2/10.3/10.4/10.5/10.6 全 Done — PR 顺序合）
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过（console_smoke.sh 真跑 + spec_drift_lint.sh --touched 0 violation）
- [ ] 关联风险 ADR-015 §Rollback 5 条 / R1 / R6 / ADR-014 治理风险缓解措施已落地
- [ ] adapter §Phase 状态索引该行 Status 同步更新（closeout PR）
- [ ] ADR-015 状态推进 Proposed → Accepted（closeout PR）
- [ ] PRD §Implementation Phases Phase 10 行新增（含 Status=Done / 描述 / 范围 / 依赖 / 可并行 ）+ §Open Questions O13 标记 resolved by ADR-015
- [ ] **ADR-014 D1 mapping 表**：closeout PR body 含 Phase §6 ↔ Task §6 AC 映射（AC1-7 每行 4 字段：phase AC 字面 / 拥有 task or 验证方式 / task §6 AC 字面 / Evidence 链接）
- [ ] **ADR-014 D2 lint 输出**：closeout PR body 含 `bash scripts/spec_drift_lint.sh --touched origin/master` 0 violation 输出
- [ ] §4 Gate 4.5 ADR-014 cross-validation gate 通过 — v0.3.0 release tag prep ready
