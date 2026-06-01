# Task `28.1`: `multi-arch-image-and-anonymous-pull — release.yml QEMU + platforms linux/amd64,linux/arm64 manifest list 推 GHCR + verify-image.yml 未鉴权匿名 pull 守护 + 多架构 manifest assert + index digest parity 对齐`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 28 (release-ci-hardening)
**Dependencies**: Phase 16（v0.9.0 release-candidate，`verify-image.yml` 起源，其注释 `:11` 记「first Phase 16 release-candidate tag」）/ 既有 `release.yml`（`v*` tag push → GHCR 推送，v0.8-v0.9 起）/ ADR-007（minimal-tarball-distribution，本 task 扩展部署发布面，task-28.4 add-only Amendment）/ ADR-033（release-ci-hardening §D1）/ ADR-004（local-first-privacy-baseline，镜像运行时不变）/ ADR-012（main-agent-governance-autonomy，tag/release outward-facing 须用户显式授权）/ ADR-013（禁伪造凭据红线）/ ADR-014 D1-D5（第十九次激活）

## 1. Background

`release.yml`（`v*` tag push / `workflow_dispatch` 触发）现仅构建单架构 linux/amd64（`.github/workflows/release.yml:55` `platforms: linux/amd64`），arm64 用户（Apple Silicon / arm 服务器）无原生镜像可拉（`[SPEC-DEFER:phase-future.multi-arch-image]`，PRD:524）。`docker/setup-buildx-action@v3` 已在（`release.yml:40-41`），但 `docker/setup-qemu-action` **缺**（multi-arch emulation 必需）。

`verify-image.yml`（`workflow_dispatch`）现仅经**鉴权** `docker/login-action`（`GITHUB_TOKEN`，`verify-image.yml:33-38`）pull 镜像，验不出「匿名（未鉴权）可拉取」——而 v0.10.0 GHCR 镜像 + `:latest` 初始为 PRIVATE，Console team 匿名 pull 得 403（RELEASE_NOTES:451），事后由 owner 人工翻 public，**无回归守护**（`[SPEC-DEFER:phase-future.verify-image-anonymous-pull]`）。

此外 `verify-image.yml:104-105` 的 `:latest` digest parity 校验用 `docker inspect --format '{{ index .RepoDigests 0 }}'`——`docker pull` 单架构后取到的是**平台特定** digest，引入 multi-arch manifest list（OCI index）后该校验语义错（比的是单架构 digest 而非 index digest）。

## 2. Goal

`release.yml` 加 `docker/setup-qemu-action@v3` + `docker/build-push-action@v5` `platforms: linux/amd64,linux/arm64`，使真实 v0.21.0 release run 推出**多架构 manifest list（OCI index）**到 GHCR（`:tag` + `:latest` 均 index）。`verify-image.yml` add-only 一个**未鉴权（logged-out）匿名 pull** 步断言 GHCR 包公开可拉取 + 多架构 manifest assert（含 amd64 + arm64），并把 `:latest` digest parity 校验对齐到 **index digest** 维度。`Dockerfile` 不改（已确认 arch-clean：`CGO_ENABLED=0` Go 静态 + 多架构 base 镜像 + 无 amd64 硬编码）。pass bar：**不越 outward-facing 红线**完成实现 + 验证（见 §5.2 验证策略：多架构构建经 `push: false` 证明、匿名 pull 对现有公开 `:latest` 证明，真正推 GHCR 只在已授权的 v0.21.0 release 发生）；0 新代码依赖；既有鉴权 pull + run + `/v1/health` 不退化；D2 lint `--touched origin/master` 0 未标注命中。

## 3. Scope

### In Scope

- 修改 `.github/workflows/release.yml`——「Set up Docker Buildx」步前加 `docker/setup-qemu-action@v3`；「Build and push」步 `platforms: linux/amd64,linux/arm64`（`:55` 改）。`tags` / `cache` / push 逻辑不动。
- 修改 `.github/workflows/verify-image.yml`——(a) 「Log in to GHCR」步**前** add-only「Anonymous (unauthenticated) pull」步：`docker logout ghcr.io` 后 `docker pull "$IMAGE"` 断言 exit 0（守 PRIVATE → 403 回归）；(b) add-only「Multi-arch manifest assert」步：`docker buildx imagetools inspect "$IMAGE"` 断言含 `linux/amd64` + `linux/arm64`；(c) 「Verify :latest matches the tag digest」步（`:96-113`）parity 改用 index digest（`docker buildx imagetools inspect --format '{{json .Manifest.Digest}}'` / `docker manifest inspect`），不再用单架构 `docker inspect RepoDigests`。既有鉴权 pull + run + `/v1/health` + contract_version=v1 步保留不退化。
- `Dockerfile` 不改（arch-clean 已确认；arm64 仅 emulation 构建耗时风险，非正确性风险）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- cosign keyless 签名 + SBOM + provenance attestation [SPEC-OWNER:task-28.2-image-signing-sbom-provenance]
- CI 强 lint（clippy / gofmt）[SPEC-OWNER:task-28.3-ci-strict-lint]
- 原生 arm64 runner / 交叉编译替代 QEMU emulation [SPEC-DEFER:phase-future.multi-arch-native-runner]
- 镜像瘦身 / distroless 运行时基座 [SPEC-DEFER:phase-future.image-slim-distroless]

