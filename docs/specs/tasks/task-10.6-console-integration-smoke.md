# Task `10.6`: `console-integration-smoke — scripts/console_smoke.sh docker compose + Console UI 真调真 ContextForge`

**Status**: Ready

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 10 (console-contract-v1)
**Dependencies**: task-10.5 (conformance test PASS) — 本 task 是 Phase 10 收口 task，前 5 task 都需 merge

## 1. Background

task-10.4 9 REST endpoint + task-10.5 conformance PASS 之后，需要 docker compose 启动真实 Console v1.0 + 真实 ContextForge daemon + 真 Postgres / Redis，验证 Console UI 端到端能通过 Console BFF (内部走 HTTPAdapter) 调真 ContextForge 返回 workspace 列表（非 Mock 数据）。详 [ADR-015](../../decisions/adr-015-console-contract-v1-compatibility.md) §D6。

## 2. Goal

`scripts/console_smoke.sh` 启动 docker compose stack (Console v1.0 image + ContextForge daemon + Postgres + Redis) + 健康检查 + curl Console UI `/api/workspaces` 真验证 + `CONSOLE_SMOKE_EXIT=0` final marker；`deploy/console-stack.yml` 落 compose 描述；README 加 v0.3 Console 集成段；v0.3.0 release docs 落地。

## 3. Scope

### In Scope

- **新增 `deploy/console-stack.yml`**：docker compose v3.8+ 描述：
  - service `postgres`: postgres:16 image, env POSTGRES_USER/PASSWORD/DB, volume mount
  - service `redis`: redis:7 image
  - service `contextforge-daemon`: 本仓 Dockerfile build (`build: .` 或拉 release image); env CONTEXTFORGE_DATA_DIR=/data, port 48181, healthcheck `curl localhost:48181/v1/health`
  - service `console-api`: Console 仓库 build image (`image: contextforge-console-api:v1.0.0` 或 `build: ../ContextForge-Console/console-api`); env CONSOLE_API_CORE_ADAPTER=http + CONSOLE_API_CORE_URL=http://contextforge-daemon:48181 + 数据库连接; healthcheck
  - service `console-web`: Console 仓库 build image (`image: contextforge-console-web:v1.0.0`); env NEXT_PUBLIC_API_URL=http://localhost:3000/api; port 3000
  - 网络：bridge default 让容器间 hostname resolve
- **新增 `scripts/console_smoke.sh`**：
  ```bash
  #!/usr/bin/env bash
  set -euo pipefail
  ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  cd "$ROOT"
  
  echo "[1/6] docker compose up"
  docker compose -f deploy/console-stack.yml up -d --wait
  
  echo "[2/6] wait for ContextForge daemon healthy"
  for i in $(seq 1 30); do
    if curl -s http://localhost:48181/v1/health | grep -q '"contract_version":"v1"'; then break; fi
    sleep 2
  done
  
  echo "[3/6] wait for Console API + Web healthy"
  curl -s http://localhost:3000/api/healthz || true  # Console healthz endpoint
  curl -s http://localhost:3000/                # SSR root
  
  echo "[4/6] create test workspace via Console BFF (proxies to ContextForge)"
  curl -s -XPOST http://localhost:3000/api/workspaces \
       -H 'Content-Type: application/json' \
       -d '{"name":"smoke-test","root_path":"/data/smoke","allowlist":[],"denylist":[]}'
  
  echo "[5/6] list workspaces via Console BFF → assert real data (not Mock)"
  curl -s http://localhost:3000/api/workspaces \
       | grep -q '"name":"smoke-test"' \
       || { echo "FAIL: workspace not in list (Console possibly served Mock data)"; exit 1; }
  
  echo "[6/6] teardown"
  docker compose -f deploy/console-stack.yml down -v
  
  echo "CONSOLE_SMOKE_EXIT=0"
  ```
- **新增 `Dockerfile`**：用于 contextforge-daemon image build (`golang:1.22 + rust:1.75` 多阶段；输出单 binary)
- **README v0.3 段更新**：在 README.md 加 "v0.3 Console Integration" 段，文档：
  - docker compose 启动方式 + 步骤
  - Console UI 端口 (3000) + ContextForge daemon 端口 (48181)
  - Cross-repo 依赖 (Console 仓库需 ship 到 v1.0 image)
  - v0.3 limitations (Linux/WSL2 only)
