# Task `28.2`: `image-signing-sbom-provenance — release.yml GitHub 原生签名 attestation（actions/attest-build-provenance SLSA provenance + actions/attest-sbom syft SPDX SBOM，push 到 GitHub 证明库 + GHCR OCI referrer）+ verify-image.yml gh attestation verify 验证`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 28 (release-ci-hardening)
**Dependencies**: 既有 `release.yml`（`v*` tag push → GHCR push，`docker/build-push-action` 输出 digest）/ Phase 16（`verify-image.yml` 起源）/ ADR-033（release-ci-hardening §D2——**本 task 据 recon 由 cosign 改为 GitHub 原生 attestation**，closeout task-28.4 以 add-only Amendment 记录，ADR-033 Proposed 决策细化）/ ADR-007（minimal-tarball-distribution，部署发布面扩展，task-28.4 Amendment）/ ADR-004（local-first-privacy-baseline，镜像运行时不变）/ ADR-012（main-agent-governance-autonomy，tag/release outward-facing 须用户显式授权）/ ADR-008（attest actions / syft 为 CI action，无 Cargo / go.mod direct dep → 无 Amendment）/ ADR-013（禁伪造凭据红线）/ ADR-014 D1-D5（第十九次激活）

## 1. Background

`release.yml`（`v*` tag push）现 push 镜像到 GHCR（`docker/build-push-action@v5`，单架构 amd64，task-28.1 确认）但**无任何供应链证明**：实测 `permissions`（`:14-16`）仅 `contents: read` + `packages: write`（**无 `id-token: write`**），「Build and push」步（`:50-61`）**无 `id:`**（无法引用 `steps.<id>.outputs.digest`），push 后仅一个 `Image summary` echo 步（`:63-71`）。下游无法验镜像**来源**（哪个 workflow/commit 构建）或**成分**（SBOM）——`[SPEC-DEFER:phase-future.image-signing-and-sbom]`（v0.9-v0.11 artifacts）。

`verify-image.yml`（task-28.1 后）有匿名 pull + 鉴权 pull + `/v1/health` + parity，但**无证明验证**步。

**方案选型（已据 recon + 用户决策）**：采用 **GitHub 原生签名 attestation**（`actions/attest-build-provenance` + `actions/attest-sbom`，Sigstore bundle 存 GitHub 证明库 + 可选 GHCR OCI referrer），消费方用 `gh attestation verify` 验证——免密钥管理、与 GitHub Actions 身份天然集成，比 cosign-为中心方案更简、更现代（recon 2026 最佳实践）。ADR-033 §D2 原文写 cosign，本 task 据此细化为原生 attestation，closeout 以 add-only Amendment 记录（ADR-033 Proposed）。

## 2. Goal

`release.yml` push 后产**两个签名 attestation**绑定镜像 manifest digest：(1) **SLSA build provenance**（`actions/attest-build-provenance@v4`，记构建来源 workflow/commit/inputs）；(2) **SPDX SBOM**（`anchore/sbom-action` syft 扫 pushed digest 出 SPDX-JSON → `actions/attest-sbom@v4` 签名）。二者 push 到 GitHub 证明库（`attestations: write`）+ GHCR OCI referrer（`push-to-registry: true`），经 Sigstore（Fulcio cert + Rekor log，`id-token: write`）签名。`verify-image.yml` 加 `gh attestation verify` 步验 provenance + SBOM。

pass bar：**不越 outward-facing 红线**——经本地 `registry:2` + 小测试镜像在 CI 内跑通 attest → `gh attestation verify` 全机制（不碰 GHCR、不跑 ~20min 真实构建）；真实 GHCR 镜像证明在**已授权的 v0.21.0 release**（task-28.4）产生。0 新代码依赖（attest actions / syft 均 CI action）；既有 release push + verify 步不退化；D2 lint 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 修改 `.github/workflows/release.yml`——(a) `permissions` 加 `id-token: write` + `attestations: write`（保留 `contents: read` + `packages: write`）；(b) 「Build and push」步加 `id: build`（暴露 `steps.build.outputs.digest`）；(c) push 后加 `actions/attest-build-provenance@v4`（`subject-name: ${{ env.IMAGE_NAME }}` 无 tag + `subject-digest: ${{ steps.build.outputs.digest }}` + `push-to-registry: true`）；(d) 加 `anchore/sbom-action`（扫 `${IMAGE_NAME}@${digest}` 出 SPDX-JSON）+ `actions/attest-sbom@v4`（同 subject + `sbom-path` + `push-to-registry: true`）。既有 build/push/tags/cache/summary 步不动。
- 修改 `.github/workflows/verify-image.yml`——鉴权 pull 后、`docker run` 前加 `gh attestation verify oci://$IMAGE`（provenance：`--signer-workflow <owner>/<repo>/.github/workflows/release.yml`；SBOM：`--predicate-type https://spdx.dev/Document/v2.3`），`GH_TOKEN` 用 `secrets.GITHUB_TOKEN`；Summary 步加 attestation 行。`verify-image.yml` `permissions` 视需要加 `id-token: write`（gh attestation verify 默认从 GitHub API 取 bundle，通常 `packages: read` + `GITHUB_TOKEN` 足够；按实测定）。
- `Dockerfile` 不改。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- cosign 分离签名（detached `.sig`，供 admission controller / Kyverno 策略）[SPEC-DEFER:phase-future.cosign-detached-signature]——本 task 用 GitHub 原生 attestation；若后续需 cosign sig 再加。
- CI 强 lint（clippy / gofmt）[SPEC-OWNER:task-28.3-ci-strict-lint]
- 签名密钥轮换 / 私有 KMS / Rekor 自托管 [SPEC-DEFER:phase-future.signing-key-management]
- multi-arch（arm64）镜像 [SPEC-DEFER:phase-future.multi-arch-native-runner]（task-28.1 已据实延后）

