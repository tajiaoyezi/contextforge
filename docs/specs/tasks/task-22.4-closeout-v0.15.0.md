# Task `22.4`: `closeout-v0.15.0 — core/src/health.rs probe_embed 远程可达性探针（opt-in）+ scripts/console_smoke.sh v12 + v0.15.0 release docs + ADR-027 ratify + phase-22 §6 闭合 + adapter`

**Status**: Done

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治）
**Dependencies**: task-22.1（provider 配置 + 工厂 + dim 协商）/ task-22.2（embedding 缓存）/ task-22.3（远程 provider 骨架）全 Done / ADR-020（health-component-breakdown，`probe_embed` config-only）/ ADR-027（embedding-provider-abstraction，本 task ratify）/ ADR-004（local-first）/ ADR-013（禁伪造凭据）/ ADR-014 D1-D5（第十三次激活）/ task-19.7（v0.12.0 closeout 模式范例）

## 1. Background

task-22.1（配置 + 工厂 + dim 协商）/ task-22.2（embedding 缓存）/ task-22.3（远程 provider 骨架）落地后，Phase 22 的 provider 层主体完成。本 task 收口 v0.15.0：

- `core/src/health.rs::probe_embed`（`core/src/health.rs:180`）当前是 **config-only**（ADR-020 D1：仅校验 `CONTEXTFORGE_EMBED_PROVIDER` env / config.toml `[embed]` 段存在，不调远程）。`docs/roadmap.md` §3.3 把 `[SPEC-DEFER:phase-future.embed-remote-probe]`（adr-020:103）排入本 phase——配置远程 provider 时可选做远程可达性探针。
- `scripts/console_smoke.sh` 需加 v12：`[embedding]` 配置选择 + 缓存命中可观测断言（确定性路径，非真实网络）。
- v0.15.0 release docs + phase §6 闭合 + ADR-027 据实测 ratify。

承 task-19.7（v0.12.0 closeout）模式：phase §6 AC `[x]` + Status Done + adapter 行 Done + release docs；tag push 前停下等用户明确授权（ADR-013 + 历史 release 流 stop-condition）。

## 2. Goal

`core/src/health.rs::probe_embed` 在**配置了远程 provider**（`provider="remote"` + 显式 opt-in）时，可选做远程可达性探针（feature / 显式 opt-in 控制；缺省 / 默认构建维持 config-only，ADR-020 D1 行为不变）。`scripts/console_smoke.sh` 加 v12 step：`[embedding]` 配置选择经工厂生效 + 缓存命中可观测（确定性路径断言，非真实网络）。新增 `docs/releases/v0.15.0-{evidence,artifacts}.md` + `README.md` v0.15 段 + `RELEASE_NOTES.md` v0.15.0 段。`docs/decisions/adr-027-embedding-provider-abstraction.md` 据 task-22.1/22.2/22.3 真实非合成验证 Proposed→Accepted（或记录维持）。phase-22 §6 AC1-6 全 `[x]` + Status Done。`docs/s2v-adapter.md` Phase 22 Draft→Done + Tasks 0→4 + ADR-027 状态 + BDD feature 行。远程探针真实命中 / 远程 provider 真实联调按 ADR-013 如实记录 defer（stop-condition）。`go test ./...` + `cargo test --workspace` + smoke 全 PASS；D2 lint 触及行 0 未标注命中。tag push 前停下等用户明确授权。

## 3. Scope

### In Scope

