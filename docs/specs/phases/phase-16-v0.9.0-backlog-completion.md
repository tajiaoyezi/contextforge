# Phase 16 · v0.9.0-backlog-completion

**Status**: Ready

> Phase Spec（s2v full-standard §8.2）。本 phase 是 v0.9.0 minor release + **ContextForge-Console PR #91/#93 backlog 剩余 5 项中 4 项 closure 收口 phase** — 关闭 P3 (2 项) + P4 (2 项)。最后剩 P2 #6（`MemoryItem.is_pinned` ADR-015 D5 amendment，需 cross-repo coordination）独立留 [SPEC-OWNER:phase-17.is-pinned-amendment]。
>
> - **P4 #10 — TraceStore SQLite 持久化**：解决 `GET /v1/queries` / `GET /v1/search/<query_id>/trace` daemon 重启即丢痛点（当前 `core/src/data_plane/search.rs::TraceStore` 是 in-memory HashMap+VecDeque cap=1000；migration `0015_search_traces.sql` + 写穿 SQLite + 内存 LRU 作 hot cache）
> - **P4 #11 — events `?wait=` 真 long-poll**：解决 `GET /v1/observability/events?wait=30s` 当前等价 batch polling 痛点（`internal/consoleapi/handlers.go::handleEvents` line 641 现 `_ = parseWaitParam(r)` 显式丢弃 wait；改为真把 wait 传到下游 gRPC stream + 在 stream 上 block 等 ≥1 event 或 timeout）
> - **P3 #8 — ghcr.io image push CI/CD**：解决 v0.7-0.8 `Dockerfile` 已 ship 但用户须本地 `docker build` 痛点（新建 `.github/workflows/release.yml`，`v*` tag push 触发 build + push `ghcr.io/tajiaoyezi/contextforge-daemon:{tag}` + `:latest`）
> - **P3 #9 — docker-compose.production.yml 范例**：解决既存 `deploy/console-stack.yml` 是 dev/PoC stack（`CONSOLE_API_FALLBACK_INMEM=1` + 单容器）痛点（新加 `deploy/docker-compose.production.yml` 范例 — `contextforge-core` daemon + `console-api-serve` 双进程 + fallback deny + 数据卷 + healthcheck）
>
> 治理基线：本 phase 按 ADR-011 单驱动 + ADR-012 主 agent 自治 + **ADR-014 cross-validation gate（D1/D2/D3/D4/D5 第七次完整激活）**。本 phase **不引入新 ADR** — 4 task 全部为既有 ADR-013/015/016/017/018 的延伸实施；ghcr/compose 是 ops 实践不构成 architectural decision。详见 [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) + [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) + [ADR-018](../../decisions/adr-018-fallback-inmem-default-reversal.md) + [ADR-014](../../decisions/adr-014-cross-phase-exit-criteria-validation.md)。
>
> **v0.9.0 ship 后**：Console UI 端历史查询面板跨 daemon 重启不丢；events stream 响应及时；用户可 `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0` 直接获镜像；`docker compose -f deploy/docker-compose.production.yml up -d` 起真持久化 stack。`is_pinned` 字段 amendment（P2 #6）作为最后一项 backlog 独立 Phase 17 推进 — 需 Console 端先 ship contractv1 字段 amend [SPEC-OWNER:phase-17.is-pinned-amendment]。

## 1. 阶段目标

实现 ContextForge backend 端 4 项 Console residual backlog closure + 1 个 production-readiness deploy 范例 + v0.9.0 minor release：

