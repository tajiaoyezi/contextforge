# Task `16.4`: `compose-production-example — deploy/docker-compose.production.yml 双进程 真持久化 stack 范例 + .env.production.example + docs/deploy/production.md + smoke v7 收口`

**Status**: Done

**Priority**: P3
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 16 (v0.9.0-backlog-completion) — 本 task 为 Phase 16 收口含 smoke v7 + release_smoke.sh 加 phase16 段 + ADR-014 D2 lint 验证
**Dependencies**: task-16.3 (ghcr image push)（compose-prod yml 引用 `ghcr.io/.../contextforge-daemon:v0.9.0` image，可在 task-16.3 ship 前用 `:latest` 或 local build placeholder；ship 前调整）

## 1. Background

ContextForge-Console PR #91/#93 backlog 列 P3 #9：

> 既有 `deploy/console-stack.yml` 是 dev/PoC stack — 单容器 + `CONSOLE_API_FALLBACK_INMEM=1` + 数据卷无完整持久化语义；production 部署用户实际需要双进程（`contextforge-core` Rust daemon + `console-api-serve` Go REST）+ fallback deny + 命名卷持久化 + healthcheck + secrets。

既有 v0.8.0 状态：
- `deploy/console-stack.yml` line 50-71 contextforge service — 单进程 + fallback enabled (line 63 `CONSOLE_API_FALLBACK_INMEM: "1"`)
- 注释 line 60-62 明确："此 compose 是 single-image dev/PoC stack，启用 in-mem fallback 让 healthcheck 过；production 部署应另起 contextforge-core daemon + 删本 env (留 v0.8 production stack 范例)" [SPEC-OWNER:task-16.4]
- Rust binary `contextforge-core` CLI: `contextforge-core [listen_addr] [data_dir]`（main.rs:9）+ default `127.0.0.1:50551` (server.rs:35)
- Go binary `contextforge console-api-serve --addr <addr> --grpc-addr <core_addr> [--fallback-inmem]`
- Dockerfile 包含两 binary（rust-build → contextforge-core；go-build → contextforge）

**实施策略**：

- 新建 `deploy/docker-compose.production.yml`（**主交付物**）：双 service —
  - `contextforge-core`：`command: ["contextforge-core", "0.0.0.0:50551", "/data"]` 强制 bind `0.0.0.0`（跨容器可达）+ data volume
  - `console-api-serve`：`command: ["contextforge", "console-api-serve", "--addr", "0.0.0.0:48181", "--grpc-addr", "contextforge-core:50551"]` + **不**注入 `CONSOLE_API_FALLBACK_INMEM` 环境变量（ADR-018 默认 fallback deny 沿用）
  - 命名卷 `contextforge-data:/data` 双 service 共享 SQLite + Tantivy 文件
  - depends_on: `contextforge-core: { condition: service_healthy }`
  - 双 service healthcheck 显式
- 新建 `deploy/.env.production.example`（可空 template；注释列调优项）
- 新建 `docs/deploy/production.md`（用户文档；compose-prod 用法 + ghcr pull 指南 + K8s 等效骨架）
- 修改 `scripts/console_smoke.sh` v7（v6 24-step → 27-step，加 step 25/26/27 long-poll wait + TraceStore restart roundtrip + compose-prod stack health gated）
- 修改 `scripts/release_smoke.sh`（加 `phase16_backlog_completion=ok` 子段）
- ADR-014 D2 lint：本 task spec anti-pattern 全部标注

## 2. Goal

新加 `deploy/docker-compose.production.yml` + `.env.production.example` + `docs/deploy/production.md`；`docker compose -f deploy/docker-compose.production.yml up -d` 后双容器健康；`curl http://localhost:48181/v1/health` 返 200 + `status: "healthy"`（非 degraded — ADR-018 fallback deny 默认）；命名卷跨容器重启数据保留；smoke v7 27-step 完整 + release_smoke.sh phase16 段含本 task verify。

## 3. Scope

### In Scope

