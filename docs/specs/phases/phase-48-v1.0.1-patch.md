# Phase 48 · v1.0.1-patch

**Status**: Ready

> Phase Spec（s2v full-standard §8.2）。本 phase 是 **v1.0 收口后审查发现的 4 个残留的 patch 修复**。v1.0.0 ship 后做了一次全面审查（grounding 全维度），发现 1 个 P0 代码缺陷 + 3 个 P1-P3 文档过时：(P0) **CLI `version` 字符串过时**——`internal/cli/cli.go:31` `var Version = "0.38.0-dev"` 且 Dockerfile/release.yml 都无 ldflags 注入，导致 v1.0.0 镜像里 `contextforge version` 打印 `0.38.0-dev` 而非 `1.0.0`（D2 API/CLI 冻结的直接缺陷）；(P1) docs/decisions/README.md ADR-050 状态漏更新（仍 Proposed，应 Accepted）；(P2) README Latest 段描述残留 v0.39.0"第二步"措辞；(P3) contextforge.example.toml header 版本 v0.38.0 过时。
>
> **修复性质**：P0 是真实代码 + CI 改动（CLI version 字符串默认值 + Dockerfile ARG/ldflags 注入）；P1-P3 是纯文档。**0 新 dep / 0 migration / 0 proto / 0 schema change**。默认行为 / 既有契约 / 三门不退化（仅 CLI version 输出从错误变正确）。

> **入读顺序**：本 phase spec → v1.0 收口审查残留清单 → 源码锚点（`internal/cli/cli.go:31` Version 变量 + `Dockerfile:43` go build 无 ldflags + `.github/workflows/release.yml` build-push step + `docs/decisions/README.md:60` ADR-050 行 + `README.md:48` Latest 段 + `contextforge.example.toml:1` header）→ ADR-014（D1-D5，第三十九次激活）。

## 1. 阶段目标

v1.0.0 ship 后审查发现的 4 个残留修复。P0 是 D2 API/CLI 冻结的真实缺陷（version 字符串），P1-P3 是文档过时。本 patch 让 v1.0.1 镜像的 `contextforge version` 正确打印 `1.0.1`（或 release tag），并清文档残留。

**具体 exit criteria（§6 AC）**：
1. **P0 CLI version 字符串修复**：cli.go Version 默认值 `"0.38.0-dev"` → `"1.0.1-dev"` + Dockerfile ARG + ldflags 注入（build-time 读 ARG VERSION → `-ldflags -X`）+ release.yml build-push 传 VERSION tag — verified by **TEST-48.1.1**（AC1）
2. **P1-P3 文档残留清理**：docs/decisions/README.md ADR-050 Accepted + README Latest 段 v1.0 描述 + example.toml header v1.0.1 — verified by **TEST-48.1.2**（AC2）
3. **v1.0.1 closeout**：smoke v37→v38[57/57] + release docs + roadmap/adapter — verified by **TEST-48.1.3**（AC3）
4. ADR-014 D1-D5（第三十九次激活）全通过（AC4）

**版本号**：v1.0.1（Phase 48，承 v1.0.0），theme v1.0.1-patch。**patch release**（v1.0.0 收口审查残留修复；P0 CLI version 字符串是 D2 缺陷修复，非 breaking）。

## 2. 业务价值

v1.0 收口审查残留修复——让 v1.0.1 镜像的 `contextforge version` 正确报版本（D2 API/CLI 冻结承诺兑现），并清文档过时（ADR-050 状态 / README 描述 / example.toml header）。

### 48.1 v1.0.1-patch（🟢 代码 + CI + 文档）
单聚焦 task：(1) cli.go Version 默认值更新 + Dockerfile ARG VERSION + ldflags 注入 + release.yml build-push 传 tag；(2) docs/decisions/README.md ADR-050 Accepted；(3) README Latest 段描述 v1.0；(4) example.toml header v1.0.1；(5) smoke v38[57/57] + release docs。

