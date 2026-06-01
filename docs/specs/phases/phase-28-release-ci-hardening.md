# Phase 28 · release-ci-hardening

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。本 phase 硬化 v0.1-v0.20 一路沿用的发布 / CI 流水线，兑现 `docs/roadmap.md §3.9` 与 PRD §524 / RELEASE_NOTES §451 一路 `[SPEC-DEFER:phase-future.*]` 延后的四项发布硬化 marker：**multi-arch 镜像**（`release.yml` 现 linux/amd64 only → +linux/arm64 manifest list）、**匿名可拉取验证**（`verify-image.yml` 现仅鉴权 pull → 加未鉴权 pull 守 v0.10.0「shipped PRIVATE → 403」回归）、**镜像供应链证明**（cosign keyless 签名 + SBOM + provenance attestation，现全空白）、**CI 强 lint**（clippy / gofmt 卡红，现完全缺失非阻断；须先测存量再定卡红时机）。全部为 `.github/workflows/*` 配置层 + 必要 lint 修复改动；镜像运行时行为 + 默认构建 0-network / 0 新依赖 baseline 不变（ADR-004），🟢 可在真实 CI / release run 验证。v0.21.0 收口。对应 `docs/roadmap.md §3.9`。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md §3.9`（发布 / CI 硬化候选 + 四 marker + ci-strict-lint 须先测存量告警量再定卡红时机的明确告诫）→ `.github/workflows/release.yml`（`v*` tag push 触发 + `docker/build-push-action@v5` `platforms: linux/amd64` + `docker/setup-buildx-action@v3` 已在、`setup-qemu-action` 缺、`permissions: contents:read + packages:write`（无 `id-token: write`）+ push `:tag`+`:latest`）→ `.github/workflows/verify-image.yml`（`workflow_dispatch` + 鉴权 `docker/login-action` pull + `/v1/health` contract_version=v1 + `:latest` digest parity；无未鉴权 pull 步）→ `.github/workflows/ci.yml`（`pull_request`+`push master` 三 job：`cargo-test` / `go-test` / `spec-lint`；无任何 clippy/rustfmt/gofmt/golangci job）→ `Dockerfile`（3-stage rust+go+debian-slim；`CGO_ENABLED=0` Go 静态；多架构 base 镜像；无 amd64 硬编码——multi-arch 仅 workflow 改动，Dockerfile 无需改）→ `docs/prds/context-forge.prd.md:524`（multi-arch + ci-strict-lint defer 出处）→ `RELEASE_NOTES.md:451`（verify-image-anonymous-pull defer 出处：v0.10.0 GHCR 初始 PRIVATE → 匿名 403）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第十九次激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造凭据红线：真实 CI run 出 multi-arch manifest / cosign 验签 / SBOM / 真实 lint 存量计数，受阻如实记录不伪造）→ `docs/decisions/adr-007-minimal-tarball-distribution.md`（部署发布基线，本 phase 扩展到 multi-arch 签名 OCI 镜像）→ `docs/decisions/adr-004-local-first-privacy-baseline.md`（镜像运行时 0-network / 默认构建 0-dep baseline 不变）。
>
> **ADR 影响面（已识别）**：
> - **ADR-033 release-ci-hardening（新，Proposed）**：记 multi-arch 镜像 + 匿名可拉取验证（D1）+ 镜像供应链证明 cosign 签名 + SBOM + provenance attestation（D2）+ CI 强 lint 先测存量再定卡红时机（D3）+ 镜像运行时 / 默认构建 0-network / 0-dep baseline 不变（D4）。落地后据真实 CI / release run 产物（multi-arch manifest list digest / `cosign verify` / SBOM attestation / 真实 clippy·gofmt 存量计数）ratify；某 marker 受阻（如 arm64 emulation 构建超时 / lint 存量过大只能 warn-first）则据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify、不伪造（ADR-013）。
> - 触及 **ADR-007（minimal-tarball-distribution）**：部署发布面由「最小 tarball + 单架构镜像」扩展到 multi-arch 签名 OCI 镜像 + SBOM——以 add-only Amendment 记录扩展结果，不溯改 ADR-007 正文（D5）。
> - 触及 **ADR-004（local-first-privacy-baseline）**：multi-arch / 签名 / lint 均 build / publish / CI 层，镜像运行时行为 + 默认 0-network / 默认构建 0 新依赖 baseline 不变（守线，非推翻）。
> - 触及 **ADR-012（main-agent-governance-autonomy）**：tag / release 仍主 agent 自治触发，但发布硬化（签名 / 多架构镜像推送）属 outward-facing 不可逆——按既有「tag + ghcr release 须用户显式授权」红线（沿用 v0.13-v0.20 模式），不自行 tag。

## 1. 阶段目标

v0.20.0 ship 后，ContextForge 的发布 / CI 流水线具备**多架构分发**（linux/amd64 + linux/arm64 manifest list 推 GHCR）、**可公开验证的分发面**（未鉴权匿名 pull 守 PRIVATE 回归）、**供应链可证明**（镜像 digest 经 cosign keyless 签名 + SBOM + provenance attestation 可验）、以及**可阻断的代码质量门**（clippy / gofmt CI lint，按真实存量告警量决定 warn-first 或卡红）。全部为 `.github/workflows/*` + 必要 lint 修复层改动；镜像运行时行为 + 默认构建 0-network / 0 新依赖 baseline 不变（ADR-004）；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. `release.yml` 经 `docker/setup-qemu-action` + `platforms: linux/amd64,linux/arm64` 推出**多架构 manifest list**（OCI index）到 GHCR（`:tag` + `:latest` 均为 index），真实 release run 可验 manifest list 含 amd64 + arm64 digest；`verify-image.yml` 加**未鉴权（logged-out）匿名 pull** 步断言 GHCR 包公开可拉取（守 v0.10.0 PRIVATE → 403 回归）（AC1）
2. `release.yml` 加 `id-token: write` 权限 + **cosign keyless 签名** 推出的镜像 manifest digest + **SBOM 生成**（syft 或 build-push-action `sbom: true`）+ **provenance attestation**；真实 release run 产 `.sig` / SBOM / attestation 且 `cosign verify` 通过（AC2）
3. **CI 强 lint**：先**实测存量** `cargo clippy --workspace -- -D warnings` + `gofmt -l` + `go vet` 告警计数（ADR-013 真实非合成），据存量决定——存量清零则加阻断 lint job、存量过大则 warn-first（不卡红）+ 文档化存量 + `[SPEC-DEFER]` follow-up；无论哪种，新增 `lint` job 入 CI 且既有三门不退化（AC3）
4. v0.21.0 release docs + `scripts/console_smoke.sh` v18（发布硬化相关 smoke 断言 + 既有 step 不退化）+ phase §6 闭合 + ADR-033 据真实 CI / release run 产物 ratify 或受阻如实记录维持 + ADR-007 add-only Amendment（AC4）
5. ADR-014 D1-D5（第十九次激活）全通过（AC5）

**v0.x 版本号决策**：v0.21.0 minor release（发布 / CI 硬化收口；纯 `.github/workflows/*` + lint 修复层改动，镜像运行时行为不变、不破坏既有 v0.6-v0.20 client、默认构建 0 新依赖 + 0 网络）。

## 2. 业务价值

兑现 PRD §524 / RELEASE_NOTES §451 / roadmap §3.9 一路刻意延后的发布硬化 marker，补齐发布 / 供应链 / 质量门缺口：

- **multi-arch 镜像 + 匿名可拉取**：`release.yml` 现仅 linux/amd64（`[SPEC-DEFER:phase-future.multi-arch-image]`，PRD:524），arm64 用户（Apple Silicon / arm 服务器）无法原生拉取；v0.10.0 GHCR 初始 PRIVATE 致 Console team 匿名 pull 403（RELEASE_NOTES:451，`[SPEC-DEFER:phase-future.verify-image-anonymous-pull]`），事后人工翻 public 无回归守护。本 phase 推 multi-arch manifest list + 加未鉴权 pull 守护。
- **镜像供应链证明**：cosign 签名 / SBOM / provenance 现全空白（`[SPEC-DEFER:phase-future.image-signing-and-sbom]`，v0.9-v0.11 artifacts）——下游无法验镜像来源 / 成分。本 phase 加 keyless 签名 + SBOM + attestation，使镜像可验、可审、符合供应链基线。
- **CI 强 lint**：clippy / gofmt 卡红现完全缺失（`[SPEC-DEFER:phase-future.ci-strict-lint]`，PRD:524）——代码风格 / lint 退化无门禁。本 phase 加 `lint` job（按真实存量决定卡红时机，roadmap §3.9 明确告诫先测存量避免一次性大面积变红）。
- **PRD §Constraints 发布 / §Decisions Log D7（部署发布基线）**：发布面从「价值闭环优先的最小分发」演进到「多架构 + 可验证 + 有质量门」的生产级发布流水线，符合 ADR-007 部署发布基线的自然延伸。

**不在本 phase scope**：

- 镜像瘦身 / distroless 运行时基座 [SPEC-DEFER:phase-future.image-slim-distroless]
- 多架构构建加速（原生 arm64 runner / 交叉编译替代 QEMU emulation）[SPEC-DEFER:phase-future.multi-arch-native-runner]
- 签名密钥轮换 / 私有 KMS / Rekor 透明日志自托管 [SPEC-DEFER:phase-future.signing-key-management]
- clippy / gofmt 存量告警全量清零（若 AC3 选 warn-first）[SPEC-DEFER:phase-future.lint-backlog-cleanup]
- release.yml 之外的 SLSA L3+ / 可复现构建（reproducible build）[SPEC-DEFER:phase-future.reproducible-build-slsa]

## 3. 涉及模块

### 28.1 multi-arch 镜像 + 匿名可拉取验证（task-28.1）

- 修改 `.github/workflows/release.yml`——加 `docker/setup-qemu-action@v3` 步（现缺）+ `docker/build-push-action@v5` `platforms: linux/amd64,linux/arm64`（现 amd64 only）；`:tag` + `:latest` 推出多架构 manifest list（OCI index）
- 修改 `.github/workflows/verify-image.yml`——add-only 未鉴权（logged-out，无 `docker/login-action` / 清 token）`docker pull` 步断言 GHCR 包公开可拉取（守 v0.10.0 PRIVATE → 403 回归）+ 复核多架构 manifest list 含 amd64 + arm64 digest（`docker buildx imagetools inspect` / `docker manifest inspect`）
- `Dockerfile` 无需改（已确认无 amd64 硬编码 / `CGO_ENABLED=0` Go 静态 / 多架构 base 镜像；arm64 仅 emulation 构建耗时风险，非正确性风险）
- 同源验证（≥2，真实 release run / workflow_dispatch：manifest list inspect 含 2 架构 digest / 未鉴权 pull exit 0）

### 28.2 镜像供应链证明：cosign 签名 + SBOM + provenance（task-28.2）

- 修改 `.github/workflows/release.yml`——`permissions` 加 `id-token: write`（OIDC keyless）；push 后加 cosign keyless 签 manifest digest 步（`sigstore/cosign-installer` + `cosign sign`）+ SBOM 生成（`docker/build-push-action` `sbom: true` 或 `anchore/sbom-action` syft）+ provenance attestation（`provenance: true` / `attest`）
- 修改 `.github/workflows/verify-image.yml`（或新 verify 步）——`cosign verify` 验签 + attestation / SBOM 存在性断言
- 同源验证（≥2，真实 release run：`cosign sign` 产 `.sig` + `cosign verify` exit 0 / SBOM attestation 可拉取）
- 0 新代码依赖（cosign / syft 为 CI 工具 action，非 Cargo / go.mod direct dep）

### 28.3 CI 强 lint（clippy / gofmt 先测存量再定卡红）（task-28.3）

- 先实测存量（ADR-013 真实非合成）：`cargo clippy --workspace -- -D warnings` + `gofmt -l .` + `go vet ./...` 告警 / 文件计数，记真实数字
- 修改 `.github/workflows/ci.yml`——add-only `lint` job（clippy + gofmt + go vet）；据存量决定：清零 → 阻断（`-D warnings`）；存量大 → warn-first（`continue-on-error` 或非阻断报告）+ 文档化存量 + `[SPEC-DEFER:phase-future.lint-backlog-cleanup]`
- 若选阻断：随 PR 修复触及代码的 lint（surgical，仅清能清的；不大面积重构既有代码）
- 同源验证：`lint` job 入 CI 且既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化；lint 结论（阻断 / warn-first + 存量数）如实记录

### 28.4 v0.21.0 closeout（task-28.4）

- 修改 `scripts/console_smoke.sh`——v18：发布硬化相关 smoke 断言（multi-arch / 签名 / lint 状态文档化 step + 既有 step 不退化）
- 修改 `internal/cli/smoke_syntax_test.go`——既有 step markers 同步 + 新 step 断言
- 新增 `docs/releases/v0.21.0-{evidence,artifacts}.md` + `README.md` v0.21 段 + `RELEASE_NOTES.md` v0.21.0 段
- 修改 `docs/decisions/adr-033-release-ci-hardening.md`——据真实 CI / release run 产物 Proposed→Accepted（§Ratification 回填）或受阻如实记录维持 + ADR-007 add-only Amendment（部署发布面扩展，不溯改正文 D5）
- 修改 `docs/s2v-adapter.md`（Phase 28 Draft→Done + Tasks 0→4；ADR-033 状态；ADR-007 Amendment 记录；§BDD 追加 phase-28 feature 行）

### BDD feature

- 新增 `test/features/phase-28-release-ci-hardening.feature`（≥4 scenario：multi-arch manifest + 匿名 pull / cosign 签名 + SBOM / CI 强 lint 存量门 / v0.21.0 收口）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 28.1 | `.github/workflows/release.yml` QEMU + `platforms: amd64,arm64` manifest list + `verify-image.yml` 未鉴权匿名 pull 守护 + index digest parity | `../tasks/task-28.1-multi-arch-image-and-anonymous-pull.md` |
| 28.2 | `.github/workflows/release.yml` `id-token:write` + cosign keyless 签名 + SBOM（syft）+ provenance attestation + `verify-image.yml` `cosign verify` | `../tasks/task-28.2-image-signing-sbom-provenance.md` |
| 28.3 | `.github/workflows/ci.yml` add-only `lint` job（clippy `-D warnings` + gofmt + go vet）+ 先测存量定卡红时机（warn-first / 阻断） | `../tasks/task-28.3-ci-strict-lint.md` |
| 28.4 | smoke v18 + v0.21.0 closeout + ADR-033 ratify + ADR-007 Amendment | `../tasks/task-28.4-closeout-v0.21.0.md` |

## 5. 依赖关系

- **task-28.1**（multi-arch + 匿名 pull）dep 既有 `release.yml`（buildx 已在，仅缺 QEMU）+ `verify-image.yml`（鉴权 pull 框架已在）；可独立先行（不依赖 28.2/28.3）。
- **task-28.2**（签名 + SBOM）建议 28.1 先 merge（multi-arch manifest list 基线稳定后签 index digest）+ dep 既有 `release.yml` push 步；新增 `id-token: write` 权限。
- **task-28.3**（CI 强 lint）dep 无（独立改 `ci.yml`）；先测存量为本 task 子项，可与 28.1/28.2 并行。
- **task-28.4**（closeout）dep 28.1 + 28.2 + 28.3 全 Done；release docs / smoke v18 / ADR-033 ratify 据三 task 真实 CI / release run 产物。
- 外部：ADR-033（本 phase 新 Proposed）/ ADR-007（minimal-tarball-distribution，本 phase 扩展部署发布面到 multi-arch 签名镜像，add-only Amendment）/ ADR-004（本地优先，镜像运行时 / 默认 0-network / 0-dep baseline 不变）/ ADR-012（tag/release 主 agent 自治触发，但 outward-facing 不可逆须用户显式授权）/ ADR-014 第十九次激活 / ADR-013（禁伪造凭据红线，真实 CI run 产物 / 真实 lint 存量计数，受阻不伪造）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [ ] **AC1**：`release.yml` 经 `docker/setup-qemu-action` + `platforms: linux/amd64,linux/arm64` 推出多架构 manifest list（`:tag` + `:latest` 均 OCI index）到 GHCR，真实 release run 可验 manifest list 含 amd64 + arm64 digest；`verify-image.yml` add-only 未鉴权（logged-out）匿名 pull 步断言 GHCR 包公开可拉取（守 v0.10.0 PRIVATE → 403 回归）— verified by task-28.1 §6 AC1-2 + phase-smoke step 1
- [ ] **AC2**：`release.yml` 加 `id-token: write` + cosign keyless 签 manifest digest + SBOM（syft / build-push-action）+ provenance attestation；真实 release run 产 `.sig` / SBOM / attestation 且 `cosign verify` 通过 — verified by task-28.2 §6 AC1-2 + phase-smoke step 2
- [ ] **AC3**：CI 强 lint——先实测 `cargo clippy --workspace -- -D warnings` + `gofmt -l` + `go vet` 真实存量计数（ADR-013 非合成），据存量加阻断 `lint` job（存量清零）或 warn-first + 文档化存量 + `[SPEC-DEFER:phase-future.lint-backlog-cleanup]`（存量大）；`lint` job 入 `ci.yml` 且既有 `cargo-test`/`go-test`/`spec-lint` 三门不退化；卡红 / warn-first 结论如实记录 — verified by task-28.3 §6 AC1-2 + phase-smoke step 3
- [ ] **AC4**：v0.21.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ `scripts/console_smoke.sh` v18（发布硬化 smoke + 既有 step 不退化）+ ADR-033 据真实 CI / release run 产物 ratify 或受阻如实记录维持 + ADR-007 add-only Amendment（部署发布面扩展，不溯改正文 D5）+ phase §6 闭合 — verified by task-28.4 §6 AC2-3
- [ ] **AC5**：ADR-014 cross-validation gate 全套通过（第十九次激活）— D1 mapping + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-27 不溯改 — verified by task-28.4 closeout PR body

**端到端 smoke（C1 集成兜底）**：(1) 真实 release run 推出 multi-arch manifest list（amd64+arm64）+ 未鉴权匿名 pull exit 0；(2) cosign keyless 签名 + SBOM + attestation 产出且 `cosign verify` 通过；(3) CI `lint` job 入门 + 真实 clippy/gofmt 存量结论（阻断 / warn-first）如实记录、既有三门不退化 全 PASS（受阻如实标注）。

## 7. 阶段级风险

- **R1（中）arm64 multi-arch 构建 emulation 超时 / 失败**：QEMU emulation 下 Rust `cargo build --release` arm64 估 ≥20 min（task-16.3 §estimate），可能超 CI 时限或不稳。
  - **缓解**：task-28.1 先 workflow_dispatch 试构 arm64 计时；超时则评估 `cache-to/from: gha` 复用 / 拆分 per-arch job；若 emulation 不可行则 amd64 保底 + arm64 `[SPEC-DEFER:phase-future.multi-arch-native-runner]`（原生 runner）如实延后。stop-condition：arm64 真实构建不过则 AC1 arm64 维度不标 `[x]`（amd64 + 匿名 pull 达成则部分 ratify，不伪造）。
- **R2（中）cosign keyless OIDC 在 GHCR 推送上下文权限 / Rekor 可达性**：keyless 签名依赖 `id-token: write` + Fulcio / Rekor 公共实例可达；GHCR digest 签名需 manifest 已 push。
  - **缓解**：task-28.2 在 push 步后取 build-push-action 输出的 digest 再签（非 tag）；`cosign verify` 在同 run 即时验；Fulcio / Rekor 不可达则记录受阻 + 评估 `--key` 模式 `[SPEC-DEFER:phase-future.signing-key-management]`。stop-condition：`cosign verify` 不过则 AC2 不标 `[x]`（不伪造签名通过）。
- **R3（高）ci-strict-lint 存量告警一次性大面积变红**：clippy / gofmt 卡红现完全缺失，存量未知；直接 `-D warnings` 可能令既有 PR / master 大面积变红（roadmap §3.9 明确告诫）。
  - **缓解**：task-28.3 **先实测存量计数**再决定——存量清零（少量可 surgical 修）才加阻断；存量大则 warn-first（不卡红）+ 文档化真实存量 + `[SPEC-DEFER:phase-future.lint-backlog-cleanup]` 留后续清零。stop-condition：不为达「卡红」伪造存量为零 / 不大面积重构既有代码（surgical 红线）；AC3 以「lint job 入门 + 真实存量结论如实记录」满足，卡红与否据真实存量。
- **R4（中）多架构 manifest list 致 `:latest` digest parity 校验语义变化**：`verify-image.yml` 现校 `:latest` 单 digest parity；manifest list（index）digest 与单架构 digest 语义不同。
  - **缓解**：task-28.1 同步更 `verify-image.yml` parity 校验为 index digest 维度（`docker buildx imagetools inspect` / `docker manifest inspect`）；既有鉴权 pull + `/v1/health` 步保留不退化。stop-condition：parity 校验逻辑改动须经真实 workflow_dispatch run 验过再标 AC1。

## 8. Definition of Done

- 4 task spec（28.1-28.4）顶部 `**Status**: Done`
- §6 阶段级 AC1-5 全 `[x]`（受阻态按 ADR-013 如实记录，不伪造——如 arm64 emulation 不可行 / lint warn-first 据真实存量）
- 端到端 smoke 3 step 全 PASS（含受阻态如实标注）
- **ADR**：ADR-033 `Proposed → Accepted`（据真实 CI / release run 产物：multi-arch manifest list digest / `cosign verify` / SBOM attestation / 真实 clippy·gofmt 存量计数）或据实测受阻记录维持 + 文档化；ADR-007 经 add-only Amendment 记录部署发布面扩展（不溯改正文，ADR-014 D5）
- **adapter**：§Phase 索引 Phase 28 `Draft → Done` + `Tasks 0 → 4`；§ADR 索引 ADR-033；§BDD 追加 phase-28 feature 行；ADR-007 Amendment 记录
- **release**：`docs/releases/v0.21.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.21 段 + README v0.21 段
- **smoke**：`scripts/console_smoke.sh` v18（发布硬化 smoke + 既有 step 不退化）+ `internal/cli/smoke_syntax_test.go` markers 同步
- **follow-up**：multi-arch 原生 runner `[SPEC-DEFER:phase-future.multi-arch-native-runner]` + 签名密钥管理 `[SPEC-DEFER:phase-future.signing-key-management]` + lint 存量清零 `[SPEC-DEFER:phase-future.lint-backlog-cleanup]` + 镜像瘦身 `[SPEC-DEFER:phase-future.image-slim-distroless]` + 可复现构建 SLSA `[SPEC-DEFER:phase-future.reproducible-build-slsa]` 留 backlog