## 4. Actors

- 主 agent（ADR-012 自治；outward-facing tag/release 须用户授权）
- `.github/workflows/release.yml`（多架构构建 + 推送）
- `.github/workflows/verify-image.yml`（匿名 pull 守护 + 多架构 assert + index parity）
- `docker buildx` + QEMU（multi-arch emulation 构建）
- GHCR（`ghcr.io/<owner>/contextforge-daemon`，manifest list 宿主）

## 5. Behavior Contract

### 5.1 Required Reading

- `.github/workflows/release.yml:40-61`（buildx 步 + build-push-action platforms/tags/cache）
- `.github/workflows/verify-image.yml:33-113`（login → pull → run → /v1/health → parity 全链）
- `Dockerfile`（3-stage rust+go+debian-slim；`CGO_ENABLED=0`；多架构 base）
- `RELEASE_NOTES.md:451`（v0.10.0 PRIVATE → 匿名 403 回归出处）
- `docs/roadmap.md §3.9`（multi-arch + anon-pull marker）
- `docs/decisions/adr-033-release-ci-hardening.md §D1`

### 5.2 关键设计 — 验证策略（不越 outward-facing 红线）

multi-arch 推 GHCR 是 outward-facing 不可逆（污染 `:latest` + 不可删 immutable tag），且真实 arm64 emulation 构建估 ≥20 min（task-16.3）。**本 task 的实现验证不触发真实 GHCR 推送**：

- **AC1 多架构构建**：经 `build-push-action` `push: false`（或 `docker buildx build --platform linux/amd64,linux/arm64 --output type=image,push=false`）在 CI / workflow_dispatch 证明 arm64 emulation **能构建** + 计时（不推 registry）。真正「推多架构 manifest list 到 GHCR」在**已授权的 v0.21.0 release**（task-28.4）发生。
- **AC2 匿名 pull**：新匿名 pull 步对**现有已公开**的 `:latest`（v0.20.0）跑，证明「未鉴权 pull exit 0」逻辑成立（不新推任何镜像）。
- arm64 构建超时 / emulation 不可行 → amd64 保底 + arm64 `[SPEC-DEFER:phase-future.multi-arch-native-runner]` 如实延后（ADR-013，不伪造「multi-arch 成功」）。

**二选一 — 匿名 pull 实现**：(A) 在 login 步**之前**加 pull 步（runner 此时未 login ghcr.io，天然匿名）；(B) `docker logout ghcr.io` 后 pull。**优先 (B)**（显式 `docker logout` 自证匿名意图，不依赖步序隐式假设；更 surgical 且对 verify 已有 login 步顺序无侵入）。

**index digest parity**：multi-arch manifest list digest（index）≠ 单架构 image digest。parity 改用 `docker buildx imagetools inspect "$IMAGE" --format '{{json .Manifest}}'` 取 index digest 比对 `:tag` vs `:latest`。

### 5.3 不变量

- 0 新代码依赖（纯 `.github/workflows/*` YAML；无 Cargo / go.mod 改动）。
- 镜像运行时行为不变（multi-arch 是「如何分发」非「分发什么」；Dockerfile 不改）。
- `verify-image.yml` 既有鉴权 pull + `docker run` + `/v1/health` + `contract_version=v1` + cleanup 步**不退化**（匿名 pull + manifest assert 为 add-only 旁挂）。
- 默认构建 0-network / 0-dep baseline 不变（ADR-004）。
- tag / release outward-facing 不可逆 → 不自行 tag（ADR-012，沿用 v0.13-v0.20）。

## 6. Acceptance Criteria

- [ ] **AC1**: `release.yml` 「Set up Docker Buildx」前 add `docker/setup-qemu-action@v3` + 「Build and push」`platforms: linux/amd64,linux/arm64`；CI / workflow_dispatch 经 `push: false` 多架构构建证明 arm64 emulation 能构建（含 amd64 + arm64 两 target），真实 release 时推出 manifest list（OCI index）；既有 `tags`/`cache`/push 逻辑不退化 — verified by **TEST-28.1.1**（真实 workflow_dispatch / CI multi-arch `push:false` build run，§10 记 run id）
- [ ] **AC2**: `verify-image.yml` add-only 未鉴权（`docker logout ghcr.io` 后）匿名 pull 步断言 GHCR 包公开可拉取（对现有公开 `:latest` 验，守 v0.10.0 PRIVATE → 403 回归）+ 多架构 manifest assert（`docker buildx imagetools inspect` 含 amd64 + arm64）+ `:latest` parity 改用 index digest — verified by **TEST-28.1.2**（真实 workflow_dispatch verify run，§10 记 run id）
- [ ] **AC3**: 既有不退化 + 0 新代码依赖——`verify-image.yml` 既有鉴权 pull + `docker run` + `/v1/health` `contract_version=v1` 步保留；`Dockerfile` 未改；无 Cargo / go.mod 改动；`cargo test --workspace` + `go test ./...` 不受 workflow-only 改动影响 — verified by **TEST-28.1.3** + §10 实测
- [ ] **AC4**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-28.1.4** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-28.1.1 | `release.yml` + setup-qemu-action + `platforms: amd64,arm64`；真实 multi-arch `push:false` build run 证明 arm64 emulation 能构建（amd64+arm64 两 target）+ 既有 tags/cache/push 不退化 | `.github/workflows/release.yml` | Draft |
| TEST-28.1.2 | `verify-image.yml` 未鉴权匿名 pull（logout 后）exit 0（对公开 `:latest`）+ 多架构 manifest assert（imagetools inspect 含 amd64+arm64）+ `:latest` parity 改 index digest | `.github/workflows/verify-image.yml` | Draft |
| TEST-28.1.3 | 既有鉴权 pull + run + `/v1/health` contract_version=v1 不退化 + Dockerfile/Cargo/go.mod 未改 + 0 新代码依赖 | `.github/workflows/verify-image.yml` + 全 workspace | Draft |
| TEST-28.1.4 | D2 lint `--touched origin/master` 0 未标注命中 | `scripts/spec_drift_lint.sh` | Draft |

