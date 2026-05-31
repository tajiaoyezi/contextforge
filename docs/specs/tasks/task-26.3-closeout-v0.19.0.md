# Task `26.3`: `closeout-v0.19.0 — event-bus 分区 + 容量 + drain 超时配置（event-bus-partition / event-bus-capacity / events-drain-timeout-config）+ scripts/console_smoke.sh v16 events SSE/trace FTS/event-bus 配置 smoke + v0.19.0 release docs（README/RELEASE_NOTES/evidence/artifacts）+ ADR-031 据真实结果 ratify + ADR-021/015 add-only Amendment + phase-26 §6 闭合 + adapter`

**Status**: Draft

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 26 (observability-hardening)
**Dependencies**: task-26.1（trace FTS5 + VACUUM）/ task-26.2（events SSE 推送 + 从 audit log 重放）/ task-19.7（closeout 模板 + tag/backfill pattern）/ task-23.3（向量 phase closeout pattern）/ ADR-031（observability-hardening，本 phase 新 Proposed）/ ADR-021（memory-event-bus-bridge — `EventBus` `with_capacity` seam + Rollback path 容量 / 分区预见 `adr-021:153`）/ ADR-015（console-contract add-only）/ ADR-004（local-first 0 新 dep / 0 network）/ ADR-013（禁伪造）/ ADR-014 D1-D5（第十七次激活）

## 1. Background

task-26.1 已让 `SqliteTracePersist` 支持 FTS5 按内容检索 + 周期 VACUUM；task-26.2 已加 events SSE 实时推送 + 从 audit log 重放。本 task 收口 Phase 26：(1) 兑现 ADR-021 Rollback path 预见的 **event-bus 配置**——`event-bus-capacity`（替换硬编码 `broadcast::channel(1000)`，复用 `core/src/data_plane/events.rs:35` `EventBus::with_capacity` seam）+ `event-bus-partition`（`memory.*` / `indexing.*` 分区可选，缓解 ADR-021 D4 预见的「memory 高频挤占 indexing」丢事件）+ `events-drain-timeout-config`（grpcclient phase-2 `~100ms` drainTimeout 可配），带保守默认使既有行为默认不变；(2) 把 smoke 升 v16，加 events SSE / trace FTS / event-bus 配置相关断言；(3) 产出 v0.19.0 release docs；(4) 据真实非合成结果 ratify ADR-031 + ADR-021/015 add-only Amendment 记录推进结果（不溯改正文，ADR-014 D5）；(5) 闭合 phase-26 §6 AC；(6) 更新 s2v-adapter。

承 v0.12.0 / v0.16.0 收口模式：closeout = smoke final + release evidence/artifacts + README/RELEASE_NOTES + ADR 状态 + adapter；tag push 经用户授权 / 主 agent 自治（ADR-012）触发 release.yml + post-tag-push backfill。

## 2. Goal