**不在本 phase 范围**：任何新功能 / multi-user/认证/自动更新/arm64 native（v2.0）/ 重构既有代码（本 patch 仅修 version 字符串 + 文档过时）。

## 3. 涉及模块

- **48.1**：
  - `internal/cli/cli.go`（Version 默认值 `"0.38.0-dev"` → `"1.0.1-dev"`）
  - `Dockerfile`（go-build stage 加 `ARG VERSION` + `-ldflags "-X github.com/tajiaoyezi/contextforge/internal/cli.Version=${VERSION}"`）
  - `.github/workflows/release.yml`（build-push step 加 `build-args: VERSION=${{ steps.ref.outputs.tag }}`）
  - `internal/cli/cli_test.go` 或 smoke_syntax_test.go（TEST-48.1.1 Version 默认值 + Dockerfile ldflags grep）
  - `docs/decisions/README.md`（ADR-050 行 Accepted）
  - `README.md`（Latest 段描述 v1.0）
  - `contextforge.example.toml`（header v1.0.1）
  - `scripts/console_smoke.sh`（v37→v38[57/57]）+ `internal/cli/smoke_syntax_test.go`（TestTask481）
  - `docs/releases/v1.0.1-evidence.md` + `v1.0.1-artifacts.md`（新增）+ `docs/roadmap.md`（§3.30）+ `docs/s2v-adapter.md`
- BDD：`test/features/phase-48-v1.0.1-patch.feature`

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 48.1 | v1.0.1-patch：P0 CLI version ldflags + P1-P3 文档 + smoke v38[57/57] + closeout | `../tasks/task-48.1-v1.0.1-patch.md` |

## 5. 依赖关系

- 48.1 dep v1.0.0（已 ship）+ v1.0 收口审查残留清单。
- ADR-050（v1.0.0 已 Accepted，本 patch 不动 ADR-050 D-body）/ ADR-014（第三十九次激活）/ ADR-004/008（守 0 dep baseline）守线。

## 6. 阶段级验收标准 + 端到端 smoke

- [ ] **AC1**（P0 CLI version 字符串修复 🟢 代码 + CI）: cli.go Version 默认值 `"1.0.1-dev"` + Dockerfile ARG VERSION + ldflags 注入 + release.yml build-args VERSION — verified by **TEST-48.1.1**（cli.go grep + Dockerfile ldflags grep + release.yml build-args grep）
- [ ] **AC2**（P1-P3 文档残留清理 🟢 纯文档）: docs/decisions/README.md ADR-050 Accepted + README Latest 段 v1.0 描述 + example.toml header v1.0.1 — verified by **TEST-48.1.2**
- [ ] **AC3**（v1.0.1 closeout）: smoke v38[57/57] + release docs + roadmap/adapter — verified by **TEST-48.1.3**
- [ ] **AC4**（ADR-014 cross-validation gate）: D1-D5（第三十九次激活）— verified by task-48.1 PR body + LAST TEST

## 7. 阶段级风险

- **R1（低）Dockerfile ldflags 注入首次实践**：本仓库 Dockerfile 从未用 ldflags 注入 version。首次接入可能 build 失败。
  - **缓解**：严格按 Go ldflags 标准模式 `-X github.com/tajiaoyezi/contextforge/internal/cli.Version=${VERSION}`；CI build-and-push job 会实测；若失败 → v1.0.2 修或 honest-defer ldflags 到后续（cli.go 默认值已对）。stop-condition：ldflags 路径错导致 build 失败 → 改 cli.go 默认值兜底（至少 `contextforge version` 不再报 0.38.0-dev）。

## 8. Definition of Done

- 1 task spec 顶部 Status Done；§6 AC1-4 全 [x]；smoke 全 PASS。
- release：v1.0.1-{evidence,artifacts}.md + RELEASE_NOTES v1.0.1 段 + CHANGELOG [v1.0.1] + roadmap §3.30 + adapter。
- smoke：v38[57/57] + TestTask481。
- follow-up：v2.0 路线（multi-user/认证/自动更新/arm64 native + large-corpus benchmarks）。
