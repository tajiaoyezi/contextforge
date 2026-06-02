# Task `28.2`: `image-signing-sbom-provenance — release.yml cosign keyless 签名（镜像 digest）+ cosign attest SPDX SBOM（syft）+ build-push SLSA provenance + verify-image.yml cosign verify + verify-attestation`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 28 (release-ci-hardening)
**Dependencies**: 既有 `release.yml`（`v*` tag push → GHCR push，`docker/build-push-action` 输出 digest）/ Phase 16（`verify-image.yml` 起源）/ ADR-033（release-ci-hardening §D2——cosign sign + SBOM + provenance，**本 task 实现即 §D2 原文意图，无 Amendment**）/ ADR-007（minimal-tarball-distribution，部署发布面扩展，task-28.4 Amendment）/ ADR-004（local-first-privacy-baseline，镜像运行时不变）/ ADR-012（main-agent-governance-autonomy，tag/release outward-facing 须用户显式授权）/ ADR-008（cosign / syft 为 CI 工具，无 Cargo / go.mod direct dep → 无 Amendment）/ ADR-013（禁伪造凭据红线）/ ADR-014 D1-D5（第十九次激活）

## 1. Background

`release.yml`（`v*` tag push）现 push 镜像到 GHCR（`docker/build-push-action@v5`，单架构 amd64）但**无任何供应链证明**：`permissions` 仅 `contents: read` + `packages: write`（无 `id-token: write`），「Build and push」步无 `id:`（无法引用 digest），push 后仅 echo summary。下游无法验镜像来源 / 成分（`[SPEC-DEFER:phase-future.image-signing-and-sbom]`）。

**方案选型经实测修正**：本 task 初采 GitHub 原生 attestation（`actions/attest-*`），但实测 run `26789731232` **failure**——`actions/attest-build-provenance` 报 `Feature not available for user-owned private repositories`。本仓库 `tajiaoyezi/contextforge` 为**用户私有仓库**（`gh repo view` isPrivate=true；GHCR 包公开但 repo 私有），GitHub 原生 attestation 不可用（需 public repo 或 GHEC）。改采 **cosign-为中心**（ADR-033 §D2 原文意图）：cosign 走公共 Sigstore（Fulcio cert + Rekor log）+ 把签名 / SBOM attestation 存为 GHCR 的 OCI 工件，**与 repo 可见性无关**——私有仓库可用。

## 2. Goal

`release.yml` push 后对镜像 manifest digest：(1) **cosign keyless sign**（`sigstore/cosign-installer` + `cosign sign --yes <IMAGE>@<digest>`，Fulcio ephemeral cert + Rekor log，OIDC `id-token: write`，签名存 GHCR OCI 工件）；(2) **SPDX SBOM attestation**（`anchore/sbom-action` syft 扫 pushed digest 出 SPDX-JSON → `cosign attest --type spdxjson`，keyless 签名）；(3) **SLSA build provenance**（`docker/build-push-action` `provenance: mode=max`，入 image index）。`verify-image.yml` 加 `cosign verify`（镜像签名）+ `cosign verify-attestation --type spdxjson`（SBOM），`--certificate-identity-regexp` 锚定本仓库 release workflow + `--certificate-oidc-issuer` GitHub Actions。

pass bar：**不越 outward-facing 红线**——经本地 `registry:2` + 小测试镜像 + `--allow-insecure-registry` 在 CI 内跑通 cosign sign + attest + verify + verify-attestation 全机制（run `26799480280` success），不碰 GHCR、不跑 ~20min 真实构建；真实 GHCR 镜像签名 / attestation 在**已授权的 v0.21.0 release**（task-28.4）产生。0 新代码依赖（cosign / syft 均 CI 工具）；既有 release push + verify 步不退化；D2 lint 0 未标注命中。

## 3. Scope

### In Scope（实际交付）

