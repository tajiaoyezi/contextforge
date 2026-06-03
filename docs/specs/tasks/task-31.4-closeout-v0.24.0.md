# Task `31.4`: `closeout-v0.24.0 — smoke v21 step [40/40] + v0.24.0 release docs + ADR-036 据 D1-D5 真实 ratify（D2 compose TLS 真实 cert / D4 native-runner / attestation 受阻维度如实）+ add-only Amendments（ADR-021/027/029/033）+ roadmap §4 event-bus 更正 + phase-31 §6 闭合`

**Status**: Done

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 31 (governance-debt-cleanup)
**Dependencies**: task-31.1（observability + memstore-event-emit Go parity + event-bus partition/capacity verify-only）/ task-31.2（embedding-cache LRU + memstore cache cap 可配置 + compose 资源限/可选 TLS proxy）/ task-31.3（eval case-results 子表 + exporter 全文 + 3 MCP nits + 诚实 defer 重申）全 Done / ADR-036（governance-debt-cleanup，本 task ratify）/ ADR-021（event-bus / memory-bus-bridge，本 task add-only Amendment）/ ADR-027（embedding-provider，本 task add-only Amendment）/ ADR-029（eval，本 task add-only Amendment）/ ADR-033（release，本 task add-only Amendment）/ ADR-004（默认行为 / 既有契约不变）/ ADR-012（tag/release outward-facing 须用户显式授权）/ ADR-013（禁伪造凭据红线）/ ADR-014 D1-D5（第二十二次激活）

## 1. Background

Phase 31 三个实现 task 全 Done：31.1（Go fallback memstore 内存操作 emit `memory.*` 事件与 workspace/job + Rust 路径对齐；event-bus partition/capacity 经核 Phase 26 已交付 → verify-only + roadmap §4 add-only 更正）/ 31.2（embedding-cache L1 LRU/cap 上界 + memstore cache cap 经 config/env 可配置 + 生产 compose 资源限 + 可选 TLS-terminating 反代）/ 31.3（eval per-case 结果提升为可查询子表 `eval_case_results` + add-only migration 0018 + exporter 经新 `ListAllChunks` RPC 取全文 + 3 MCP nits 修 + rust-native-eval-runner / multi-arch-native-runner / github-native-attestation 诚实重申延后）。本 task 收口 v0.24.0：smoke v21 + release docs + ADR-036 据真实结果 ratify + add-only Amendments（ADR-021/027/029/033）+ roadmap §4 event-bus 更正 + phase §6 闭合 + adapter + feature。

## 2. Goal

据 31.1/31.2/31.3 **真实 CI / 实测产物**收口 v0.24.0：ADR-036 `Proposed → Accepted`（逐 D 项如实——D1 memstore-event Go parity 达成 + event-bus partition/capacity verify-only 更正、D2 cache+deploy 硬化中 compose-config parse 🟢 达成 / 真实 TLS cert 须域名 🟡 待实测回填、D3 eval 子表+exporter 全文+MCP nits 达成、D4 honest defer 重申、D5 baseline 不变）；ADR-021/027/029/033 add-only Amendment（治理面扩展，不溯改正文 D5）；roadmap §4 add-only 更正（剔除 event-bus-partition/capacity——经核 Phase 26 已交付）；phase-31 §6 AC 置 `[x]` + Status Done；smoke v21 step `[40/40]`（治理债清理状态，default build baseline intact）；release docs（evidence/artifacts/README/RELEASE_NOTES，tag/run/digest 用 backfill 待回填）；adapter（Phase 31 Done + Tasks 4 + ADR-036 Accepted + feature 行）。**真实 v0.24.0 tag/release 须用户显式授权**（不自行 tag，ADR-012）。

## 3. Scope

### In Scope（计划交付）