## 4. Actors

- 主 agent（ADR-012 自治；outward-facing release 须用户授权）
- `.github/workflows/release.yml`（attest provenance + SBOM 产出 + 签名）
- `.github/workflows/verify-image.yml`（`gh attestation verify` 验证）
- Sigstore（Fulcio ephemeral cert + Rekor 透明日志，经 GitHub OIDC `id-token`）
- GitHub 证明库（attestation API）+ GHCR OCI referrer（`sha256-<digest>` referrer manifest）
- `anchore/sbom-action`（syft，扫镜像出 SPDX）

## 5. Behavior Contract

### 5.1 Required Reading

- `.github/workflows/release.yml:14-16`（permissions）+ `:50-61`（build-push-action，加 `id:` + digest 输出）+ `:63-71`（summary，attest 步插于 push 与 summary 间）
- `.github/workflows/verify-image.yml:22-24`（permissions）+ `:45-58`（login + 鉴权 pull，verify 步插于其后）+ `:127-139`（Summary）
- `docs/decisions/adr-033-release-ci-hardening.md §D2`（cosign 原文 → 本 task 细化为原生 attestation）
- recon 凭据：`actions/attest-build-provenance@v4`（subject-name 无 tag + subject-digest + push-to-registry）/ `actions/attest-sbom@v4` / `anchore/sbom-action@v0`（format spdx-json）/ `gh attestation verify oci://...`（`--signer-workflow` / `--predicate-type`）

### 5.2 关键设计 — 验证策略（不越 outward-facing 红线）

真实 GHCR 镜像证明需 push 到 GHCR（outward-facing 不可逆），且证明绑定真实 release digest。**本 task 实现验证经本地 `registry:2` + 小测试镜像证明全机制，不碰 GHCR、不跑 ~20min 真实 Rust 构建**：

- 临时验证 workflow（push 触发本分支，验完即删）：起 `services: registry:2`（localhost:5000）→ build + push 一个**极小测试镜像**（`FROM alpine` 量级，秒级）到 localhost:5000 → `actions/attest-build-provenance@v4` + `anchore/sbom-action` + `actions/attest-sbom@v4`（subject = localhost digest，`push-to-registry: true`）→ `gh attestation verify oci://localhost:5000/... --bundle-from-oci`（provenance + SBOM predicate-type）。证明 attest 产出 + 签名 + `gh attestation verify` 全链在本 repo CI 上下文成立。
- 真实 GHCR 镜像（amd64）的 provenance + SBOM attestation 在**已授权的 v0.21.0 release**（task-28.4）产生 + 真验。
- **stop-condition**：Fulcio / Rekor 公共实例不可达 / attest action 在本 repo 上下文失败 → 如实记录受阻 + 评估替代（ADR-013，不伪造「attestation 成功」）。

**关键技术约束（recon）**：attest 步**要求 `push: true`**（local image store 不能承载 attestation index）；`subject-name` 须**全限定无 tag**（digest 标识镜像）；perms 须 `id-token: write` + `attestations: write` + `packages: write`；`mode=max` provenance 会泄漏 build-arg（本 repo Dockerfile 不传 secret build-arg，安全）。

### 5.3 不变量

- 0 新代码依赖（纯 `.github/workflows/*` YAML + CI action；无 Cargo / go.mod 改动；ADR-008 无 Amendment）。
- 镜像运行时行为不变（attestation 是「关于镜像的元数据」非镜像内容；Dockerfile 不改）。
- `release.yml` 既有 build/push/tags/cache/summary + `verify-image.yml` 既有匿名 pull / 鉴权 pull / run / `/v1/health` / parity 步**不退化**。
- 默认构建 0-network / 0-dep baseline 不变（ADR-004）。
- tag / release outward-facing 不可逆 → 不自行 tag（ADR-012，沿用 v0.13-v0.20）。

## 6. Acceptance Criteria