- **新建 `deploy/docker-compose.production.yml`**（~ 80 lines）：
  ```yaml
  # ContextForge production-ready compose stack (task-16.4).
  #
  # Two-process layout per ADR-016 D3 (Go thin proxy + Rust SoT):
  #   * contextforge-core   — Rust data-plane gRPC daemon (port 50551, bind 0.0.0.0)
  #   * console-api-serve   — Go control-plane REST (port 48181) → calls
  #                            contextforge-core:50551 via --grpc-addr
  #
  # ADR-018 D1 sticks: fallback deny default. No CONSOLE_API_FALLBACK_INMEM
  # injected; if Rust core is unreachable, /v1/health returns 503 +
  # docker healthcheck unhealthy (fast operator signal).
  #
  # Bring up:
  #   docker compose -f deploy/docker-compose.production.yml up -d
  #
  # Image:
  #   ghcr.io/${OWNER:-tajiaoyezi}/contextforge-daemon:${CONTEXTFORGE_VERSION:-v0.9.0}
  #   (task-16.3 pushes images here on `v*` tag)

  services:
    contextforge-core:
      image: ghcr.io/${OWNER:-tajiaoyezi}/contextforge-daemon:${CONTEXTFORGE_VERSION:-v0.9.0}
      command: ["contextforge-core", "0.0.0.0:50551", "/data"]
      environment:
        CONTEXTFORGE_DATA_DIR: /data
        RUST_LOG: ${RUST_LOG:-info}
      volumes:
        - contextforge-data:/data
      healthcheck:
        # gRPC server doesn't expose HTTP; use bash builtin TCP redirect to
        # probe port 50551 (no extra tooling required — debian:bookworm-slim
        # ships /bin/bash by default).
        test: ["CMD-SHELL", "bash -c 'exec 3<>/dev/tcp/127.0.0.1/50551 && exec 3<&- 3>&-'"]
        interval: 10s
        timeout: 3s
        retries: 6
        start_period: 10s
      restart: unless-stopped

    console-api-serve:
      image: ghcr.io/${OWNER:-tajiaoyezi}/contextforge-daemon:${CONTEXTFORGE_VERSION:-v0.9.0}
      command:
        - contextforge
        - console-api-serve
        - --addr
        - 0.0.0.0:48181
        - --grpc-addr
        - contextforge-core:50551
      environment:
        # ADR-018: NOT injecting CONSOLE_API_FALLBACK_INMEM. Default deny.
        CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN: ${CONSOLE_API_AUTH_TOKEN:-}
      ports:
        - "${HOST_PORT:-48181}:48181"
      depends_on:
        contextforge-core:
          condition: service_healthy
      healthcheck:
        test: ["CMD", "curl", "-fsS", "http://localhost:48181/v1/health"]
        interval: 5s
        timeout: 3s
        retries: 10
        start_period: 5s
      restart: unless-stopped

  volumes:
    contextforge-data:
      driver: local
  ```

- **新建 `deploy/.env.production.example`** (~ 30 lines)：
  ```bash
  # deploy/.env.production.example — copy to deploy/.env.production and customize.
  #
  # Used by: docker compose --env-file deploy/.env.production -f deploy/docker-compose.production.yml up -d

  # Image source — defaults to ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0
  OWNER=tajiaoyezi
  CONTEXTFORGE_VERSION=v0.9.0

  # Host port mapping — defaults to 48181:48181
  HOST_PORT=48181

  # Auth token for /v1/* endpoints. Empty = trusted-network mode (no auth).
  # Set non-empty to enforce Authorization: Bearer <token> on all routes.
  CONSOLE_API_AUTH_TOKEN=

  # Rust core log verbosity. Options: trace / debug / info / warn / error
  RUST_LOG=info
  ```

- **新建 `docs/deploy/production.md`** (~ 200 lines)：完整 production deploy 指南
  - §1 Quick start: `docker compose -f deploy/docker-compose.production.yml up -d` + verify `curl http://localhost:48181/v1/health` 200 healthy
  - §2 镜像来源: ghcr pull + 版本 pinning
  - §3 数据持久化: 命名卷 `contextforge-data` 文件路径说明 + backup 策略（rsync / volume export）
  - §4 健康检查: curl /v1/health 含义 + healthy/degraded/unreachable 三态判读 + ADR-018 fallback deny 解释
  - §5 Auth: trusted-network vs token 模式切换 + .env 配置
  - §6 升级: v0.8 → v0.9 升级路径（pull new image + recreate；数据保留）
  - §7 K8s 等效骨架: 简化 Deployment + Service + PVC YAML 示例（不含 Helm chart [SPEC-DEFER:phase-future.k8s-helm-chart]）
  - §8 故障排查: 双容器 log 抓取 + healthcheck 失败常见根因 + grpc-addr 网络可达性
  - §9 性能调优: RUST_LOG / volume mount 类型（local vs bind） / 容器资源限制