- **新增 `docs/releases/v0.3.0-{evidence,artifacts}.md`**：按 v0.2 evidence + artifacts 模板 (task-9.6 已落) 镜像 + 填 Phase 10 内容
- **release_smoke.sh 第 5 段**：在现有 `scripts/release_smoke.sh` 加 phase 10 console smoke 段（条件跑 — env `RELEASE_SMOKE_CONSOLE=1` 时跑，默认 SKIP；avoid CI 默认拉 docker image 失败）
- 文件锚点：`deploy/console-stack.yml` + `scripts/console_smoke.sh` + `Dockerfile` + `README.md` + `RELEASE_NOTES.md` + `docs/releases/v0.3.0-{evidence,artifacts}.md` + `scripts/release_smoke.sh` (修改)

### Out Of Scope

- **macOS / Windows native docker compose smoke**：v0.3 Linux / WSL2 only；macOS docker desktop 应能跑但不在 §6 AC；Windows native 走 WSL2
- **真 Postgres / Redis 持久化数据校验** [SPEC-DEFER:task-future.smoke-data-validation]：v0.3 smoke 仅验 workspace 创建 + 列表；持久化数据细化（如 IndexJob 历史 / Search trace 持久化跨重启）留 v0.4
- **Performance benchmark (P95 < 200ms etc.)**：v0.3 smoke 不带性能 gate；性能基准留 v0.4
- **Multi-instance HA / Kubernetes deployment**：v0.3 single-node docker compose；K8s 留 v0.5
- **Production-ready TLS**：v0.3 docker compose 内 HTTP-only (localhost only)；TLS 配置留 v0.4
- **Console image 自动拉 / 自动 build 切换**：v0.3 smoke 默认期望 Console image 已本地存在；若不存在 → 文档化 fallback 路径 (`docker compose build console-api console-web`)
- **快照恢复 / blue-green deploy 之类的 ops 题**：v0.3 仅 smoke；ops 路径留 v0.4+

## 4. Users / Actors

