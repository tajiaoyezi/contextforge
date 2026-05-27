# Task `16.3`: `ghcr-image-push-ci — .github/workflows/release.yml ghcr.io image push (v* tag trigger) + .github/workflows/ci.yml PR/push test gate`

**Status**: Ready

**Priority**: P3
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 16 (v0.9.0-backlog-completion)
**Dependencies**: 既有 `Dockerfile` multi-stage build (v0.7.2 ship)

## 1. Background

ContextForge-Console PR #91/#93 backlog 列 P3 #8：

> v0.7.0 ship Dockerfile + `docker build` 可用，但用户须 `git clone https://github.com/tajiaoyezi/contextforge && cd contextforge && docker build .` —— 不友好 + 重复编译 cargo + go 长达 5-10 分钟。期望：`docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0` 一行命令拉到镜像。

既有 v0.8.0 状态：
- `Dockerfile` multi-stage (rust:1.93-slim-bookworm + golang:1.26-bookworm → debian:bookworm-slim)，task-10.6 ship + v0.7.1/v0.7.2 修
- `.github/workflows/` 目录**不存在** — 仓库无任何 CI/CD workflow
- 既有 release flow 是手动：tag 推 → GitHub Release 页面创建 → 用户本地 build；无自动 image artifact

**实施策略**：

- 新建 `.github/workflows/release.yml`（**核心交付物**）：
  - Trigger：`push.tags: ['v*']`（自动）+ `workflow_dispatch` 手动 override（参 `tag` 指定 ref；用于 hotfix retag）
  - Job `build-and-push`：`runs-on: ubuntu-22.04` + checkout → `docker/setup-buildx-action@v3` → `docker/login-action@v3` (ghcr.io / `${{ github.actor }}` / `${{ secrets.GITHUB_TOKEN }}`) → `docker/build-push-action@v5` push 双 tag（`{tag}` + `latest`）
  - Image name：`ghcr.io/${{ github.repository_owner }}/contextforge-daemon`
  - Permissions：`packages: write` + `contents: read`（不用 `id-token: write` — 不签名 v0.9）
  - Platform：linux/amd64 only v0.9（arm64 留 [SPEC-DEFER:phase-future.multi-arch-image]）
- 新建 `.github/workflows/ci.yml`（**add-on**）：
  - Trigger：`pull_request` + `push.branches: [master]`
  - Job `cargo-test`：rust:1.93 + `cargo test --workspace`
  - Job `go-test`：golang:1.26 + `go test ./...`
  - Job `spec-lint`：ubuntu + `bash scripts/spec_drift_lint.sh --touched origin/master`
  - 三 job 并行 + fail-fast: false（独立报告）
  - 不在本 phase 引入 cargo clippy / gofmt strict（接受 lint 不卡 CI；future 强制留 [SPEC-DEFER:phase-future.ci-strict-lint]）
- ADR-014 D2 lint：本 task spec anti-pattern 全部标注

## 2. Goal

新建 `.github/workflows/release.yml` + `.github/workflows/ci.yml`；v0.9.0-rc1 tag push 实测 ghcr.io 镜像 pullable；`docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0-rc1` + `docker run` 容器健康；ci.yml PR 触发 cargo + go + lint 全 PASS；既有 cargo + go test baseline 不退化；workflow yml syntax 验证（actionlint OR `gh workflow run --dry-run`）通过。

## 3. Scope

### In Scope