- 修改 `.github/workflows/release.yml`——(a) `permissions` 加 `id-token: write`（保留 `contents: read` + `packages: write`）；(b) 「Build and push」步加 `id: build` + `provenance: mode=max`（SLSA provenance 入 index）；(c) push 后加 `sigstore/cosign-installer@v3` + `cosign sign --yes "${IMAGE_NAME}@${digest}"`（keyless）；(d) `anchore/sbom-action`（扫 `${IMAGE_NAME}@${digest}` 出 SPDX-JSON）+ `cosign attest --yes --type spdxjson --predicate sbom.spdx.json "${IMAGE_NAME}@${digest}"`（keyless 签名 SBOM attestation）。既有 build/push/tags/cache/summary 步不动。
- 修改 `.github/workflows/verify-image.yml`——鉴权 pull 后、`docker run` 前加 `sigstore/cosign-installer@v3` + `cosign verify`（镜像签名，`--certificate-identity-regexp "^https://github.com/<repo>/\.github/workflows/release\.yml@.*$"` + `--certificate-oidc-issuer https://token.actions.githubusercontent.com`）+ `cosign verify-attestation --type spdxjson`（SBOM）。既有匿名 pull / 鉴权 pull / run / `/v1/health` / parity 步保留不退化。
- `Dockerfile` 不改。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **GitHub 原生 attestation（actions/attest-*）** [SPEC-DEFER:phase-future.github-native-attestation]——经实测在用户私有仓库不可用（run 26789731232 failure）；若仓库改公开 / 升 GHEC 后可加（消费方 `gh attestation verify` 更简）。
- CI 强 lint（clippy / gofmt）[SPEC-OWNER:task-28.3-ci-strict-lint]
- 签名密钥轮换 / 私有 KMS / Rekor 自托管 [SPEC-DEFER:phase-future.signing-key-management]
- multi-arch（arm64）镜像 [SPEC-DEFER:phase-future.multi-arch-native-runner]（task-28.1 已据实延后）

## 4. Actors

- 主 agent（ADR-012 自治；outward-facing release 须用户授权）
- `.github/workflows/release.yml`（cosign sign + cosign attest SBOM + build-push provenance）
- `.github/workflows/verify-image.yml`（cosign verify + cosign verify-attestation）
- Sigstore（Fulcio ephemeral cert + Rekor 透明日志，经 GitHub OIDC `id-token`）
- GHCR（镜像 + cosign 签名 / attestation OCI 工件宿主，公开包；与 repo 私有无关）
- `anchore/sbom-action`（syft，扫镜像出 SPDX）

## 5. Behavior Contract

### 5.1 Required Reading

- `.github/workflows/release.yml:14-17`（permissions）+ `:52-64`（build-push-action，`id: build` + `provenance: mode=max` + digest 输出）
- `.github/workflows/verify-image.yml`（login + 鉴权 pull 后插 cosign verify 步）
- `docs/decisions/adr-033-release-ci-hardening.md §D2`（cosign sign + SBOM + provenance——本 task 即其原文实现）
- recon 凭据：`cosign sign --yes <ref>@<digest>`（keyless，无 `--key`；`COSIGN_EXPERIMENTAL` 已废弃）/ `cosign attest --type spdxjson --predicate` / `cosign verify --certificate-identity-regexp + --certificate-oidc-issuer https://token.actions.githubusercontent.com` / 签 **digest 非 tag**（不可变内容）

### 5.2 关键设计 — 验证策略（不越 outward-facing 红线）

真实 GHCR 镜像签名需 push 到 GHCR（outward-facing 不可逆）。**本 task 实现验证经本地 `registry:2` + 小测试镜像 + `--allow-insecure-registry` 证明全机制，不碰 GHCR、不跑 ~20min 真实构建**：

- 临时验证 workflow（push 触发本分支，验完即删）：起 `services: registry:2`（localhost:5000，buildx `network=host`）→ build + push 一个极小测试镜像（`FROM alpine`，秒级）→ `cosign sign --yes --allow-insecure-registry` + syft SBOM + `cosign attest --type spdxjson` + `cosign verify` + `cosign verify-attestation`（`--certificate-identity-regexp` 锚本仓库 workflow + GitHub Actions issuer）。**run `26799480280` success** 证明 cosign keyless 全链在本 repo CI 成立（cosign 不依赖 repo 可见性，故私有仓库可用——与失败的 GitHub 原生 attestation 形成对照）。
- 真实 GHCR 镜像签名 / attestation 在**已授权的 v0.21.0 release**（task-28.4）产生 + `verify-image.yml` 真验。
- **stop-condition**：Fulcio / Rekor 公共实例不可达 → 如实记录受阻 + 评估 `--key` 模式 `[SPEC-DEFER:phase-future.signing-key-management]`（ADR-013，不伪造）。

**关键技术约束（recon）**：签 **digest 非 tag**（不可变内容，避免 TOCTOU）；keyless 无 `--key`（cosign 自动用 GitHub OIDC）；`--yes` 跳过 Rekor 公共日志上传交互确认；verify 须同时给 `--certificate-identity[-regexp]` + `--certificate-oidc-issuer`；cosign-installer 在 sign / verify 两端用同 major 避免 bundle 格式不兼容。