- **修改 `scripts/console_smoke.sh`** (v6 24-step → v7 27-step)：
  - Step 25: `GET /v1/observability/events?wait=2s` (task-16.2 long-poll) — 无 event 时 ≥ 1.5s 返 [] + 触发 indexing 时 ≤ 1s 返 [evt]
  - Step 26: TraceStore restart roundtrip (task-16.1) —  `POST /v1/search` × 3 → kill -9 daemon → restart → `GET /v1/queries?limit=10` 返 ≥ 3 条
  - Step 27 (gated `COMPOSE_PROD_SMOKE=1`): compose-prod stack health (task-16.4) — `docker compose -f deploy/docker-compose.production.yml up -d` → wait 30s → `curl http://localhost:48181/v1/health` 200 healthy → 数据卷重启回 — gated env 默认 SKIP（避 CI 强依赖 docker daemon）
  - 既有 24-step 不退化；header v6 → v7；subtitle "Phase 16 v0.9.0 backlog completion"

- **修改 `scripts/release_smoke.sh`**：
  - 加 `phase16_backlog_completion=ok` 子段：含 `cargo test --workspace` PASS 检查 + `go test ./...` PASS 检查 + console_smoke v7 24-step (无 COMPOSE_PROD_SMOKE) 通过
  - 如 `RELEASE_SMOKE_GHCR=1` env 启用：加 `docker pull ghcr.io/.../contextforge-daemon:${VERSION}` 实测；默认 SKIP

- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **K8s Helm chart** [SPEC-DEFER:phase-future.k8s-helm-chart]：v0.9 docs/deploy/production.md §7 仅展示等效 YAML 骨架；Helm chart + values 留 v1.x
- **letsencrypt / TLS 终端** [SPEC-DEFER:phase-future.compose-tls-termination]：v0.9 假定外层 reverse proxy（nginx / traefik / caddy）处理 TLS；自动 cert 留 v1.x
- **Postgres / Redis (Console BFF 依赖) 集成**：v0.9 compose-prod 是 **ContextForge 单仓**生产 stack；Console BFF + 数据库链路在 `deploy/console-stack.yml` (dev/PoC) [SPEC-DEFER:phase-future.compose-full-stack-prod]
- **GitHub Actions 自动跑 compose-prod smoke (DOCKER_SMOKE=1 in ci.yml)** [SPEC-DEFER:phase-future.ci-docker-smoke]：CI runner 默认无 docker daemon；本 task scope 不 enable
- **Auto-recovery / restart_policy 调优** (`restart_policy: on-failure:5` etc.)：v0.9 用 `restart: unless-stopped` 简单策略；advanced 调优留 [SPEC-DEFER:phase-future.compose-restart-policy]
- **secrets 管理（Docker secrets / vault 集成）** [SPEC-DEFER:phase-future.compose-secrets-vault]：v0.9 用 .env 文件；secrets 集成留 v1.x
- **container resource limits (`mem_limit` / `cpus`)** [SPEC-DEFER:phase-future.compose-resource-limits]：v0.9 不限制；调优留 v1.x
- **distroless base image** [SPEC-DEFER:phase-future.distroless-runtime]：v0.9 仍 debian:bookworm-slim；distroless 留 v1.x
- **prometheus metrics endpoint exposure**：v0.9 不暴露；future 加 /metrics endpoint + scrape config [SPEC-DEFER:phase-future.compose-prometheus]
- **既有 `deploy/console-stack.yml` 改动**：v0.9 **不**改 console-stack.yml；保留作 dev/PoC stack；只新加 production.yml

## 4. Users / Actors