- **新建 `.github/workflows/release.yml`** (主交付物，~ 80 lines)：
  ```yaml
  name: Release — push image to GHCR

  on:
    push:
      tags:
        - 'v*'
    workflow_dispatch:
      inputs:
        tag:
          description: 'tag to build/push (e.g. v0.9.0-rc1)'
          required: true
          type: string

  permissions:
    contents: read
    packages: write

  jobs:
    build-and-push:
      runs-on: ubuntu-22.04
      env:
        IMAGE_NAME: ghcr.io/${{ github.repository_owner }}/contextforge-daemon
      steps:
        - name: Determine ref
          id: ref
          run: |
            if [ "${{ github.event_name }}" = "workflow_dispatch" ]; then
              echo "tag=${{ inputs.tag }}" >> $GITHUB_OUTPUT
              echo "ref=refs/tags/${{ inputs.tag }}" >> $GITHUB_OUTPUT
            else
              echo "tag=${GITHUB_REF##refs/tags/}" >> $GITHUB_OUTPUT
              echo "ref=${{ github.ref }}" >> $GITHUB_OUTPUT
            fi

        - name: Checkout
          uses: actions/checkout@v4
          with:
            ref: ${{ steps.ref.outputs.ref }}

        - name: Set up Docker Buildx
          uses: docker/setup-buildx-action@v3

        - name: Log in to GHCR
          uses: docker/login-action@v3
          with:
            registry: ghcr.io
            username: ${{ github.actor }}
            password: ${{ secrets.GITHUB_TOKEN }}

        - name: Build and push
          uses: docker/build-push-action@v5
          with:
            context: .
            file: ./Dockerfile
            platforms: linux/amd64
            push: true
            tags: |
              ${{ env.IMAGE_NAME }}:${{ steps.ref.outputs.tag }}
              ${{ env.IMAGE_NAME }}:latest
            cache-from: type=gha
            cache-to: type=gha,mode=max

        - name: Image summary
          run: |
            echo "✅ Pushed:" >> $GITHUB_STEP_SUMMARY
            echo "  - ${{ env.IMAGE_NAME }}:${{ steps.ref.outputs.tag }}" >> $GITHUB_STEP_SUMMARY
            echo "  - ${{ env.IMAGE_NAME }}:latest" >> $GITHUB_STEP_SUMMARY
            echo "" >> $GITHUB_STEP_SUMMARY
            echo "Pull command:" >> $GITHUB_STEP_SUMMARY
            echo '  `docker pull ghcr.io/${{ github.repository_owner }}/contextforge-daemon:${{ steps.ref.outputs.tag }}`' >> $GITHUB_STEP_SUMMARY
  ```

- **新建 `.github/workflows/ci.yml`** (add-on，~ 70 lines)：
  ```yaml
  name: CI — test + lint

  on:
    pull_request:
      branches: [master]
    push:
      branches: [master]

  permissions:
    contents: read

  jobs:
    cargo-test:
      runs-on: ubuntu-22.04
      timeout-minutes: 30
      steps:
        - uses: actions/checkout@v4
        - name: Set up Rust
          uses: dtolnay/rust-toolchain@stable
          with:
            toolchain: '1.93'
        - name: Cache cargo
          uses: actions/cache@v4
          with:
            path: |
              ~/.cargo/registry
              ~/.cargo/git
              target
            key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        - name: cargo test
          run: cargo test --workspace

    go-test:
      runs-on: ubuntu-22.04
      timeout-minutes: 20
      steps:
        - uses: actions/checkout@v4
        - name: Set up Go
          uses: actions/setup-go@v5
          with:
            go-version: '1.26'
            cache: true
        - name: go test
          run: go test ./...

    spec-lint:
      runs-on: ubuntu-22.04
      timeout-minutes: 5
      steps:
        - uses: actions/checkout@v4
          with:
            fetch-depth: 0  # for git diff against origin/master
        - name: spec drift lint --touched
          run: bash scripts/spec_drift_lint.sh --touched origin/master
  ```

- **不修改 Dockerfile**：复用 v0.7.2 既有 multi-stage build；release.yml 直接调 `docker build .`

- **GHCR 包 visibility**：默认 private（GitHub Packages 创建时跟仓库 visibility）；ContextForge 仓库是 public → 镜像首次 push 后须**手动在 GitHub UI 切到 public**（一次性 setup；workflow 无法自动；记录到 task §10 完工备忘）

- **依赖管理**：
  - 不引入新依赖 — 全部 GitHub-hosted action（docker/login@v3 / docker/setup-buildx@v3 / docker/build-push@v5 / actions/checkout@v4 / actions/cache@v4）
  - action 版本固定（不用 `@main`）— R7 dep gate 沿用

- **测试 / 验证**：
  - workflow yml syntax：使用 `actionlint` (如本地有) OR `gh workflow run release.yml --ref master -f tag=v0.9.0-rc1` 触发 workflow_dispatch 实测
  - 端到端 verify：
    1. 推 `v0.9.0-rc1` annotated tag → workflow 自动触发
    2. workflow run 完毕（≤ 10 min）
    3. `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0-rc1` 拉取成功
    4. `docker run ... :v0.9.0-rc1` 容器 healthcheck OK
    5. `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:latest` 拿到 v0.9.0-rc1

