# ADR `033`: `release-ci-hardening`

**Status**: Proposed
**Category**: 部署发布 / CI / 供应链
**Date**: 2026-06-01
**Decided By**: 主 agent（ADR-012 自治）；tajiaoyezi ratification at v0.21.0 closeout
**Related**: ADR-007 (minimal-tarball-distribution — 本 ADR 扩展部署发布面到 multi-arch 签名 OCI 镜像 + SBOM，add-only Amendment) / ADR-004 (local-first-privacy-baseline — 镜像运行时 / 默认构建 0-network / 0-dep baseline 不变) / ADR-012 (main-agent-governance-autonomy — tag/release 主 agent 自治触发，outward-facing 不可逆须用户显式授权) / ADR-013 (禁伪造凭据红线 — 真实 CI / release run 产物，受阻不伪造) / ADR-014 (D1-D5，第十九次激活) / PRD §Constraints 发布 / §Decisions Log D7 / roadmap §3.9

## Context

ContextForge v0.1-v0.20 的发布 / CI 流水线在「价值闭环优先」原则下保持最小形态（ADR-007）。截至 v0.20.0，实测现状（`.github/workflows/{release,verify-image,ci}.yml` + `Dockerfile`）：

- `release.yml`（`v*` tag push / `workflow_dispatch` 触发）经 `docker/build-push-action@v5` `platforms: linux/amd64` 推 `:tag` + `:latest` 到 `ghcr.io/<owner>/contextforge-daemon`；`docker/setup-buildx-action@v3` 已在，`docker/setup-qemu-action` **缺**；`permissions: contents:read + packages:write`（无 `id-token: write`）。
- `verify-image.yml`（`workflow_dispatch`）经**鉴权** `docker/login-action`（`GITHUB_TOKEN`）pull + `docker run` + `/v1/health` `contract_version=v1` + `:latest` digest parity 校验；无未鉴权（匿名）pull 步。
- `ci.yml`（`pull_request` + `push master`）三 job：`cargo-test` / `go-test` / `spec-lint`；**无任何** clippy / rustfmt / gofmt / golangci-lint job，仓内无 `.golangci.*` / `clippy.toml` / `rustfmt.toml`。
- `Dockerfile`（root，3-stage rust+go+debian-slim）：`CGO_ENABLED=0` Go 静态产物 + 多架构 base 镜像 + 无 amd64 硬编码（multi-arch 仅 workflow 改动，Dockerfile 无需改）。

PRD §524 / RELEASE_NOTES §451 / roadmap §3.9 显式记录了四项本可早做、但刻意延后到「发布硬化小 Phase」的 marker：

1. **multi-arch 镜像缺失**：`release.yml` 仅 linux/amd64（`[SPEC-DEFER:phase-future.multi-arch-image]`，PRD:524）。arm64 用户（Apple Silicon / arm 服务器）无原生镜像可拉。

2. **匿名可拉取验证缺失**：v0.10.0 GHCR 镜像 + `:latest` 初始为 PRIVATE，Console team 匿名 pull 得 403（RELEASE_NOTES:451），事后由 owner 人工翻 public，**无回归守护**（`[SPEC-DEFER:phase-future.verify-image-anonymous-pull]`）。`verify-image.yml` 现仅鉴权 pull，验不出「公开可拉取」回归。

3. **镜像供应链证明缺失**：cosign 签名 / SBOM / provenance attestation 全空白（`[SPEC-DEFER:phase-future.image-signing-and-sbom]`，v0.9-v0.11 artifacts）。下游无法验镜像来源 / 成分。

4. **CI 强 lint 缺失**：clippy / gofmt 卡红完全缺失、非阻断（`[SPEC-DEFER:phase-future.ci-strict-lint]`，PRD:524）。代码风格 / lint 退化无门禁；存量告警量未知，roadmap §3.9 **明确告诫**「须先评估存量 clippy/gofmt 告警量再决定卡红时机（避免一次性大面积变红）」。

本 ADR 记录上述四块发布 / CI 硬化的处理策略。**全部为 `.github/workflows/*` 配置层 + 必要 lint 修复改动**：镜像运行时行为不变、默认构建 0-network / 0 新依赖 baseline 不变（ADR-004），既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化，且 🟢 可在真实 CI / release run 验证。

## Decision

发布 / CI 硬化采用 **multi-arch manifest list + 匿名可拉取守护 + keyless 供应链证明 + 先测存量再定卡红的强 lint** 策略：

### D1 — multi-arch 镜像 + 匿名可拉取验证（task-28.1）

`release.yml` 加 `docker/setup-qemu-action@v3` 步 + `docker/build-push-action@v5` `platforms: linux/amd64,linux/arm64`，`:tag` + `:latest` 推出多架构 manifest list（OCI index）。`verify-image.yml` add-only 一个未鉴权（logged-out，无 `docker/login-action` / 清 token）`docker pull` 步，断言 GHCR 包公开可拉取（守 v0.10.0 PRIVATE → 403 回归），并把既有 `:latest` digest parity 校验对齐到 manifest list（index）digest 维度（`docker buildx imagetools inspect` / `docker manifest inspect`）。`Dockerfile` 不改（已确认 arch-clean）。

**理由**：buildx 半套基建已在（仅缺 QEMU），multi-arch 边际成本小、覆盖 arm64 真实用户；匿名 pull 守护是对 v0.10.0 真实回归（RELEASE_NOTES:451）的最小、最 surgical 的护栏，扩展既有 `verify-image.yml`。arm64 的真实风险是 emulation 构建耗时（task-16.3 估 ≥20 min），非 Dockerfile 正确性。

### D2 — 镜像供应链证明：cosign 签名 + SBOM + provenance（task-28.2）