`core/src/data_plane/events.rs` 的 `EventBus` + consoleapi 加配置：`event-bus-capacity`（替换硬编码 `broadcast::channel(1000)`，复用 `EventBus::with_capacity` seam）+ `event-bus-partition`（`memory.*` / `indexing.*` 分区可选）+ `events-drain-timeout-config`（grpcclient phase-2 drainTimeout 可配），保守默认使既有行为默认不变（容量默认 1000 / 不分区 / drain 默认 ~100ms）+ deterministic 单测可断言配置生效 + 默认等价。`scripts/console_smoke.sh` 升 v16：既有 step 不退化 + 新增 events SSE 帧 / trace FTS / event-bus 配置 smoke 断言（合规环境跑 SSE / FTS，或如实标注受阻态）。新增 `docs/releases/v0.19.0-{evidence,artifacts}.md` + `README.md` v0.19 段 + `RELEASE_NOTES.md` v0.19.0 段。`docs/decisions/adr-031-observability-hardening.md` 据 task-26.1/26.2 真实结果 Status `Proposed → Accepted`（或记录维持）+ ADR-021/015 add-only Amendment。`docs/specs/phases/phase-26-*.md` §6 AC1-5 全 `[x]` + Status `Draft → Done`。`docs/s2v-adapter.md` Phase 26 `Draft → Done` + Tasks `0 → 3` + ADR-031 索引 + ADR-021 预留兑现记录。ADR-014 D1-D5 第十七次激活 closeout PR body。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **修改 `core/src/data_plane/events.rs`（event-bus 配置）**：`event-bus-capacity`——把 `EventBus::new()` 硬编码 `broadcast::channel(1000)`（`events.rs:31`）改为读配置容量（复用既有 `with_capacity` seam `events.rs:35`），默认仍 1000；`event-bus-partition`——可选按命名空间（`memory.*` / `indexing.*`）分独立 broadcast channel，默认不分区（单 channel，既有行为）；deterministic 单测断言配置生效（容量 / 分区）+ 默认等价（默认配置 == 既有 `broadcast::channel(1000)` 单 channel 行为）。
- **修改 `internal/consoleapi/grpcclient/grpcclient.go`（events-drain-timeout-config）**：phase-2 硬编码 `~100ms` `eventsDrainTimeout` 提为可配（保守默认仍 ~100ms），既有 `Recent(limit, wait)` 两阶段语义默认不变。
- **修改 `scripts/console_smoke.sh`**：v16 注释段 + 新增 events SSE / trace FTS / event-bus 配置 smoke 断言（合规环境跑 SSE 帧 / FTS 检索，或据 task-26.2 结论如实标注 SSE live-server 受阻态）；既有 step 标号 / 断言不动语义（step 25 events long-poll / step 26 TraceStore roundtrip 不退化）；终态 marker 保留。
- **新增 `docs/releases/v0.19.0-evidence.md` + `docs/releases/v0.19.0-artifacts.md`**：承 v0.12.0/v0.16.0 模板（合入记录 / S2V 状态 / 验证证据 / CI / AC 达成 / 平台矩阵 / upgrade-rollback / ADR-014 record / tag+镜像 SHA backfill 段）。
- **修改 `README.md`**：v0.19 段——trace FTS 检索 + 周期 VACUUM + events SSE 实时推送 + 从 audit log 重放 + event-bus 配置如实记录。
- **修改 `RELEASE_NOTES.md`**：v0.19.0 段（task 表 + trace FTS/VACUUM + events SSE/重放 + event-bus 配置结论 + upgrade/rollback）。
- **修改 `docs/decisions/adr-031-observability-hardening.md`**：据 task-26.1/26.2 真实结果 Status `Proposed → Accepted`（或受阻维度记录维持）；ADR-021 预留兑现（events-replay + event-bus 配置）+ ADR-015 SSE add-only 以 add-only Amendment 记录（不溯改 ADR-021/015 正文，ADR-014 D5）；ADR-031 §Ratification 段回填真实依据。
- **修改 `docs/specs/phases/phase-26-observability-hardening.md`**：§6 AC1-5 全 `[x]` + Status `Draft → Done` + §8 DoD 勾选。
- **修改 `docs/s2v-adapter.md`**：Phase 26 行 `Draft → Done` + `Tasks 0 → 3` + Task 索引 26.1-26.3 Done + ADR-031 索引行 + BDD phase-26 feature 行 + ADR-021 预留兑现注。
- **新增 `test/features/phase-26-observability-hardening.feature`**（≥3 scenario）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **trace FTS / VACUUM 实现** [SPEC-OWNER:task-26.1-tracestore-fts-and-vacuum]：本 task 在 smoke / release docs 引用它，不实现。
- **events SSE / 重放实现** [SPEC-OWNER:task-26.2-events-sse-push-and-replay]：本 task 引用其能力，不重做。
- **重放扩展到 `indexing.*` 类事件**（需 indexing 持久化源）[SPEC-DEFER:phase-future.indexing-event-persistence]：本 task event-bus 配置不引入 indexing 持久化。
- **跨进程 / 多节点事件广播（Kafka/NATS 类替换属 ADR-004 local-first 红线外）** [SPEC-DEFER:phase-future.distributed-event-bus]：event-bus 分区限单进程 `memory.*` / `indexing.*` 粗粒度。
- **SSE 多客户端 fan-out 背压调优** [SPEC-DEFER:phase-future.sse-backpressure-tuning]：本 task 不调 SSE 背压。
- **v0.19.0 tag push 实际执行**：closeout PR 合入后据用户明确授权 / 主 agent 自治（ADR-012）push `v0.19.0` annotated tag 触发 release.yml（沿用历史 release 流）。post-tag-push backfill 填实 tag SHA / run ID / 镜像 digest 由独立 backfill PR 承接（仿 v0.12.0/v0.16.0 pattern）。
- **multi-arch 镜像 / 签名 / SBOM** [SPEC-DEFER:phase-future.multi-arch-image] / [SPEC-DEFER:phase-future.image-signing-and-sbom]：发布硬化项，独立推进。