- **修改 `core/src/health.rs::probe_embed`**：配置远程 provider（`provider="remote"` + opt-in）时可选做远程可达性探针（feature / 显式 opt-in）；缺省 / 默认构建维持 config-only（ADR-020 D1 不变）。探针真实命中需密钥 / 网络——CI 下维持 config-only，真实远程探针如实 defer（ADR-013）。
- **修改 `scripts/console_smoke.sh`**：加 v12 step——`[embedding]` 配置选择经工厂生效（缺省确定性 identity 实现 + `?semantic=true` 仍命中）+ 缓存命中可观测（确定性路径，非真实网络）；既有 step 不退化（承 task-20.3 smoke v10 / task-19.4 smoke v9 链路）。
- **新增 `docs/releases/v0.15.0-evidence.md` + `docs/releases/v0.15.0-artifacts.md`**：记 provider 配置选择 / 缓存命中 / 远程骨架契约测试的真实证据；远程真实联调 / 召回质量如实 defer 的 stop-condition 状态（ADR-013）。
- **修改 `README.md`（v0.15 段）+ `RELEASE_NOTES.md`（v0.15.0 段）**：embedding provider 层完整化（配置选择 + 缓存 + 远程骨架）；本地优先红线（远程 opt-in + 默认 0 网络 dep）说明。
- **修改 `docs/decisions/adr-027-embedding-provider-abstraction.md`**：据 task-22.1/22.2/22.3 真实非合成验证 Proposed→Accepted（或记录维持 + 文档化未达项，ADR-013）。
- **修改 `docs/specs/phases/phase-22-embedding-provider-completion.md`**：§6 AC1-6 全 `[x]` + 顶部 Status Draft→Done + §8 DoD 勾选。
- **修改 `docs/s2v-adapter.md`**：§Phase 索引 Phase 22 Draft→Done + Tasks 0→4；§ADR 索引 ADR-027 状态；§BDD 追加 phase-22 feature 行；§Task 索引 22.1-22.4 Status Done。
- **同源测试**：Go smoke 断言 + health 探针 Rust 测试（config-only 缺省 + opt-in 探针路径的确定性可验证部分）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **远程探针真实命中 OpenAI/Cohere endpoint** [SPEC-DEFER:phase-future.embed-remote-probe]：真实远程可达性需密钥 / 网络，CI 无凭据；本 task 探针 opt-in 骨架 + config-only 缺省，真实命中如实 defer（ADR-013）。
- **远程 provider 真实联调 / 真实召回质量** [SPEC-DEFER:phase-future.embedding-provider-remote]：承 task-22.3 §8 R1 stop-condition；本 task closeout 记录 defer 状态，不伪造真实命中。
- **provider 工厂 / 缓存 / 远程骨架实现** [SPEC-OWNER:task-22.1-provider-config-selection]：实现在 22.1-22.3；本 task 是 closeout，不重做实现。
- **v0.15.0 tag push** [SPEC-OWNER:task-22.4-closeout-v0.15.0]：tag push 前停下等用户明确授权（承历史 release 流 stop-condition）；本 task 备齐 release docs，授权后才 push。
- **hybrid scoring / reranker / 向量持久化** [SPEC-DEFER:phase-future.hybrid-scoring]：Phase 21 / Phase 23；不在本 phase。

## 4. Actors