## 8. Risks

- **R1（高）arm64 multi-arch emulation 构建超时 / 失败**：QEMU 下 Rust `cargo build --release` arm64 估 ≥20 min（task-16.3），可能超 runner 时限或不稳。
  - **缓解**：先 workflow_dispatch `push:false` 试构 arm64 计时；超时则评估 `cache-from/to: gha` 复用 / 拆分 per-arch job；不可行则 amd64 保底 + arm64 `[SPEC-DEFER:phase-future.multi-arch-native-runner]`。stop-condition：arm64 真实构建不过则 AC1 arm64 维度不标 `[x]`（amd64 + 匿名 pull 达成则部分达标，不伪造）。
- **R2（中）匿名 pull 步残留 ambient 凭据**：runner 上 `docker login` 状态 / `~/.docker/config.json` 可能令「匿名」pull 实际带凭据。
  - **缓解**：用显式 `docker logout ghcr.io`（§5.2 (B)）确保无凭据再 pull；对**已知公开**的 `:latest` 验（v0.20.0 已 public，§v0.18.0 回填确认 `:latest`=v0.20.0）。stop-condition：若 logout 后 pull 仍带凭据嫌疑则改用独立 runner job / `docker --config <empty-dir>`。
- **R3（中）index digest parity 改动误伤既有校验**：parity 步从单架构 `docker inspect` 改 `buildx imagetools inspect` 语义/格式不同。
  - **缓解**：parity 改动经真实 workflow_dispatch verify run 验过再标 AC2；既有鉴权 pull + run + /v1/health 步**不动**（仅改 parity 步 + add 两 add-only 步）。stop-condition：parity run 不过则该维度不标 `[x]`。

## 9. Verification Plan

```bash
# 0. 本机 YAML 语法（actionlint 若装；否则 CI 校验权威）
#    actionlint .github/workflows/release.yml .github/workflows/verify-image.yml

# 1. AC1 — multi-arch 构建证明（push:false，不推 GHCR；workflow_dispatch 触发本分支版 release.yml 的 push:false 验证变体，或临时 build job）
#    真实 run id 记入 §10；期望 amd64 + arm64 两 target 构建成功 + arm64 构建耗时记录
gh workflow run release.yml --ref task/task-28.1-multi-arch-image-and-anonymous-pull -f tag=<dry-run-tag>   # 见 §5.2：验证用 push:false 变体，不污染 GHCR

# 2. AC2 — 匿名 pull + 多架构 assert + index parity（对现有公开 :latest）
gh workflow run verify-image.yml -f tag=latest
#    期望：anonymous pull exit 0 / imagetools inspect 含 linux/amd64 + linux/arm64（v0.21.0 release 后）/ parity index digest

# 3. AC3 — 既有不退化（workflow-only 改动不影响 workspace）
cargo test --workspace
go test ./...

# 4. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **outward-facing 红线**：真正推多架构 manifest list 到 GHCR + 移动 `:latest` 在**已授权的 v0.21.0 release**（task-28.4）发生；本 task 验证阶段经 `push:false` 构建证明 + 对现有公开 `:latest` 验匿名 pull，不触发真实 GHCR 推送（ADR-012）。

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Draft（待实施）。
- **完成日期**：（待填）。
- **改动文件**：（待填——预期 `.github/workflows/release.yml` + `.github/workflows/verify-image.yml`）。
- **commit 列表（RED→GREEN）**：（待填——workflow 改动无传统单测 RED；以真实 workflow_dispatch run 为 verified 依据）。
- **§9 Verification 实测结果（ADR-013 真实非合成）**：（待填——multi-arch `push:false` build run id + arm64 构建耗时 + 匿名 pull run id + parity 结论；受阻维度如实记录）。
- **设计取舍**：（待填——见 §5.2 验证策略 + 匿名 pull 二选一）。
- **剩余风险 + 下游影响**：（待填——arm64 emulation 耗时 / 原生 runner 延后；task-28.2 接签名 + SBOM）。