`release.yml` `permissions` 加 `id-token: write`（OIDC keyless）；push 后取 build-push-action 输出的 manifest digest，经 `sigstore/cosign-installer` + `cosign sign` keyless 签名（Fulcio 证书 + Rekor 透明日志）；SBOM 经 `docker/build-push-action` `sbom: true` 或 `anchore/sbom-action`（syft）生成；provenance attestation 经 `provenance: true` / `attest`。`verify-image.yml`（或新 verify 步）加 `cosign verify` 验签 + attestation / SBOM 存在性断言。

**理由**：keyless（OIDC）签名免密钥管理、与 GitHub Actions 身份天然集成；签 manifest digest（非 tag）保证签名绑定不可变内容；SBOM + attestation 使镜像成分 / 来源可审。cosign / syft 为 CI 工具 action，**不引入** Cargo / go.mod direct dep（ADR-008 无 Amendment）。

### D3 — CI 强 lint：先测存量再定卡红时机（task-28.3）

先**实测存量**（ADR-013 真实非合成）：`cargo clippy --workspace -- -D warnings` + `gofmt -l .` + `go vet ./...` 的真实告警 / 文件计数。据存量决定 `ci.yml` add-only `lint` job 的卡红形态：

- 存量清零（或少量可 surgical 修）→ 阻断（`clippy -D warnings` + `gofmt` diff 非空即 fail + `go vet`）。
- 存量过大 → warn-first（`continue-on-error` / 非阻断报告）+ 文档化真实存量 + `[SPEC-DEFER:phase-future.lint-backlog-cleanup]` 留后续清零。

无论哪种，新增 `lint` job 入 CI，既有三门不退化；若选阻断则随 PR 仅 surgical 修触及代码的 lint（**不大面积重构既有代码**）。

**理由**：roadmap §3.9 明确告诫先测存量避免一次性大面积变红；「先量化再决策」既兑现质量门方向、又不为达「卡红」伪造存量为零或破坏 surgical 红线。warn-first 是诚实的中间态（ADR-013），优于「假装全绿卡红」或「永不引入」。

### D4 — 镜像运行时 / 默认构建 baseline 不变 + outward-facing 授权红线

所有改动限于 `.github/workflows/*` + 必要 lint 修复：镜像运行时行为、默认构建 0-network / 0 新依赖 baseline（ADR-004）、Console Contract v1 shape 均不变；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。multi-arch / 签名 / SBOM 均 build / publish 层，不改产物功能。tag + ghcr release（含多架构镜像推送 + 签名）属 outward-facing 不可逆操作——沿用 v0.13-v0.20 既有红线，由主 agent 自治准备但 **tag / release 须用户显式授权**（ADR-012），不自行 tag。

**理由**：发布硬化是「如何分发」而非「分发什么」的演进，与 ADR-004 本地优先 / 隐私基线正交、与 ADR-007 部署发布基线一脉相承（扩展非推翻）。outward-facing 授权红线守住不可逆操作的人类决断点。

## Consequences

- **Positive**: arm64 用户得原生镜像（multi-arch manifest list）；匿名 pull 回归（v0.10.0 PRIVATE → 403）有自动守护；镜像 digest 可 `cosign verify` + SBOM / provenance 可审（供应链可信）；CI 得 clippy / gofmt 质量门（按真实存量稳妥引入）；全部 build / publish / CI 层，镜像运行时 + 默认构建 baseline 不变（ADR-004），既有三门不退化；0 新代码依赖（cosign / syft 为 CI action）。
- **Negative / open**: arm64 经 QEMU emulation 构建耗时（估 ≥20 min，task-16.3），可能逼近 CI 时限——若不可行则 amd64 保底 + arm64 经原生 runner 延后（如实记录，不伪造「multi-arch 成功」）；keyless 签名依赖 Fulcio / Rekor 公共实例可达性，不可达则记录受阻 + 评估 `--key` 模式；ci-strict-lint 存量未知，若过大则只能 warn-first（卡红延后，如实记录真实存量），不为「卡红达成」伪造零存量；多架构 manifest list digest 与单架构 digest 语义不同，`verify-image.yml` parity 校验须同步对齐到 index 维度。
- **Ratification**: 本 ADR **Proposed**。task-28.1（multi-arch manifest list + 匿名 pull 真实 release run / workflow_dispatch）+ task-28.2（cosign sign + `cosign verify` 真实通过 + SBOM / attestation）+ task-28.3（真实 clippy / gofmt 存量计数 + lint job 入 CI）通过后，于 v0.21.0 closeout（task-28.4）据真实 CI / release run 产物 ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）；某 marker 受阻（arm64 emulation 超时 / Fulcio·Rekor 不可达 / lint 存量过大 warn-first）则据「已达维度 ratify + 受阻维度如实记录」处理，不强 ratify。
- **Follow-ups**: multi-arch 原生 runner / 交叉编译替代 QEMU emulation `[SPEC-DEFER:phase-future.multi-arch-native-runner]`；签名密钥轮换 / 私有 KMS / Rekor 自托管 `[SPEC-DEFER:phase-future.signing-key-management]`；clippy / gofmt 存量告警全量清零（若 D3 选 warn-first）`[SPEC-DEFER:phase-future.lint-backlog-cleanup]`；镜像瘦身 / distroless 运行时基座 `[SPEC-DEFER:phase-future.image-slim-distroless]`；SLSA L3+ / 可复现构建 `[SPEC-DEFER:phase-future.reproducible-build-slsa]`。ADR-007 部署发布面扩展以 add-only Amendment 记录（task-28.4，不溯改 ADR-007 正文，ADR-014 D5）。