- **task spec §6 / §7 / §10 / Status 推进**：完工时按 standard.md §8.3 6 项 schema 回填

### Out Of Scope

- **multi-arch (linux/arm64)** [SPEC-DEFER:phase-future.multi-arch-image]：v0.9 仅 linux/amd64；arm64 build 时长 ≥ 20 min + apple silicon 用户量再扩
- **镜像签名 (cosign) / SBOM** [SPEC-DEFER:phase-future.image-signing-and-sbom]：v0.9 仅 build + push；签名 + SBOM 留 v1.x
- **release notes 自动生成**：v0.9 仍手动 `gh release create` (E5 release docs PR 后)；自动 release-please 留 [SPEC-DEFER:phase-future.release-please-automation]
- **release.yml 触发 GitHub Release 创建**：v0.9 仅 push image；Release 页面创建独立步骤；workflow 自动 create release 留 [SPEC-DEFER:phase-future.release-auto-create]
- **CI 强 lint (`cargo clippy -- -D warnings` / `gofmt -l` 报错卡 CI)** [SPEC-DEFER:phase-future.ci-strict-lint]：v0.9 ci.yml 仅 cargo test + go test + spec_drift_lint；clippy/gofmt 强制留 v1.x
- **CI 跨 OS（Windows / macOS）**：v0.9 仅 ubuntu-22.04；跨 OS 留 [SPEC-DEFER:phase-future.ci-multi-os]
- **CI Console smoke (DOCKER_SMOKE=1)**：v0.9 ci.yml 不跑 docker daemon-level smoke（CI runner 没 docker daemon by default）；留 [SPEC-DEFER:phase-future.ci-docker-smoke]
- **GitHub Container Registry retention policy**（旧 tag 自动清理）：v0.9 接受手动 / GitHub 默认；自动清理留 [SPEC-DEFER:phase-future.ghcr-retention-policy]
- **release.yml dry-run mode** (workflow_dispatch flag `--dry-run`)：v0.9 不实施；future 加 [SPEC-DEFER:phase-future.release-dry-run]

## 4. Users / Actors

- **end users**（最大受益方）：`docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0` 一行可用
- **CI/CD 用户**：ci.yml PR 触发自动 test gate；PR check 失败 = 拒合
- **release manager**（主 agent / 项目 owner）：v* tag push 后自动 ship image；不需手动 docker push

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-16-v0.9.0-backlog-completion.md` §3 / §6 AC3
- `Dockerfile`（v0.7.2 既有 multi-stage build；本 task 不动）
- GitHub Actions docs: https://docs.github.com/en/actions
- docker/build-push-action v5 docs: https://github.com/docker/build-push-action

### 5.2 Imports

- **GitHub Actions**: actions/checkout@v4 / docker/setup-buildx-action@v3 / docker/login-action@v3 / docker/build-push-action@v5 / dtolnay/rust-toolchain@stable / actions/setup-go@v5 / actions/cache@v4
- **不引入新本地依赖**（cargo / go / docker 都在 GitHub runner image 内）

### 5.3 GHCR 权限模型

- `GITHUB_TOKEN` 自动注入（每个 workflow job 短时令牌）+ workflow `permissions: packages: write` 显式声明 = 推 GHCR 包到本仓库 namespace 的最小权限
- 不用 PAT secrets — 避免长期 token 暴露风险
- 镜像 visibility：包首次 push 后跟随仓库 visibility（public repo → public package after manual UI 切换）；私仓 → 私包

## 6. Acceptance Criteria

- [ ] AC1：`.github/workflows/release.yml` syntax 验证（actionlint OR yamllint）通过；workflow_dispatch 手动 trigger `gh workflow run release.yml -f tag=v0.9.0-rc1` 成功启动 — **verified by `gh workflow view release.yml` 显示 enabled + 1 successful run**
- [ ] AC2：v0.9.0-rc1 annotated tag push 后 workflow 自动触发 + 完毕；GitHub Actions 页面显示 `build-and-push` job ✅ — **verified by `gh run list --workflow=release.yml --limit 1` 显示 success status + ≤ 10 min completion**
- [ ] AC3：`docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0-rc1` 拉取成功；`docker run --rm -p 48181:48181 -e CONSOLE_API_FALLBACK_INMEM=1 ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0-rc1` 容器健康 + `curl http://localhost:48181/v1/health` 返 200 — **verified by E5 release docs PR 内手动 verify + docker pull stdout 含 digest sha256:* + docker run + curl 200 stdout 落 PR body**
- [ ] AC4：`docker pull ghcr.io/tajiaoyezi/contextforge-daemon:latest` 拿到 v0.9.0-rc1（同 digest）— **verified by `docker images --digests | grep contextforge-daemon` 两 tag digest 一致**
- [ ] AC5：`.github/workflows/ci.yml` PR 触发 3 job (cargo-test / go-test / spec-lint) 全 PASS — **verified by 本 phase E1 spec PR 自身的 CI 报告显示 3 job ✅**
- [ ] AC6：既有 `docker build .` 本地 build 不退化（Dockerfile 不动 → build 流程对等）— **verified by closeout PR body `docker build .` 实测**

