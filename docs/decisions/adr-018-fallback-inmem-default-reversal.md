# ADR `018`: `fallback-inmem-default-reversal`

**Status**: Accepted (2026-07-03 promote — D1-D4 已实际 ship 且下游依赖在用，证据见 §Amendment (2026-07-03 promote)；原 Proposed 2026-05-26)
**Category**: 部署 / 安全 / Docker
**Date**: 2026-05-26
**Decided By**: tajiaoyezi objective + PR #94 reviewer subagent + ContextForge-Console 团队独立 ack
**Related**: ADR-016 (cross-process-rust-go-via-grpc-bridge) / v0.7.1 ship (PR #94, master `233ced5`) / Console PR #91 (master `3370a92`) / RELEASE_NOTES v0.7.1 §"v0.7.2 pre-announce"

## Context

ContextForge v0.7.1 (PR #94, `233ced5`) Dockerfile 加 `ENV CONSOLE_API_FALLBACK_INMEM=1`，让 single-image deployment 默认走 in-memory MemStore 模式 —— `docker run contextforge-daemon:v0.7.1` 一键跑通 + healthcheck 立刻 200。

**问题：silent footgun**

PR #94 reviewer subagent round 1 + ContextForge-Console 团队 (PR #91, master `3370a92`) 独立 flag 同一风险：

- single-image 默认 `inmem-fallback` 模式 → **容器重启数据全失**
- HTTP 200 healthcheck 把"degraded but functional"状态掩盖为"healthy"
- 用户起 image → 配置 workspace + index 数据 → 重启 → 数据消失，且 healthcheck 全程绿 → 无告警
- 3 道现有 telemetry（stderr WARN / `/v1/health` status=degraded JSON 字段 / error_reason 文案）都**充分**，但被 HTTP status code = 200 这一最强信号覆盖

**v0.7.1 ship 当时 trade-off**：silent footgun vs out-of-box friendliness。当时选 friendliness（让 ContextForge-Console 联调期能立刻起 docker container），把 reversal 推到 v0.7.2 follow-up。

**Console 团队 ack 立场（PR #91 master `3370a92`）**：

| 方案 | Console 端立场 | 理由 |
|---|---|---|
| (a) 默认改 503 强制 opt-in | **推荐** | deployment correctness > out-of-box friendliness；docker healthcheck 立刻报 unhealthy 是最强信号；用户 fork 时强制看到 env 行 |
| (b) startup-banner WARN | 次选 | Console 端只跟 console-api container log，可能漏看 contextforge container stderr |
| (c) ship 2 进程 image | 长期路径不要塞 patch | 等同 F4 v0.8 提前；Console docker-compose 负担大 |

ContextForge 团队 v0.7.2 选 (a)。

## Decision

ContextForge v0.7.2 patch release 通过 **4 个 Decision** 反转 single-image deployment 默认行为：fallback 默认 deny + 显式 opt-in；env 名保留；最小 code 变更 + Console standby 同步。

### D1 — Default fallback 反转：deny by default

- v0.7.1 行为：`ENV CONSOLE_API_FALLBACK_INMEM=1` 在 Dockerfile 强制 set → daemon 默认 fallback-inmem 模式 → `/v1/health` 200
- v0.7.2 行为：
  - 删 Dockerfile `ENV CONSOLE_API_FALLBACK_INMEM=1` 行
  - daemon binary default 已经是 `false`（`internal/cli/console_api_serve.go:46` `envBoolTrue("")` returns false） → **代码无需改动**
  - 用户显式 opt-in：`docker run -e CONSOLE_API_FALLBACK_INMEM=1 ...`
  - 不显式 opt-in + gRPC core 不可达 → `/v1/health` 返 **503** + docker healthcheck 立即 unhealthy

**理由**：
- 让 deployment correctness 凌驾 out-of-box friendliness — v0.7.1 已 ship 一波 friendliness，v0.7.2 修正方向
- 用户需主动设 env 行 = 主动 ack「我知道 in-mem fallback 重启会丢数据」
- docker healthcheck unhealthy 是 ops 工具链最敏感的信号（k8s readiness probe / docker-compose `depends_on: service_healthy` 全级联）
- 现有 3 道 telemetry 不变；删 ENV 行后它们重新成为主信号

**B0 sub-decision 确认（2026-05-26）**：
- env 名**保留** `CONSOLE_API_FALLBACK_INMEM`（不改 `CONSOLE_API_ALLOW_INMEM`）—— Console 端 docker-compose.yml + .env.example 已 ack 此名；改名留 v0.8/v1.0 大版本一起做
- Dockerfile 操作选**删 ENV 行**（不反转值为 0）—— Dockerfile state 不覆盖 binary contract，最干净
- **不加 deprecate window**（仅默认值反转，无 env 名变更，无需兼容 path）

### D2 — Env semantics 不变 / dual-name 不引入

- `CONSOLE_API_FALLBACK_INMEM=1` / `0` 接受值不变（`envBoolTrue` 接受 `1`/`true`/`yes`/`on` case-insensitive → true；其它 → false）
- `--fallback-inmem` CLI flag 行为不变（覆盖 env）
- **不引入** `CONSOLE_API_ALLOW_INMEM` 别名 / dual-name compat / deprecate warning
  - 理由：v0.7.x 是 patch series，引入别名 + deprecate 路径 = 多一层 cognitive load + 后续 v0.8 又要清理；保持单一 env 名简洁
  - 改名留 v0.8 或 v1.0 大版本一次性处理（届时可一起改名为 `ALLOW_INMEM` 语义更明确，做 `FALLBACK_INMEM` deprecate window）

**理由**：minor patch (v0.7.x) 不背 dual-name 包袱；语义反转是单一改动；用户已熟悉 `FALLBACK_INMEM` 名。

### D3 — 实施最小 scope（仅 Dockerfile + docs）

- **删** `Dockerfile:46-52`（ENV 行 + 7 行注释）
- **更新** `Dockerfile:6-15` 头部 v0.7.1 vs v0.7.0 注释段加 v0.7.2 反转说明（或换成 v0.7.2 vs v0.7.1 段）
- **不改** `internal/cli/console_api_serve.go`（binary default 已是 false）
- **不改** `internal/consoleapi/`（router / handlers / memstore 全无关）
- **不改** test code（`TestBuildDeps_DegradedWhenNoDaemon` + `TestRouter_HealthDegraded_503` 已覆盖期望路径）
- **新增** docker container 实测 verify：v0.7.2 image 默认 `/v1/health` 返 503，显式 opt-in 后返 200

**理由**：最小变更面积 = 最小回归风险；现有 test suite 已锁定期望行为。

### D4 — Console standby 跨仓同步

- ContextForge-Console 主仓 docker-compose.yml `contextforge` service `environment` 段需加 `CONSOLE_API_FALLBACK_INMEM=1` 显式 opt-in（v0.7.2 ship 后切到 v0.7.2 image 时同步生效）
- Console 端 standby chore PR：依据 Console PR #91 中 §6.5 F1 standby action — "F1=(a) → docker-compose.yml + .env.example 各加一行"
- Cross-repo coordinate 路径：
  - **v0.7.2 ship 完成** → ContextForge 团队通过用户转发 GitHub Release link
  - **Console 主 Agent 启动 standby chore PR** → docker-compose.yml + .env.example 加 env + checklist §6.5 F1 标 ✅
  - **Console 端可同步切到 v0.7.2 image** 而不破坏自身 conformance suite

**理由**：跨仓 break change 必须双向 coordinate；Console 端已 standby ready，仅等 ContextForge 信号。

## Trade-offs / Conscious limitations

- **v0.7.1 用户 docker run 命令需加 `-e CONSOLE_API_FALLBACK_INMEM=1` 才能保留旧行为**——这是 BREAKING change，但仅影响 1 minor 版本 (v0.7.1 → v0.7.2) 内的 single-image 用户；evidence 通过 RELEASE_NOTES + README + v0.7.1 pre-announce 提前 1 patch 周期通告
- **不引入 `CONSOLE_API_ALLOW_INMEM` 显式名**（B0 decision）—— 牺牲短期"I know what I'm doing"语义清晰度；换长期 env 名稳定性 + 单一 patch scope
- **不加 startup-banner WARN**（B0 decision）—— 牺牲 (b) 方案的双重防御；理由是 (a) 已通过 503 healthcheck 触发 ops 链路告警，banner WARN 在 docker compose 多 container 时易被掩盖
- **现有 stderr WARN 文案保留**（`buildDeps` 中 `WARN console-api: gRPC ... Ping failed (...)`）—— degraded mode 的 log 信号不动

## Verification (v0.7.2 ship 时)

```bash
# 1. Code 无 diff（仅 Dockerfile + docs）
git diff v0.7.1..HEAD -- internal/ core/   # expect empty

# 2. Dockerfile 删 ENV 行验证
git diff v0.7.1..HEAD -- Dockerfile        # expect: -ENV CONSOLE_API_FALLBACK_INMEM=1

# 3. Test 不退化
cargo test -p contextforge-core            # expect: all PASS
go test ./...                              # expect: 43 packages PASS

# 4. Docker 实测 default deny
docker build -t contextforge-daemon:v0.7.2 .
docker run -d --name v072 -p 48181:48181 contextforge-daemon:v0.7.2
sleep 5
curl -o /dev/null -w '%{http_code}\n' localhost:48181/v1/health
# expect: 503

# 5. Docker 实测显式 opt-in 回退到旧行为
docker run -d --name v072-optin -e CONSOLE_API_FALLBACK_INMEM=1 -p 48182:48181 contextforge-daemon:v0.7.2
sleep 5
curl -o /dev/null -w '%{http_code}\n' localhost:48182/v1/health
# expect: 200 + status="degraded" + error_reason=...

# 6. healthcheck 链路验证
docker inspect v072 --format '{{.State.Health.Status}}'
# expect: unhealthy

docker inspect v072-optin --format '{{.State.Health.Status}}'
# expect: healthy
```

## Rollback path

如 v0.7.2 ship 后发现 (a) 方案不可接受（如 Console 端 standby PR 来不及同步、其它用户 ops 链路无法适配）：

1. `git revert <v0.7.2 commit>` 重新走 ENV CONSOLE_API_FALLBACK_INMEM=1 默认
2. ship v0.7.3 patch + ADR-018 status 改 "Reverted"
3. 重新 design：可能切到方案 (b) startup-banner WARN 双重防御 + 或者干脆等 v0.8 ship 2 进程 image 一起解决
4. 跨仓通知 Console 团队 v0.7.3 ship + standby PR 撤回

## Upgrade path (v0.7.1 → v0.7.2)

### 单 image docker run 用户

- 新行为：`docker run contextforge-daemon:v0.7.2` → `/v1/health` 返 503（unhealthy）
- 显式选择 1：保留 v0.7.1 行为 → `docker run -e CONSOLE_API_FALLBACK_INMEM=1 contextforge-daemon:v0.7.2`
- 显式选择 2：升级到真 multi-process 部署 → 另起 `contextforge-core` daemon + 配 `--grpc-addr` 指向 core

### docker-compose 用户

- ContextForge-Console: `docker-compose.yml` contextforge service `environment` 段加 `CONSOLE_API_FALLBACK_INMEM=1`（Console 端 standby chore PR ship 时同步）
- 自定义 compose: 同样 env 加在 contextforge service block

### 纯 binary 部署用户

- 不受影响（binary default 已经是 false；v0.7.1 / v0.7.2 行为一致）
- 仅 docker image 用户受 break change 影响

### k8s 用户

- Deployment manifest 的 env 段加 `CONSOLE_API_FALLBACK_INMEM=1` 即可保留 v0.7.1 行为
- readinessProbe 当前打 `/v1/health` 200 的：v0.7.2 不显式 opt-in 会一直 not-ready

## Amendment (2026-07-03 promote)

**Status 推进**: Proposed → Accepted。

**背景**：本 ADR header 原写「将于 v0.7.2 ship 时 promote 到 Accepted」，但项目实际走 v0.8 → v1.0.1 路径，v0.7.2 patch version 从未独立 ship，导致 promote 动作遗漏——header Status 滞留 Proposed。

**ratify 证据**（D1-D4 已实际交付且在用，非纸面决策）：

- **D1 (default deny)**: `Dockerfile` 中 `ENV CONSOLE_API_FALLBACK_INMEM=1` 行已被删除（git history 可见 `+ENV` → `-ENV` 转换）；`internal/cli/console_api_serve.go:46` binary default = `envBoolTrue(os.Getenv(...))` 即 unset 时返 false；不 opt-in + gRPC 不可达 → `/v1/health` 返 503。
- **D2 (env semantics 不变)**: `CONSOLE_API_FALLBACK_INMEM` 单一 env 名保留至今，无 dual-name / 别名引入。
- **D3 (最小 scope)**: 改动仅 Dockerfile + docs，binary/router/handlers/memstore 行为不变；`TestBuildDeps_DegradedWhenNoDaemon` + `TestRouter_HealthDegraded_503` 锁定期望路径。
- **D4 (Console standby)**: ContextForge-Console docker-compose 已加显式 opt-in env（跨仓 coordinate 完成）。

**下游依赖已生效**: Phase 16 production compose stack (`deploy/docker-compose.production.yml`) + ADR-022 (memory-is-pinned) 的部署假设均建立在 ADR-018 D1-D4 在 force 的基础上。本次 promote 是把已生效的决策补登为 Accepted，**不改变任何运行行为**。

**无 breaking change**: 本次仅状态推进 + 留痕，代码 / 配置 / 部署 / 升级路径零变更。