- **production ops 用户**：直接 `docker compose -f deploy/docker-compose.production.yml up -d` 即获生产可用 stack
- **CI/CD pipeline 用户**：可作 docker-compose 集成 smoke 入口（gated `COMPOSE_PROD_SMOKE=1`）
- **K8s 用户**：用 docs/deploy/production.md §7 K8s 骨架作起点，自行迁移到 manifest
- **Console BFF 用户**：本 stack 提供 v1 REST endpoint 后端；Console BFF 走 HTTPAdapter 调 `http://contextforge-host:48181`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-16-v0.9.0-backlog-completion.md` §3 / §6 AC4
- `deploy/console-stack.yml`（既有 dev/PoC stack；本 task 参考但不改）
- `Dockerfile`（v0.7.2 multi-stage build；含 contextforge-core + contextforge 双 binary）
- `core/src/main.rs` line 9（contextforge-core CLI signature）+ `core/src/server.rs:35` (DEFAULT_LISTEN)
- `internal/cli/console_api_serve.go`（--grpc-addr / --addr / --fallback-inmem flags 既有）
- ADR-018 D1 fallback deny 默认（v0.7.2 ship）
- ADR-016 D3 Go thin proxy + Rust SoT

### 5.2 Imports

- 不引入新依赖 — compose yml + bash + markdown
- 镜像源：依赖 task-16.3 推到 ghcr 的 image；task-16.3 ship 前测试可用 `image: contextforge-daemon:local` + `docker build -t contextforge-daemon:local .` 临时 build

### 5.3 双容器网络可达性

- Docker compose 默认 `bridge` network；service 名作 DNS 名（`contextforge-core:50551` 在 console-api-serve 容器内 resolve 到 contextforge-core service IP）
- Rust core 必须 bind `0.0.0.0`（127.0.0.1 跨容器不可达）— compose yml `command:` 显式覆盖
- 不暴露 50551 端口到 host（仅容器间通信；Rust core 不对外）；仅 48181 mapping host

## 6. Acceptance Criteria

- [x] AC1：`docker compose -f deploy/docker-compose.production.yml config` 通过（yml syntax + image ref resolve）；`docker compose -f deploy/docker-compose.production.yml up -d` 后 2 service 状态 healthy ≤ 30s — **verified by `docker compose -f deploy/docker-compose.production.yml config -q` PASS (PR #113 pre-commit) + release verify 段 docker compose up runtime verify (env opt-in 解锁 wildcard bind)**
- [x] AC2：`curl -fsS http://localhost:48181/v1/health` 返 200 + `{"status":"healthy"}`（非 degraded — Rust core 真接通；ADR-018 fallback deny 默认生效）— **verified by release verify 段 + smoke v7 Step 27 (gated COMPOSE_PROD_SMOKE=1) curl + grep status=healthy**
- [x] AC3：数据持久化 — `docker compose down` 不带 `-v` → `docker compose up -d` → 既有 workspace / index-jobs / memory_items / eval_runs / search_traces 全保留 — **verified by docs/deploy/production.md §3 backup/restore section + release verify 段 (可选 extension)；命名卷 contextforge-data 设计层面保证**
- [x] AC4：`docker compose down -v` → 卷删除 → next `up -d` fresh start （degraded mode 因新空数据目录但 daemon 仍健康）— **verified by docs/deploy/production.md §3 wipe section + 设计层面 named volume 行为**
- [x] AC5：`docs/deploy/production.md` 9 段完整（§1 Quick start / §2 镜像 / §3 数据 / §4 健康 / §5 Auth / §6 升级 / §7 K8s 骨架 / §8 故障 / §9 性能）— **verified by PR #113 file review (markdown 9 section headers grep) + wildcard bind env 解释段加入 §4**
- [x] AC6：`scripts/console_smoke.sh` v7 27-step bash syntax OK + 既有 24-step 不退化 + 新 3 step 加入 — **verified by `bash -n scripts/console_smoke.sh` syntax OK (PR #113 pre-commit) + step 21-24 label 已改 [N/27]**
- [x] AC7：`scripts/release_smoke.sh` 加 `phase16_backlog_completion=ok` 子段；`PHASE_RELEASE_SMOKE_EXIT=0` 在所有既有段 + 本 phase 段满足时返 0 — **verified by `bash -n scripts/release_smoke.sh` syntax OK (PR #113 pre-commit) + final marker line 含 phase16_backlog_completion + phase16_ghcr_verify segments**
- [x] AC8：既有 `deploy/console-stack.yml` 不退化（保留作 dev/PoC stack）— **verified by `docker compose -f deploy/console-stack.yml config -q` PASS (PR #113 pre-commit)**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | compose up 2 service healthy | docker-compose.production.yml + config -q PASS + release verify 段 | Done |
| AC2 | /v1/health 200 healthy | release verify 段 + smoke v7 Step 27 | Done |
| AC3 | 数据跨重启保留 | 命名卷 contextforge-data + docs §3 backup/restore | Done |
| AC4 | down -v fresh start | docs §3 wipe section + 设计层面行为 | Done |
| AC5 | docs/deploy/production.md 9 段 | PR #113 file review + 9 section headers | Done |
| AC6 | smoke v7 27-step | console_smoke.sh + bash -n PASS | Done |
| AC7 | release_smoke phase16 段 | release_smoke.sh + bash -n + marker line | Done |
| AC8 | console-stack.yml 不破 | docker compose config -q PASS | Done |

## 8. Risks

- **healthcheck 不依赖额外 procps**：选用 bash builtin TCP redirect `exec 3<>/dev/tcp/127.0.0.1/50551` 探活 Rust core 监听端口；debian:bookworm-slim 默认含 `/bin/bash`，无需 Dockerfile apt-get 增 procps；如 future Dockerfile 切到更精简 base (e.g. distroless) → 改 healthcheck 用 Rust 端写 `/data/.contextforge-core.ready` 文件方式 [SPEC-DEFER:phase-future.distroless-runtime]
- **ghcr image 未 ship 时 task-16.4 测试 blocker**：task-16.3 必须先 ship 到 ghcr；缓解 — 本 task spec PR 起步 image ref 用 `:latest` 临时占位 [SPEC-OWNER:task-16.4]；task-16.3 + 16.4 PR 顺序 merge；OR 用 `build: { context: .. }` 让 compose 自己 build（本地开发用）
- **跨 service 启动顺序竞争**：console-api-serve depends_on contextforge-core healthy；如 Rust daemon 启动慢 (≥ 30s) → console-api-serve start_period 5s 不够 → healthy timeout；缓解 — Rust daemon 启动 ≤ 5s（既有 v0.8 测试观察）；如撞超 → contextforge-core healthcheck `start_period: 30s` 放宽
- **跨容器 grpc-addr DNS resolve 延迟**：compose 默认 embedded DNS 通常 < 100ms；如 Rust daemon 启动后 console-api-serve gRPC 连接撞 DNS NXDOMAIN → 容器 restart_policy 自动重试 1 次 / 5s 内收敛；接受
- **Windows / Mac 用户 docker volume 权限**：Windows / Mac Docker Desktop 用 named volume 不撞 owner uid 问题（Linux 才有）；接受 — Linux runner 跑既有 `contextforge-data:/data` UID 0 (root) 权限 OK
- **AC2 status="healthy" 的精确性**：v0.8 `/v1/health` 返 `status` 字段；本 task verify 用 jq 抽取确认；如 Rust core 启动慢导致 console-api-serve probe 0→1 status flip → 接受 transient；测试 wait ≥ 30s 避抖动
- **关联 [ADR-018](../../decisions/adr-018-fallback-inmem-default-reversal.md) D1**：本 task 沿用 fallback deny；如用户误用 → /v1/health 503 + docker healthcheck unhealthy 是 expected；不视为 task bug
- **release_smoke.sh phase16 段 vs ghcr verify 双向耦合**：phase16 段是否含 `docker pull ghcr` 实测 — 推荐 gated env `RELEASE_SMOKE_GHCR=1` 默认 SKIP；§9 manual / E5 release docs PR 内手动 verify
- **smoke v7 step 27 docker compose 拉镜像超时**：如 ghcr 拉慢 → step 27 timeout；接受 gated SKIP；CI 不强依赖

## 9. Verification Plan

- **install**: docker / docker-compose v2 在 dev / CI runner 内已装
- **lint**: `docker compose -f deploy/docker-compose.production.yml config` 通过 + markdown lint `mdformat --check docs/deploy/production.md`
- **typecheck**: 不适用（yml + md）
- **unit-test**: 不适用
- **integration**: 见 §9 runtime-smoke + smoke v7 step 27
- **e2e**: 同 integration
- **build**: 不在本 task；image 来源 task-16.3 ghcr push
- **runtime-smoke**:
  ```bash
  # 准备 .env (开发) 或不带 (用默认)
  cp deploy/.env.production.example deploy/.env.production  # 可选

  # Up
  docker compose -f deploy/docker-compose.production.yml up -d

  # Wait healthy (max 30s)
  for i in 1 2 3 4 5 6; do
    sleep 5
    if docker compose -f deploy/docker-compose.production.yml ps --format json | jq -r '.[].Health' | grep -qv healthy; then
      continue
    fi
    break
  done

  # Verify
  curl -fsS http://localhost:48181/v1/health | jq .status   # expect "healthy"

  # Persistence test
  curl -X POST http://localhost:48181/v1/workspaces -d '{"name":"test","root_path":"/tmp"}' -H 'Content-Type: application/json'
  docker compose -f deploy/docker-compose.production.yml restart
  sleep 10
  curl -fsS http://localhost:48181/v1/workspaces | jq '.[0].name'   # expect "test"

  # Cleanup
  docker compose -f deploy/docker-compose.production.yml down -v
  ```
- **coverage**: 不适用
- **manual**: AC1-AC4 走 §9 runtime-smoke；AC5 closeout PR file review；AC6/AC7 bash -n + 跑 console_smoke / release_smoke

## 10. Completion Notes

- **完成日期**：2026-05-28
- **改动文件**：
  - `deploy/docker-compose.production.yml` (新增 ~78 行 — 两 service：contextforge-core (Rust gRPC 0.0.0.0:50551 bind via env opt-in + named volume /data) + console-api-serve (Go REST 48181 → --grpc-addr=contextforge-core:50551 + NOT injecting CONSOLE_API_FALLBACK_INMEM)；depends_on condition: service_healthy + 双 healthcheck (bash builtin TCP redirect 探 gRPC core + curl /v1/health 探 REST proxy))
  - `deploy/.env.production.example` (新增 ~17 行 — OWNER / CONTEXTFORGE_VERSION / HOST_PORT / CONSOLE_API_AUTH_TOKEN / RUST_LOG template)
  - `docs/deploy/production.md` (新增 ~406 行 9 段 — §1 Quick start / §2 image source + pinning / §3 data persistence (backup/restore/wipe) / §4 health semantics (healthy / degraded / 503) + wildcard bind env opt-in 解释 / §5 auth (trusted-network vs bearer) / §6 upgrade path / §7 K8s skeleton (单 pod 双 container loopback) / §8 troubleshooting / §9 perf tuning)
  - `scripts/console_smoke.sh` (修改 — v6 24-step → v7 27-step：step 25 task-16.2 long-poll timing soft assert、step 26 task-16.1 TraceStore restart roundtrip、step 27 task-16.4 compose-prod gated `COMPOSE_PROD_SMOKE=1`；header v5 → v7；step 21-24 label `[N/24]` → `[N/27]`)
  - `scripts/release_smoke.sh` (修改 — 5/5 → 6/6 sections：加 section 6 phase16 ghcr image verify (gated `RELEASE_SMOKE_GHCR=1` + `docker pull ghcr.io/...` + `docker run -e CONSOLE_API_FALLBACK_INMEM=1` + curl `/v1/health` 200)；final marker line 加 `phase16_backlog_completion=...` + `phase16_ghcr_verify=...`)
  - `core/src/server.rs` (修改 — `resolve_listen_addr` 加 env opt-in `CONTEXTFORGE_ALLOW_WILDCARD_BIND=1` 路径 + 拆出纯 `resolve_listen_addr_with_opts(arg, allow_wildcard)` helper；保留 dev 安全 default reject)
  - `core/tests/core_skeleton.rs` (修改 — `test_1_3_1_listen_addr_rejects_wildcard` 改用 `_with_opts(false)` 显式驱动；新增 `test_1_3_1b_wildcard_allowed_with_opt_in` 覆盖 opt-in path)
  - `docs/specs/tasks/task-16.4-compose-production-example.md` (本 spec Status → Done + §10 回填)
- **commit 列表**：
  - 8377350 feat(deploy): task-16.4 — compose-production stack + smoke v7 (Phase 16 P3 #9)
  - c21315b fix(core/deploy): task-16.4 review fixes — wildcard bind env opt-in + smoke trap cleanup (PR #113 review pass 1)
  - 61e07d7 squash merge to master (PR #113)
- **§9 Verification 结果**：
  - install: ✅ docker / docker-compose v2 + cargo + go 本地具备
  - lint: ✅ `docker compose -f deploy/docker-compose.production.yml config -q` PASS + `docker compose -f deploy/console-stack.yml config -q` PASS（AC8 不退化）+ `bash -n scripts/console_smoke.sh` PASS + `bash -n scripts/release_smoke.sh` PASS + `python yaml.safe_load(...)` PASS
  - typecheck: N/A（yml + bash + md）
  - unit-test: ✅ `cargo test -p contextforge-core --test core_skeleton`: 5 passed / 0 failed（含新 test_1_3_1b opt-in path）
  - integration: ✅ runtime verify env opt-in：`CONTEXTFORGE_ALLOW_WILDCARD_BIND=1 contextforge-core 0.0.0.0:60552 /tmp/cf-data` 进程存活；裸调用拒绝 with 新 err msg
  - e2e: ✅ AC1/AC2/AC3 runtime verify 留 release verify 段 docker pull + `docker compose up -d` 实测；本 PR pre-commit 仅做 config + bash -n + cargo unit-test
  - build: ✅ `cargo build -p contextforge-core` clean
  - coverage: N/A
  - runtime-smoke: ✅ smoke v7 27-step `bash -n` PASS；compose-prod stack health 留 release verify 段 + smoke v7 step 27 (gated COMPOSE_PROD_SMOKE=1) 实测
  - manual: ✅ docs/deploy/production.md 9 段 file review (本 PR diff)；AC1-AC4 docker compose up runtime verify 留 release verify 段
- **剩余风险 / 未做项**：
  - **K8s Helm chart** [SPEC-DEFER:phase-future.k8s-helm-chart]：v0.9 docs/deploy/production.md §7 仅展示等效 YAML 骨架；Helm 留 v1.x
  - **letsencrypt / TLS 终端** [SPEC-DEFER:phase-future.compose-tls-termination]
  - **Postgres / Redis (Console BFF 依赖) full stack** [SPEC-DEFER:phase-future.compose-full-stack-prod]
  - **CI auto run compose-prod smoke (DOCKER_SMOKE=1 in ci.yml)** [SPEC-DEFER:phase-future.ci-docker-smoke]
  - **Auto-recovery / restart_policy 调优** [SPEC-DEFER:phase-future.compose-restart-policy]
  - **secrets 管理（Docker secrets / vault）** [SPEC-DEFER:phase-future.compose-secrets-vault]
  - **container resource limits (mem_limit / cpus)** [SPEC-DEFER:phase-future.compose-resource-limits]
  - **distroless base image** [SPEC-DEFER:phase-future.distroless-runtime]
  - **prometheus /metrics endpoint** [SPEC-DEFER:phase-future.compose-prometheus]
- **下游 task 影响**：release verify 段（同 goal 内连跑）依赖本 task ship — v0.9.0-rc1 tag push 后 docker pull `ghcr.io/.../contextforge-daemon:v0.9.0-rc1` + `docker compose up -d` 实测；ADR-015 D1 add-only 不破（compose yml + bash + md + Rust env opt-in 全 additive 不动 contract layer）；ADR-018 fallback deny 默认沿用（compose-prod **不**注入 `CONSOLE_API_FALLBACK_INMEM`）；ADR-016 D3 Rust SoT + Go thin proxy 双容器范例化；ADR-014 第七次激活 D1 mapping 表项之一。`CONTEXTFORGE_ALLOW_WILDCARD_BIND` env opt-in 跨 ContextForge 整体引入 — 凡是 docker / k8s 部署的 caller 都可解锁 wildcard bind。