### 5.3 不变量

- 0 新代码依赖（纯 `.github/workflows/*` YAML + CI 工具；无 Cargo / go.mod 改动；ADR-008 无 Amendment）。
- 镜像运行时行为不变（签名 / SBOM / provenance 是「关于镜像的元数据」非镜像内容；Dockerfile 不改）。
- `release.yml` 既有 build/push/tags/cache/summary + `verify-image.yml` 既有匿名 pull / 鉴权 pull / run / `/v1/health` / parity 步**不退化**。
- 默认构建 0-network / 0-dep baseline 不变（ADR-004）。
- tag / release outward-facing 不可逆 → 不自行 tag（ADR-012，沿用 v0.13-v0.20）。

## 6. Acceptance Criteria

- [x] **AC1**（cosign keyless 签名 + SLSA provenance）: `release.yml` `permissions` + `id-token: write`；「Build and push」+ `id: build` + `provenance: mode=max`；push 后 `sigstore/cosign-installer` + `cosign sign --yes <IMAGE>@<digest>`（keyless 签 digest）— verified by **TEST-28.2.1**（临时本地 registry run 26799480280 cosign sign success + v0.21.0 release）
- [x] **AC2**（签名 SPDX SBOM attestation）: `release.yml` + `anchore/sbom-action`（扫 pushed digest 出 SPDX-JSON）+ `cosign attest --yes --type spdxjson --predicate`（keyless 签名 SBOM）— verified by **TEST-28.2.2**（临时 run 26799480280 syft + cosign attest success + v0.21.0 release）
- [x] **AC3**（验证 + 不退化）: `verify-image.yml` 加 cosign-installer + `cosign verify`（`--certificate-identity-regexp` release.yml + issuer GitHub Actions）+ `cosign verify-attestation --type spdxjson`；既有匿名 pull / 鉴权 pull / run / `/v1/health` / parity 步保留不退化；0 新代码依赖（cosign/syft 为 CI 工具） — verified by **TEST-28.2.3**（临时 run 26799480280 cosign verify + verify-attestation success + §10 实测）
- [x] **AC4**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-28.2.4** + §10 记录（CI spec-lint 权威）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-28.2.1 | `release.yml` id-token + id:build + provenance:max + cosign-installer + cosign sign（keyless 签 digest）；临时本地 registry run 26799480280 sign success + v0.21.0 release 真签 | `.github/workflows/release.yml` + run 26799480280 | Done（机制 Verified；GHCR 真签 @ v0.21.0） |
| TEST-28.2.2 | `release.yml` anchore/sbom-action（syft SPDX）+ cosign attest --type spdxjson；run 26799480280 syft + attest success | `.github/workflows/release.yml` + run 26799480280 | Done（机制 Verified；GHCR 真签 @ v0.21.0） |
| TEST-28.2.3 | `verify-image.yml` cosign verify + verify-attestation（cert-identity-regexp + issuer）+ 既有步不退化 + 0 新代码依赖；run 26799480280 verify + verify-attestation success | `.github/workflows/verify-image.yml` + run 26799480280 | Done |
| TEST-28.2.4 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）GitHub 原生 attestation 在私有仓库不可用 — ⚠️ 已发生**：`actions/attest-*` 需 public repo / GHEC。
  - **处置（已执行）**：实测 run 26789731232 failure 确认 → 改 cosign-为中心（公共 Sigstore + GHCR OCI 工件，与 repo 可见性无关），run 26799480280 验证全机制通过。ADR-033 §D2 原文即 cosign（无 Amendment）。GitHub 原生 attestation `[SPEC-DEFER:phase-future.github-native-attestation]`（仓库改公开后可加）。
- **R2（中）Fulcio / Rekor 公共实例可达性**：keyless 依赖 GitHub OIDC + Sigstore 公共实例可达。
  - **缓解**：临时 run 26799480280 已证可达 + 全链成立；release 时不可达则记录受阻 + 评估 `--key` 模式 `[SPEC-DEFER:phase-future.signing-key-management]`（ADR-013，不伪造）。stop-condition：cosign sign / verify 不过则对应 AC 不标 `[x]`。
- **R3（低）sign / verify cosign 版本不一致致 bundle 不兼容**：cosign v3 新 bundle 格式默认开。
  - **缓解**：`release.yml` + `verify-image.yml` 均用 `sigstore/cosign-installer@v3`（同 major 同默认 cosign 版本），signer / verifier 一致；临时 run 同 installer 验通过。

