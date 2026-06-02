# language: en
# Maps to:
#   - docs/specs/phases/phase-28-release-ci-hardening.md
#   - docs/specs/tasks/task-28.1-multi-arch-image-and-anonymous-pull.md
#   - docs/specs/tasks/task-28.2-image-signing-sbom-provenance.md
#   - docs/specs/tasks/task-28.3-ci-strict-lint.md
#   - docs/specs/tasks/task-28.4-closeout-v0.21.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 28 release-ci-hardening。Scenario ID 在各 task spec §7 追踪表映射到具体测试 / 真实 CI run。

Feature: phase-28-release-ci-hardening
  In order to 让发布 / CI 流水线具备可公开验证的分发面、供应链可证明（签名 + SBOM + provenance）、与可阻断的代码质量门
  As Phase 28 内核（multi-arch（arm64 deferred）+ 匿名可拉取守护 + cosign 签名/SBOM/provenance + CI 强 lint + v0.21.0 收口）
  I want verify-image 加未鉴权匿名 pull 守护 + release.yml 加 cosign keyless 签名/SBOM/provenance + ci.yml 加阻断 lint job，且全为 .github/workflows/* + surgical clippy/gofmt 修复、镜像运行时 / 默认构建 0-network/0-dep baseline 不变、不破坏既有 v0.6-v0.20 client、受阻态（arm64 emulation / 真实签名尚未发生 / native-attestation 私有 repo 不可用）如实记录不伪造

  # ---
  # Maps to: docs/specs/tasks/task-28.1-multi-arch-image-and-anonymous-pull.md (TEST-28.1.1/28.1.2)
  Scenario: SCEN-28.1.1 — 对应 AC1（multi-arch manifest list + 未鉴权匿名 pull 守护）
    Given verify-image.yml 加 add-only 未鉴权（logged-out / 清 ghcr token）docker pull 守护步 + release.yml 经 setup-qemu-action + platforms 试推 multi-arch manifest list
    When  跑 verify-image.yml workflow_dispatch 未鉴权 docker pull ghcr 包，并经 QEMU emulation workflow_dispatch 试构 linux/arm64 计时
    Then  未鉴权匿名 pull exit 0（守 v0.10.0 GHCR 初始 PRIVATE → Console team 匿名 403 回归）+ 既有鉴权 pull + /v1/health contract_version=v1 + :latest digest parity 步不退化（TEST-28.1.1，真实 run 26788773926 success）；arm64 QEMU emulation 实测不可行——run 26757640892 在 45 min cancelled（Rust cargo build --release arm64 仍在编 deps，未达可推 manifest 阶段，坐实 R1 emulation 超时）→ release.yml 保 platforms: linux/amd64 单架构（net-zero），arm64 未 ship DEFERRED [SPEC-DEFER:phase-future.multi-arch-native-runner]（amd64 + 匿名 pull 达成 → 部分 ratify，不伪造 arm64 manifest 存在）（TEST-28.1.2）；默认构建 0 新 dep（QEMU 为 CI action 工具）

  # ---
  # Maps to: docs/specs/tasks/task-28.2-image-signing-sbom-provenance.md (TEST-28.2.1/28.2.2)
  Scenario: SCEN-28.2.1 — 对应 AC2（cosign keyless 签名 + cosign attest SBOM + provenance + cosign verify）
    Given release.yml 加 id-token: write + cosign-installer + cosign sign manifest digest + cosign attest SPDX SBOM（anchore/sbom-action syft）+ build-push-action provenance: mode=max + verify-image.yml 加 cosign verify + verify-attestation
    When  push 后取 build-push-action 输出 digest 签（非 tag）+ attest SBOM + 同 run cosign verify / verify-attestation 即时验（机制经 local registry 端到端）
    Then  cosign sign + cosign attest SBOM + cosign verify + cosign verify-attestation 全通过（TEST-28.2.1，机制经 local registry run 26799480280 success VERIFIED）；verify-image.yml cosign verify + verify-attestation 步入门（predicate-type SBOM/provenance 存在性断言，TEST-28.2.2）；GitHub-native attestation（actions/attest-*）先试但在 user-owned 私有 repo 不可用（run 26789731232 failure）→ 切 cosign（= ADR-033 §D2 原始意图），如实记录 [SPEC-DEFER:phase-future.github-native-attestation]；真实 GHCR 镜像签名 / SBOM / attestation 在 v0.21.0 release run（用户授权，本 closeout 之后）兑现——尚未发生，不伪造「真实 GHCR 已签」（ADR-013）；0 新代码依赖（cosign/syft 为 CI action 工具非 Cargo/go.mod direct dep）；签名密钥轮换 [SPEC-DEFER:phase-future.signing-key-management]

  # ---
  # Maps to: docs/specs/tasks/task-28.3-ci-strict-lint.md (TEST-28.3.1/28.3.2)
  Scenario: SCEN-28.3.1 — 对应 AC3（CI 强 lint：clippy -D warnings + gofmt + go vet 阻断门 + 真实存量全修）
    Given ci.yml 加 add-only lint job（cargo clippy --workspace -- -D warnings + gofmt -l + go vet 全阻断）+ 先实测真实存量计数（ADR-013 非合成）
    When  实测 clippy / gofmt / go vet 存量（CI/LF checkout 权威）并据存量决定卡红时机 + 修复触及存量
    Then  真实存量 CI/LF 权威——gofmt 15 real files（local 96 = 15 real + 81 Windows-CRLF false positive，初测误判经 CI 纠正）/ go vet 0 / clippy ~33——全量修复（gofmt -w + strip-pipe；clippy --fix + manual + 2 targeted allow：generated pb/pb_console clippy::all + EventBus::send result_large_err，既有签名不变 surgical）（TEST-28.3.2）；clippy -D warnings exit 0 + cargo test --workspace all pass（core lib 187 passed）+ lint job 入 ci.yml 且既有 cargo-test/go-test/spec-lint 三门不退化（TEST-28.3.1，PR #190 run 26820737785 PASS）；local-CRLF vs CI-LF 存量差异如实记录不伪造为零（ADR-013，surgical 红线不大面积重构）；存量根因清零 [SPEC-DEFER:phase-future.lint-backlog-cleanup]；rustfmt 门 [SPEC-DEFER:phase-future.rustfmt-gate]；golangci-lint [SPEC-DEFER:phase-future.golangci-lint]

  # ---
  # Maps to: docs/specs/tasks/task-28.4-closeout-v0.21.0.md (TEST-28.4.1/28.4.2/28.4.3)
  Scenario: SCEN-28.4.1 — 对应 AC4/AC5（v0.21.0 收口 + smoke v18 + ADR-033 ratify + ADR-007 Amendment）
    Given v0.21.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ scripts/console_smoke.sh v18（发布硬化文档化 step）+ ADR-033（release-ci-hardening）+ ADR-007（minimal-tarball-distribution）
    When  据 task-28.1/28.2/28.3 真实 CI / release run 产物 ratify ADR-033 + smoke v18 跑发布硬化文档化断言 + 记部署发布面扩展 Amendment
    Then  ADR-033 据真实非合成验证 Proposed→Accepted——D1 PARTIAL（anon-pull 真实 / arm64 DEFERRED）/ D2 Accepted（cosign 机制 VERIFIED；真实 GHCR 签名 at release run）/ D3 Accepted（lint 门 + 存量全修）/ D4 Accepted（运行时 / 0-dep / 0-network baseline 不变）（TEST-28.4.2）；ADR-007 add-only Amendment（部署发布面扩展到 multi-arch 签名 OCI + SBOM，arm64 deferred，不溯改正文 D1-D5）（TEST-28.4.3）；smoke 既有 v13-v17 step 不退化 + bash -n exit 0（TEST-28.4.1）；phase-28 §6 AC1-5 全 met（AC1 arm64 维度 DEFERRED 如实标注）；ADR-014 D1-D5（第十九次激活，history Phase 1-27 不溯改）全通过；真实 GHCR 镜像签名 / multi-arch manifest 属 release run 时刻（post-tag-push backfill），不在 closeout / smoke 内伪造