- `scripts/console_smoke.sh`——banner v20→v21 + v21 changelog 块 + step `[40/40]`（治理债清理状态 + default build init baseline 不变；既有 step 不退化 + denominator 不溯改 ADR-014 D5）。当前 live 脚本为 `[37/37]`；Phase 29 计划 `[38/38]`；Phase 30 计划 `[39/39]`；故 Phase 31 顺接 `[40/40]`。step 为文档/状态步：断言 default-build init baseline + 有运行时面的治理债修复（如 memstore-event-emit parity 若可达，否则文档/状态）。
- `internal/cli/smoke_syntax_test.go`——新增 `TestTask314_SmokeV21GovernanceDebtCleanupStep`（断言 `[40/40]` + 标记 + 无回归既有 `[33/33]`..`[39/39]`，denominator 不溯改）。
- 新增 `docs/releases/v0.24.0-{evidence,artifacts}.md`（tag SHA / run id / digest 用 `<backfill>` 待回填）+ `README.md` v0.24 段 + `RELEASE_NOTES.md` v0.24.0 段。
- `docs/decisions/adr-036-governance-debt-cleanup.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification（v0.24.0 / task-31.4）` 节（逐 D 真实依据；TLS 真实 cert / native arm64 runner / github-native-attestation 受阻维度据已达维度 ratify + 如实记录）。
- add-only Amendments（不溯改正文，ADR-014 D5）：`docs/decisions/adr-021-*.md`（`## Amendment (Phase 31 / v0.24.0)` — memstore-event-emit Go parity + event-bus partition/capacity 经核 Phase 26 已交付的更正记录）；`docs/decisions/adr-027-*.md`（cache LRU）；`docs/decisions/adr-029-*.md`（case-results 子表）；`docs/decisions/adr-033-*.md`（multi-arch-native-runner / github-native-attestation defer 重申）。
- `docs/roadmap.md` §4 add-only 更正注记——剔除 event-bus-partition/capacity（经核 Phase 26 已交付，§4 line 230/236 旧条目更正）。
- `docs/specs/phases/phase-31-governance-debt-cleanup.md`——Status Draft→Done + §6 AC `[x]`（honest per-dim：TLS 真实 cert / native runner / attestation 维度如实标注）。
- `docs/s2v-adapter.md`——§Phase 31 In Progress→Done + Tasks 3→4；§Task +31.4；§ADR 036 Proposed→Accepted；§BDD +phase-31 行。
- `test/features/phase-31-governance-debt-cleanup.feature`（已创建）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 真实 v0.24.0 tag push + release run（cosign 真签 + GHCR 推送）[SPEC-OWNER:user-authorized-release]——outward-facing 不可逆须用户授权（ADR-012）；post-tag-push backfill 填实 tag SHA / run id / digest。
- compose 真实 TLS cert 自动签发（须域名）[SPEC-DEFER:phase-future.compose-tls-auto-cert]——compose-config parse 🟢 已达，真实证书签发 🟡 须域名环境。
- rust-native-eval-runner [SPEC-DEFER:phase-future.rust-native-eval-runner] / multi-arch-native-runner [SPEC-DEFER:phase-future.multi-arch-native-runner] / github-native-attestation [SPEC-DEFER:phase-future.github-native-attestation]——据 31.3 §3 范围外诚实重申，受阻/无驱动维度不伪造完成。

## 4. Actors