## 7. 追踪表

| Anchor | 描述 | 落地位置 | Status |
|---|---|---|---|
| AC1 | release.yml syntax | .github/workflows/release.yml + actionlint | Ready |
| AC2 | tag push auto trigger | release.yml + gh CLI 实测 | Ready |
| AC3 | docker pull v0.9.0-rc1 | E5 release docs PR 手动 verify | Ready |
| AC4 | docker pull latest 同 digest | docker images verify | Ready |
| AC5 | ci.yml 3 job pass | .github/workflows/ci.yml + PR check | Ready |
| AC6 | docker build 不退化 | closeout PR docker build 实测 | Ready |

## 8. Risks

- **GHCR 镜像首次 visibility 私有**：默认私包；手动 UI 切到 public 一次性；记录到 §10 完工备忘 + docs/deploy/production.md
- **workflow GITHUB_TOKEN scope 不够**：`permissions: packages: write` 显式声明已足；如撞 403 → fall back PAT secret `GHCR_PAT` (low confidence；预测不需) [SPEC-DEFER:phase-future.ghcr-pat-fallback]
- **build cache miss 首次 build 时长 ≥ 10 min**：cargo + go 全编译；GHA cache `cache-from: gha + cache-to: gha,mode=max` 第二次起 ≤ 3 min
- **single arch linux/amd64 限制**：apple silicon 用户 `docker run` 走 emulation 慢；接受 v0.9；future arm64 见 [SPEC-DEFER:phase-future.multi-arch-image]
- **release.yml workflow_dispatch tag 输入校验**：用户传不存在 tag → checkout fails → workflow red；接受 — actionable error message；future 加预检 [SPEC-DEFER:phase-future.release-tag-validation]
- **ci.yml cargo test 时长 ≥ 15 min**：worst case；cache 命中后 ≤ 5 min；timeout 30 min；如频繁超时 → split per-crate test [SPEC-DEFER:phase-future.ci-cargo-split]
- **ci.yml spec_drift_lint --touched origin/master 需 fetch-depth: 0**：默认 GHA checkout shallow → git diff base 不可达；workflow 内显式 `fetch-depth: 0` 解决
- **关联 [ADR-018](../../decisions/adr-018-fallback-inmem-default-reversal.md)**：AC3 验证 docker run 带 `-e CONSOLE_API_FALLBACK_INMEM=1` 显式 opt-in；默认 fallback deny → curl /v1/health 返 503 是 expected 行为；AC3 测试用 opt-in 模式

## 9. Verification Plan

- **install**: 无（GitHub Actions runner 已提供 docker + cargo + go）
- **lint**: actionlint (local) OR `gh workflow view release.yml` 显示无 schema error
- **typecheck**: 不适用（yml）
- **unit-test**: 不适用（workflow 自身是 IaC，不是代码）
- **integration**: `gh workflow run release.yml -f tag=v0.9.0-rc1` 触发实测 + 检查 GitHub Actions 页面 run status
- **e2e**: `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0-rc1` + `docker run` + `curl /v1/health` 200
- **build**: workflow 内 docker build (cache enabled)
- **runtime-smoke**: 见 §9 e2e
- **coverage**: 不适用
- **manual**: AC3 / AC4 / AC6 涉及手动 verify；记录到 E5 release docs PR body

## 10. Completion Notes

(待 Done 时回填 — standard.md §8.3 6 项 schema)