- **TraceStore SQLite 持久化 (P4 #10)**：新增 SQLite migration `core/migrations/0015_search_traces.sql`（5 列 + 1 索引 + cap-by-DELETE 自维护策略）+ `core/src/data_plane/search.rs::TraceStore` 改为 **write-through 设计** — `put` 写穿 SQLite + 同步保留内存 LRU（既有 cap=1000 不变）作 hot cache + `get` / `list` 内存 miss 时落 SQLite 回填；daemon 重启后内存 LRU 从 SQLite ORDER BY ts_unix DESC LIMIT 1000 warm restore [SPEC-OWNER:task-16.1]
- **events `?wait=` 真 long-poll (P4 #11)**：`internal/consoleapi/handlers.go::handleEvents` 把 `parseWaitParam` 真传到 `deps.Events.Recent`；`internal/consoleapi/types.go::EventsClient.Recent` 签名加 `wait time.Duration` 参；`internal/consoleapi/grpcclient/grpcclient.go::eventsClient.Recent` 用 `ctx context.Context` deadline = `wait` 调 gRPC server-stream `Subscribe` + block 等 ≥1 event 或 deadline；既有 `Recent(limit int)` callers 全部更新为 `Recent(limit int, wait time.Duration)` [SPEC-OWNER:task-16.2]
- **ghcr.io image push CI (P3 #8)**：新建 `.github/workflows/release.yml`（`workflow_dispatch` + `push: tags: ['v*']` 触发；用 `docker/build-push-action@v5` + `docker/login-action@v3` push 到 `ghcr.io/${{ github.repository_owner }}/contextforge-daemon:${{ github.ref_name }}` + `:latest`；single arch linux/amd64 v0.9 ship；multi-arch arm64 留 [SPEC-DEFER:phase-future.multi-arch-image]）[SPEC-OWNER:task-16.3]
- **docker-compose.production.yml (P3 #9)**：新加 `deploy/docker-compose.production.yml` —— 拆 `contextforge-core` 容器（Rust daemon，端口 50551 gRPC bind 0.0.0.0）+ `console-api-serve` 容器（Go REST 48181 + `--grpc-addr=contextforge-core:50551`）；fallback deny 默认（ADR-018 沿用，**不**注入 `CONSOLE_API_FALLBACK_INMEM=1`）；命名卷 `contextforge-data:/data` 持久化；healthcheck `curl /v1/health` 200；含 `.env.example`；附 K8s Deployment 等效 manifest 留 [SPEC-DEFER:phase-future.k8s-helm-chart] [SPEC-OWNER:task-16.4]

**关键 scope 决策（§3）**：本 phase 实施 4 项 backend / CI / deploy 范例 → v0.9.0 ship；**不实施** P2 #6 `is_pinned` amendment（独立 Phase 17 + ADR-022，需 Console 端先 ship contractv1 字段 amend cross-repo PR）。 [SPEC-OWNER:phase-17.is-pinned-amendment]

来源：[ContextForge-Console PR #91/#93](https://github.com/contextforge-console/PR#91) backlog 11 项中 P3+P4 共 4 项（Phase 15 ship 后剩余）/ [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D4 fallback-inmem 默认 deny / [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D4 long-poll v1.0 lock（无 SSE）/ [ADR-018](../../decisions/adr-018-fallback-inmem-default-reversal.md) D1-D4 production default fallback deny / [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) D1 add-only 约束（本 phase 不动 contractv1 字段集合）。

## 2. 业务价值

直接支撑 ContextForge PRD §Core Capabilities #1-5 的 UI 完整闭环 + 部署体验完善：

- **历史查询跨 daemon 重启不丢**：v0.9 ship 后 Console UI Dashboard "最近查询" 面板 + `GET /v1/search/{query_id}/trace` drill-down 在 daemon 重启后仍能返历史，不再"近 1000 条 in-memory" 痛点 [SPEC-OWNER:task-16.1]
- **events stream 响应及时**：long-poll wait 真生效 → Console UI Memory 操作历史 / IndexJob 进度面板从"batch poll 30s"升级"poll wakeup on event"，UX 实时性显著提升
- **docker pull 即可获镜像**：用户 `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0` 一行命令拉到 v0.9.0 image；不再需要本地 `git clone + docker build`；release distribution 闭环
- **production-ready compose 范例**：用户 `docker compose -f deploy/docker-compose.production.yml up -d` 即获真持久化双进程 stack；既存 `deploy/console-stack.yml` 是 dev/PoC（注释 v0.8 已埋伏笔）
- **ADR-014 第七次激活**：v0.3-v0.8 六次跑通 + Phase 16 第七次再验证；制度稳定性跨 7 phase 累计自信
- **Console PR #91/#93 backlog closure 推进至 10/11 (91%)**：仅剩 P2 #6 `is_pinned` cross-repo coordination 项

不在本 phase scope：

- **P2 #6 `MemoryItem.is_pinned`** 字段（独立 Phase 17 + ADR-022 — 需 Console 端先 ship contractv1.go 字段 amend）[SPEC-OWNER:phase-17.is-pinned-amendment]
- **多 arch 镜像 (linux/arm64)** [SPEC-DEFER:phase-future.multi-arch-image]：v0.9 仅 linux/amd64；arm64 build 时长 + apple silicon 用户量增加后再扩
- **K8s Helm Chart** [SPEC-DEFER:phase-future.k8s-helm-chart]：v0.9 仅 docker-compose 范例；K8s manifest / Helm 留 v1.x
- **events SSE / WebSocket** [SPEC-DEFER:phase-future.events-sse-push]：v0.9 仍 long-poll（ADR-017 D4 lock 沿用）；SSE 留 v1.x（Console v1.0 HTTPAdapter 不消费 SSE）
- **TraceStore FULLTEXT 检索** [SPEC-DEFER:phase-future.tracestore-fts]：v0.9 仅 list by recency + get by id；FTS（按 query 文本搜历史）留 v1.x
- **TraceStore 跨 workspace 隔离严格化** [SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]：v0.9 沿用 task-15.5 既有 workspace_id 顺带字段；strict isolation 留 v1.x
- **events ?since=cursor 增量拉取** [SPEC-DEFER:phase-future.events-cursor-pagination]：v0.9 仍按 limit 拉最近 N；cursor 增量留 v1.x
- **ghcr 镜像签名 (cosign) / SBOM** [SPEC-DEFER:phase-future.image-signing-and-sbom]：v0.9 仅 build + push；签名 + SBOM 留 v1.x
- **compose production 自动 letsencrypt TLS** [SPEC-DEFER:phase-future.compose-tls-termination]：v0.9 范例假定外层 reverse proxy 处理 TLS；自动 cert 留 v1.x

## 3. 涉及模块

- `core/migrations/0015_search_traces.sql`（新增：表 `search_traces` 5 列 [query_id PK / trace_json TEXT / workspace_id TEXT / ts_unix INTEGER / created_at TEXT] + 1 索引 idx_traces_ts_desc on (ts_unix DESC)）— task-16.1
- `core/src/data_plane/search.rs`（修改：`TraceStore` 改 write-through 设计 — 持 `Arc<Mutex<TraceMem>>` (既有 HashMap+VecDeque) + `Arc<SqliteTracePersist>` (新)；`new` 接 `data_dir` 启动时 warm restore；`put` 双写；`get` / `list` 内存 miss 时落 SQLite）— task-16.1
- `core/src/data_plane/search_persist.rs`（新增：`SqliteTracePersist` struct + open(data_dir) / put / list / load_warm 方法 + 单元测试）— task-16.1
- `core/src/data_plane/mod.rs`（修改：注册 `pub mod search_persist`；`SearchServer::new` 签名加 `data_dir: PathBuf` 参；既有 callers 更新）— task-16.1
- `core/src/server.rs`（修改：`serve_full` 把 `data_dir` 传给 `SearchServer::new`）— task-16.1
- `internal/consoleapi/handlers.go`（修改：`handleEvents` line 637-653 把 `parseWaitParam` 结果真传到 `deps.Events.Recent`）— task-16.2
- `internal/consoleapi/types.go`（修改：`EventsClient.Recent` 签名加 `wait time.Duration` 参；line 77-79）— task-16.2
- `internal/consoleapi/grpcclient/grpcclient.go`（修改：`eventsClient.Recent` 实现真 long-poll — `ctx, cancel := context.WithTimeout(ctx, wait)` + 调 gRPC `Subscribe` + 在 stream 上 `Recv()` block 等 ≥1 event 或 ctx.Done()；返时 cancel 释放 broadcast::Receiver）— task-16.2
- `internal/consoleapi/memstore.go`（修改：`MemStore.Recent` 加 `wait time.Duration` 参 — fallback 模式无真 event source，wait 直接 sleep min(wait, 1s) 后返既有 in-memory ring buffer）— task-16.2
- `internal/consoleapi/handlers_test.go`（新增：`TestHandleEvents_Wait5s_Blocks_When_NoEvent` + `TestHandleEvents_Wait5s_Returns_Early_OnEvent` ≥2 unit test）— task-16.2
- `internal/consoleapi/e2e_grpc_test.go`（修改：既有 Step 11 events keepalive 不退化 + 加 Step 11b real long-poll wait 端到端）— task-16.2
- `.github/workflows/release.yml`（新增：tag push 触发 docker build + push ghcr）— task-16.3
- `.github/workflows/ci.yml`（新增 OR 修改：PR + push master 触发 cargo test + go test + spec_drift_lint --strict；本 phase 同 PR ship 让 CI 链路完整化）[SPEC-OWNER:task-16.3]— task-16.3
- `deploy/docker-compose.production.yml`（新增：双进程 production stack 范例 + `.env.example` 同 PR ship）— task-16.4
- `deploy/.env.production.example`（新增：可空模板，注释列 `CONSOLE_API_AUTH_TOKEN` / `CONTEXTFORGE_LOG_LEVEL` 等可调环境变量）— task-16.4
- `Dockerfile`（**不**修改 — 复用既有 multi-stage build；task-16.3 workflow 直接调 `docker build .`）
- `docs/deploy/production.md`（新增：production deploy 文档 — compose-prod 用法 + ghcr pull 指南 + K8s 等效骨架 + secrets 处理；不是 spec scope 但同 task-16.4 PR ship 文档闭环）[SPEC-OWNER:task-16.4]
- `scripts/console_smoke.sh` v7（修改：24-step v6 → 27-step v7；加 step 25/26/27 long-poll wait / TraceStore restart roundtrip / compose-prod stack health check）— task-16.4 收口
- `scripts/release_smoke.sh`（修改：加 `phase16_*=ok` 子检查 — 含 ghcr push verify + compose-prod up + smoke v7）— task-16.4 收口
- `docs/s2v-adapter.md`（修改：§Phases 加 Phase 16 行 / §Tasks 加 task-16.1-16.4 / §BDD 加 phase-16 feature 引用 / §ADRs 段不动 — 本 phase 不引入新 ADR）
- `docs/prds/context-forge.prd.md`（修改：§Implementation Phases 加 Phase 16 段；§Open Questions 不新增）
- `test/features/phase-16-v0.9.0-backlog-completion.feature`（新增：≥4 scenarios — TraceStore restart roundtrip / events long-poll wait / ghcr image pullable / compose-prod stack health）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 16.1 | `core/migrations/0015_search_traces.sql` + `core/src/data_plane/search_persist.rs` + `core/src/data_plane/search.rs` write-through | `../tasks/task-16.1-tracestore-sqlite-persistence.md` |
| 16.2 | `internal/consoleapi/handlers.go` + `types.go` + `grpcclient/grpcclient.go` + `memstore.go` | `../tasks/task-16.2-events-real-long-poll.md` |
| 16.3 | `.github/workflows/release.yml` + `.github/workflows/ci.yml` | `../tasks/task-16.3-ghcr-image-push-ci.md` |
| 16.4 | `deploy/docker-compose.production.yml` + `deploy/.env.production.example` + `docs/deploy/production.md` + smoke v7 + release_smoke.sh | `../tasks/task-16.4-compose-production-example.md` |

## 5. 依赖关系

- **依赖**：
  - Phase 11（console-real-data-plane）— 复用 `EventBus` broadcast::Sender + EventsService gRPC server-stream（task-16.2 直接消费现有 stream，不改 Rust 侧）
  - Phase 12（console-contract-completion）— 复用既有 `TraceStore` 内存 LRU 实现（task-16.1 改 write-through，不破现有 cap=1000 LRU 行为）
  - Phase 15（console-functional-gap-closure）— 复用 task-15.5 `TraceStore.list` + `TraceRecord {trace, workspace_id, ts_unix}` 结构（task-16.1 持久化字段集合直接对齐）
  - [ADR-016](../../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) D1 Rust SoT + D3 Go thin proxy（task-16.1 Rust 持久化；task-16.2 Go 改 client 行为不动 Rust）
  - [ADR-017](../../decisions/adr-017-console-contract-completion-22-endpoint.md) D4 long-poll v1.0 lock（task-16.2 沿用 long-poll；不引入 SSE）
  - [ADR-018](../../decisions/adr-018-fallback-inmem-default-reversal.md) D1-D4 fallback deny 默认（task-16.4 production stack **不**注入 `CONSOLE_API_FALLBACK_INMEM=1`）
  - [ADR-014](../../decisions/adr-014-cross-phase-exit-criteria-validation.md) 第七次激活
- **可并行**：4 task 内 task-16.1（纯 Rust SoT）+ task-16.2（纯 Go thin proxy）跨 tier **可并行**但因 events.rs / search.rs 关联弱；推荐顺序：task-16.1 → task-16.2 → (task-16.3 // task-16.4 并行)；task-16.3 / 16.4 完全独立可并行
- **Phase 内推荐序**：task-16.1（SoT 持久化基础）→ task-16.2（Go thin proxy 行为修正）→ task-16.3 // task-16.4（独立 ops 项可并行 ship）

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（任务 16.1-16.4 全 Done，实测验证；每条 AC 含 ADR-014 D3 verified by 显式 owner）**：

- [ ] AC1：TraceStore SQLite roundtrip — `POST /v1/search` × 3 → daemon 强制 kill + 重启 → `GET /v1/queries?limit=10` 返 3 条历史 + `GET /v1/search/{query_id}/trace` per id 返 200 with 真 trace；`0015_search_traces.sql` migration 自动建表（IF NOT EXISTS 幂等）— **verified by smoke v7 Step 26 (daemon-level kill+restart roundtrip; task-16.4 收口) + `core/tests/search_persist_integration.rs::test_tracestore_persists_across_restart` (Rust-level Store drop+recreate; task-16.1 §6 AC3) + `core/src/data_plane/search_persist.rs::tests::test_put_then_load_warm_restores_recent_1000` (persist 层 unit; task-16.1 §6 AC2) PASS**
- [ ] AC2：events `?wait=5s` real long-poll — `GET /v1/observability/events?wait=5s` 在无新 event 时真 block 5s 返 200 + []；触发 indexing.progress 事件后立刻返 200 + [evt]（≤ 200ms latency）；多 client subscribe 并行不互相阻塞 — **verified by task-16.2 §6 AC1/AC2 + `internal/consoleapi/handlers_test.go::TestHandleEvents_Wait5s_Blocks_When_NoEvent` + `TestHandleEvents_Returns_Early_OnEvent` + e2e_grpc Step 11b PASS**
- [ ] AC3：`.github/workflows/release.yml` tag push 触发 docker build + push 到 `ghcr.io/tajiaoyezi/contextforge-daemon:{tag}` + `:latest`；`docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0` 拉取成功；`docker run` 容器 healthy；`docker pull ghcr.io/tajiaoyezi/contextforge-daemon:latest` 拿到 v0.9.0 — **verified by task-16.3 §6 AC1/AC2/AC3 + workflow `gh workflow run release.yml -f tag=v0.9.0-rc1` 实测 ghcr 包列表显式列出包 + `docker pull` 实测 ship 前手动 verify**
- [ ] AC4：`docker compose -f deploy/docker-compose.production.yml up -d` 后 `contextforge-core` + `console-api-serve` 两容器健康；`curl http://localhost:48181/v1/health` 返 200 + `status: "healthy"`（非 degraded — ADR-018 fallback deny 默认）；数据卷 `contextforge-data` 跨容器重启数据保留 — **verified by task-16.4 §6 AC1/AC2/AC3 + task-16.4 §9 runtime-smoke segment (docker compose up + curl + restart 卷验证) + smoke v7 Step 27 (gated `COMPOSE_PROD_SMOKE=1`，task-16.4 收口) PASS**
- [ ] AC5：既有 `cargo test --workspace` 121 lib + 17 integration 不退化；`go test ./...` 22 packages 不退化；`test/conformance` 22-endpoint Console contract 不退化；`scripts/console_smoke.sh` v7 27-step bash 语法 OK — **verified by closeout PR body PR diff 跑 cargo + go + bash -n 实测**
- [ ] AC6：ADR-014 cross-validation gate 全套通过 — D1 mapping table (Phase §6 ↔ Task §6 AC) + D2 lint `scripts/spec_drift_lint.sh --touched origin/master` 0 unannotated hits + D3 verified-by 显式 + D4 governance 主 agent 自治 + D5 历史 Phase 1-15 spec 不溯改 — **verified by closeout PR body 含 D1 mapping 表 + D2 输出段 + D3 上述 §6 AC 全含 verified-by + D5 git diff 仅触新加 spec 文件**

**端到端 smoke**：

```bash
# step 1 — Phase 16 主集成 smoke (v7，含 27 step flow)
bash scripts/console_smoke.sh
# 1) spawn contextforge-core daemon
# 2) spawn console-api-serve
# 3) curl 27 endpoint:
#    含 既有 24 endpoint 不退化 (Phase 15 v6 baseline)
#    含 step 25: GET /v1/observability/events?wait=2s (在无 event 时 block 2s 返 [])
#    含 step 26: TraceStore restart roundtrip (kill -9 daemon + restart + GET /v1/queries 拿历史)
#    含 step 27: compose-prod stack health (gated env COMPOSE_PROD_SMOKE=1)
# 4) CONSOLE_REAL_SMOKE_EXIT=0

# step 2 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
# expect: 0 unannotated hits

# step 3 — Release smoke (v0.9.0 release prep)
bash scripts/release_smoke.sh
# PHASE_RELEASE_SMOKE_EXIT=0
# phase16_backlog_completion=ok 段加入
```

step 1 是 task-16.4 Gate 3 入口；27 step flow 是 Phase 16 ship 收口标志。

step 3 release_smoke.sh 在本 phase 加入 `phase16_*=ok` 子段 = v0.9.0 ship gate 最后一道。

## 7. 阶段级风险

- **TraceStore SQLite migration 跨 daemon 升级**：v0.8 → v0.9 fresh install OK（migration IF NOT EXISTS 幂等）；升级用户重启 daemon 后 `search_traces` 表自动创建 + 内存 LRU warm restore 从空 SQLite 开始；首次 search 后开始累积；不破坏既有 v0.8 行为
- **events long-poll Go ctx cancel**：client disconnect 时 grpcclient.eventsClient.Recent 必须 cancel ctx 释放 broadcast::Receiver；不当处理会 leak 后端 stream goroutine；task-16.2 §10 trade-off 记录 + handlers_test.go 加 ctx cancel test
- **ghcr secrets 权限**：workflow 用 `GITHUB_TOKEN` 自动注入 + `permissions: packages: write` 显式声明；不引入 PAT secrets；token 仅 workflow job 内可见
- **compose-prod 跨容器 gRPC bind**：`contextforge-core` 必须 bind `0.0.0.0:50551`（默认 `127.0.0.1` 跨容器不可达）+ `console-api-serve --grpc-addr=contextforge-core:50551` 用 service 名 DNS resolve；compose-prod yml 显式覆盖 `command:` 字段
- **关联 ADR-014 governance 第七次激活风险**：v0.3-v0.8 六次跑通 + Phase 16 第七次；closeout PR 体不引入新 ADR（4 task 全部既有 ADR 延伸）→ D1 mapping table 必须显式标明"无新 ADR Status 推进"
- **ADR-015 D1 add-only 红线**：4 task 全 add-only 改动 — 不动 contractv1.go 字段集合 + 不动 proto wire format；新加 `search_traces` SQLite 表为内部 schema（不暴露 contract layer）
- **release_smoke.sh ghcr verify 双向耦合**：phase16 segment 是否包含 `docker pull ghcr.io/...:v0.9.0` 实测 — 推荐 gated env `RELEASE_SMOKE_GHCR=1` 默认 SKIP（避 CI 强依赖 docker daemon）
- **TraceStore 表大小不收敛**：v0.9 ship 仅依赖内存 LRU cap=1000 写穿；SQLite 端无 LRU eviction → 长时间运行后 `search_traces` 表可能数百万行；task-16.1 §10 trade-off 记录 + 留 `[SPEC-DEFER:phase-future.tracestore-sqlite-vacuum]` 自动维护

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done（16.1-16.4 全 Done — PR 顺序合到 master）
- [ ] §6 阶段级 AC 全部满足；smoke v7 含 3 新 step（bash syntax 验证 + REAL daemon 实测）；spec_drift_lint.sh --touched 0 violation；既有 22-endpoint conformance 不退化
- [ ] 关联风险（migration upgrade / ctx cancel / ghcr secrets / compose-prod bind / table size）缓解措施已落地（write-through 双写 + ctx cancel 解 broadcast / GITHUB_TOKEN scoped permissions / yml command 覆盖 / [SPEC-DEFER] 标 SQLite vacuum）
- [ ] adapter §Phase 状态索引 Phase 16 → Done（本 closeout PR）
- [ ] **本 phase 不引入新 ADR**（4 task 全为既有 ADR-013/015/016/017/018 的延伸实施；ghcr/compose 是 ops 实践不构成 architectural decision；closeout PR body 明示）
- [ ] PRD §Implementation Phases Phase 16 段新增（E1 spec PR 内落地）
- [ ] **ADR-014 D1 mapping 表**：closeout PR body 含 Phase §6 ↔ Task §6 AC 映射（5 行表）
- [ ] **ADR-014 D2 lint 输出**：closeout PR body 含 0 unannotated hits 输出
- [ ] v0.9.0 release tag prep ready + **Console PR #91/#93 backlog 10/11 项 closed 证据** — 移至 E5 release docs PR + E6 tag/release
- [ ] cross-repo follow-up：通知 Console 团队 ContextForge v0.9.0 release ship + **Phase 17 is_pinned amendment 启动信号**（剩 1 项 backlog；需 Console 端先 ship contractv1.go IsPinned 字段 amend PR）— 移至 E6 cross-repo notify (user-forwarded)