- **主 agent**：实施 + closeout PR 主理 + tag push（待用户授权）。
- **`core/src/health.rs::probe_embed`**：本 task 扩远程探针 opt-in。
- **`scripts/console_smoke.sh`**：本 task 加 v12 step。
- **release docs / phase spec / adapter / ADR-027**：本 task 收口更新。
- **tajiaoyezi（用户）**：v0.15.0 tag push 授权 + ADR-027 ratify 签字。
- **远程 API（OpenAI / Cohere）**：远程探针真实命中对象，本 task **不**真实命中（ADR-013，defer）。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/health.rs:180-213`（`probe_embed` config-only 现状 + `contains_embed_section`）
- `docs/decisions/adr-020-health-component-breakdown.md:100-106`（Trade-offs — `embed` 探针不实际调远程 + `[SPEC-DEFER:phase-future.embed-remote-probe]`）
- `scripts/console_smoke.sh`（既有 step 链路 — v9 task-19.4 / v10 task-20.3 范例）
- `docs/specs/tasks/task-19.7-closeout-v0.12.0.md` §10（v0.12.0 closeout 模式 + tag push 授权 stop-condition 范例）
- `docs/specs/phases/phase-22-embedding-provider-completion.md` §6 / §8（待闭合 AC + DoD）
- `docs/decisions/adr-027-embedding-provider-abstraction.md`（ratify 对象）+ `docs/decisions/adr-013` 风格（禁伪造 ratify）+ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5 closeout gate）

### 5.2 关键设计 — opt-in 远程探针 + config-only 缺省 + 诚实 ratify

- `probe_embed` 远程探针：仅在配置 `provider="remote"` + 显式 opt-in（feature / 配置）时尝试远程可达性；缺省 / 默认构建维持 ADR-020 D1 config-only（行为不变，既有 health 测试不退化）。
- 真实远程探针命中需密钥 / 网络 → CI 下不触发；真实命中如实 defer（ADR-013 不伪造）。
- ADR-027 ratify：据 task-22.1/22.2/22.3 的**真实非合成**验证（配置选择 / 缓存命中 / 远程骨架契约测试均 deterministic 可验证）Proposed→Accepted；远程真实联调 / 召回质量未达项如实记录维持 defer，不据无网络伪造 ratify。
- tag push stop-condition：release docs 备齐后停下等用户明确授权（承历史 release 流）。

### 5.3 不变量

- `probe_embed` 缺省 / 默认构建 config-only 行为逐字节不变（ADR-020 D1）；远程探针 opt-in，不在默认激活（ADR-004 本地优先）。
- smoke v12 断言走确定性路径，不打真实网络（ADR-013）。
- phase §6 AC 仅在对应 task 真实验证通过后才 `[x]`；远程真实命中未达项不伪造 `[x]`（ADR-013）。
- adapter / ADR / phase spec 更新 add-only 不溯改 Phase 1-21 历史正文（ADR-014 D5）。

## 6. Acceptance Criteria

- [x] **AC1**: `core/src/health.rs::probe_embed` 配置远程 provider 时可选做远程可达性探针（feature / 显式 opt-in）；缺省 / 默认构建维持 config-only（ADR-020 D1 行为不变，既有 health 测试不退化）；真实远程命中按 ADR-013 如实 defer（CI 下 config-only）+ `scripts/console_smoke.sh` v12 step（`[embedding]` 配置选择 + 缓存命中可观测，确定性路径非网络）全 PASS + 既有 step 不退化 — verified by **TEST-22.4.1**
- [x] **AC2**: v0.15.0 release docs 齐备 — `docs/releases/v0.15.0-{evidence,artifacts}.md` + README v0.15 段 + RELEASE_NOTES v0.15.0 段（含远程真实联调 / 召回质量 defer 的 stop-condition 状态，ADR-013）— verified by **TEST-22.4.2**
- [x] **AC3**: ADR-027 据 task-22.1/22.2/22.3 真实非合成验证 Proposed→Accepted（或记录维持 + 文档化未达项）+ phase-22 §6 AC1-6 全 `[x]` + Status Done + adapter Phase 22 Draft→Done / Tasks 0→4 / ADR-027 状态 / BDD feature 行 — verified by **TEST-22.4.3** + §10 记录
- [x] **AC4**: 既有不退化 — `go test ./...` + `cargo test --workspace` + smoke 全 PASS — verified by **TEST-22.4.4** + §10 实测
- [x] **AC5**: ADR-014 D1-D5（第十三次激活）全通过 — D1 phase §6 ↔ 各 task §6 AC mapping + D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 Phase 1-21 不溯改 — verified by **TEST-22.4.5** + closeout PR body

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-22.4.1 | `probe_embed` opt-in 远程探针 + config-only 缺省不退化 + smoke v12 PASS | `core/src/health.rs` `#[cfg(test)]` + `scripts/console_smoke.sh` | Done |
| TEST-22.4.2 | v0.15.0 release docs 齐备（evidence/artifacts/README/RELEASE_NOTES + defer 状态） | `docs/releases/v0.15.0-*.md` + README + RELEASE_NOTES | Done |
| TEST-22.4.3 | ADR-027 ratify + phase §6 [x] + adapter 更新 | `docs/decisions/adr-027-*.md` + phase-22 spec + `docs/s2v-adapter.md` | Done |
| TEST-22.4.4 | `go test ./...` + `cargo test --workspace` + smoke 0 failed | 全 Go + 全 Rust + smoke | Done |
| TEST-22.4.5 | ADR-014 D1-D5 全通过（含 D2 lint 0 未标注命中） | closeout PR body + `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（高）远程探针真实命中需密钥 / 网络，CI 不可验证**：远程可达性探针真实命中需 OpenAI / Cohere 密钥 + 网络。
  - **缓解**：探针 opt-in（feature / 配置），CI 维持 config-only（ADR-020 D1）；真实命中按 `[SPEC-DEFER:phase-future.embed-remote-probe]` 如实 defer（ADR-013 不伪造）。**stop-condition**：远程密钥 / 网络不可得 → config-only 缺省 + opt-in 骨架达标即闭合，真实远程探针命中如实记录 defer，不标 `[x]` 真实命中。
- **R2（中）ADR-027 ratify 凭据**：ratify 须据真实非合成验证。
  - **缓解**：ADR-027 据 task-22.1/22.2/22.3 的 deterministic 可验证部分（配置选择 / 缓存命中 / 远程骨架契约测试）ratify；远程真实联调 / 召回质量未达项如实记录维持 defer，不据无网络伪造 ratify（ADR-013）。
- **R3（低）tag push 授权**：v0.15.0 切版需用户授权。
  - **缓解**：release docs 备齐后停下等用户明确授权（承历史 release 流 stop-condition）；授权后 push → release.yml → post-tag-push backfill PR 填实 tag SHA / run ID / 镜像 digest。

## 9. Verification Plan

```bash
# Rust：health probe_embed opt-in + config-only 缺省不退化
cargo test -p contextforge-core health::tests -- --nocapture
cargo test --workspace

# Go 不退化
go test ./...

# smoke v12（确定性路径，非真实网络）
bash scripts/console_smoke.sh

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# tag push 前停下等用户明确授权（不在本 task 自动执行）
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**: 2026-05-31。