- [ ] **AC1**（签名 SLSA provenance）: `release.yml` `permissions` + `id-token: write` + `attestations: write`；「Build and push」+ `id: build`；push 后 `actions/attest-build-provenance@v4`（subject-name 无 tag + subject-digest = `steps.build.outputs.digest` + `push-to-registry: true`）产签名 SLSA provenance（GitHub 证明库 + GHCR referrer）— verified by **TEST-28.2.1**（临时本地 registry run + v0.21.0 release）
- [ ] **AC2**（签名 SPDX SBOM）: `release.yml` 加 `anchore/sbom-action`（扫 pushed digest 出 SPDX-JSON）+ `actions/attest-sbom@v4`（签名 push 证明库 + referrer）— verified by **TEST-28.2.2**（临时本地 registry run + v0.21.0 release）
- [ ] **AC3**（验证 + 不退化）: `verify-image.yml` 加 `gh attestation verify oci://$IMAGE`（provenance `--signer-workflow .../release.yml` + SBOM `--predicate-type https://spdx.dev/Document/v2.3`）；既有匿名 pull / 鉴权 pull / run / `/v1/health` / parity 步保留不退化；0 新代码依赖（attest/syft 为 CI action） — verified by **TEST-28.2.3**（临时 run `gh attestation verify --bundle-from-oci` 通过 + §10 实测）
- [ ] **AC4**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-28.2.4** + §10 记录（CI spec-lint 权威）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-28.2.1 | `release.yml` id-token+attestations perms + id:build + attest-build-provenance 产签名 SLSA provenance（临时本地 registry 验机制 + v0.21.0 release 真产） | `.github/workflows/release.yml` | Planned |
| TEST-28.2.2 | `release.yml` syft SPDX SBOM + attest-sbom 签名（临时本地 registry 验 + v0.21.0 release 真产） | `.github/workflows/release.yml` | Planned |
| TEST-28.2.3 | `verify-image.yml` gh attestation verify（provenance signer-workflow + SBOM predicate-type）通过 + 既有步不退化 + 0 新代码依赖 | `.github/workflows/verify-image.yml` | Planned |
| TEST-28.2.4 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）Fulcio / Rekor 公共实例可达性 / attest action 在本 repo 上下文失败**：keyless attestation 依赖 `id-token: write` OIDC + Sigstore 公共实例可达。
  - **缓解**：临时本地 registry run 先证 attest + verify 全链在本 repo CI 成立；不可达 / 失败则如实记录受阻 + 评估（ADR-013，不伪造）。stop-condition：attest 或 `gh attestation verify` 不过则对应 AC 不标 `[x]`。
- **R2（中）`gh attestation verify` 在 private repo / 镜像可见性约束**：原生 attestation verify 对 private/internal repo 需 GitHub Enterprise Cloud；本 repo 镜像公开（task-28.1 匿名 pull 确认），repo 公开性须确认。
  - **缓解**：临时 run 用 `--bundle-from-oci` 从 registry 取 bundle（不依赖 GitHub API 可见性）；若 verify 需特定 perms 则 `verify-image.yml` 加 `id-token: write`（按实测）。stop-condition：verify 路径不通则记录 + 评估 cosign verify 替代。
- **R3（低）attest 步要求 `push: true` + 多平台 index 交互**：attest 须镜像已 push（local store 不行）；task-28.1 已确认单架构（无 multi-arch index 复杂度）。
  - **缓解**：attest 步置于 build-push（`push: true`）之后，引用其 digest；单架构 digest 直接 attest，无 index 子清单枚举。

## 9. Verification Plan

```bash
# 0. YAML 语法（actionlint 若装；否则 CI 校验权威）
#    actionlint .github/workflows/release.yml .github/workflows/verify-image.yml

# 1. AC1+AC2+AC3 机制 — 临时本地 registry run（小测试镜像，不碰 GHCR、不跑真实构建）
#    push 触发 _tmp-validate-attest.yml：registry:2 → build+push localhost:5000/test →
#    attest-build-provenance + sbom-action + attest-sbom（push-to-registry）→ gh attestation verify --bundle-from-oci
#    真实 run id 记入 §10；期望 attest 产出 + verify 通过

# 2. AC3 — 既有不退化（workflow-only 改动不影响 workspace）
cargo test --workspace
go test ./...

# 3. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **outward-facing 红线**：真实 GHCR 镜像的 provenance + SBOM attestation 在**已授权的 v0.21.0 release**（task-28.4）产生；本 task 验证经本地 `registry:2` + 小测试镜像证明全机制，不触发 GHCR 推送（ADR-012）。

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Draft（待实施）。
- **完成日期**：（待填）。
- **改动文件**：（待填——预期 `.github/workflows/release.yml` + `.github/workflows/verify-image.yml`）。
- **commit 列表**：（待填——workflow 改动以临时本地 registry run + v0.21.0 release run 为 verified 依据）。
- **§9 Verification 实测结果（ADR-013 真实非合成）**：（待填——临时 attest+verify run id + 结论；真实 GHCR attestation 在 v0.21.0 release，受阻维度如实记录）。
- **设计取舍**：（待填——GitHub 原生 attestation vs cosign；本地 registry 验证策略；ADR-033 §D2 由 cosign 改原生 attestation 的 Amendment 口径）。
- **剩余风险 + 下游影响**：（待填——Fulcio/Rekor 可达性 / verify perms；task-28.3 接 CI 强 lint；task-28.4 closeout ratify ADR-033 + §D2 Amendment）。