- **Phase 10 closeout reviewer**：本 task 是 Phase §6 AC6 owner + phase smoke step 1 owner
- **Release engineer**：v0.3.0 tag 前 release smoke 跑 (release_smoke.sh 第 5 段)
- **下游用户 (v0.3 用户)**：README v0.3 段是用户首次 Console 集成入口

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/decisions/adr-015-console-contract-v1-compatibility.md` §D6
- `docs/specs/phases/phase-10-console-contract-v1.md`
- `docs/specs/tasks/task-10.4-rest-endpoints.md`
- `docs/specs/tasks/task-10.5-conformance-test.md`
- `scripts/release_smoke.sh` (现有 4 段结构 — task-9.5)
- `scripts/quickstart_smoke.sh` (v0.2 7-step 模式参考 — task-9.6)
- Console PRD §Constraints "发布" 段 (Console docker compose 主发布形态)

### 5.2 Imports / 工具链

- **bash** (smoke script)
- **docker + docker compose v2** (compose 启动)
- **curl** (健康检查 + 数据验证)
- **Dockerfile 多阶段** (golang:1.22 + rust:1.75 alpine)
- **不引入新 Go / Rust dep**：R7 不触发

### 5.3 函数签名 / 命令契约

```bash
# scripts/console_smoke.sh
# 退出码：0 PASS / 非 0 FAIL
# 最终标记行：CONSOLE_SMOKE_EXIT=0
# 入口：bash scripts/console_smoke.sh
# 需求：docker compose v2 + curl 在 PATH + ports 3000/48181/5432/6379 空闲
# 副作用：docker compose up -d → down -v (清空 volumes)
```

## 6. Acceptance Criteria

- [ ] AC1：`deploy/console-stack.yml` 含 4 service (postgres + redis + contextforge-daemon + console-api + console-web) + healthcheck + 网络配置 — **verified by manual `docker compose -f deploy/console-stack.yml config` lint 全过**
- [ ] AC2：`Dockerfile` 多阶段 build 含 golang + rust 阶段 + 输出 contextforge daemon binary；`docker build -t contextforge-daemon:test .` 成功 — **verified by integration-test step `docker build -t contextforge-daemon:test .` exit 0**
- [ ] AC3：`scripts/console_smoke.sh` 端到端 PASS — docker compose up + ContextForge daemon healthy (contract_version="v1") + Console UI 真返回创建的 workspace + 输出 `CONSOLE_SMOKE_EXIT=0` — **verified by integration-test step `bash scripts/console_smoke.sh` exit 0 + grep "CONSOLE_SMOKE_EXIT=0" 输出**
- [ ] AC4：README v0.3 Console Integration 段 + docs/releases/v0.3.0-{evidence,artifacts}.md 填实（HEAD SHA 在 closeout PR 内回填） — **verified by manual cat + grep "v0.3.0" + grep "console"**
- [ ] AC5：`scripts/release_smoke.sh` 第 5 段新增 phase 10 console smoke 条件段 (env `RELEASE_SMOKE_CONSOLE=1` 启用) + `PHASE_RELEASE_SMOKE_EXIT=0` 兼容 — **verified by integration-test step `RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh` exit 0**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | compose yml 4 service | deploy/console-stack.yml | Not Started |
| AC2 | Dockerfile build PASS | Dockerfile | Not Started |
| AC3 | console_smoke.sh PASS | scripts/console_smoke.sh | Not Started |
| AC4 | README + release docs | README.md + docs/releases/v0.3.0-*.md | Not Started |
| AC5 | release_smoke.sh 第 5 段 | scripts/release_smoke.sh | Not Started |

## 8. Risks

- **Console v1.0 docker image 不可用**：Console 仓库可能尚未 build image；缓解 compose yml 默认 `build:` Console 源码 path + 文档化拉 image fallback
- **跨容器网络问题**：contextforge-daemon ↔ console-api 容器间 hostname resolve；缓解用 docker compose 默认 bridge network + service name 作为 hostname
- **端口冲突 (3000/48181/5432/6379)**：smoke script 启动前 check 端口空闲；占用 → 提示用户停占用进程
- **docker compose v2 vs v1 语法**：smoke script 显式用 `docker compose`（v2 标准）不用 `docker-compose` (v1 deprecated)
- **Cross-repo Console build 时间**：compose 拉 Console 源码 build 可能慢；缓解 README 说明可预先 docker pull
- **release_smoke.sh 第 5 段默认 SKIP**：避免 CI 默认拉 docker 失败；显式 env `RELEASE_SMOKE_CONSOLE=1` 启用

## 9. Verification Plan

- **install**: `docker version && docker compose version` 检查可用
- **lint**: `docker compose -f deploy/console-stack.yml config` (yml lint)
- **typecheck**: N/A (bash + yml)
- **unit-test**: N/A (smoke script 是 e2e)
- **integration**: `bash scripts/console_smoke.sh` exit 0 + grep `CONSOLE_SMOKE_EXIT=0`
- **e2e**: 复用 integration
- **build**: `docker build -t contextforge-daemon:test .` (Dockerfile build verify)
- **coverage**: N/A
- **runtime-smoke**: smoke script 自身即 runtime smoke
- **manual**: 浏览器打开 http://localhost:3000 看 Console UI Workspaces 页是否列出 smoke 创建的 workspace

## 10. Completion Notes

<!-- 完工时按 standard.md §8.3 6 项 schema 回填 -->

- **完成日期**：<TBD-after-impl>
- **改动文件**：<TBD-after-impl>
- **commit 列表**：<TBD-after-impl>
- **§9 Verification 结果**：
  - install: <TBD-after-impl>
  - lint: <TBD-after-impl>
  - integration: <TBD-after-impl>
  - build: <TBD-after-impl>
  - runtime-smoke: <TBD-after-impl>
  - manual: <TBD-after-impl>
- **剩余风险 / 未做项**：<TBD-after-impl>
- **下游 task 影响**：v0.3.0 tag 前需 PASS；release docs 落地后启动 closeout PR