- 主 agent（ADR-012 自治；真实 release 须用户授权）
- closeout 文档集（smoke / release docs / ADR-036 ratify / add-only Amendments / roadmap 更正 / phase spec / adapter / feature）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-31-governance-debt-cleanup.md §6/§8`（AC + DoD）
- `docs/decisions/adr-036-governance-debt-cleanup.md`（§D1-D5 + Consequences Ratification 条款）
- `docs/specs/tasks/task-31.1-observability-memstore-event-parity.md §10` + `task-31.2-cache-and-deploy-hardening.md §10` + `task-31.3-eval-exporter-and-mcp-nits.md §10`（真实测试结果 + 结论）
- `internal/consoleapi/memstore.go:100-115`（`emitEvent` helper）+ `:590-657`（`MemMemoryStore` Pin/Deprecate/SoftDelete/Unpin/HardDelete——31.1 parity 锚点）
- `docs/roadmap.md §4`（event-bus-partition/capacity 旧 backlog 条目，line 230/236——本 task add-only 更正锚点）
- `docs/releases/v0.21.0-{evidence,artifacts}.md`（模板）

### 5.2 关键设计 — 诚实 per-D ratify + backfill 待回填

- ADR-036 ratify **逐 D 项据真实结果**：D1（memstore-event Go parity + event-bus verify-only 更正）/ D2（cache+deploy 硬化——embedding-cache LRU + memstore cap 可配置 + compose 资源限 + 可选 TLS proxy 中 compose-config parse 🟢 达成；真实 TLS cert 须域名 🟡 待实测回填）/ D3（eval 子表 + exporter 全文 + MCP nits）/ D4（honest defer 重申）/ D5（baseline 不变）。各 D 的真实测试 / 实测结果待 31.1-31.3 实施后跑出再回填，不为「全 Accepted」伪造真实 TLS cert 已签发或 native arm64 runner 已就绪（ADR-013）。
- tag SHA / release run id / 镜像 digest 在 release docs 用 `<backfill: ...>` 待回填——真实 v0.24.0 tag/release 是 closeout 合入后的**用户授权步**，post-tag-push backfill PR 填实（承 v0.8–v0.23 pattern）。
- roadmap §4 event-bus 更正为 **add-only 注记**（不删旧条目正文、加更正脚注：经核 Phase 26 / ADR-031 D5 已交付 `events.rs from_config` + `server.rs:602-603` + TEST-26.3.1a/b/c），如实记录已交付、不重复实现（ADR-013）。
- smoke step `[40/40]` 为文档/状态步：验 default build init baseline 不变（ADR-004）+ 文档化三 task 状态；memstore-event-emit parity 若运行时可达则附加断言其事件可达，否则退为文档/状态。

### 5.3 不变量

- 0 行为变更 / 0 新依赖（closeout 纯文档 + smoke step；smoke 既有 step + denominator 不溯改 D5）。
- ADR-014 D5：历史 Phase 1-30 spec 不溯改；ADR-021/027/029/033 add-only Amendment 不改正文；roadmap §4 更正为 add-only 注记不删旧条目正文。
- 真实 tag/release 不自行触发（ADR-012）。

## 6. Acceptance Criteria

- [x] AC1（smoke v21 step）: smoke banner v20→v21 + step `[40/40]`（治理债清理状态 + default build baseline intact）+ `TestTask314_SmokeV21GovernanceDebtCleanupStep`（含无回归既有 `[36/36]`..`[39/39]`，denominator 不溯改）— verified by TEST-31.4.1（`bash -n` exit 0 + `go test -run TestTask314` PASS）
- [x] AC2（closeout 文档闭合）: v0.24.0 release docs（`v0.24.0-{evidence,artifacts}.md` `<backfill>` 待回填 + README v0.24 段 + RELEASE_NOTES v0.24.0 段）+ ADR-036 per-D ratify `Proposed→Accepted`（D1/D3/D5 Accepted；D2 真实 cert + D4 native-runner/attestation honest-defer PARTIAL）+ add-only Amendments（ADR-021/027/029/033）+ roadmap §4 event-bus add-only 更正（规划 PR #196 已落地）+ phase-31 §6 AC1-5 `[x]` + Status Done + adapter 闭合（Phase 31 Done/Tasks 4/ADR-036 Accepted）+ feature — verified by TEST-31.4.2
- [x] AC3（ADR-014 D2 lint）: bash scripts/spec_drift_lint.sh --touched origin/master PR 触及行 0 未标注命中 — verified by TEST-31.4.3

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-31.4.1 | smoke v21 step `[40/40]`（治理债清理标记 + 无回归既有 denominator）+ `bash -n` 过 + `go test -run TestTask314` 过 | `scripts/console_smoke.sh` + `internal/cli/smoke_syntax_test.go` | Done (PASS) |
| TEST-31.4.2 | release docs + ADR-036 per-D ratify Accepted（D2/D4 honest-defer 如实）+ add-only Amendments（ADR-021/027/029/033）+ roadmap §4 event-bus 更正 + phase-31 §6 闭合 + adapter + feature | release/ADR/roadmap/phase/adapter/feature | Done |
| TEST-31.4.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Done (PASS) |

## 8. Risks

- **R1（低）closeout 误报 TLS 真实 cert / native arm64 runner / attestation 为已达成**：诚实风险。
  - **缓解**：ADR-036 ratify + release docs + smoke + phase §6 全逐维如实——compose-config parse 🟢 达成、真实 TLS cert 🟡 须域名待实测回填、native runner / github-native-attestation 受阻维度据 31.3 范围外重申延后；不伪造（ADR-013）。stop-condition：任何「真实 cert 已签发」/「arm64 原生已就绪」表述须有真实凭据，否则标受阻维度 / backfill。
- **R2（低）smoke denominator 误溯改**：新 step 须 `[40/40]`，既有 `[33/33]`..`[39/39]` 不动。
  - **缓解**：`TestTask314` 无回归断言守护；ADR-014 D5。
- **R3（低）roadmap §4 event-bus 更正误删旧条目正文**：须 add-only 注记不删正文（D5）。
  - **缓解**：仅追加更正脚注（经核 Phase 26 已交付 + 真实锚点 `events.rs from_config` / `server.rs:602-603` / TEST-26.3.1a/b/c）；不改既有 backlog 行正文。

## 9. Verification Plan

```bash
# AC1 — smoke 语法 + syntax test
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask314