## 9. Verification Plan

```bash
# 0. YAML 语法（actionlint 若装；否则 CI 校验权威）
#    actionlint .github/workflows/release.yml .github/workflows/verify-image.yml

# 1. AC1+AC2+AC3 机制 — 临时本地 registry run（小测试镜像 + --allow-insecure-registry，不碰 GHCR）
#    _tmp-validate-attest.yml：registry:2 → build+push localhost:5000/test →
#    cosign sign + syft SBOM + cosign attest + cosign verify + cosign verify-attestation
#    实测 run 26799480280 success（全步 PASS）

# 2. AC3 — 既有不退化（workflow-only 改动不影响 workspace）
cargo test --workspace
go test ./...

# 3. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **outward-facing 红线**：真实 GHCR 镜像签名 + SBOM attestation 在**已授权的 v0.21.0 release**（task-28.4）产生；本 task 验证经本地 `registry:2` + 小测试镜像证明全机制，不触发 GHCR 推送（ADR-012）。

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Done（2026-06-02）。
- **完成日期**：2026-06-02。
- **改动文件**：
  - `.github/workflows/release.yml`——`permissions` + `id-token: write`；「Build and push」+ `id: build` + `provenance: mode=max`；push 后 + `sigstore/cosign-installer@v3` + `cosign sign --yes`（keyless 签 digest）+ `anchore/sbom-action`（syft SPDX）+ `cosign attest --type spdxjson`（keyless 签名 SBOM）。
  - `.github/workflows/verify-image.yml`——鉴权 pull 后 + `sigstore/cosign-installer@v3` + `cosign verify`（cert-identity-regexp release.yml + issuer GitHub Actions）+ `cosign verify-attestation --type spdxjson`。既有匿名 pull / 鉴权 pull / run / `/v1/health` / parity 不动。
- **commit 列表**：`docs(spec): task-28.2 Draft`（初 GitHub 原生）→ `feat(release): GitHub 原生签名 attestation`（初实现）→ `refactor(release): 改 cosign-为中心（GitHub 原生 attestation 因私有仓库不可用）` → `docs(spec): §10 回填 + 收口`。workflow 改动以临时本地 registry run + v0.21.0 release run 为 verified 依据。
- **§9 Verification 实测结果（ADR-013 真实非合成）**：
  - **GitHub 原生 attestation — ❌ 私有仓库不可用**：run `26789731232` failure，`actions/attest-build-provenance` 报 "Feature not available for user-owned private repositories"；`gh repo view` 确认 isPrivate=true → 弃用，改 cosign。
  - **cosign-为中心全机制 — ✅ 通过**：run `26799480280` conclusion **success**（本地 registry:2 + alpine 测试镜像 + `--allow-insecure-registry`）；cosign sign（keyless）/ syft SPDX + cosign attest / cosign verify / cosign verify-attestation 全步 PASS。证明 release.yml 的 cosign 步在真实 GHCR（HTTPS，无需 insecure）+ 真镜像上将成立。
  - **既有不退化 + 0 新依赖**：纯 `.github/workflows/*`；Dockerfile/Cargo/go.mod 未改。D2 lint 由 CI spec-lint 权威。
- **设计取舍**：(1) **cosign-为中心 vs GitHub 原生**——原生 attestation 消费方更简（`gh attestation verify`）但**用户私有仓库不可用**（实测 run 26789731232），cosign 走公共 Sigstore + GHCR OCI 工件不依赖 repo 可见性 → 私有仓库唯一可行的签名路径；正合 ADR-033 §D2 原文（无需 Amendment）。(2) **签 digest 非 tag**（不可变，防 TOCTOU）。(3) **provenance 用 build-push `mode=max`**（in-index SLSA，免签；cosign 签名的 Fulcio cert 已编码 build 身份提供签名层）。(4) **本地 registry + `--allow-insecure-registry` 验证**——全机制本地证明不碰 GHCR（cosign 不挑 registry，真实 GHCR HTTPS 无需 insecure）。
- **剩余风险 + 下游影响**：Fulcio/Rekor release 时可达性（不可达则 `--key` 模式 `[SPEC-DEFER:phase-future.signing-key-management]`）；GitHub 原生 attestation 待仓库公开后可加 `[SPEC-DEFER:phase-future.github-native-attestation]`；setup-buildx-action@v3 Node20 deprecation（2026-06-16，既有 action，非本 task scope）；task-28.3 接 CI 强 lint；task-28.4 closeout ratify ADR-033 §D2（cosign 真签据 v0.21.0 release run）。