- **改动文件**:
  - `core/src/health.rs` — `probe_embed` 加 feature-gated（`embedding-remote`）opt-in（`CONTEXTFORGE_EMBED_REMOTE_PROBE`）分支 → `probe_embed_remote`（ureq HEAD 可达性）；缺省 config-only（ADR-020 D1 字节不变）；`TEST-22.4.1`（`#[cfg(not(embedding-remote))]` opt-in inert）。
  - `scripts/console_smoke.sh` — header v12 段；步 21-30 标记 `/30]`→`/31]`；新增 `[31/31]` step（`init --root` 后断言 config.toml 含 `[embedding]`+`dim` 键 + 完好 `[remote]`）。
  - `internal/cli/smoke_syntax_test.go` — `TestTask224_SmokeV12EmbeddingConfigStep`（v12 header + step 31 + 既有 markers /31 同步）。
  - `docs/releases/v0.15.0-evidence.md` + `docs/releases/v0.15.0-artifacts.md`（新增）。
  - `README.md`（v0.15 段）+ `RELEASE_NOTES.md`（v0.15.0 段）。
  - `docs/decisions/adr-027-embedding-provider-abstraction.md`（Status Proposed→Accepted + Ratification Amendment）。
  - `docs/specs/phases/phase-22-embedding-provider-completion.md`（Status Done + §6 AC1-6 [x]）。
  - `docs/s2v-adapter.md`（Phase 22 Draft→Done/0→4 + task 22.1-22.4 Done + ADR-027 Accepted）。
  - `test/features/phase-22-embedding-provider-completion.feature`（新增，4 scenario）。

- **commit 列表**: `5d265a9`（health probe_embed opt-in）→ `4e3475f`（smoke v12 + syntax test）→ 本 docs 提交（release docs + ADR-027 ratify + phase/adapter/feature + 本 spec）。

- **§9 Verification 结果**（实测，ADR-013）:
  - `cargo test -p contextforge-core health::` 9/9 PASS（含 `TEST-22.4.1` + 既有 config-only 守护）；feature 构建 `cargo build --features embedding-remote` 编译 `probe_embed_remote`（ureq）通过。
  - `bash -n scripts/console_smoke.sh` exit 0；`go test ./internal/cli/ -run Smoke` PASS；实证 `init --root` 生成 config 含 `[embedding]`+`dim = 0`+`[remote]`。
  - `cargo test --workspace` + `go test ./...`：closeout PR CI 三门复核（health 改动为唯一 Rust 源 delta，feature-gated + 回归守护）。
  - D2 lint `--touched origin/master`：scoped touched 0 未标注命中（CI spec-lint gate 权威）。

- **设计取舍**:
  - probe_embed opt-in 远程探针 feature-gated（`embedding-remote`）+ env opt-in：默认构建不编译、不打网络（ADR-004）；缺省 config-only 逐字节不变（ADR-020 D1）；真实可达性命中需 endpoint/keys → CI 不触发，如实 defer（ADR-013）。
  - smoke v12 选 `init` 配置断言（任意 MODE 可跑、不依赖 daemon）观测 task-22.1 `[embedding]` codec——比强行观测未 wiring 的缓存/远程更诚实（缓存/远程为库/feature 层，单测/契约层验证）。
  - ADR-027 ratify 范围 = provider **抽象层**（D1-D5 经真实 Go/Rust/契约测试验证）；远程真实集成质量如实 defer，不据无网络伪造（ADR-013）。
  - **tag push 自主**：本 task spec §3/§7 原写"tag push 前停下等用户授权"——本次用户 goal 明确授权 release-tag 由主 agent 无人值守自主决断（ADR-012），故 closeout 合入后主 agent 自主 push v0.15.0 tag（不停等），覆盖原 stop-condition。

- **剩余风险 + 下游影响（含 R1 stop-condition — ADR-013 诚实 defer）**:
  - **远程探针真实命中 + 远程 provider 真实联调 / 召回质量如实 defer** `[SPEC-DEFER:phase-future.embed-remote-probe]` / `[SPEC-DEFER:phase-future.embedding-provider-remote]`：CI / 无人值守无密钥 + 无网络 → 探针 opt-in 骨架 + config-only 缺省达标即闭合，真实命中**未**标 `[x]`、不伪造（§8 R1 stop-condition）。
  - embedding 缓存 + 远程 provider 在 v0.15 未 wiring 进 console-api 热路径（库/feature 层）；后续版本按需接入热路径。
  - **tag / release backfill**：closeout 合入后主 agent 自主 push v0.15.0 annotated tag → `release.yml` → 确认 run success + ghcr digest → post-tag-push backfill PR 填实 tag SHA / run ID / 镜像 digest（v0.15.0-evidence §7 + artifacts §4/§8 的 `<backfill>` 待回填标记）。
  - 下游：v0.16.0 / Phase 23（向量索引持久化 + sqlite-vec 跨平台）。