# AC2 — 文档闭合人工核（ADR-036 Accepted + per-D / add-only Amendments ADR-021·027·029·033 /
#        roadmap §4 event-bus 更正 / phase §6 [x] / adapter Done / feature 存在）
# AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# 既有不退化（closeout 文档+smoke 不影响 workspace）
cargo test --workspace && go test ./...
```

> **outward-facing 红线**：真实 v0.24.0 tag push + release run（cosign 真签 + GHCR 推送）是 closeout 合入后的**用户授权步**（ADR-012）；本 task 不自行 tag，release docs 的 tag/run/digest 用 `<backfill>` 待回填待 post-tag-push backfill 填实。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification 实测证据**：`bash -n scripts/console_smoke.sh` exit 0；`go test ./internal/cli/ -run TestTask314` PASS（`[40/40]` + governance-debt-cleanup + TEST-31.1/31.2/31.3 + memstore-event + ListAllChunks 标记 + 无回归 `[36/36]`..`[39/39]`）；`cargo test --workspace` 0 failed + `go test ./...` 不退化；spec-lint `--touched origin/master` 0 未标注命中。ADR-036 据 31.1/31.2/31.3 真实产物 per-D ratify Accepted（D2 真实 TLS cert + D4 native-runner/attestation honest-defer PARTIAL，不伪造 ADR-013）；ADR-021/027/029/033 add-only Phase 31 Amendment 落地；roadmap §4 event-bus 更正规划 PR #196 已落地（本 task 不重复）；release docs tag/run/digest 待用户授权 tag 后 post-tag-push backfill 填实。

**计划改动文件**：
- `scripts/console_smoke.sh`——banner v20→v21 + v21 changelog 块 + step `[40/40]`（治理债清理状态 + default build init baseline 不变）。
- `internal/cli/smoke_syntax_test.go`——`TestTask314_SmokeV21GovernanceDebtCleanupStep`（断言 `[40/40]` + 标记 + 无回归既有 `[33/33]`..`[39/39]`，denominator 不溯改）。
- `docs/releases/v0.24.0-{evidence,artifacts}.md`（新，tag/run/digest `<backfill>` 待回填）+ `README.md` v0.24 段 + `RELEASE_NOTES.md` v0.24.0 段。
- `docs/decisions/adr-036-governance-debt-cleanup.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification（v0.24.0 / task-31.4）` 节。
- add-only Amendments：`docs/decisions/adr-021-*.md`（memstore-event-emit Go parity + event-bus partition/capacity 经核 Phase 26 已交付的更正记录）+ `docs/decisions/adr-027-*.md`（cache LRU）+ `docs/decisions/adr-029-*.md`（case-results 子表）+ `docs/decisions/adr-033-*.md`（multi-arch-native-runner / github-native-attestation defer 重申）。
- `docs/roadmap.md` §4 add-only event-bus 更正注记（剔除 event-bus-partition/capacity——经核 Phase 26 已交付）。
- `docs/specs/phases/phase-31-governance-debt-cleanup.md`——Status Draft→Done + §6 AC `[x]`（honest per-dim）。
- `docs/s2v-adapter.md`——Phase 31 Done + Tasks 4 + ADR-036 Accepted + BDD 行。
- `test/features/phase-31-governance-debt-cleanup.feature`（已创建）。

**§9 Verification 计划** (will record real evidence at impl)：
- `bash -n scripts/console_smoke.sh` + `go test ./internal/cli/ -run TestTask314`（smoke 语法 + syntax test）——真实跑出后回填。
- `cargo test --workspace` + `go test ./...`（既有不退化）——真实跑出后回填。
- `bash scripts/spec_drift_lint.sh --touched origin/master`（D2 lint，CI spec-lint 权威）——真实跑出后回填。
- ADR-036 ratify 逐 D 据 31.1-31.3 真实测试 / 实测结果——待实测回填；TLS 真实 cert / native arm64 runner / github-native-attestation 受阻维度据已达维度 ratify + 如实记录，不强 ratify（ADR-013）。
- 真实 v0.24.0 tag/release（cosign 真签 + GHCR 推送）待用户授权 → post-tag-push backfill 填实 evidence/artifacts 待回填。