## 4. Actors

- **主 agent**：实施 + PR 主理 + closeout 决策（event-bus 配置默认值 + ADR-031 ratify vs 受阻维度记录）。
- **`core/src/data_plane/events.rs::EventBus`**：本 task 加容量 / 分区配置（复用 `with_capacity` seam）。
- **`internal/consoleapi/grpcclient/grpcclient.go`**：本 task 加 drain 超时配置。
- **`scripts/console_smoke.sh`**：端到端 C1 兜底 smoke，本 task 升 v16。
- **`docs/releases/` + `README.md` + `RELEASE_NOTES.md`**：v0.19.0 release 文档面。
- **`docs/decisions/adr-031-*.md`**：本 phase 新 ADR，本 task ratify；ADR-021/015 add-only Amendment。
- **`docs/s2v-adapter.md`**：Phase/task/ADR/BDD 索引。
- **用户**：v0.19.0 tag push 授权 / 主 agent 自治决断（ADR-012）。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/tasks/task-19.7-closeout-v0.12.0.md`（closeout 模板 + tag/backfill pattern）+ `docs/specs/tasks/task-23.3-closeout-v0.16.0.md`（phase closeout pattern + ratify 口径）
- `docs/releases/v0.16.0-evidence.md` + `docs/releases/v0.16.0-artifacts.md`（release 文档结构 + 平台矩阵 + backfill 段）
- `scripts/console_smoke.sh`（既有 step + 终态 marker；`:497` step 25 events long-poll + `:528` step 26 TraceStore roundtrip — 本 task v16 旁挂硬化断言不退化既有）
- `docs/specs/tasks/task-26.1-tracestore-fts-and-vacuum.md` + `task-26.2-events-sse-push-and-replay.md`（本 phase 上游交付）
- `docs/decisions/adr-031-observability-hardening.md`（本 phase ADR D1-D6）+ `docs/decisions/adr-021-memory-event-bus-bridge.md`（D4 best-effort + Rollback path `:153` 容量 / 分区预见 + Trade-off `:115` 重放预留）+ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）
- `core/src/data_plane/events.rs`（`EventBus::new` `:31` 硬编码 1000 + `with_capacity` `:35` seam + `subscribe` + `EventsServer.Subscribe`）
- `internal/consoleapi/grpcclient/grpcclient.go`（`eventsClient.Recent` 两阶段 + phase-2 drainTimeout）
- `docs/s2v-adapter.md` §Phase / §Task / §ADR / §BDD 索引

### 5.2 关键设计 — event-bus 配置 + smoke v16 + ADR ratify

- **event-bus 配置**：`event-bus-capacity`——`EventBus::new` 读配置容量（复用 `with_capacity` seam `events.rs:35`），默认 1000（既有 `broadcast::channel(1000)` 等价）；`event-bus-partition`——可选 `memory.*` / `indexing.*` 分独立 channel（缓解 ADR-021 D4 预见的 memory 挤占 indexing），默认不分区单 channel；`events-drain-timeout-config`——grpcclient phase-2 `~100ms` drainTimeout 可配，默认 ~100ms。deterministic 单测断言配置生效（容量值 / 分区路由）+ 默认等价（默认配置行为 == 既有）。
- **smoke v16**：新增 events SSE / trace FTS / event-bus 配置 smoke——合规环境跑 SSE 帧断言（`text/event-stream` 帧格式）+ trace FTS 检索（`search_fts` 命中）+ event-bus 配置默认等价 note；SSE 真实起服据 task-26.2 结论如实标（合规环境跑则加 SSE 帧 smoke note，受阻则记录 stop-condition note，不伪造 live 通过）。既有 step 25/26 断言不动；终态 marker 保留。
- **ADR-031 ratify（ADR-013）**：据 task-26.1 真实 FTS 往返 + VACUUM 回收 + task-26.2 真实 SSE 帧契约 + 重放顺序契约 Proposed→Accepted；若某维度受阻（如 SSE live-server 端到端未在合规环境验证）则 ADR-031 据「已达维度 ratify + 受阻维度如实记录」处理，不据合成 / 伪造 ratify。
- **ADR-021/015 add-only Amendment**：推进结果（events 重放兑现 `[SPEC-DEFER:phase-future.events-replay-from-audit]` / event-bus 容量 / 分区兑现 Rollback path 预见）以 add-only Amendment 记录在 ADR-021，不溯改 D1-D4 正文（ADR-014 D5）；SSE endpoint add-only 记录在 ADR-015 思想下（不溯改其正文）。

### 5.3 不变量

- smoke 既有 step（含 step 25 events long-poll / step 26 TraceStore roundtrip）不退化（仅新增 SSE / FTS / event-bus 配置 step + v16 注释）。
- release docs 诚实口径（承 task-19.7 / task-23.3 §10）：deterministic 默认 / 合规环境 SSE / 受阻三态如实标；SSE live-server 据 task-26.2 真实结论记录，不伪造。
- ADR-031 ratify 仅在 task-26.1/26.2 真实落地后（ADR-013：据真实非合成）；受阻维度不强 ratify。
- event-bus 配置保守默认使既有行为默认不变（容量 1000 / 不分区 / drain ~100ms）；既有 22-endpoint + long-poll endpoint + `Recent` 签名不退化（ADR-015 add-only）。
- 默认构建 0 新依赖 / 0 network（ADR-004）。

## 6. Acceptance Criteria

- [ ] **AC1**: event-bus 配置完成 — `event-bus-capacity`（复用 `with_capacity` seam，默认 1000）+ `event-bus-partition`（`memory.*` / `indexing.*` 可选，默认不分区）+ `events-drain-timeout-config`（默认 ~100ms）+ deterministic 单测断言配置生效 + 默认等价；`scripts/console_smoke.sh` v16 通过 `bash -n`（exit 0）+ events SSE / trace FTS / event-bus 配置 smoke 断言 + 既有 step（25/26）不退化 — verified by **TEST-26.3.1**
- [ ] **AC2**: v0.19.0 release docs 齐备（`docs/releases/v0.19.0-{evidence,artifacts}.md` + `README.md` v0.19 段 + `RELEASE_NOTES.md` v0.19.0 段）；evidence 含 task 表 / CI / AC 达成 / 平台矩阵 / upgrade-rollback / §tag-backfill 待回填段 — verified by **TEST-26.3.2**
- [ ] **AC3**: ADR-031 据 task-26.1/26.2 真实结果 Status `Proposed → Accepted`（或受阻维度记录维持）+ §Ratification 回填真实依据；ADR-021/015 add-only Amendment 记推进结果（events-replay 兑现 + event-bus 配置兑现 Rollback path 预见 + SSE add-only，不溯改正文）；phase-26 §6 AC1-5 全 `[x]` + Status `Draft → Done`；adapter Phase 26 `Draft → Done` + Tasks `0 → 3` + ADR-031 索引 + ADR-021 预留兑现注 — verified by **TEST-26.3.3**
- [ ] **AC4**: 既有不退化 — 默认 `cargo test --workspace` + `go test ./...` 全 PASS（trace FTS / VACUUM / SSE 帧 / 重放 / event-bus 配置单测）；0 新依赖 / 0 network — verified by **TEST-26.3.4** + §10
- [ ] **AC5**: ADR-014 D1-D5 第十七次激活全通过（D1 phase§6↔task§6 mapping 表 + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-25 不溯改）— verified by **TEST-26.3.5** + 本 closeout PR body

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-26.3.1 | event-bus 配置（容量 / 分区 / drain）单测 + 默认等价 + smoke v16 `bash -n` + SSE/FTS/event-bus 断言 | `core/src/data_plane/events.rs`（`mod tests`）+ `internal/cli/smoke_syntax_test.go` + `scripts/console_smoke.sh` | Planned |
| TEST-26.3.2 | v0.19.0 release docs 齐备 + 结构校验 | `docs/releases/v0.19.0-*.md` + README + RELEASE_NOTES | Planned |
| TEST-26.3.3 | ADR-031 ratify + ADR-021/015 Amendment + phase-26 闭合 + adapter | `docs/decisions/adr-031-*.md` + phase-26 spec + s2v-adapter | Planned |
| TEST-26.3.4 | 默认 `cargo test --workspace` + `go test ./...` + 0 新依赖 0 failed | 全 Rust + Go | Planned |
| TEST-26.3.5 | ADR-014 D1-D5 record（mapping + D2 lint） | 本 closeout PR body | Planned |

## 8. Risks

- **R1（中）event-bus 分区分得过细反增复杂度**（承 phase-26 §7 R4）：partition channel 过多 → 订阅 fan-in 复杂。
  - **缓解**：默认不分区（保守默认，既有行为不变）；分区仅 opt-in 且限 `memory.*` / `indexing.*` 两命名空间粗粒度；deterministic 单测断言默认等价 + 配置生效。AC1 以「配置生效 + 默认等价」满足。
- **R2（中）ADR-031 某维度依赖受阻**（SSE live-server 端到端未在合规环境验证）：ratify 须真实结果。
  - **缓解**：ADR-031 据「trace FTS/VACUUM 已达 + SSE 帧契约/重放顺序 deterministic 已达 + SSE live-server 据 task-26.2 真实结论」处理——已达维度 ratify，受阻维度如实记录（ADR-013），不据合成 ratify。
- **R3（低）v0.19.0 tag 误在授权前 push**：release stop-condition。
  - **缓解**：closeout PR 仅备齐 release docs；tag push 经用户明确授权 / 主 agent 自治决断（ADR-012）后单独执行（沿用历史 release 流）。
- **R4（低）smoke v16 SSE 断言在 CI 默认不可跑**（需 live server）：默认 CI 无 daemon。
  - **缓解**：SSE / FTS smoke 在合规环境 / REAL mode 跑（承 step 26 `REAL mode only` pattern），默认 CI 跑 `bash -n` 语法门 + 既有 step 不退化；如实标 live 依赖（ADR-013），受阻则 stop-condition note 不伪造 live 通过。

## 9. Verification Plan

```bash
# smoke v16 语法 + step 标号
bash -n scripts/console_smoke.sh
go test ./internal/cli/... -run 'TestTask26|TestSmoke' -v

# 既有不退化（22-endpoint + long-poll + trace 持久 + 重放查询面）
go test ./...
cargo test --workspace

# event-bus 配置 + trace FTS + 重放查询面单测
cargo test -p contextforge-core data_plane::events
cargo test -p contextforge-core data_plane::search_persist

# 端到端 smoke（合规环境 / REAL mode；SSE 帧 + FTS 检索）
bash scripts/console_smoke.sh        # 期望末行 CONSOLE_REAL_SMOKE_EXIT=0

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按 6 项回填：完成日期 / 改动文件（含 event-bus 配置 + smoke v16 + release docs + ADR-031 ratify + ADR-021/015 Amendment + phase/adapter/feature）/ commit 列表（配置 + smoke v16 + syntax test → docs）/ §9 Verification 实测结果（ADR-013 真实非合成；含上游 task-26.1/26.2 真实凭据引用 + SSE live-server 若受阻如实标 stop-condition）/ 设计取舍（event-bus 配置默认值 + ADR-031 ratify 维度 + Amendment 口径）/ 剩余风险 + 下游影响（indexing 事件重放 / SSE 背压 / 分布式 event-bus 各 `[SPEC-DEFER:phase-future.*]` + tag push 授权 / 自治 + tag/release backfill）。
